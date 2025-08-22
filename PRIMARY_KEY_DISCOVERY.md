# Primary Key Discovery Report

## Executive Summary

This document details the investigation into how Wren Engine uses the `primaryKey` field in models, clarifying misconceptions and documenting the actual implementation.

## Key Findings

### 1. Primary Key is NOT Used for Relationship Definitions

Despite documentation suggesting that "primaryKey is used by relationship querying", our investigation reveals:
- **Relationships do NOT use or validate against primary keys**
- Relationship conditions are defined explicitly in the `condition` field (e.g., `"orders.customer_id = customers.id"`)
- The join columns in relationships are independent of the model's primary key

### 2. Primary Key is Required for Calculated Columns

The primary key serves a critical role when a model has **calculated columns that reference other models**:

```rust
// From wren-core/core/src/logical_plan/analyze/plan.rs:363-368
let Some(join_key) = model.primary_key() else {
    return plan_err!(
        "Model {} should have primary key for calculation",
        model.name()
    );
};
```

### 3. How Primary Keys Enable Calculated Columns

When a model has a calculated column that traverses relationships, Wren:

1. **Creates a CalculationPlanNode** that:
   - Executes the calculation query (joining through relationships)
   - **Includes the source model's primary key in the SELECT clause**
   - Returns both the calculated value and the primary key

2. **Joins the calculation result back** to the main model using:
   ```sql
   main_model.primary_key = calculation_subquery.primary_key
   ```

### 4. Primary Key as Correlation Key

The primary key acts as a **correlation key** to correctly match calculated results with their parent rows:

```rust
// CalculationPlanNode output includes both calculation and PK
let output_field = vec![
    Arc::new(Field::new(calculation.column.name(), ...)),  // Calculated column
    Arc::new(Field::new(pk_column.name(), ...))            // Primary key for joining
];
```

## Example Scenario

### Model Definition
```json
{
  "models": [{
    "name": "customers",
    "primaryKey": "customer_id",  // Required for calculated column below
    "columns": [
      {"name": "customer_id", "type": "integer"},
      {"name": "name", "type": "varchar"},
      {
        "name": "total_orders_amount",
        "type": "decimal",
        "isCalculated": true,
        "expression": "sum(orders.amount)"  // References another model
      }
    ]
  }]
}
```

### Query Transformation

When querying `SELECT name, total_orders_amount FROM customers`, Wren:

1. **Calculation Subquery**:
   ```sql
   SELECT 
     customers.customer_id,  -- Primary key included!
     sum(orders.amount) AS total_orders_amount
   FROM customers
   LEFT JOIN orders ON customers.customer_id = orders.customer_id
   GROUP BY customers.customer_id
   ```

2. **Final Query**:
   ```sql
   SELECT 
     main.name,
     calc.total_orders_amount
   FROM customers AS main
   LEFT JOIN (calculation_subquery) AS calc 
     ON main.customer_id = calc.customer_id  -- PK used for correlation
   ```

## Important Clarifications

### Primary Key Source
- The `primaryKey` is defined at the **model level**, not inherited from the source table
- Must reference a column that exists in the model's `columns` array
- Can be different from the actual database table's primary key

### When Primary Key is Required
- **Required**: When the model has calculated columns that reference other models
- **Not Required**: For simple models without cross-model calculated columns
- **Not Required**: For defining or using relationships

### Uniqueness Requirement
- The primary key **should be unique** within the model's result set
- Non-unique primary keys will cause incorrect calculation results (data duplication)
- Wren does not validate uniqueness - it's the model designer's responsibility

## Implementation Details

### Code Locations
- Primary key validation: `wren-core/core/src/logical_plan/analyze/plan.rs:363-368, 990-993`
- CalculationPlanNode creation: `wren-core/core/src/logical_plan/analyze/plan.rs:980-1029`
- Join back logic: `wren-core/core/src/logical_plan/analyze/plan.rs:369-382`
- Model definition: `wren-core-base/src/mdl/manifest.rs:277-279`

### Data Flow
```
Model with calculated column
    ↓
CalculationPlanNode created (requires PK)
    ↓
Subquery includes PK + calculated value
    ↓
Join back on PK to reassemble results
    ↓
Final result with calculated columns
```

## Recommendations

1. **Documentation Update**: Clarify that primary keys are specifically for calculated columns, not relationships
2. **Validation**: Add compile-time validation to ensure:
   - Primary key exists when calculated columns reference other models
   - Primary key references a valid column in the model
3. **UI Enhancement**: Show primary key as dropdown of available columns (non-calculated, non-relationship)
4. **Best Practices**: 
   - Always define primary key for models with cross-model calculations
   - Ensure primary key column is unique in the result set
   - Use meaningful column names for primary keys

## Conclusion

The primary key in Wren serves a specific architectural purpose: enabling calculated columns that traverse relationships by providing a correlation key for result reassembly. It is not directly involved in relationship definitions, contrary to some documentation. This discovery clarifies when and why primary keys are required in Wren models.