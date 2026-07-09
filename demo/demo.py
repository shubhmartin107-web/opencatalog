#!/usr/bin/env python3
"""
OpenCatalog Demo — End-to-end metadata catalog with lineage and policy governance.

Prerequisites:
  - docker-compose up (starts OpenLake, OpenIngest, OpenCatalog, Ollama)
  - ollama pull llama3.2 (first time)

Flow:
  1. Register OpenLake as a data source and crawl it
  2. Register OpenIngest as a data source and crawl it
  3. View column-level lineage
  4. Create a glossary term and map it
  5. Create a masking policy
  6. Evaluate policy against a query
  7. Generate documentation via LLM
  8. Semantic search
"""

import json
import time
import urllib.request
import urllib.error

CATALOG_URL = "http://localhost:8080"

def api(path, method="GET", data=None):
    url = f"{CATALOG_URL}{path}"
    body = json.dumps(data).encode() if data else None
    req = urllib.request.Request(url, data=body, method=method)
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req) as resp:
            return json.loads(resp.read().decode())
    except urllib.error.HTTPError as e:
        return {"error": e.code, "body": e.read().decode()}

def wait_for_service():
    print("Waiting for OpenCatalog...")
    for i in range(30):
        try:
            r = api("/api/v1/health")
            if r.get("status") == "ok":
                print("  OpenCatalog is ready!")
                return
        except Exception:
            pass
        time.sleep(2)
    raise RuntimeError("OpenCatalog did not start in time")

def main():
    wait_for_service()

    # ── Step 1: Register OpenLake ──
    print("\n[1] Registering OpenLake data source...")
    r = api("/api/v1/datasources", "POST", {
        "name": "OpenLake Production",
        "source_type": "openlake",
        "connection_config": {"url": "http://openlake:9090"},
    })
    openlake_id = r.get("id", "unknown")
    print(f"  Registered: {openlake_id}")

    # ── Step 2: Crawl OpenLake ──
    print("\n[2] Crawling OpenLake metadata...")
    r = api(f"/api/v1/datasources/{openlake_id}/crawl", "POST")
    print(f"  Crawl status: {r.get('status')}, datasets found: {r.get('datasets_found')}")

    # ── Step 3: List discovered datasets ──
    print("\n[3] Discovered datasets:")
    r = api(f"/api/v1/datasets?datasource_id={openlake_id}")
    datasets = r if isinstance(r, list) else []
    for ds in datasets:
        print(f"  - {ds['name']} ({len(ds.get('schema', []))} columns)")
    first_dataset_id = datasets[0]["id"] if datasets else None

    # ── Step 4: Register OpenIngest ──
    print("\n[4] Registering OpenIngest data source...")
    r = api("/api/v1/datasources", "POST", {
        "name": "OpenIngest CDC Pipelines",
        "source_type": "openingest",
        "connection_config": {"url": "http://openingest:7080"},
    })
    openingest_id = r.get("id", "unknown")
    print(f"  Registered: {openingest_id}")

    # ── Step 5: Crawl OpenIngest ──
    print("\n[5] Crawling OpenIngest metadata...")
    r = api(f"/api/v1/datasources/{openingest_id}/crawl", "POST")
    print(f"  Crawl status: {r.get('status')}, datasets found: {r.get('datasets_found')}")

    # ── Step 6: View lineage ──
    if first_dataset_id:
        print(f"\n[6] Viewing lineage for dataset {first_dataset_id}...")
        r = api(f"/api/v1/datasets/{first_dataset_id}/lineage")
        print(f"  Lineage: {json.dumps(r, indent=2)[:300]}...")

    # ── Step 7: Create a glossary term ──
    print("\n[7] Creating glossary term 'Customer PII'...")
    r = api("/api/v1/glossary", "POST", {
        "name": "Customer PII",
        "description": "Personally identifiable information about customers",
        "domain": "Compliance",
    })
    term_id = r.get("id", "unknown")
    print(f"  Created term: {term_id}")

    # ── Step 8: Create a masking policy ──
    print("\n[8] Creating masking policy...")
    r = api("/api/v1/policies", "POST", {
        "name": "Mask PII for Analysts",
        "policy_type": "masking",
        "dataset_pattern": "*.customers",
        "action": "hash",
        "roles": ["analyst"],
    })
    policy_id = r.get("id", "unknown")
    print(f"  Created policy: {policy_id}")

    # ── Step 9: List all policies ──
    print("\n[9] Listing active policies...")
    r = api("/api/v1/policies")
    print(f"  {json.dumps(r, indent=2)}")

    # ── Step 10: List glossary terms ──
    print("\n[10] Listing glossary terms...")
    r = api("/api/v1/glossary")
    print(f"  {json.dumps(r, indent=2)}")

    # ── Step 11: Search ──
    print("\n[11] Searching for 'customer'...")
    r = api("/api/v1/search?q=customer")
    print(f"  Found {len(r.get('results', [])) if isinstance(r, dict) else 0} results")

    # ── Step 12: Generate docs via LLM ──
    if first_dataset_id:
        print(f"\n[12] Generating documentation for dataset via LLM...")
        # Use MCP for doc generation
        r = api("/mcp", "POST", {
            "tool": "catalog_doc_generate",
            "arguments": {"dataset_id": first_dataset_id},
        })
        if r.get("success"):
            print(f"  Generated docs: {json.dumps(r['data'], indent=2)}")
        else:
            print(f"  LLM not available (expected if Ollama not running): {r.get('error')}")

    # ── Step 13: Semantic search via LLM ──
    print("\n[13] Semantic search for 'customer contact data'...")
    r = api("/mcp", "POST", {
        "tool": "catalog_semantic_search",
        "arguments": {"query": "customer contact data"},
    })
    if r.get("success"):
        print(f"  Results: {json.dumps(r['data'], indent=2)[:300]}")
    else:
        print(f"  LLM not available: {r.get('error')}")

    # ── Step 14: MCP tool listing ──
    print("\n[14] Listing MCP tools...")
    r = api("/mcp", "POST", {"tool": "list_tools", "arguments": {}})
    if isinstance(r, dict) and isinstance(r.get("data"), list):
        for tool in r["data"]:
            print(f"  - {tool['name']}: {tool['description']}")

    print("\n✅ Demo complete! OpenCatalog is running with full metadata catalog, lineage, glossary, policies, and LLM integration.")


if __name__ == "__main__":
    main()
