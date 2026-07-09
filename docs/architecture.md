# OpenCatalog Architecture

## System Overview

OpenCatalog is an automated metadata catalog that discovers, organizes, and governs data assets across the OpenLake ecosystem. It provides column-level lineage, a business glossary, policy-based access control, and LLM-powered documentation and search.

## Architecture Diagram

```
┌─────────────┐    ┌──────────────┐    ┌─────────────┐
│  OpenLake   │    │  OpenIngest  │    │  OpenPipe   │
│  (Iceberg)  │    │   (CDC/PII)  │    │   (Models)  │
└──────┬──────┘    └──────┬───────┘    └──────┬──────┘
       │                  │                    │
       │  Iceberg REST    │  REST API          │  OpenLineage
       │  API             │  + Pipeline Spec   │  Events
       ▼                  ▼                    ▼
┌─────────────────────────────────────────────────────┐
│                  OpenCatalog Server                  │
│  ┌─────────┐  ┌──────────┐  ┌────────┐  ┌───────┐  │
│  │ Crawler │  │  Lineage │  │Glossary│  │Policy │  │
│  │Registry │  │  Engine  │  │ Engine │  │Engine │  │
│  └────┬────┘  └────┬─────┘  └───┬────┘  └──┬────┘  │
│       │            │            │          │        │
│  ┌────┴────────────┴────────────┴──────────┴────┐   │
│  │              CatalogStore                    │   │
│  │     (In-Memory / SQLite / PostgreSQL)        │   │
│  └──────────────────────────────────────────────┘   │
│  ┌──────────────┐  ┌──────────────┐                 │
│  │  REST API    │  │  MCP Server  │                 │
│  │  (Axum)      │  │  (Axum)      │                 │
│  └──────┬───────┘  └──────┬───────┘                 │
└─────────┼─────────────────┼─────────────────────────┘
          │                 │
          ▼                 ▼
    ┌────────────┐    ┌──────────┐
    │  CLI / Web │    │  AI Agent│
    │  Dashboard │    │  (Cursor)│
    └────────────┘    └──────────┘
```

## Component Details

### CatalogStore
Stores all catalog metadata. The `CatalogStore` trait abstracts the storage backend. Currently implements:
- **InMemory**: HashMap + petgraph-based, for development/testing
- **SQLite** (planned): rusqlite-backed for single-node deployments
- **PostgreSQL** (planned): for production HA deployments

### Crawler Framework
Each data source type has a corresponding crawler that implements a crawl function returning `CrawlResult`. The registry dispatches to the correct crawler based on `SourceType`.

**OpenLake Crawler**: Connects to the Iceberg REST API (`/api/v1/namespaces/{ns}/tables`) to discover namespaces, tables, and their schemas.

**OpenIngest Crawler**: Connects to the OpenIngest REST API (`/api/v1/pipelines`) to discover pipelines, source/target schemas, and PII masking rules. Records lineage edges between source and target datasets.

### Lineage Engine
Built on petgraph's `DiGraph`. Supports:
- Dataset-level lineage (edges between datasets)
- Column-level lineage via OpenLineage `columnLineage` facet
- Upstream/downstream traversal
- Impact analysis

### Policy Engine
Evaluates governance policies against query contexts:
1. Match dataset name against glob patterns
2. Match column names against regex patterns
3. Check role membership
4. Return applicable transformations (masking, filter, deny)

### LLM Integration
Connects to Ollama via HTTP API for:
- **Documentation generation**: Generates business descriptions for columns
- **Semantic search**: Embeds queries and dataset descriptions, finds by cosine similarity

## Data Flow: End-to-End Lineage

```
Source DB (Postgres)
  │
  │  OpenIngest CDC pipeline
  │  ├── Reads `customers` table
  │  ├── Applies PII masking on `email`, `ssn`
  │  └── Writes to `lakehouse.masked_customers`
  │
  ▼
OpenLake (Iceberg table)
  │
  │  OpenPipe model
  │  ├── SELECT * FROM masked_customers
  │  ├── JOIN with orders
  │  └── CREATE analytics.customer_orders
  │
  ▼
analytics.customer_orders
```

OpenCatalog captures this chain by crawling both OpenIngest (source→target lineage) and OpenPipe (OpenLineage events with column-level facets).

## OpenLineage Standard

OpenCatalog is an OpenLineage consumer and producer. It:
- **Consumes** OpenLineage `COMPLETE` events from OpenPipe and OpenIngest
- **Parses** the `columnLineage` facet for column-level lineage
- **Stores** lineage as a directed graph
- **Produces** OpenLineage events for catalog operations

Schema version: 1.1.0 (https://openlineage.io/spec/1-1-0/OpenLineage.json)
