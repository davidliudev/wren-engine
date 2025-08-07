use std::collections::HashMap;
use std::sync::Arc;

use datafusion::error::Result;
use datafusion::prelude::{CsvReadOptions, SessionContext};

use wren_core::mdl::builder::{
    ColumnBuilder, ManifestBuilder, ModelBuilder,
};
use wren_core::mdl::manifest::Manifest;
use wren_core::mdl::{transform_sql_with_ctx, AnalyzedWrenMDL};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    println!("=== Testing refSql Support ===\n");
    
    // Register a CSV table first
    let ctx = SessionContext::new();
    ctx.register_csv(
        "orders",
        "sqllogictest/tests/resources/ecommerce/orders.csv",
        CsvReadOptions::new(),
    )
    .await?;
    
    let provider = ctx
        .catalog("datafusion")
        .unwrap()
        .schema("public")
        .unwrap()
        .table("orders")
        .await?
        .unwrap();

    // Test 1: Model with tableReference (should work)
    println!("Test 1: Model with tableReference");
    let manifest_with_table_ref = create_manifest_with_table_reference();
    let register = HashMap::from([
        ("datafusion.public.orders".to_string(), Arc::clone(&provider)),
    ]);
    
    let analyzed_mdl = Arc::new(AnalyzedWrenMDL::analyze_with_tables(
        manifest_with_table_ref, 
        register.clone()
    )?);
    
    let sql = "SELECT order_id FROM wrenai.public.orders_model LIMIT 5";
    match transform_sql_with_ctx(
        &ctx,
        Arc::clone(&analyzed_mdl),
        &[],
        HashMap::new().into(),
        sql,
    )
    .await
    {
        Ok(transformed) => {
            println!("✓ Query with tableReference works!");
            println!("  Transformed: {}\n", transformed);
        }
        Err(e) => {
            println!("✗ Failed with tableReference: {}\n", e);
        }
    }
    
    // Test 2: Model with refSql (test if it works)
    println!("Test 2: Model with refSql instead of tableReference");
    let manifest_with_ref_sql = create_manifest_with_ref_sql();
    
    match AnalyzedWrenMDL::analyze_with_tables(manifest_with_ref_sql, register) {
        Ok(analyzed_mdl) => {
            let analyzed_mdl = Arc::new(analyzed_mdl);
            let sql = "SELECT order_id FROM wrenai.public.orders_model_sql LIMIT 5";
            
            match transform_sql_with_ctx(
                &ctx,
                Arc::clone(&analyzed_mdl),
                &[],
                HashMap::new().into(),
                sql,
            )
            .await
            {
                Ok(transformed) => {
                    println!("✓ Query with refSql works!");
                    println!("  Transformed: {}\n", transformed);
                    
                    // Try to execute it
                    match ctx.sql(&transformed).await {
                        Ok(df) => {
                            println!("✓ Execution successful!");
                            df.show().await?;
                        }
                        Err(e) => println!("✗ Execution failed: {}", e),
                    }
                }
                Err(e) => {
                    println!("✗ Transform failed with refSql: {}", e);
                    println!("  This confirms refSql is NOT implemented\n");
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to analyze MDL with refSql: {}", e);
            println!("  This confirms refSql is NOT implemented\n");
        }
    }
    
    println!("\n=== Conclusion ===");
    println!("If Test 2 failed, it confirms that refSql is defined in the schema");
    println!("but not actually implemented in the query processing engine.");
    
    Ok(())
}

fn create_manifest_with_table_reference() -> Manifest {
    ManifestBuilder::new()
        .catalog("wrenai")
        .schema("public")
        .model(
            ModelBuilder::new("orders_model")
                .table_reference("datafusion.public.orders")  // Using tableReference
                .column(ColumnBuilder::new("order_id", "varchar").build())
                .column(ColumnBuilder::new("customer_id", "varchar").build())
                .primary_key("order_id")
                .build(),
        )
        .build()
}

fn create_manifest_with_ref_sql() -> Manifest {
    ManifestBuilder::new()
        .catalog("wrenai")
        .schema("public")
        .model(
            ModelBuilder::new("orders_model_sql")
                .ref_sql("SELECT * FROM datafusion.public.orders")  // Using refSql instead
                // Note: NOT setting table_reference
                .column(ColumnBuilder::new("order_id", "varchar").build())
                .column(ColumnBuilder::new("customer_id", "varchar").build())
                .primary_key("order_id")
                .build(),
        )
        .build()
}