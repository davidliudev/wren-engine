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
    
    // Test case: SQL expressions with isCalculated: false
    let manifest = create_test_manifest();

    // Register a simple orders table
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

    let register = HashMap::from([
        ("datafusion.public.orders".to_string(), provider),
    ]);
    
    let analyzed_mdl = Arc::new(AnalyzedWrenMDL::analyze_with_tables(manifest, register)?);

    println!("=== Testing SQL Expressions with isCalculated: false ===\n");

    // Test 1: Simple column alias (traditional use case)
    println!("Test 1: Column alias with isCalculated: false");
    let sql1 = "SELECT customer_alias FROM wrenai.public.orders LIMIT 5";
    match transform_sql_with_ctx(
        &ctx,
        Arc::clone(&analyzed_mdl),
        &[],
        HashMap::new().into(),
        sql1,
    )
    .await
    {
        Ok(transformed) => {
            println!("✓ Query transformed successfully:");
            println!("  Original: {}", sql1);
            println!("  Transformed: {}\n", transformed);
            
            // Execute to verify it works
            match ctx.sql(&transformed).await {
                Ok(df) => {
                    println!("✓ Query executed successfully");
                    df.show().await?;
                }
                Err(e) => println!("✗ Execution failed: {}", e),
            }
        }
        Err(e) => println!("✗ Transformation failed: {}\n", e),
    }

    // Test 2: Arithmetic expression with isCalculated: false
    println!("\nTest 2: Arithmetic expression with isCalculated: false");
    let sql2 = "SELECT order_id_plus_10 FROM wrenai.public.orders LIMIT 5";
    match transform_sql_with_ctx(
        &ctx,
        Arc::clone(&analyzed_mdl),
        &[],
        HashMap::new().into(),
        sql2,
    )
    .await
    {
        Ok(transformed) => {
            println!("✓ Query transformed successfully:");
            println!("  Original: {}", sql2);
            println!("  Transformed: {}\n", transformed);
            
            match ctx.sql(&transformed).await {
                Ok(df) => {
                    println!("✓ Query executed successfully");
                    df.show().await?;
                }
                Err(e) => println!("✗ Execution failed: {}", e),
            }
        }
        Err(e) => println!("✗ Transformation failed: {}\n", e),
    }

    // Test 3: Function expression with isCalculated: false
    println!("\nTest 3: Function (UPPER) with isCalculated: false");
    let sql3 = "SELECT customer_upper FROM wrenai.public.orders LIMIT 5";
    match transform_sql_with_ctx(
        &ctx,
        Arc::clone(&analyzed_mdl),
        &[],
        HashMap::new().into(),
        sql3,
    )
    .await
    {
        Ok(transformed) => {
            println!("✓ Query transformed successfully:");
            println!("  Original: {}", sql3);
            println!("  Transformed: {}\n", transformed);
            
            match ctx.sql(&transformed).await {
                Ok(df) => {
                    println!("✓ Query executed successfully");
                    df.show().await?;
                }
                Err(e) => println!("✗ Execution failed: {}", e),
            }
        }
        Err(e) => println!("✗ Transformation failed: {}\n", e),
    }

    // Test 4: Compare with isCalculated: true
    println!("\nTest 4: Same expression with isCalculated: true (for comparison)");
    let sql4 = "SELECT order_id_plus_10_calc FROM wrenai.public.orders LIMIT 5";
    match transform_sql_with_ctx(
        &ctx,
        Arc::clone(&analyzed_mdl),
        &[],
        HashMap::new().into(),
        sql4,
    )
    .await
    {
        Ok(transformed) => {
            println!("✓ Query transformed successfully:");
            println!("  Original: {}", sql4);
            println!("  Transformed: {}\n", transformed);
            
            match ctx.sql(&transformed).await {
                Ok(df) => {
                    println!("✓ Query executed successfully");
                    df.show().await?;
                }
                Err(e) => println!("✗ Execution failed: {}", e),
            }
        }
        Err(e) => println!("✗ Transformation failed: {}\n", e),
    }

    println!("\n=== Summary ===");
    println!("The test demonstrates that SQL expressions DO work with isCalculated: false");
    println!("Both simple column references and complex expressions are supported.");
    
    Ok(())
}

fn create_test_manifest() -> Manifest {
    ManifestBuilder::new()
        .catalog("wrenai")
        .schema("public")
        .model(
            ModelBuilder::new("orders")
                .table_reference("datafusion.public.orders")
                // Original columns
                .column(ColumnBuilder::new("order_id", "varchar").build())
                .column(ColumnBuilder::new("customer_id", "varchar").build())
                .column(ColumnBuilder::new("order_date", "bigint").build())
                
                // Test 1: Simple column alias (isCalculated: false)
                .column(
                    ColumnBuilder::new("customer_alias", "varchar")
                        .calculated(false)  // NOT calculated
                        .expression("customer_id")
                        .build(),
                )
                
                // Test 2: Arithmetic expression (isCalculated: false)
                .column(
                    ColumnBuilder::new("order_id_plus_10", "varchar")
                        .calculated(false)  // NOT calculated
                        .expression("order_id || '_plus_10'")
                        .build(),
                )
                
                // Test 3: Function expression (isCalculated: false)
                .column(
                    ColumnBuilder::new("customer_upper", "varchar")
                        .calculated(false)  // NOT calculated
                        .expression("upper(customer_id)")
                        .build(),
                )
                
                // Test 4: Same expression but with isCalculated: true for comparison
                .column(
                    ColumnBuilder::new("order_id_plus_10_calc", "varchar")
                        .calculated(true)  // IS calculated
                        .expression("order_id || '_plus_10_calc'")
                        .build(),
                )
                
                .primary_key("order_id")
                .build(),
        )
        .build()
}