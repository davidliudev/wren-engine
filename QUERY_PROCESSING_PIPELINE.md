# Wren Engine Query Processing Pipeline

This document provides an in-depth explanation of how Wren Engine processes queries through its 5-step pipeline, transforming raw SQL queries with semantic models (MDL) into optimized, dialect-specific SQL executed on target data sources.

## Pipeline Overview

```
Raw SQL + MDL → Semantic Analysis → Logical Plan → Optimized Plan → Dialect SQL → Data Source Execution
      ↓                ↓                ↓              ↓             ↓
   wren-core     Apache DataFusion   wren-core     SQLGlot    Ibis/connectors
```

---

## Step 1: Raw SQL + MDL → Semantic Analysis (wren-core)

### Purpose
Transform raw SQL queries using MDL (Model Definition Language) to understand business context and semantic relationships.

### Key Components

**Entry Point**: `wren-core/core/src/mdl/mod.rs:378` - `transform_sql_with_ctx()`

**Core Files**:
- `wren-core/core/src/mdl/context.rs` - Context creation and mode handling
- `wren-core/core/src/logical_plan/analyze/plan.rs` - Plan node definitions  
- `wren-core/core/src/logical_plan/analyze/model_anlayze.rs` - Model analysis rules

### Process Flow

1. **MDL Analysis**: Parse and analyze the manifest to understand model relationships
   ```rust
   let analyzed_mdl = Arc::new(AnalyzedWrenMDL::analyze(
       manifest,
       Arc::clone(&properties), 
       Mode::Unparse,
   )?);
   ```

2. **Context Creation**: Create DataFusion session context with MDL awareness
   ```rust
   let ctx = create_ctx_with_mdl(
       ctx,
       Arc::clone(&analyzed_mdl),
       Arc::clone(&properties),
       Mode::Unparse,
   ).await?;
   ```

3. **Model Registration**: Register semantic models as virtual tables
   ```rust
   register_table_with_mdl(&ctx, analyzed_mdl.wren_mdl(), properties, mode).await?;
   ```

4. **SQL Parsing**: Parse raw SQL into DataFusion logical plan
   ```rust
   let plan = ctx.state().create_logical_plan(sql).await?;
   ```

### Key Data Structures

- **`AnalyzedWrenMDL`**: Contains parsed manifest with lineage information
- **`ModelPlanNode`**: Represents a logical model with required expressions and relation chains
- **`RelationChain`**: Manages relationships between models and joins

### Example Transformation

**Input SQL**: `SELECT customer_state FROM wren.tpch.orders_model`

**MDL Definition**:
```json
{
  "name": "orders_model",
  "columns": [
    {
      "name": "customer_state",
      "type": "varchar",
      "calculated": true,
      "expression": "customers_model.state"
    }
  ]
}
```

**Result**: Creates a `ModelPlanNode` that understands the relationship between `orders_model` and `customers_model`.

---

## Step 2: Semantic SQL → Logical Plan (Apache DataFusion)

### Purpose
Convert semantically-enriched SQL into DataFusion logical plans using custom Wren plan nodes.

### Key Components

**DataFusion Integration**: Uses DataFusion's SQL parser and logical plan builder

**Core Files**:
- `wren-core/core/src/logical_plan/analyze/model_generation.rs` - Model plan generation

### Process Flow

1. **Custom Plan Nodes**: Wren extends DataFusion with custom logical plan nodes
   ```rust
   pub(crate) enum WrenPlan {
       Calculation(Arc<CalculationPlanNode>),
   }
   ```

2. **Model Plan Generation**: Convert models to logical plans with relationships
   ```rust
   impl UserDefinedLogicalNodeCore for ModelPlanNode {
       fn name(&self) -> &str { "Model" }
       fn inputs(&self) -> Vec<&LogicalPlan> { vec![] }
       fn schema(&self) -> &DFSchemaRef { &self.schema_ref }
   }
   ```

3. **Relationship Planning**: Plan joins between models through RelationChain
   ```rust
   let (source_plan, alias) = model_plan.relation_chain.clone().plan(
       ModelGenerationRule::new(/*...*/),
       &alias_generator,
   )?;
   ```

### DataFusion Components Used

- **`LogicalPlan`**: DataFusion's core logical plan representation
- **`LogicalPlanBuilder`**: For building complex query plans
- **`Extension`**: For custom Wren plan nodes
- **`UserDefinedLogicalNode`**: Interface for custom plan nodes

### Example Transformation

**Input**: `ModelPlanNode` for `orders_model` with `customer_state` calculation

**Result**: DataFusion logical plan with explicit join between `orders` and `customers` tables.

---

## Step 3: Logical Plan → Optimized Plan (wren-core optimizations)

### Purpose
Apply Wren-specific optimizations and resolve semantic relationships.

### Key Components

**Core Files**:
- `wren-core/core/src/mdl/context.rs:195` - `analyze_rule_for_unparsing()`
- `wren-core/core/src/logical_plan/optimize/mod.rs` - Optimization rules

### Process Flow

1. **Analyzer Rules Sequence** (Applied in order):
   ```rust
   vec![
       Arc::new(ExpandWildcardRule::new()),
       Arc::new(ExpandWrenViewRule::new(/*...*/)),
       Arc::new(ModelAnalyzeRule::new(/*...*/)),
       Arc::new(ModelGenerationRule::new(/*...*/)),
       Arc::new(InlineTableScan::new()),
       Arc::new(TimestampSimplify::new()),
       Arc::new(TypeCoercion::new()),
   ]
   ```

2. **Model Analysis**: `ModelAnalyzeRule` resolves model dependencies
   ```rust
   impl AnalyzerRule for ModelAnalyzeRule {
       fn analyze(&self, plan: LogicalPlan, _: &ConfigOptions) -> Result<LogicalPlan> {
           // Analyze model dependencies and create Wren plan nodes
       }
   }
   ```

3. **Optimization Application**: Apply optimizations to the logical plan
   ```rust
   let analyzed = ctx.state().optimize(&plan)?;
   ```

### Current Optimization Rules

- **`ExpandWildcardRule`**: Expands `SELECT *` to explicit columns
- **`ExpandWrenViewRule`**: Expands Wren views to underlying queries
- **`ModelAnalyzeRule`**: Analyzes model dependencies and relationships
- **`ModelGenerationRule`**: Generates executable plans from model definitions
- **`TimestampSimplify`**: Simplifies timestamp operations
- **`TypeCoercion`**: Handles type conversions

### Example Transformation

**Input**: Logical plan with `ModelPlanNode` references

**Result**: Optimized plan with explicit joins, resolved relationships, and applied access controls.

---

## Step 4: Optimized Plan → Dialect-specific SQL (SQLGlot)

### Purpose
Convert optimized logical plans back to SQL and translate to target data source dialect.

### Key Components

**Core Files**:
- `ibis-server/app/mdl/rewriter.py:48` - `_transpile()`
- `wren-core/core/src/mdl/dialect/wren_dialect.rs` - Wren dialect definition

### Process Flow

1. **Plan to SQL Conversion**: DataFusion unparser converts logical plan to SQL
   ```rust
   let data_source = analyzed_mdl.wren_mdl().data_source().unwrap_or_default();
   let wren_dialect = WrenDialect::new(&data_source);
   let unparser = Unparser::new(&wren_dialect).with_pretty(true);
   ```

2. **Dialect Translation**: SQLGlot transpiles between SQL dialects
   ```python
   def _transpile(self, planned_sql: str) -> str:
       read = self._get_read_dialect(self.experiment)
       write = self._get_write_dialect(self.data_source)
       return sqlglot.transpile(planned_sql, read=read, write=write)[0]
   ```

3. **Dialect Mapping**: Map data sources to SQL dialects
   ```python
   @classmethod
   def _get_write_dialect(cls, data_source: DataSource) -> str:
       if data_source == DataSource.canner:
           return "trino"
       elif data_source in {DataSource.local_file, DataSource.s3_file, /*...*/}:
           return "duckdb"
       return data_source.name
   ```

### Supported Dialects

- **BigQuery**: Google BigQuery SQL
- **PostgreSQL**: PostgreSQL SQL  
- **DuckDB**: DuckDB SQL (for file sources)
- **Trino**: Trino SQL (for Canner)
- **MySQL, Oracle, Snowflake, etc.**

### Example Transformation

**Input**: Optimized logical plan with joins

**Result**: Dialect-specific SQL:
```sql
SELECT c.state as customer_state 
FROM orders o 
JOIN customers c ON o.customer_id = c.id
```

---

## Step 5: Final SQL → Data Source Execution (Ibis/connectors)

### Purpose
Execute the final SQL query against the target data source and return results.

### Key Components

**Core Files**:
- `ibis-server/app/model/connector.py` - Connector implementations
- `ibis-server/app/routers/v3/connector.py` - Query execution endpoint

### Process Flow

1. **Connector Selection**: Choose appropriate connector based on data source
   ```python
   def __init__(self, data_source: DataSource, connection_info: ConnectionInfo):
       if data_source == DataSource.mssql:
           self._connector = MSSqlConnector(connection_info)
       elif data_source == DataSource.bigquery:
           self._connector = BigQueryConnector(connection_info)
       # ... other connectors
   ```

2. **Query Execution**: Execute SQL using Ibis
   ```python
   def query(self, sql: str, limit: int) -> pa.Table:
       ibis_table = self.connection.sql(sql).limit(limit)
       ibis_table = round_decimal_columns(ibis_table)
       return ibis_table.to_pyarrow()
   ```

3. **Result Processing**: Convert results to PyArrow tables and JSON response
   ```python
   response = ORJSONResponse(to_json(result, headers, data_source=data_source))
   ```

### Error Handling

- **Dry Run Validation**: Validates SQL before execution
- **Fallback Mechanism**: Falls back to v2 API if v3 fails
- **Connection Pooling**: Manages database connections efficiently

### Example Transformation

**Input**: Dialect-specific SQL query

**Result**: PyArrow table with query results returned as JSON response.

---

## Complete Example Flow

### Input Query
```sql
SELECT customer_state FROM wren.tpch.orders_model
```

### MDL Definition
```json
{
  "name": "orders_model",
  "columns": [
    {
      "name": "customer_state", 
      "type": "varchar",
      "calculated": true,
      "expression": "customers_model.state"
    }
  ],
  "relationships": [
    {
      "name": "customer_relationship",
      "models": ["orders_model", "customers_model"],
      "joinType": "INNER",
      "condition": "orders_model.customer_id = customers_model.id"
    }
  ]
}
```

### Processing Steps

1. **Step 1**: Parse SQL and MDL, create `ModelPlanNode` for `orders_model`
2. **Step 2**: Convert to DataFusion logical plan with join to `customers_model`
3. **Step 3**: Optimize plan, resolve relationships, apply access controls
4. **Step 4**: Generate SQL with proper joins:
   ```sql
   SELECT c.state as customer_state 
   FROM orders o 
   JOIN customers c ON o.customer_id = c.id
   ```
5. **Step 5**: Execute via Ibis connector, return results as PyArrow table

### Key Integration Points

- **Python-Rust Bridge**: `wren-core-py/src/lib.rs` - PyO3 bindings
- **Session Context**: `wren-core-py/src/context.rs` - Python wrapper for Rust context  
- **Manifest Processing**: `wren-core-py/src/manifest.rs` - MDL handling

---

## Architecture Benefits

This pipeline architecture provides:

1. **Semantic Understanding**: Transforms business logic into executable SQL
2. **Dialect Agnostic**: Supports multiple data source types
3. **Optimization**: Applies semantic-aware optimizations
4. **Security**: Enforces access controls and governance
5. **Extensibility**: Pluggable components for customization

The pipeline ensures that business logic defined in MDL is correctly translated into efficient, dialect-specific SQL queries while maintaining semantic correctness and applying proper access controls.