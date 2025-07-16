use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use wren_core::mdl::{self, AnalyzedWrenMDL};
use wren_core::mdl::context::Mode;
use wren_core_base::mdl::manifest::{Manifest, DataSource};

#[derive(Parser, Debug)]
#[command(name = "wren-cli")]
#[command(about = "Transform SQL queries using Wren MDL", long_about = None)]
struct Args {
    /// Path to the MDL JSON file
    #[arg(short, long)]
    mdl: String,

    /// SQL query to transform
    #[arg(short, long)]
    sql: String,

    /// Optional session properties as key=value pairs
    #[arg(short, long, value_delimiter = ',')]
    properties: Option<Vec<String>>,

    /// SQL dialect to use (bigquery, mysql, postgres, snowflake, mssql, trino, clickhouse, canner, datafusion, duckdb, oracle)
    #[arg(short, long, default_value = "datafusion")]
    dialect: String,
}

fn parse_dialect(dialect_str: &str) -> Result<DataSource, Box<dyn std::error::Error>> {
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
        _ => Err(format!("Unsupported dialect: {}. Supported dialects: bigquery, mysql, postgres, snowflake, mssql, trino, clickhouse, canner, datafusion, duckdb, oracle", dialect_str).into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // Parse dialect parameter
    let data_source = parse_dialect(&args.dialect)?;

    // Read MDL file
    let mdl_content = fs::read_to_string(&args.mdl)?;
    let mut manifest: Manifest = serde_json::from_str(&mdl_content)?;
    
    // Override the data source with the CLI parameter
    manifest.data_source = Some(data_source);

    // Parse session properties
    let mut session_props = HashMap::new();
    if let Some(props) = args.properties {
        for prop in props {
            let parts: Vec<&str> = prop.split('=').collect();
            if parts.len() == 2 {
                session_props.insert(parts[0].to_string(), Some(parts[1].to_string()));
            }
        }
    }
    let session_properties = Arc::new(session_props);

    // Create a default SessionContext
    let ctx = datafusion::execution::context::SessionContext::new();

    // Analyze the MDL
    let analyzed_mdl = Arc::new(AnalyzedWrenMDL::analyze(
        manifest,
        Arc::clone(&session_properties),
        Mode::Unparse,
    )?);

    // Transform the SQL
    let transformed_sql = mdl::transform_sql_with_ctx(
        &ctx,
        analyzed_mdl,
        &[],  // No remote functions for CLI
        session_properties,
        &args.sql,
    )
    .await?;

    // Print the transformed SQL
    println!("{}", transformed_sql);

    Ok(())
}