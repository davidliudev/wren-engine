# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Wren Engine is a semantic engine for MCP (Model Context Protocol) clients and AI agents. It provides a semantic layer that enables AI agents to understand and query business data with proper context, governance, and accuracy across various data sources (PostgreSQL, MySQL, BigQuery, Snowflake, etc.).

## Architecture

### Core Components (4 main modules)

1. **`mcp-server/`** - MCP server providing tools for manifest management and query execution
2. **`ibis-server/`** - Main FastAPI web server with v2/v3 APIs for query processing  
3. **`wren-core-py/`** - Python bindings for the Rust semantic engine
4. **`wren-core/`** - Core semantic processing engine (Rust) powered by Apache DataFusion

### Data Flow
```
MCP Client → mcp-server → ibis-server → wren-core-py → wren-core → Data Source
```

### Key Architecture Patterns

**Semantic Layer Processing:**
- MDL (Modeling Definition Language) defines semantic models in JSON
- Two-phase query processing: analyze (semantic analysis) → optimize (query optimization)
- Manifest contains models, relationships, columns, metrics, views with lineage tracking

**Query Processing Pipeline:**
1. Raw SQL + MDL → Semantic Analysis (wren-core)
2. Semantic SQL → Logical Plan (Apache DataFusion) 
3. Logical Plan → Optimized Plan (wren-core optimizations)
4. Optimized Plan → Dialect-specific SQL (SQLGlot)
5. Final SQL → Data Source Execution (Ibis/connectors)

**Security & Governance:**
- Row-Level Access Control (RLAC) and Column-Level Access Control (CLAC)
- Session properties for context-aware security rules

## Development Commands

### Rust Components (wren-core)
```bash
# Build all Rust components
cargo build
cargo test

# Format Rust code
cargo fmt

# Run specific tests
cargo test --package wren-core
```

### Python Components (ibis-server)
```bash
# Navigate to ibis-server directory
cd ibis-server

# Install dependencies (requires wren-core-py wheel)
just install

# Run development server
just dev

# Run production server
just run

# Run tests with marker
just test <MARKER>
just test-verbose <MARKER>

# Lint and format
just lint
just format
```

### wren-core-py (Python bindings)
```bash
cd wren-core-py

# Install dependencies
just install

# Build Python wheel
just build

# Run tests
just test        # Both Rust and Python tests
just test-rs     # Rust tests only
just test-py     # Python tests only

# Format code
just format
```

### MCP Server
```bash
cd mcp-server

# Build wren-core-py dependency
just build-core

# Install core wheel
just install-core
```

## Key File Structure

### wren-core (Rust)
- `core/src/logical_plan/analyze/` - Semantic analysis (access control, model expansion, view expansion)
- `core/src/logical_plan/optimize/` - Query optimization
- `core/src/mdl/` - MDL processing, context management, lineage tracking
- `core/src/mdl/dialect/` - SQL dialect handling
- `sqllogictest/` - SQL end-to-end testing framework

### ibis-server (Python)
- `app/mdl/rewriter.py` - Core query rewriting logic (embedded vs external engine)
- `app/routers/v2/` - Legacy API endpoints
- `app/routers/v3/` - Modern connector-specific API endpoints
- `app/model/metadata/` - Data source metadata extractors
- `tests/routers/v3/connector/` - Connector-specific tests

### wren-core-base
- `src/mdl/` - Core MDL data structures (Manifest, Model, Column, Relationship, etc.)
- `manifest-macro/` - Procedural macros for Python/Rust bindings

## Testing

### SQL Logic Tests
```bash
cd wren-core/sqllogictest
cargo test
```

### Integration Tests
```bash
cd ibis-server
just test <connector_name>  # e.g., postgres, bigquery, mysql
```

### Python Tests
```bash
cd wren-core-py
just test-py
```

## Important Development Notes

- **Dual Engine Support**: ibis-server supports both embedded Rust engine (wren-core-py) and external Java engine for backward compatibility
- **Semantic Transformations**: `TableScan` nodes become `ModelPlanNode` with semantic context during analysis
- **Relationship Processing**: Joins handled through `RelationChain` structure with automatic relationship resolution
- **Lineage Tracking**: Automatic dependency resolution between semantic objects (models, columns, metrics)
- **Error Handling**: Rust uses `WrenError` for semantic errors, Python wraps with custom exception hierarchy

## Common Patterns

- **MDL Validation**: Always validate manifest structure before processing
- **Connector Testing**: Each data source has dedicated test suites in `tests/routers/v3/connector/`
- **SQL Dialect Handling**: Use SQLGlot for multi-dialect SQL generation
- **Access Control**: Apply RLAC/CLAC during semantic analysis phase
- **Function Registration**: Remote function support varies by data source connector