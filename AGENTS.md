# OpenCatalog — Agent Instructions

## Overview

OpenCatalog is an automated metadata catalog with column-level lineage, policy governance, and local LLM integration. It crawls data sources (OpenLake, OpenIngest), builds an OpenLineage-compatible lineage graph, manages a business glossary, enforces access/masking policies, and provides an MCP server for AI tool access.

## Code Layout

```
crates/
  opencatalog-core/       # Core types, traits, errors
  opencatalog-store/      # In-memory + SQLite storage (CatalogStore trait)
  opencatalog-crawler/    # Crawler trait + OpenLake/OpenIngest crawlers
  opencatalog-lineage/    # OpenLineage event parsing + column-level lineage
  opencatalog-glossary/   # Glossary term suggestion and mapping
  opencatalog-policy/     # Policy evaluator with masking transforms
  opencatalog-llm/        # Ollama client for doc gen + semantic search
  opencatalog-server/     # Axum REST API + MCP server
  opencatalog-cli/        # Clap CLI binary
tests/integration/        # Integration tests
demo/                     # Docker Compose + Python demo
docs/                     # Architecture guide + getting started
```

## Architecture

```
[OpenLake] ──crawl──┐
[OpenIngest]─crawl──┼──> [CatalogStore] ──> [REST API / MCP]
                    │         │
                    │    [LineageEngine] (petgraph + OpenLineage)
                    │    [GlossaryEngine]
                    │    [PolicyEngine]  (masking/filter/deny)
                    │    [LLM Client]    (Ollama for docs + search)
```

## Key Traits

- `CatalogStore`: Storage abstraction (in-memory, SQLite, Postgres)
- `Crawler`: Source-specific metadata crawler
- `PolicyEngine`: Policy evaluation against queries/roles
- `LlmClient`: Local LLM integration (Ollama)

## Adding a New Crawler

1. Create a new module in `crates/opencatalog-crawler/src/`
2. Implement a crawl function that returns `CrawlResult`
3. Register in `registry.rs` by matching on `SourceType`

## Adding a New REST Endpoint

1. Add handler to `crates/opencatalog-server/src/rest/`
2. Add route in `crates/opencatalog-server/src/main.rs`
3. Add MCP tool in `crates/opencatalog-server/src/mcp/tools.rs`

## Common Tasks

- `cargo check --workspace` — Verify compilation
- `cargo test --workspace` — Run all tests
- `cargo run -p opencatalog-server` — Start the server
- `cargo run -p opencatalog-cli -- --help` — CLI help
- `cargo doc --workspace --no-deps --open` — Build docs

## Database / Storage

By default, uses `MemoryCatalogStore` for development. SQLite support is available via the `opencatalog-store` crate's `sqlite` module (future). For production, implement `CatalogStore` against PostgreSQL.

## Lineage / OpenLineage

All lineage processing follows the OpenLineage 1.1.0 spec. Column-level lineage is encoded in the `columnLineage` facet of output datasets. The `extract_column_lineage` function parses these events.

## Policy Engine

Policies are matched by dataset glob pattern + column regex + role. The evaluator returns applicable transforms. Masking methods: `Redact`, `Hash`, `Nullify`, `Partial(n)`, `Tokenize`, `Sha256`, `Mask`, `Custom`.

## LLM Integration

Requires Ollama running on `localhost:11434`. The default model is `llama3.2`. Embeddings are used for semantic search with cosine similarity.
