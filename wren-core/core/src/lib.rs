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
/// Returns a JSON C string that must be freed by the caller using free_string
/// The JSON contains either success with result or error with details
#[no_mangle]
pub extern "C" fn wren_transform_sql(
    mdl_json: *const c_char,
    sql: *const c_char,
    dialect: *const c_char,
) -> *mut c_char {
    // Helper function to create error response
    fn create_error_response(error_type: &str, message: String, details: Option<String>) -> *mut c_char {
        let response = serde_json::json!({
            "success": false,
            "result": null,
            "error": {
                "type": error_type,
                "message": message,
                "details": details.unwrap_or_default()
            }
        });
        
        match CString::new(response.to_string()) {
            Ok(c_string) => c_string.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }
    
    // Helper function to create success response
    fn create_success_response(result: String) -> *mut c_char {
        let response = serde_json::json!({
            "success": true,
            "result": result,
            "error": null
        });
        
        match CString::new(response.to_string()) {
            Ok(c_string) => c_string.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }
    
    if mdl_json.is_null() || sql.is_null() || dialect.is_null() {
        return create_error_response("input", "Invalid input: null pointer provided".to_string(), None);
    }

    let mdl_json_str = match unsafe { CStr::from_ptr(mdl_json) }.to_str() {
        Ok(s) => s,
        Err(e) => return create_error_response("input", "Invalid MDL JSON string encoding".to_string(), Some(e.to_string())),
    };

    let sql_str = match unsafe { CStr::from_ptr(sql) }.to_str() {
        Ok(s) => s,
        Err(e) => return create_error_response("input", "Invalid SQL string encoding".to_string(), Some(e.to_string())),
    };

    let dialect_str = match unsafe { CStr::from_ptr(dialect) }.to_str() {
        Ok(s) => s,
        Err(e) => return create_error_response("input", "Invalid dialect string encoding".to_string(), Some(e.to_string())),
    };

    // Create a new Tokio runtime for this operation
    let rt = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return create_error_response("runtime", "Failed to create async runtime".to_string(), Some(e.to_string())),
    };

    let result = rt.block_on(async {
        transform_sql_internal(mdl_json_str, sql_str, dialect_str).await
    });

    match result {
        Ok(transformed_sql) => create_success_response(transformed_sql),
        Err(e) => {
            // Parse the error to determine type and extract meaningful message
            let error_str = e.to_string();
            
            // Determine error type based on error message patterns
            let (error_type, message, details) = if error_str.contains("Unsupported dialect") {
                ("dialect", error_str.clone(), None)
            } else if error_str.contains("MDL") || error_str.contains("manifest") || error_str.contains("JSON") {
                ("mdl", "Invalid MDL structure".to_string(), Some(error_str))
            } else if error_str.contains("SQL syntax") || error_str.contains("parse") {
                ("syntax", "SQL parsing error".to_string(), Some(error_str))
            } else if error_str.contains("column") || error_str.contains("Column") {
                ("semantic", "Column reference error".to_string(), Some(error_str))
            } else if error_str.contains("table") || error_str.contains("Table") || error_str.contains("model") || error_str.contains("Model") {
                ("semantic", "Model/Table reference error".to_string(), Some(error_str))
            } else if error_str.contains("relationship") || error_str.contains("Relationship") {
                ("semantic", "Relationship error".to_string(), Some(error_str))
            } else {
                ("transformation", "SQL transformation failed".to_string(), Some(error_str))
            };
            
            create_error_response(error_type, message, details)
        }
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






