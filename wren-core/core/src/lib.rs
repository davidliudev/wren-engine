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
