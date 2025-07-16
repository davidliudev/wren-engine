use clap::Parser;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use wren_core::mdl::{self, AnalyzedWrenMDL};
use wren_core::mdl::context::Mode;
use wren_core_base::mdl::manifest::Manifest;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    // Parse command line arguments
    let args = Args::parse();

    // Read MDL file
    let mdl_content = fs::read_to_string(&args.mdl)?;
    let manifest: Manifest = serde_json::from_str(&mdl_content)?;

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