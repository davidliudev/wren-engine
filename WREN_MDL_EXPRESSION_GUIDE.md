# Wren MDL Expression Field Guide

## Overview

The `expression` field in Wren MDL is used to define SQL expressions for columns. It works differently based on the `isCalculated` flag, enabling both source-level transformations and dynamic calculations.

## Key Concepts

### Three Types of Columns

1. **Physical Column**: Direct database column mapping (`isCalculated: false/omitted`, no expression)
2. **Source-Level Expression**: SQL expression evaluated during table scan (`isCalculated: false` with expression)
3. **Calculated Column**: Computed value in separate layer (`isCalculated: true` with expression - **expression is REQUIRED**)

### Important Discovery (Verified by Testing)

**When `isCalculated: false` with an expression, you can use ANY valid SQL expression, not just column names.** This includes:
- Simple column references (aliases)
- Arithmetic operations
- SQL functions
- String concatenation
- CASE statements
- Any SQL expression that only references columns from the source table

## Source-Level Expressions (isCalculated: false with expression)

Evaluated during the table scan, these expressions can transform data at the source level efficiently.

### Simple Column Aliasing

| Use Case | Configuration | Result |
|----------|--------------|--------|
| **Rename technical column** | `name: "customer_id"`<br>`isCalculated: false`<br>`expression: "cust_id"` | `SELECT cust_id AS customer_id` |
| **Business-friendly naming** | `name: "order_amount"`<br>`isCalculated: false`<br>`expression: "amt"` | `SELECT amt AS order_amount` |

### SQL Expressions at Source Level (Verified by Testing)

| Use Case | Configuration | Result |
|----------|--------------|--------|
| **String concatenation** | `name: "order_code"`<br>`isCalculated: false`<br>`expression: "order_id \|\| '_v2'"` | `SELECT order_id \|\| '_v2' AS order_code` |
| **Apply function** | `name: "customer_upper"`<br>`isCalculated: false`<br>`expression: "upper(customer_id)"` | `SELECT upper(customer_id) AS customer_upper` |
| **Arithmetic** | `name: "amount_doubled"`<br>`isCalculated: false`<br>`expression: "amount * 2"` | `SELECT amount * 2 AS amount_doubled` |
| **CASE statement** | `name: "status_label"`<br>`isCalculated: false`<br>`expression: "CASE status WHEN 1 THEN 'Active' ELSE 'Inactive' END"` | `SELECT CASE status... AS status_label` |

## Calculated Columns (isCalculated: true with expression)

Used to compute new values based on existing columns or relationships.

### Basic Arithmetic

| Example | Configuration | SQL Generated |
|---------|--------------|---------------|
| Convert to cents | `name: "amount_cents"`<br>`isCalculated: true`<br>`expression: "amount * 100"` | `SELECT amount * 100 AS amount_cents` |
| Percentage | `name: "tax_rate"`<br>`isCalculated: true`<br>`expression: "tax_amount / total_amount * 100"` | `SELECT tax_amount / total_amount * 100 AS tax_rate` |
| Addition | `name: "total_with_tax"`<br>`isCalculated: true`<br>`expression: "subtotal + tax"` | `SELECT subtotal + tax AS total_with_tax` |

### Date Functions

| Example | Configuration | SQL Generated |
|---------|--------------|---------------|
| Extract year | `name: "order_year"`<br>`isCalculated: true`<br>`expression: "extract(year from order_date)"` | `SELECT extract(year from order_date) AS order_year` |
| Extract month | `name: "order_month"`<br>`isCalculated: true`<br>`expression: "extract(month from order_date)"` | `SELECT extract(month from order_date) AS order_month` |
| Date difference | `name: "days_since_order"`<br>`isCalculated: true`<br>`expression: "current_date - order_date"` | `SELECT current_date - order_date AS days_since_order` |

### String Operations

| Example | Configuration | SQL Generated |
|---------|--------------|---------------|
| Concatenation | `name: "full_name"`<br>`isCalculated: true`<br>`expression: "first_name || ' ' || last_name"` | `SELECT first_name || ' ' || last_name AS full_name` |
| Uppercase | `name: "country_code"`<br>`isCalculated: true`<br>`expression: "upper(country)"` | `SELECT upper(country) AS country_code` |
| Substring | `name: "order_prefix"`<br>`isCalculated: true`<br>`expression: "substr(order_id, 1, 3)"` | `SELECT substr(order_id, 1, 3) AS order_prefix` |
| Hash | `name: "customer_hash"`<br>`isCalculated: true`<br>`expression: "md5(customer_id)"` | `SELECT md5(customer_id) AS customer_hash` |

### Type Casting

| Example | Configuration | SQL Generated |
|---------|--------------|---------------|
| To timestamp | `name: "event_timestamp"`<br>`isCalculated: true`<br>`expression: "cast(event_time as timestamp)"` | `SELECT cast(event_time as timestamp) AS event_timestamp` |
| To integer | `name: "year_int"`<br>`isCalculated: true`<br>`expression: "cast(year_str as integer)"` | `SELECT cast(year_str as integer) AS year_int` |
| To decimal | `name: "price_decimal"`<br>`isCalculated: true`<br>`expression: "cast(price_text as decimal(10,2))"` | `SELECT cast(price_text as decimal(10,2)) AS price_decimal` |

### Conditional Logic

| Example | Configuration | SQL Generated |
|---------|--------------|---------------|
| CASE statement | `name: "order_size"`<br>`isCalculated: true`<br>`expression: "CASE WHEN amount > 1000 THEN 'Large' WHEN amount > 100 THEN 'Medium' ELSE 'Small' END"` | `SELECT CASE WHEN amount > 1000 THEN 'Large'...` |
| COALESCE | `name: "display_name"`<br>`isCalculated: true`<br>`expression: "COALESCE(nickname, first_name, 'Guest')"` | `SELECT COALESCE(nickname, first_name, 'Guest') AS display_name` |
| NULL handling | `name: "safe_amount"`<br>`isCalculated: true`<br>`expression: "NULLIF(amount, 0)"` | `SELECT NULLIF(amount, 0) AS safe_amount` |

### Relationship Navigation

Wren's powerful feature allowing traversal through defined relationships using dot notation.

| Example | Configuration | Description |
|---------|--------------|-------------|
| Single hop | `name: "customer_name"`<br>`isCalculated: true`<br>`expression: "customer.name"` | Joins customer table and selects name |
| Multiple hops | `name: "vendor_country"`<br>`isCalculated: true`<br>`expression: "order.product.vendor.country"` | Traverses order→product→vendor relationships |
| Reference calculated field | `name: "customer_status_copy"`<br>`isCalculated: true`<br>`expression: "customer.status_label"` | References another calculated field |

### Aggregations

| Example | Configuration | Description |
|---------|--------------|-------------|
| Sum related records | `name: "total_order_value"`<br>`isCalculated: true`<br>`expression: "sum(order_items.price)"` | Aggregates across relationship |
| Count related | `name: "order_count"`<br>`isCalculated: true`<br>`expression: "count(orders.id)"` | Counts related orders |
| Average | `name: "avg_item_price"`<br>`isCalculated: true`<br>`expression: "avg(order_items.unit_price)"` | Average of related items |
| Max/Min | `name: "latest_order_date"`<br>`isCalculated: true`<br>`expression: "max(orders.order_date)"` | Latest order date |
| Complex aggregation | `name: "revenue_ytd"`<br>`isCalculated: true`<br>`expression: "sum(CASE WHEN extract(year from order_date) = extract(year from current_date) THEN amount ELSE 0 END)"` | Conditional aggregation |

### Advanced SQL Features

| Example | Configuration | Description |
|---------|--------------|-------------|
| Window functions | `name: "running_total"`<br>`isCalculated: true`<br>`expression: "sum(amount) OVER (ORDER BY order_date)"` | Running total window function |
| Row number | `name: "row_num"`<br>`isCalculated: true`<br>`expression: "row_number() OVER (PARTITION BY customer_id ORDER BY order_date)"` | Row numbering within groups |
| JSON extraction | `name: "user_email"`<br>`isCalculated: true`<br>`expression: "metadata->>'email'"` | Extract from JSON field |
| Array operations | `name: "first_tag"`<br>`isCalculated: true`<br>`expression: "tags[1]"` | Array element access |
| Regular expression | `name: "phone_area_code"`<br>`isCalculated: true`<br>`expression: "regexp_substr(phone, '^\\d{3}')"` | Extract pattern with regex |

## Valid Column Configurations

### Summary of Valid Combinations

| isCalculated | expression | Result | Example |
|--------------|------------|--------|---------|
| omitted/false | absent | Physical column (direct mapping) | `{"name": "id", "type": "integer"}` |
| false | present | Source-level expression | `{"name": "id_upper", "type": "varchar", "isCalculated": false, "expression": "upper(id)"}` |
| true | present | Calculated column | `{"name": "full_name", "type": "varchar", "isCalculated": true, "expression": "first_name \|\| ' ' \|\| last_name"}` |
| true | absent | ❌ INVALID | Expression is required when isCalculated is true |

## Complete Model Example

```json
{
  "catalog": "wren",
  "schema": "public",
  "models": [
    {
      "name": "orders",
      "tableReference": {
        "schema": "public",
        "table": "raw_orders"
      },
      "columns": [
        // Case 1: Direct physical column (no expression, isCalculated false/omitted)
        {
          "name": "id",
          "type": "integer",
          "notNull": true
        },
        
        // Column alias (rename with isCalculated: false)
        {
          "name": "customer_identifier",
          "type": "integer",
          "isCalculated": false,
          "expression": "cust_id"
        },
        
        // Simple calculation
        {
          "name": "total_with_tax",
          "type": "decimal",
          "isCalculated": true,
          "expression": "subtotal * 1.08"
        },
        
        // Date extraction
        {
          "name": "order_year",
          "type": "integer",
          "isCalculated": true,
          "expression": "extract(year from order_date)"
        },
        
        // String manipulation
        {
          "name": "order_code",
          "type": "varchar",
          "isCalculated": true,
          "expression": "upper(substr(order_id, 1, 3)) || '-' || order_year"
        },
        
        // Relationship navigation
        {
          "name": "customer_tier",
          "type": "varchar",
          "isCalculated": true,
          "expression": "customer.membership.tier_name"
        },
        
        // Complex CASE logic
        {
          "name": "order_priority",
          "type": "varchar",
          "isCalculated": true,
          "expression": "CASE WHEN total_with_tax > 10000 THEN 'VIP' WHEN customer.membership.tier_name = 'Gold' THEN 'High' WHEN days_since_order > 30 THEN 'Urgent' ELSE 'Normal' END"
        },
        
        // Aggregation across relationship
        {
          "name": "items_count",
          "type": "integer",
          "isCalculated": true,
          "expression": "count(order_items.id)"
        },
        
        // Window function
        {
          "name": "customer_order_rank",
          "type": "integer",
          "isCalculated": true,
          "expression": "rank() OVER (PARTITION BY customer_id ORDER BY order_date DESC)"
        }
      ],
      "primaryKey": "id"
    }
  ],
  "relationships": [
    {
      "name": "orders_customer",
      "models": ["orders", "customer"],
      "joinType": "MANY_TO_ONE",
      "condition": "orders.customer_identifier = customer.id"
    },
    {
      "name": "orders_order_items",
      "models": ["orders", "order_items"],
      "joinType": "ONE_TO_MANY",
      "condition": "orders.id = order_items.order_id"
    }
  ]
}
```

## Important Notes

1. **Expression Parsing**: All expressions are parsed using SQL parser and validated during MDL analysis
2. **Relationship Requirements**: When using dot notation to navigate relationships, the relationship must be defined in the MDL
3. **Type Consistency**: The expression result must match the declared column `type`
4. **Performance**: 
   - `isCalculated: false` expressions are evaluated during table scan (more efficient)
   - `isCalculated: true` expressions are evaluated in a separate layer (allows more complex operations)
5. **SQL Dialect**: Expressions should use SQL syntax compatible with your target database

## Validation Logic for Implementation

### Purpose
This section provides comprehensive validation rules for implementing expression validation in the frontend or backend. These rules ensure that MDL expressions are valid and will execute correctly in the Wren Engine.

### Core Validation Rules

#### 1. Basic Structure Validation

```typescript
interface ColumnValidation {
  name: string;           // Required, must be valid identifier
  type: string;           // Required, must be valid SQL type
  expression?: string;    // Optional when isCalculated is false; REQUIRED when isCalculated is true
  isCalculated?: boolean; // Optional, defaults to false
  notNull?: boolean;      // Optional
  relationship?: string;  // Optional, mutually exclusive with expression
}
```

**Rules:**
- `name` must be a valid SQL identifier (alphanumeric + underscore, not starting with number)
- `type` must be a supported SQL data type
- If `relationship` is present, `expression` should NOT be present (they're mutually exclusive)
- **Expression requirement depends on isCalculated:**
  - If `isCalculated: true` → `expression` is REQUIRED (cannot be empty)
  - If `isCalculated: false` or omitted → `expression` is OPTIONAL (can be absent for direct column mapping)

#### 2. Expression Syntax Validation

**For ALL expressions (regardless of isCalculated):**

```javascript
function validateExpressionSyntax(expression, isCalculated) {
  // Step 1: Check if expression is non-empty
  if (!expression || expression.trim() === '') {
    return { valid: false, error: 'Expression cannot be empty' };
  }
  
  // Step 2: Parse as SQL expression
  try {
    // Attempt to parse the expression
    // This would use a SQL parser library or API
    const parsed = parseSQLExpression(expression);
    
    // Step 3: Check for forbidden constructs based on isCalculated
    if (!isCalculated) {
      // For isCalculated: false, check that expression doesn't contain:
      // - Subqueries
      // - CTEs (WITH clauses)
      // - Set operations (UNION, INTERSECT, EXCEPT)
      if (containsSubquery(parsed) || containsCTE(parsed) || containsSetOperation(parsed)) {
        return { 
          valid: false, 
          error: 'Source-level expressions cannot contain subqueries, CTEs, or set operations' 
        };
      }
    }
    
    return { valid: true };
  } catch (parseError) {
    return { valid: false, error: `Invalid SQL syntax: ${parseError.message}` };
  }
}
```

#### 3. Scope Validation

**Different scope rules based on isCalculated:**

```javascript
function validateExpressionScope(expression, isCalculated, model, manifest) {
  const referencedColumns = extractColumnReferences(expression);
  
  if (!isCalculated) {
    // SOURCE-LEVEL EXPRESSION (isCalculated: false)
    // Can ONLY reference columns from the same physical table
    
    for (const ref of referencedColumns) {
      if (ref.includes('.')) {
        // Dot notation not allowed for source-level expressions
        return {
          valid: false,
          error: `Source-level expressions cannot use relationship navigation (found: ${ref}). Set isCalculated: true to use relationships.`
        };
      }
      
      // Check if column exists in the physical table
      // Note: Currently only tableReference is supported. refSql exists but is not implemented.
      if (!model.tableReference || model.tableReference === '') {
        return {
          valid: false,
          error: 'Model must have tableReference for source-level expressions (refSql is not currently supported)'
        };
      }
      
      // Validate column exists in source table
      // This would need to check against actual database schema
      if (!isColumnInPhysicalTable(ref, model.tableReference)) {
        return {
          valid: false,
          error: `Column '${ref}' not found in source table ${model.tableReference}`
        };
      }
    }
  } else {
    // CALCULATED EXPRESSION (isCalculated: true)
    // Can reference relationships and other calculated fields
    
    for (const ref of referencedColumns) {
      if (ref.includes('.')) {
        // Validate relationship navigation
        const parts = ref.split('.');
        if (!validateRelationshipPath(parts, model, manifest)) {
          return {
            valid: false,
            error: `Invalid relationship path: ${ref}`
          };
        }
      } else {
        // Check if column exists in model (including calculated columns)
        const columnExists = model.columns.some(col => col.name === ref);
        if (!columnExists && !isColumnInPhysicalTable(ref, model.tableReference)) {
          return {
            valid: false,
            error: `Column '${ref}' not found in model or source table`
          };
        }
      }
    }
  }
  
  return { valid: true };
}
```

#### 4. Circular Dependency Validation (for isCalculated: true only)

```javascript
function validateNoCircularDependencies(column, model) {
  if (!column.isCalculated || !column.expression) {
    return { valid: true };
  }
  
  const visited = new Set();
  const recursionStack = new Set();
  
  function hasCircularDep(columnName, expression) {
    if (recursionStack.has(columnName)) {
      return true; // Circular dependency detected
    }
    
    if (visited.has(columnName)) {
      return false; // Already validated
    }
    
    visited.add(columnName);
    recursionStack.add(columnName);
    
    // Extract referenced calculated columns from expression
    const referencedCalcColumns = extractCalculatedColumnRefs(expression, model);
    
    for (const refCol of referencedCalcColumns) {
      const refColumn = model.columns.find(c => c.name === refCol);
      if (refColumn?.isCalculated && refColumn.expression) {
        if (hasCircularDep(refCol, refColumn.expression)) {
          return true;
        }
      }
    }
    
    recursionStack.delete(columnName);
    return false;
  }
  
  if (hasCircularDep(column.name, column.expression)) {
    return {
      valid: false,
      error: `Circular dependency detected for column '${column.name}'`
    };
  }
  
  return { valid: true };
}
```

#### 5. Type Compatibility Validation

```javascript
function validateTypeCompatibility(column) {
  if (!column.expression) {
    return { valid: true };
  }
  
  // Map of SQL types to compatible expression patterns
  const typeRules = {
    'integer': ['numeric_expression', 'count', 'sum', 'extract'],
    'decimal': ['numeric_expression', 'sum', 'avg'],
    'varchar': ['string_expression', 'concat', 'upper', 'lower', 'substr'],
    'boolean': ['comparison', 'logical_expression'],
    'date': ['date_expression', 'current_date'],
    'timestamp': ['timestamp_expression', 'current_timestamp', 'cast_to_timestamp']
  };
  
  // Infer expression result type (simplified)
  const inferredType = inferExpressionType(column.expression);
  
  if (!isTypeCompatible(inferredType, column.type)) {
    return {
      valid: false,
      error: `Expression result type '${inferredType}' is not compatible with column type '${column.type}'`
    };
  }
  
  return { valid: true };
}
```

### Comprehensive Validation Function

```javascript
function validateColumnExpression(column, model, manifest) {
  const validations = [];
  
  // 1. Basic structure
  if (!column.name || !column.type) {
    return { valid: false, errors: ['Column must have name and type'] };
  }
  
  // 2. Check expression requirement based on isCalculated
  if (column.isCalculated === true && !column.expression) {
    return { 
      valid: false, 
      errors: ['Expression is required when isCalculated is true'] 
    };
  }
  
  // 3. No expression and not calculated? Valid (physical column)
  if (!column.expression && !column.isCalculated) {
    return { valid: true };
  }
  
  // 4. Relationship columns shouldn't have expressions
  if (column.relationship && column.expression) {
    return { 
      valid: false, 
      errors: ['Column cannot have both relationship and expression'] 
    };
  }
  
  // 5. Validate expression syntax
  const syntaxValidation = validateExpressionSyntax(column.expression, column.isCalculated);
  if (!syntaxValidation.valid) {
    validations.push(syntaxValidation.error);
  }
  
  // 6. Validate scope
  const scopeValidation = validateExpressionScope(
    column.expression, 
    column.isCalculated, 
    model, 
    manifest
  );
  if (!scopeValidation.valid) {
    validations.push(scopeValidation.error);
  }
  
  // 7. Check circular dependencies (only for calculated)
  if (column.isCalculated) {
    const circularValidation = validateNoCircularDependencies(column, model);
    if (!circularValidation.valid) {
      validations.push(circularValidation.error);
    }
  }
  
  // 8. Validate type compatibility
  const typeValidation = validateTypeCompatibility(column);
  if (!typeValidation.valid) {
    validations.push(typeValidation.error);
  }
  
  return {
    valid: validations.length === 0,
    errors: validations
  };
}
```

### Validation Summary Table

| Validation Rule | isCalculated: false | isCalculated: true |
|----------------|--------------------|--------------------|
| **Can use column references** | ✅ Yes (same table only) | ✅ Yes (any visible column) |
| **Can use SQL functions** | ✅ Yes | ✅ Yes |
| **Can use arithmetic** | ✅ Yes | ✅ Yes |
| **Can use CASE/COALESCE** | ✅ Yes | ✅ Yes |
| **Can navigate relationships** | ❌ No | ✅ Yes |
| **Can reference other calculated columns** | ❌ No | ✅ Yes |
| **Can use aggregations** | ⚠️ Limited (no GROUP BY) | ✅ Yes |
| **Can use window functions** | ⚠️ Database dependent | ✅ Yes |
| **Can use subqueries** | ❌ No | ⚠️ Limited |
| **Circular dependencies check** | N/A | ✅ Required |

### Error Messages for User Guidance

```javascript
const ERROR_MESSAGES = {
  EXPRESSION_REQUIRED: "Expression is required when isCalculated is true",
  EMPTY_EXPRESSION: "Expression cannot be empty",
  INVALID_SYNTAX: "Invalid SQL syntax in expression",
  RELATIONSHIP_IN_SOURCE: "Cannot use relationship navigation (e.g., customer.name) with isCalculated: false. Set isCalculated: true to use relationships.",
  COLUMN_NOT_FOUND: "Column '{column}' not found in source table",
  INVALID_RELATIONSHIP: "Relationship '{relationship}' not defined in manifest",
  CIRCULAR_DEPENDENCY: "Circular dependency detected: {path}",
  TYPE_MISMATCH: "Expression returns {actual} but column type is {expected}",
  SUBQUERY_IN_SOURCE: "Subqueries not allowed in source-level expressions (isCalculated: false)",
  AGGREGATION_WITHOUT_GROUP: "Aggregation functions require proper grouping context",
  BOTH_RELATIONSHIP_AND_EXPRESSION: "Column cannot have both relationship and expression fields"
};
```

## Code Reference

The core logic for handling expressions can be found in:
- `/core/src/logical_plan/analyze/plan.rs:685-707` - `get_remote_column_exp` function (handles both calculated and non-calculated)
- `/core/src/logical_plan/analyze/plan.rs:856-890` - Source plan generation (shows how isCalculated affects processing)
- `/core/src/mdl/utils.rs:143-157` - `create_remote_expr_for_model` function (parses any SQL expression)
- `/core/src/mdl/lineage.rs` - Dependency tracking for calculated fields