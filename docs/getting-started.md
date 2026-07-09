# OpenCatalog Getting Started

## Prerequisites

- Rust 1.88+
- Docker & Docker Compose (for demo)
- Ollama (for LLM features)

## Quick Start

### 1. Build and Run

```bash
cd opencatalog

# Build all crates
cargo build --release

# Start the server
cargo run --release -p opencatalog-server
```

The server starts on `http://localhost:8080`.

### 2. Register a Data Source

```bash
# Register an OpenLake instance
curl -X POST http://localhost:8080/api/v1/datasources \
  -H "Content-Type: application/json" \
  -d '{
    "name": "OpenLake Production",
    "source_type": "openlake",
    "connection_config": {"url": "http://localhost:9090"}
  }'
```

### 3. Crawl Metadata

```bash
# Replace DATASOURCE_ID with the ID from step 2
curl -X POST http://localhost:8080/api/v1/datasources/{DATASOURCE_ID}/crawl
```

### 4. Explore the Catalog

```bash
# List datasets
curl http://localhost:8080/api/v1/datasets

# Get dataset details
curl http://localhost:8080/api/v1/datasets/{DATASET_ID}

# View lineage
curl http://localhost:8080/api/v1/datasets/{DATASET_ID}/lineage

# Search
curl "http://localhost:8080/api/v1/search?q=customer"
```

### 5. Manage Glossary

```bash
# Create a glossary term
curl -X POST http://localhost:8080/api/v1/glossary \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Customer PII",
    "description": "Personally identifiable customer information",
    "domain": "Compliance"
  }'

# List terms
curl http://localhost:8080/api/v1/glossary
```

### 6. Create a Policy

```bash
curl -X POST http://localhost:8080/api/v1/policies \
  -H "Content-Type: application/json" \
  -d '{
    "name": "Mask PII for Analysts",
    "policy_type": "masking",
    "dataset_pattern": "*.customers",
    "action": "hash",
    "roles": ["analyst"]
  }'

# List policies
curl http://localhost:8080/api/v1/policies
```

### 7. Use MCP Tools

```bash
# List available tools
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{"tool": "list_tools", "arguments": {}}'

# Semantic search (requires Ollama)
curl -X POST http://localhost:8080/mcp \
  -H "Content-Type: application/json" \
  -d '{
    "tool": "catalog_semantic_search",
    "arguments": {"query": "find customer contact data"}
  }'
```

## Demo

Run the full end-to-end demo:

```bash
cd demo
docker-compose up -d
python3 demo.py
```

This starts OpenLake (with sample tables), OpenIngest (CDC pipeline with PII masking), and OpenCatalog, then runs the demo script showing:
1. Source registration and crawling
2. Column-level lineage discovery
3. Glossary term creation
4. Policy creation and enforcement
5. LLM documentation generation
6. Semantic search

## CLI

```bash
cargo run -p opencatalog-cli -- --help
cargo run -p opencatalog-cli -- list-sources
cargo run -p opencatalog-cli -- register-source MyLake openlake http://localhost:9090
cargo run -p opencatalog-cli -- crawl <id>
cargo run -p opencatalog-cli -- list-datasets
cargo run -p opencatalog-cli -- search customer
```

## Configuration

| Env Variable | Default | Description |
|---|---|---|
| `CATALOG_HOST` | `127.0.0.1` | Server bind address |
| `CATALOG_PORT` | `8080` | Server port |
| `LLM_BASE_URL` | `http://localhost:11434` | Ollama API URL |
| `LLM_MODEL` | `llama3.2` | Ollama model name |
| `RUST_LOG` | `info` | Log level |

## Project Structure

See [AGENTS.md](../AGENTS.md) for detailed code layout and common tasks.
