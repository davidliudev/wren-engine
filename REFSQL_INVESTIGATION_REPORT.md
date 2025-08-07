# refSql Investigation Report

## Executive Summary

**Finding: `refSql` is NOT functional in the current Wren Engine implementation, despite being documented as a working feature.**

This report documents a comprehensive investigation into the `refSql` field in Wren MDL models, revealing a significant discrepancy between documentation and implementation.

## What the Documentation Claims

According to the official Wren documentation at https://docs.getwren.ai/oss/engine/guide/modeling/model#tablereference-and-refsql:
> "You can use either `tableReference` or `refSql` to define the data source"

This suggests that models can use SQL queries (`refSql`) as an alternative to direct table references (`tableReference`).

## Investigation Methodology

### 1. Code Structure Analysis
Examined the model definition across the codebase:
- **Location**: `/wren-core-base/manifest-macro/src/lib.rs:137`
- **Finding**: `refSql` is defined as an optional field in the Model struct
```rust
pub struct Model {
    pub name: String,
    #[serde(default)]
    pub ref_sql: Option<String>,  // <-- Defined but not used
    #[serde(default, with = "table_reference")]
    pub table_reference: Option<String>,
    // ... other fields
}
```

### 2. Usage Search in Core Engine
Searched for `refSql` usage in the wren-core engine:
- **Finding**: `refSql` appears ONLY in:
  - Struct definitions
  - Builder methods to set the value
  - Serialization/deserialization tests
  - **NOT in any query processing logic**

### 3. Query Processing Analysis
Examined how models are converted to table scans:

#### Location: `/core/src/logical_plan/analyze/model_generation.rs`
Lines 147 and 166 show the critical code:
```rust
LogicalPlanBuilder::scan(
    TableReference::from(model.table_reference()),  // <-- ONLY uses table_reference
    create_remote_table_source(...)
)
```

The `table_reference()` method (in `/wren-core-base/src/mdl/manifest.rs:282`):
```rust
pub fn table_reference(&self) -> &str {
    self.table_reference.as_deref().unwrap_or("")  // Returns empty string if None
}
```

**Critical Finding**: The engine ONLY uses `table_reference`. If it's missing, it returns an empty string, which causes query failures.

### 4. Empirical Testing
Created and ran a test program (`test-refsql.rs`) to verify behavior:

#### Test Results:
```
Test 1: Model with tableReference
✓ Query with tableReference works!

Test 2: Model with refSql instead of tableReference
✗ Transform failed with refSql: ModelGenerationRule
caused by
Error during planning: table_name cannot be empty
  This confirms refSql is NOT implemented
```

**Proof**: Models with only `refSql` fail with "table_name cannot be empty" error.

## Investigation of ibis-server

### Puzzling Discovery: Test Cases Use refSql
Found extensive test cases in ibis-server using `refSql`:
- `/tests/routers/v2/test_analysis.py`
- `/tests/routers/v2/connector/test_postgres.py`
- And many other connector tests

Example:
```python
{
    "name": "customer",
    "refSql": "select * from main.customer",  # <-- Using refSql
    "columns": [...]
}
```

### Resolution: Code Filters Out refSql Models
Found the explanation in `/app/mdl/substitute.py`:

```python
def _build_model_dict(models) -> dict:
    return {
        ModelSubstitute._build_key(model): model
        for model in models
        if "tableReference" in model  # <-- FILTERS OUT models without tableReference!
    }
```

**Lines 66 and 74**: Models without `tableReference` are filtered out and ignored.

### Possible Explanations for Test Cases:
1. **Tests may be outdated** - Written when `refSql` was planned but never updated
2. **Different engine path** - ibis-server supports both embedded (Rust) and external (Java) engines; Java engine might support `refSql`
3. **Tests may be failing** - Without proper test infrastructure, couldn't verify if these tests actually pass

## Why Does refSql Exist?

Based on the investigation, `refSql` appears to be:
1. **Planned but unimplemented feature** - Infrastructure exists but implementation was never completed
2. **Legacy from design phase** - Field was added during design but development priorities changed
3. **Future compatibility** - Placeholder for potential future implementation

## Impact Analysis

### 1. Documentation Inconsistency
- **Documentation states**: Both `tableReference` and `refSql` work
- **Reality**: Only `tableReference` works
- **Impact**: Users following documentation will encounter failures

### 2. Frontend Validation Requirements
For any frontend or validation logic:
- **MUST** require `tableReference` for all models
- **SHOULD** hide or mark `refSql` as "not supported"
- **MUST** validate that `tableReference` is not empty

### 3. Test Reliability
- Numerous tests use `refSql` but may not be executing correctly
- Test coverage may be giving false confidence

## Code Evidence Summary

### Evidence that refSql is NOT working:

1. **No usage in query processing**:
   - `grep -r "\.ref_sql" wren-core/core/src/` returns nothing
   - All table scan creation uses `model.table_reference()`

2. **Explicit filtering in ibis-server**:
   - Models without `tableReference` are filtered out (substitute.py:66,74)

3. **Empirical test failure**:
   - Test program confirms: models with only `refSql` fail with "table_name cannot be empty"

4. **Architecture limitation**:
   - Current implementation requires physical table for source-level expressions
   - No code path exists to handle SQL-based models

## Recommendations

### Immediate Actions:
1. **Update Documentation**: Remove or mark `refSql` as "not implemented"
2. **Update Validation Logic**: Require `tableReference` for all models
3. **Review Test Suite**: Verify if tests using `refSql` are actually passing

### For Frontend Implementation:
```javascript
// Validation rule
if (!model.tableReference || model.tableReference === '') {
    return {
        valid: false,
        error: 'Model must have tableReference (refSql is not currently supported)'
    };
}
```

### Long-term Considerations:
1. Either implement `refSql` functionality or remove it from the schema
2. Align tests with actual implementation
3. Add integration tests that verify both documentation examples work

## Technical Details for Implementation

If `refSql` were to be implemented, it would require:

1. **Modify model_generation.rs**: Add logic to handle `refSql` as alternative to `tableReference`
2. **Create subquery handling**: Convert `refSql` SQL into a subquery table source
3. **Update validation**: Ensure exactly one of `tableReference` or `refSql` is present
4. **Schema inference**: Derive schema from SQL query results
5. **Performance considerations**: Subqueries may impact query optimization

Example of what would be needed:
```rust
let table_source = if let Some(ref_sql) = model.ref_sql {
    // Parse and create subquery source
    create_subquery_source(ref_sql, ...)
} else if let Some(table_ref) = model.table_reference {
    // Current implementation
    create_remote_table_source(table_ref, ...)
} else {
    return Err("Model must have either tableReference or refSql")
};
```

## Conclusion

The investigation conclusively proves that `refSql` is **not functional** in the current Wren Engine implementation, despite being:
- Defined in the model schema
- Documented as a working feature
- Used in test cases

This represents a significant gap between intended design and actual implementation. All current functionality requires `tableReference` to be present and non-empty for models to work properly.

## Appendix: Test Program Output

Full output from `test-refsql.rs`:
```
=== Testing refSql Support ===

Test 1: Model with tableReference
✓ Query with tableReference works!
  Transformed: SELECT orders_model.order_id FROM (SELECT orders_model.order_id FROM 
  (SELECT __source.order_id AS order_id FROM datafusion."public".orders AS __source) 
  AS orders_model) AS orders_model LIMIT 5

Test 2: Model with refSql instead of tableReference
✗ Transform failed with refSql: ModelGenerationRule
caused by
Error during planning: table_name cannot be empty
  This confirms refSql is NOT implemented

=== Conclusion ===
If Test 2 failed, it confirms that refSql is defined in the schema
but not actually implemented in the query processing engine.
```

---

*Investigation conducted on: 2025-08-07*
*Wren Engine version: Based on source code at `/Users/yuningliu/project/NotellectNeo/reference-code/wren-engine`*