# OpenCatalog

Automated metadata catalog with column-level lineage, policy governance, and local LLM integration — an open-source alternative to Alation, Collibra, and Unity Catalog.

## Features

- **Auto-crawl** — Discovers metadata from OpenLake (Iceberg REST), OpenIngest (CDC pipelines), and OpenPipe (dbt models)
- **Column-level lineage** — OpenLineage 1.1.0 standard with petgraph-backed DAG traversal
- **Business glossary** — Term management with auto-suggestion from schema heuristics
- **Policy governance** — Masking (hash/redact/nullify/partial/tokenize), row-filter, and access policies evaluated by role
- **LLM integration** — Documentation generation and semantic search via Ollama (local, no API keys)
- **REST API** — Full CRUD for datasources, datasets, glossary, policies, lineage, and search
- **MCP server** — 10 tools for AI agent integration (Cursor, Claude, etc.)
- **CLI** — Command-line management

## Architecture

```
┌──────────┐  ┌───────────┐  ┌─────────┐
│ OpenLake │  │OpenIngest │  │OpenPipe │
│ (crawl)  │  │  (crawl)  │  │ (future)│
└────┬─────┘  └─────┬─────┘  └────┬────┘
     │               │              │
     └───────────────┼──────────────┘
                     ▼
            ┌────────────────┐
            │  OpenCatalog   │
            │  Server :8080  │
            ├────────────────┤
            │ REST API + MCP │
            │ Lineage Graph  │
            │ Policy Engine  │
            │ Glossary Mgmt  │
            │ LLM (Ollama)   │
            └────────────────┘
```

## Quick Start

```bash
# Build
cargo build --release

# Start server
cargo run --release -p opencatalog-server

# Register a data source
curl -X POST http://localhost:8080/api/v1/datasources \
  -H "Content-Type: application/json" \
  -d '{"name":"MyLake","source_type":"openlake","connection_config":{"url":"http://localhost:9090"}}'

# Crawl metadata
curl -X POST http://localhost:8080/api/v1/datasources/<ID>/crawl

# Explore
curl http://localhost:8080/api/v1/datasets
curl http://localhost:8080/api/v1/datasets/<ID>/lineage

# Create a policy
curl -X POST http://localhost:8080/api/v1/policies \
  -H "Content-Type: application/json" \
  -d '{"name":"Mask PII","policy_type":"masking","dataset_pattern":"*.customers","action":"hash","roles":["analyst"]}'

# MCP tools (for AI agents)
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"tool":"catalog_search","arguments":{"query":"customers"}}'
```

## Demo

```bash
cd demo
docker-compose up -d     # Starts OpenLake + OpenIngest + OpenCatalog + Ollama
python3 demo.py           # End-to-end: crawl → lineage → glossary → policy → LLM
```

## Workspace

| Crate | Description |
|-------|-------------|
| `opencatalog-core` | Types, traits, errors |
| `opencatalog-store` | In-memory (SQLite planned) storage |
| `opencatalog-crawler` | OpenLake + OpenIngest crawlers |
| `opencatalog-lineage` | OpenLineage parsing, column-level lineage |
| `opencatalog-glossary` | Business term suggestion and mapping |
| `opencatalog-policy` | Masking/access policy evaluator |
| `opencatalog-llm` | Ollama client for doc gen + semantic search |
| `opencatalog-server` | Axum REST API + MCP |
| `opencatalog-cli` | Clap CLI |
| `opencatalog-tests` | Integration tests |

## Tests

```bash
cargo test --workspace   # 231+ tests across all crates
```

## License

Apache 2.0
