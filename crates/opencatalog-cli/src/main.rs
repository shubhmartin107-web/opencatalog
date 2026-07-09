use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "opencatalog", about = "OpenCatalog — Automated Metadata Catalog")]
struct Cli {
    #[command(subcommand)]
    command: Command,
    /// Server URL (default: http://localhost:8080)
    #[arg(short, long, default_value = "http://localhost:8080")]
    server: String,
}

#[derive(Subcommand)]
enum Command {
    /// List all registered data sources
    ListSources,
    /// Register a new data source
    RegisterSource {
        name: String,
        #[arg(short, long)]
        source_type: String,
        #[arg(short, long)]
        url: String,
    },
    /// Trigger a crawl for a data source
    Crawl {
        datasource_id: String,
    },
    /// List all datasets
    ListDatasets {
        #[arg(short, long)]
        datasource_id: Option<String>,
    },
    /// Get lineage for a dataset
    Lineage {
        dataset_id: String,
    },
    /// List all governance policies
    ListPolicies,
    /// Create a masking policy
    CreatePolicy {
        name: String,
        #[arg(short, long)]
        policy_type: String,
        #[arg(short, long)]
        dataset_pattern: String,
        #[arg(short, long)]
        action: String,
        #[arg(short, long)]
        roles: String,
    },
    /// List business glossary terms
    ListGlossary,
    /// Search the catalog
    Search {
        query: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();
    let base = cli.server.trim_end_matches('/');

    match cli.command {
        Command::ListSources => {
            let resp = client.get(format!("{base}/api/v1/datasources")).send().await?;
            let sources: Vec<serde_json::Value> = resp.json().await?;
            for s in &sources {
                println!("[{}] {} ({})", s["id"].as_str().unwrap_or("?"), s["name"].as_str().unwrap_or("?"), s["source_type"].as_str().unwrap_or("?"));
            }
        }
        Command::RegisterSource { name, source_type, url } => {
            let body = serde_json::json!({
                "name": name,
                "source_type": source_type,
                "connection_config": { "url": url },
            });
            let resp = client.post(format!("{base}/api/v1/datasources")).json(&body).send().await?;
            let result: serde_json::Value = resp.json().await?;
            println!("Registered: {}", serde_json::to_string_pretty(&result)?);
        }
        Command::Crawl { datasource_id } => {
            let resp = client.post(format!("{base}/api/v1/datasources/{datasource_id}/crawl")).send().await?;
            let result: serde_json::Value = resp.json().await?;
            println!("Crawl result: {}", serde_json::to_string_pretty(&result)?);
        }
        Command::ListDatasets { datasource_id } => {
            let url = match datasource_id {
                Some(id) => format!("{base}/api/v1/datasets?datasource_id={id}"),
                None => format!("{base}/api/v1/datasets"),
            };
            let resp = client.get(&url).send().await?;
            let datasets: Vec<serde_json::Value> = resp.json().await?;
            for ds in &datasets {
                println!("[{}] {} ({} cols)", ds["id"].as_str().unwrap_or("?"), ds["name"].as_str().unwrap_or("?"), ds["schema"].as_array().map(|a| a.len()).unwrap_or(0));
            }
        }
        Command::Lineage { dataset_id } => {
            let resp = client.get(format!("{base}/api/v1/datasets/{dataset_id}/lineage")).send().await?;
            let result: serde_json::Value = resp.json().await?;
            println!("Lineage: {}", serde_json::to_string_pretty(&result)?);
        }
        Command::ListPolicies => {
            let resp = client.get(format!("{base}/api/v1/policies")).send().await?;
            // Note: policies endpoint not yet implemented in server, will add
            println!("Policies: {}", resp.text().await?);
        }
        Command::CreatePolicy { name, policy_type, dataset_pattern, action, roles } => {
            let roles_vec: Vec<String> = roles.split(',').map(|s| s.trim().into()).collect();
            let body = serde_json::json!({
                "name": name,
                "policy_type": policy_type,
                "dataset_pattern": dataset_pattern,
                "action": action,
                "roles": roles_vec,
            });
            let resp = client.post(format!("{base}/api/v1/policies")).json(&body).send().await?;
            let result: serde_json::Value = resp.json().await?;
            println!("Created: {}", serde_json::to_string_pretty(&result)?);
        }
        Command::ListGlossary => {
            let resp = client.get(format!("{base}/api/v1/glossary")).send().await?;
            println!("Glossary: {}", resp.text().await?);
        }
        Command::Search { query } => {
            let resp = client.get(format!("{base}/api/v1/search?q={query}")).send().await?;
            let result: serde_json::Value = resp.json().await?;
            println!("Results: {}", serde_json::to_string_pretty(&result)?);
        }
    }

    Ok(())
}
