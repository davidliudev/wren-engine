How the Query Processing Pipeline Works

  Based on the code and documentation, here's how the 5-step query
  processing pipeline works:

  Step 1: Raw SQL + MDL → Semantic Analysis

  Entry Point: wren-core/core/src/mdl/mod.rs:378 -
  transform_sql_with_ctx()

  pub async fn transform_sql_with_ctx(
      ctx: &SessionContext,
      analyzed_mdl: Arc<AnalyzedWrenMDL>,
      remote_functions: &[RemoteFunction],
      properties: SessionPropertiesRef,
      sql: &str,
  ) -> Result<String>

  Process:
  1. Creates DataFusion session context with MDL awareness
  2. Registers semantic models as virtual tables
  3. Parses raw SQL into logical plan with semantic nodes like
  ModelPlanNode

  Step 2: Semantic SQL → Logical Plan

  Core Components: Plan nodes in
  wren-core/core/src/logical_plan/analyze/plan.rs

  Key Data Structures:
  - ModelPlanNode - Represents semantic models with relationships
  - ModelSourceNode - Handles physical data access
  - CalculationPlanNode - Manages calculated fields and aggregations
  - RelationChain - Connects models through relationships

  Step 3: Logical Plan → Optimized Plan

  Analyzer Rules in wren-core/core/src/mdl/context.rs:195:

  vec![
      Arc::new(ExpandWildcardRule::new()),
      Arc::new(ExpandWrenViewRule::new(/*...*/)),
      Arc::new(ModelAnalyzeRule::new(/*...*/)),
      Arc::new(ModelGenerationRule::new(/*...*/)),
      Arc::new(InlineTableScan::new()),
      Arc::new(TimestampSimplify::new()),
      Arc::new(TypeCoercion::new()),
  ]

  Process:
  1. ModelAnalyzeRule - Analyzes dependencies and creates semantic
  plan nodes
  2. ModelGenerationRule - Converts semantic models to executable
  plans with joins
  3. Applies access control (RLAC/CLAC) and optimizations

  Step 4: Optimized Plan → Dialect SQL

  Core Logic: ibis-server/app/mdl/rewriter.py:48 - _transpile()

  def _transpile(self, planned_sql: str) -> str:
      read = self._get_read_dialect(self.experiment)
      write = self._get_write_dialect(self.data_source)
      return sqlglot.transpile(planned_sql, read=read,
  write=write)[0]

  Process:
  1. DataFusion unparser converts optimized logical plan to SQL
  2. SQLGlot transpiles to target dialect (PostgreSQL, BigQuery,
  etc.)

  Step 5: Final SQL → Data Source Execution

  Execution: Ibis connectors execute dialect-specific SQL and return
  PyArrow tables

  Key Transformations

  The pipeline transforms this semantic query:
  SELECT customer_state FROM wren.tpch.orders_model

  Into this physical SQL:
  SELECT c.state as customer_state
  FROM orders o
  JOIN customers c ON o.customer_id = c.id

  The semantic layer automatically resolves relationships, applies
  access controls, and generates optimized joins based on the MDL
  manifest definitions.