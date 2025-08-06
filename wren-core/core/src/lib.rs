pub mod logical_plan;
pub mod mdl;

pub use datafusion::arrow::*;
pub use datafusion::error::DataFusionError;
pub use datafusion::logical_expr::{AggregateUDF, ScalarUDF, WindowUDF};
pub use datafusion::prelude::*;
pub use datafusion::sql::sqlparser::*;
pub use logical_plan::error::WrenError;
pub use mdl::AnalyzedWrenMDL;

use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use wren_core_base::mdl::manifest::{Manifest, DataSource};
use mdl::transform_sql_with_ctx;
use mdl::context::Mode;
use datafusion::sql::parser::DFParser;
use datafusion::sql::sqlparser::dialect::GenericDialect;
use serde_json;

fn parse_dialect_internal(dialect_str: &str) -> Result<DataSource, String> {
    match dialect_str.to_lowercase().as_str() {
        "bigquery" => Ok(DataSource::BigQuery),
        "mysql" => Ok(DataSource::MySQL),
        "postgres" | "postgresql" => Ok(DataSource::Postgres),
        "snowflake" => Ok(DataSource::Snowflake),
        "mssql" | "sqlserver" => Ok(DataSource::MSSQL),
        "trino" => Ok(DataSource::Trino),
        "clickhouse" => Ok(DataSource::Clickhouse),
        "canner" => Ok(DataSource::Canner),
        "datafusion" => Ok(DataSource::Datafusion),
        "duckdb" => Ok(DataSource::DuckDB),
        "oracle" => Ok(DataSource::Oracle),
        _ => Err(format!("Unsupported dialect: {}. Supported dialects: bigquery, mysql, postgres, snowflake, mssql, trino, clickhouse, canner, datafusion, duckdb, oracle", dialect_str)),
    }
}

async fn transform_sql_internal(
    mdl_json: &str,
    sql: &str,
    dialect: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Parse dialect parameter
    let data_source = parse_dialect_internal(dialect)?;

    // Parse MDL from JSON string
    let mut manifest: Manifest = serde_json::from_str(mdl_json)?;
    
    // Override the data source with the dialect parameter
    manifest.data_source = Some(data_source);

    // Create empty session properties
    let session_properties = Arc::new(HashMap::new());

    // Create a default SessionContext
    let ctx = datafusion::execution::context::SessionContext::new();

    // Analyze the MDL
    let analyzed_mdl = Arc::new(AnalyzedWrenMDL::analyze(
        manifest,
        Arc::clone(&session_properties),
        Mode::Unparse,
    )?);

    // Transform the SQL
    let transformed_sql = transform_sql_with_ctx(
        &ctx,
        analyzed_mdl,
        &[],  // No remote functions
        session_properties,
        sql,
    )
    .await?;

    Ok(transformed_sql)
}

/// FFI function to transform SQL using Wren MDL
/// Returns a C string that must be freed by the caller using free_string
#[no_mangle]
pub extern "C" fn wren_transform_sql(
    mdl_json: *const c_char,
    sql: *const c_char,
    dialect: *const c_char,
) -> *mut c_char {
    if mdl_json.is_null() || sql.is_null() || dialect.is_null() {
        return std::ptr::null_mut();
    }

    let mdl_json_str = match unsafe { CStr::from_ptr(mdl_json) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let sql_str = match unsafe { CStr::from_ptr(sql) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let dialect_str = match unsafe { CStr::from_ptr(dialect) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Create a new Tokio runtime for this operation
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = rt.block_on(async {
        transform_sql_internal(mdl_json_str, sql_str, dialect_str).await
    });

    match result {
        Ok(transformed_sql) => {
            match CString::new(transformed_sql) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// FFI function to free strings returned by wren_transform_sql
#[no_mangle]
pub extern "C" fn wren_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

/// Validates SQL query syntax and optionally checks if referenced tables exist in the schema
/// schema_json should be a JSON array of table objects with catalog, schema, and tableName fields
async fn validate_sql_with_schema_internal(
    schema_json: Option<&str>,
    sql: &str,
    dialect_str: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // First validate SQL syntax using the appropriate dialect
    use datafusion::sql::sqlparser::dialect::{MySqlDialect, PostgreSqlDialect, SnowflakeDialect, MsSqlDialect, ClickHouseDialect, DuckDbDialect};
    
    let dialect_obj: Box<dyn datafusion::sql::sqlparser::dialect::Dialect> = match dialect_str.to_lowercase().as_str() {
        "mysql" => Box::new(MySqlDialect {}),
        "postgres" | "postgresql" => Box::new(PostgreSqlDialect {}),
        "snowflake" => Box::new(SnowflakeDialect {}),
        "mssql" | "sqlserver" => Box::new(MsSqlDialect {}),
        "clickhouse" => Box::new(ClickHouseDialect {}),
        "duckdb" => Box::new(DuckDbDialect {}),
        _ => Box::new(GenericDialect {}),
    };
    
    // Parse SQL to check syntax
    if let Err(e) = DFParser::parse_sql_with_dialect(sql, dialect_obj.as_ref()) {
        return Ok(serde_json::json!({
            "valid": false,
            "error": format!("SQL syntax error: {}", e),
            "error_type": "syntax"
        }).to_string());
    }

    // If no schema provided, only syntax validation is performed
    if schema_json.is_none() {
        return Ok(serde_json::json!({
            "valid": true,
            "message": "SQL syntax is valid"
        }).to_string());
    }

    // Parse schema tables and extract referenced tables from SQL
    let schema_str = schema_json.unwrap();
    
    // Parse schema JSON (array of table objects)
    let schema_tables: Vec<serde_json::Value> = match serde_json::from_str(schema_str) {
        Ok(tables) => tables,
        Err(e) => {
            return Ok(serde_json::json!({
                "valid": false,
                "error": format!("Invalid schema JSON: {}", e),
                "error_type": "schema"
            }).to_string());
        }
    };
    
    // For simpler validation, we'll use DataFusion's SQL parser to create a logical plan
    // This will automatically validate both tables and columns
    use datafusion::execution::context::SessionContext;
    use datafusion::datasource::empty::EmptyTable;
    use datafusion::arrow::datatypes::{Schema as ArrowSchema, Field as ArrowField, DataType};
    
    // Create a SessionContext and register tables with their schemas
    let ctx = SessionContext::new();
    
    for table in &schema_tables {
        if let Some(table_name) = table.get("tableName").and_then(|v| v.as_str()) {
            // Create Arrow schema from table columns
            let mut fields = Vec::new();
            if let Some(columns) = table.get("columns").and_then(|v| v.as_array()) {
                for column in columns {
                    if let Some(col_name) = column.get("name").and_then(|v| v.as_str()) {
                        // Map column types to Arrow DataType (simplified)
                        let data_type = if let Some(col_type) = column.get("type").and_then(|v| v.as_str()) {
                            match col_type.to_lowercase().as_str() {
                                "int" | "integer" | "int4" => DataType::Int32,
                                "bigint" | "int8" => DataType::Int64,
                                "smallint" | "int2" => DataType::Int16,
                                "float" | "real" | "float4" => DataType::Float32,
                                "double" | "float8" | "double precision" => DataType::Float64,
                                "boolean" | "bool" => DataType::Boolean,
                                "date" => DataType::Date32,
                                "timestamp" | "datetime" => DataType::Timestamp(datafusion::arrow::datatypes::TimeUnit::Microsecond, None),
                                "decimal" | "numeric" => DataType::Decimal128(38, 10),
                                _ => DataType::Utf8, // Default to string
                            }
                        } else {
                            DataType::Utf8
                        };
                        
                        let nullable = column.get("nullable")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        
                        fields.push(ArrowField::new(col_name, data_type, nullable));
                    }
                }
            }
            
            // If no columns defined, create a dummy schema with one column
            if fields.is_empty() {
                fields.push(ArrowField::new("dummy", DataType::Utf8, true));
            }
            
            let arrow_schema = Arc::new(ArrowSchema::new(fields.clone()));
            let empty_table = EmptyTable::new(arrow_schema);
            
            // Register the table
            let _ = ctx.register_table(table_name, Arc::new(empty_table));
            
            // Also register with schema prefix if available
            if let Some(schema) = table.get("schema").and_then(|v| v.as_str()) {
                let qualified_name = format!("{}.{}", schema, table_name);
                let arrow_schema = Arc::new(ArrowSchema::new(fields));
                let empty_table = EmptyTable::new(arrow_schema);
                let _ = ctx.register_table(&qualified_name, Arc::new(empty_table));
            }
        }
    }
    
    // Now try to create a logical plan - this will validate tables and columns
    match ctx.state().create_logical_plan(sql).await {
        Ok(_) => {
            Ok(serde_json::json!({
                "valid": true,
                "message": "SQL is valid - syntax correct, all tables and columns exist"
            }).to_string())
        }
        Err(e) => {
            let error_str = e.to_string();
            
            // Determine error type from error message
            let error_type = if error_str.contains("table") && (error_str.contains("not found") || error_str.contains("doesn't exist")) {
                "table_not_found"
            } else if error_str.contains("No field named") || (error_str.contains("column") && (error_str.contains("not found") || error_str.contains("doesn't exist"))) {
                "column_not_found"
            } else if error_str.contains("ambiguous") {
                "ambiguous_reference"
            } else {
                "semantic"
            };
            
            Ok(serde_json::json!({
                "valid": false,
                "error": error_str,
                "error_type": error_type
            }).to_string())
        }
    }
}

/// FFI function to validate SQL query for semantic model
/// Returns a JSON string with validation results
/// If schema_json is null, only syntax validation is performed
/// schema_json should be a JSON array of table objects
/// Returns null on internal error
#[no_mangle]
pub extern "C" fn wren_validate_sql(
    schema_json: *const c_char,
    sql: *const c_char,
    dialect: *const c_char,
) -> *mut c_char {
    if sql.is_null() || dialect.is_null() {
        return std::ptr::null_mut();
    }

    let schema_json_str = if schema_json.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(schema_json) }.to_str() {
            Ok(s) => Some(s),
            Err(_) => return std::ptr::null_mut(),
        }
    };

    let sql_str = match unsafe { CStr::from_ptr(sql) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    let dialect_str = match unsafe { CStr::from_ptr(dialect) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };

    // Create a new Tokio runtime for this operation
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };

    let result = rt.block_on(async {
        validate_sql_with_schema_internal(schema_json_str, sql_str, dialect_str).await
    });

    match result {
        Ok(validation_result) => {
            match CString::new(validation_result) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(_) => std::ptr::null_mut(),
    }
}

/// Extracts the output schema from a SQL query
/// Returns a JSON string with the schema information including column names and types
async fn extract_sql_output_schema_internal(
    schema_json: Option<&str>,
    sql: &str,
    _dialect_str: &str,
    suggested_table_name: Option<&str>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    use datafusion::execution::context::SessionContext;
    use datafusion::datasource::empty::EmptyTable;
    use datafusion::arrow::datatypes::{Schema as ArrowSchema, Field as ArrowField, DataType};
    
    // Create a SessionContext
    let ctx = SessionContext::new();
    
    // If schema is provided, register tables
    if let Some(schema_str) = schema_json {
        let schema_tables: Vec<serde_json::Value> = serde_json::from_str(schema_str)?;
        
        for table in &schema_tables {
            if let Some(table_name) = table.get("tableName").and_then(|v| v.as_str()) {
                // Create Arrow schema from table columns
                let mut fields = Vec::new();
                if let Some(columns) = table.get("columns").and_then(|v| v.as_array()) {
                    for column in columns {
                        if let Some(col_name) = column.get("name").and_then(|v| v.as_str()) {
                            // Map column types to Arrow DataType
                            let data_type = if let Some(col_type) = column.get("type").and_then(|v| v.as_str()) {
                                match col_type.to_lowercase().as_str() {
                                    "int" | "integer" | "int4" => DataType::Int32,
                                    "bigint" | "int8" => DataType::Int64,
                                    "smallint" | "int2" => DataType::Int16,
                                    "float" | "real" | "float4" => DataType::Float32,
                                    "double" | "float8" | "double precision" => DataType::Float64,
                                    "boolean" | "bool" => DataType::Boolean,
                                    "date" => DataType::Date32,
                                    "timestamp" | "datetime" => DataType::Timestamp(datafusion::arrow::datatypes::TimeUnit::Microsecond, None),
                                    "decimal" | "numeric" => DataType::Decimal128(38, 10),
                                    "varchar" | "text" | "string" => DataType::Utf8,
                                    _ => DataType::Utf8, // Default to string
                                }
                            } else {
                                DataType::Utf8
                            };
                            
                            let nullable = column.get("nullable")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(true);
                            
                            fields.push(ArrowField::new(col_name, data_type, nullable));
                        }
                    }
                }
                
                // If no columns defined, create a dummy schema
                if fields.is_empty() {
                    fields.push(ArrowField::new("dummy", DataType::Utf8, true));
                }
                
                let arrow_schema = Arc::new(ArrowSchema::new(fields.clone()));
                let empty_table = EmptyTable::new(arrow_schema);
                
                // Register the table
                let _ = ctx.register_table(table_name, Arc::new(empty_table));
                
                // Also register with schema prefix if available
                if let Some(schema) = table.get("schema").and_then(|v| v.as_str()) {
                    let qualified_name = format!("{}.{}", schema, table_name);
                    let arrow_schema = Arc::new(ArrowSchema::new(fields));
                    let empty_table = EmptyTable::new(arrow_schema);
                    let _ = ctx.register_table(&qualified_name, Arc::new(empty_table));
                }
            }
        }
    }
    
    // Parse SQL to extract alias using DataFusion's parser
    let extracted_table_name = extract_table_alias_from_parsed_sql(sql)
        .or_else(|| suggested_table_name.map(|s| s.to_string()))
        .unwrap_or_else(|| "query_result".to_string());
    
    // Create logical plan to extract schema
    let logical_plan = ctx.state().create_logical_plan(sql).await?;
    let schema = logical_plan.schema();
    
    // Convert schema to JSON format
    let mut columns = Vec::new();
    for field in schema.fields() {
        let col_type = match field.data_type() {
            DataType::Int8 | DataType::Int16 => "integer",
            DataType::Int32 => "integer",
            DataType::Int64 => "bigint",
            DataType::UInt8 | DataType::UInt16 | DataType::UInt32 | DataType::UInt64 => "bigint",
            DataType::Float32 => "float",
            DataType::Float64 => "double",
            DataType::Boolean => "boolean",
            DataType::Utf8 | DataType::LargeUtf8 => "varchar",
            DataType::Date32 | DataType::Date64 => "date",
            DataType::Timestamp(_, _) => "timestamp",
            DataType::Decimal128(_, _) | DataType::Decimal256(_, _) => "decimal",
            DataType::Binary | DataType::LargeBinary => "binary",
            _ => "varchar", // Default to varchar for unknown types
        };
        
        columns.push(serde_json::json!({
            "name": field.name(),
            "type": col_type,
            "nullable": field.is_nullable(),
        }));
    }
    
    let result = serde_json::json!({
        "success": true,
        "tableName": extracted_table_name,
        "columns": columns,
    });
    
    Ok(result.to_string())
}

/// Helper function to extract table alias from parsed SQL using DataFusion's parser
fn extract_table_alias_from_parsed_sql(sql: &str) -> Option<String> {
    use datafusion::sql::parser::{DFParser, Statement};
    
    // Parse the SQL statement
    let statements = match DFParser::parse_sql(sql) {
        Ok(stmts) => stmts,
        Err(_) => return None,
    };
    
    // We only care about the first statement
    let statement = statements.front()?;
    
    // Extract alias from the statement
    match statement {
        Statement::Statement(stmt) => {
            match stmt.as_ref() {
                datafusion::sql::sqlparser::ast::Statement::Query(query) => extract_alias_from_query(query),
                _ => None,
            }
        },
        _ => None,
    }
}

/// Helper function to extract alias from a parsed query
fn extract_alias_from_query(query: &Box<datafusion::sql::sqlparser::ast::Query>) -> Option<String> {
    use datafusion::sql::sqlparser::ast::{SetExpr, TableFactor};
    
    // Check if the query itself has an alias (for subqueries used as tables)
    // This would be in the FROM clause of an outer query
    
    match query.body.as_ref() {
        SetExpr::Select(select) => {
            // Only extract alias if there's exactly one table (no joins)
            // Check if there's only one FROM table and no joins
            if select.from.len() == 1 {
                if let Some(table_with_joins) = select.from.first() {
                    // Check if this table has any joins
                    if table_with_joins.joins.is_empty() {
                        // No joins, so we can extract the alias from the single table
                        match &table_with_joins.relation {
                            TableFactor::Derived { alias, .. } => {
                                // Subquery with alias: SELECT * FROM (SELECT ...) AS alias
                                if let Some(alias) = alias {
                                    return Some(alias.name.value.clone());
                                }
                            }
                            TableFactor::Table { alias, .. } => {
                                // Regular table with alias: SELECT * FROM users u
                                if let Some(alias) = alias {
                                    return Some(alias.name.value.clone());
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
            // For JOINs or multiple tables, don't extract alias
        }
        _ => {}
    }
    
    None
}

/// FFI function to extract output schema from SQL query
/// Returns a JSON string with schema information
/// suggested_table_name: optional suggested name for the output table (SQL alias takes precedence)
#[no_mangle]
pub extern "C" fn wren_extract_sql_schema(
    schema_json: *const c_char,
    sql: *const c_char,
    dialect: *const c_char,
    suggested_table_name: *const c_char,
) -> *mut c_char {
    if sql.is_null() || dialect.is_null() {
        return std::ptr::null_mut();
    }
    
    let schema_json_str = if schema_json.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(schema_json) }.to_str() {
            Ok(s) => Some(s),
            Err(_) => return std::ptr::null_mut(),
        }
    };
    
    let sql_str = match unsafe { CStr::from_ptr(sql) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let dialect_str = match unsafe { CStr::from_ptr(dialect) }.to_str() {
        Ok(s) => s,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let suggested_table_name_str = if suggested_table_name.is_null() {
        None
    } else {
        match unsafe { CStr::from_ptr(suggested_table_name) }.to_str() {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    };
    
    // Create a new Tokio runtime for this operation
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(_) => return std::ptr::null_mut(),
    };
    
    let result = rt.block_on(async {
        extract_sql_output_schema_internal(
            schema_json_str,
            sql_str,
            dialect_str,
            suggested_table_name_str,
        ).await
    });
    
    match result {
        Ok(schema_result) => {
            match CString::new(schema_result) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
        Err(e) => {
            // Return error as JSON
            let error_result = serde_json::json!({
                "success": false,
                "error": e.to_string(),
            }).to_string();
            
            match CString::new(error_result) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => std::ptr::null_mut(),
            }
        }
    }
}
