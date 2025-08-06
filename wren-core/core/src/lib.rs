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
