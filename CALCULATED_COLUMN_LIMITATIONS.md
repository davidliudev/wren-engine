# Calculated Column Limitations in Wren Engine

## Overview

This document details the current limitations and known bugs in Wren Engine's calculated column implementation, based on code analysis performed on 2025-08-07.

## Key Limitations

### 1. Calculated Columns Cannot Reference Other Calculated Columns

**Status**: Fundamental limitation in current architecture

**Description**: A calculated column (`isCalculated: true`) cannot reference another calculated column within the same model. It can only reference:
- Physical (non-calculated) columns from the same model
- Columns from related models through relationship navigation

**Code Evidence**:
```rust
// In /core/src/mdl/dataset.rs:65
.filter(|c| !c.is_calculated)  // Filters out calculated columns from schema
```

**Example**:
```json
{
  "columns": [
    {
      "name": "full_name",
      "isCalculated": true,
      "expression": "first_name || ' ' || last_name"  // ✅ Works
    },
    {
      "name": "display_name", 
      "isCalculated": true,
      "expression": "full_name || ' (User)'"  // ❌ FAILS - cannot reference full_name
    }
  ]
}
```

**Workaround**: Use views instead of calculated columns for complex multi-step calculations.

### 2. Bug: Mixing Relationships with Physical Columns

**Status**: Known bug with TODO in codebase

**Description**: Calculated columns have issues when an expression combines:
- Relationship navigation (e.g., `Customer.name`)
- Physical columns from the same model (e.g., `order_id`)

**Code Evidence**:
```rust
// In /sqllogictest/src/test_context.rs:215
// TODO: fix calculation with non-relationship column
// .column(
//     ColumnBuilder::new_calculated("Customer_state_order_id", "varchar")
//         .expression(r#""Customers"."State" || ' ' || "Order_id""#)
//         .build(),
// )
```

**Example**:
```json
{
  "name": "customer_order_label",
  "isCalculated": true,
  "expression": "customer.name || ' - ' || order_id"  // ❌ Known to fail
}
```

**Workaround**: Split into separate calculated columns:
```json
[
  {
    "name": "customer_name_calc",
    "isCalculated": true,
    "expression": "customer.name"
  },
  {
    "name": "order_id_formatted",
    "isCalculated": false,
    "expression": "'ORD-' || order_id"  // Source-level expression
  }
]
```

### 3. Limited Support for Standalone Calculated Columns

**Status**: Partial support with limitations

**Description**: Some calculated columns require relationship context to work properly, even when not directly using relationships.

**Code Evidence**:
```rust
// In /core/src/mdl/mod.rs:616
// TODO: support calculated without relationship
```

## Technical Architecture

### How Calculated Columns Work

1. **Processing Model**:
   - Calculated columns are processed in a separate `CalculationPlanNode`
   - Creates a subquery with the calculation expression
   - Joins back to main query using the model's primary key
   - This is why primary key is required for models with calculated columns

2. **Schema Context**:
   - **Source-level expressions** (`isCalculated: false`):
     - Use `to_remote_schema()` which excludes calculated columns
     - Can only access physical table columns
   - **Calculated expressions** (`isCalculated: true`):
     - Use `to_qualified_schema()` which includes all physical columns
     - But still cannot access other calculated columns due to filtering

3. **Dependency Resolution**:
   - Wren tracks dependencies through `lineage.rs`
   - Prevents circular dependencies
   - But doesn't support calculated-on-calculated references

## Validation Rules Summary

### For Source-Level Expressions (`isCalculated: false`)

✅ **CAN**:
- Reference physical columns from source table
- Use SQL functions, arithmetic, CASE statements
- Use any valid SQL expression on source columns

❌ **CANNOT**:
- Reference calculated columns
- Use relationship navigation (dot notation)
- Use subqueries or CTEs

### For Calculated Expressions (`isCalculated: true`)

✅ **CAN**:
- Navigate relationships using dot notation
- Reference physical columns from same model (with caveats)
- Use aggregations with proper context
- Use SQL functions, arithmetic, CASE statements

❌ **CANNOT**:
- Reference other calculated columns in same model
- Mix relationship navigation with physical columns (bug)
- Use subqueries in some contexts

## Recommended Validation Implementation

```typescript
function validateCalculatedExpression(
  expression: string,
  model: WrenModel
): ValidationResult {
  const referencedColumns = extractColumnReferences(expression);
  
  // Check for calculated column references
  for (const ref of referencedColumns) {
    if (!ref.includes('.')) { // Local column reference
      const column = model.columns.find(c => c.name === ref);
      if (column?.isCalculated) {
        return {
          valid: false,
          error: 'Calculated columns cannot reference other calculated columns'
        };
      }
    }
  }
  
  // Check for problematic mixing
  const hasRelationship = referencedColumns.some(ref => ref.includes('.'));
  const hasPhysicalColumn = referencedColumns.some(ref => {
    if (!ref.includes('.')) {
      const col = model.columns.find(c => c.name === ref);
      return col && !col.isCalculated && !col.relationship;
    }
    return false;
  });
  
  if (hasRelationship && hasPhysicalColumn) {
    return {
      valid: true,
      warning: 'Mixing relationships with physical columns may not work correctly'
    };
  }
  
  return { valid: true };
}
```

## Future Improvements

Based on TODO comments in the codebase, the Wren team is aware of these limitations:
1. Support for calculated columns referencing other calculated columns
2. Fix for mixing relationships with physical columns
3. Better support for standalone calculated columns

## References

- Issue tracking: Check Wren Engine GitHub issues
- Code locations:
  - `/core/src/mdl/dataset.rs` - Schema generation
  - `/core/src/logical_plan/analyze/plan.rs` - Calculation processing
  - `/sqllogictest/src/test_context.rs` - Test cases with TODOs
  - `/core/src/mdl/lineage.rs` - Dependency tracking

## Last Updated

2025-08-07 - Based on code analysis of current Wren Engine implementation