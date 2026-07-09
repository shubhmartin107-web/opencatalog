use std::collections::HashMap;

use opencatalog_core::error::{CatalogError, CatalogResult};
use opencatalog_core::types::*;
use serde::Deserialize;

/// Client for Ollama-compatible local LLM API.
pub struct OllamaClient {
    base_url: String,
    model: String,
    client: reqwest::Client,
}

impl OllamaClient {
    pub fn new(base_url: String, model: String) -> Self {
        Self {
            base_url,
            model,
            client: reqwest::Client::new(),
        }
    }

    async fn generate(&self, prompt: &str) -> CatalogResult<String> {
        let url = format!("{}/api/generate", self.base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
        });

        #[derive(Deserialize)]
        struct OllamaResponse {
            response: String,
        }

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CatalogError::LlmError(format!("LLM request failed: {e}")))?;

        let ol_resp: OllamaResponse = resp
            .json()
            .await
            .map_err(|e| CatalogError::LlmError(format!("LLM parse failed: {e}")))?;

        Ok(ol_resp.response)
    }

    async fn embed(&self, texts: &[String]) -> CatalogResult<Vec<Vec<f64>>> {
        let url = format!("{}/api/embed", self.base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": self.model,
            "input": texts,
        });

        #[derive(Deserialize)]
        struct EmbedResponse {
            embeddings: Vec<Vec<f64>>,
        }

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| CatalogError::LlmError(format!("Embed request failed: {e}")))?;

        let emb_resp: EmbedResponse = resp
            .json()
            .await
            .map_err(|e| CatalogError::LlmError(format!("Embed parse failed: {e}")))?;

        Ok(emb_resp.embeddings)
    }

    /// Generates column descriptions for a dataset using the LLM.
    pub async fn generate_documentation(&self, dataset: &Dataset) -> CatalogResult<HashMap<String, String>> {
        let schema_desc: Vec<String> = dataset
            .schema
            .iter()
            .map(|c| format!("  - {}: {} (nullable: {})", c.name, c.column_type, c.is_nullable))
            .collect();

        let prompt = format!(
            r#"You are a data catalog assistant. Generate concise business descriptions for each column in the dataset below.
Dataset: {name}
Columns:
{schema}

Respond with one line per column in the format: column_name: <description>
Be specific and business-focused."#,
            name = dataset.name,
            schema = schema_desc.join("\n"),
        );

        let response = self.generate(&prompt).await?;
        let mut descriptions = HashMap::new();

        for line in response.lines() {
            if let Some((col, desc)) = line.split_once(':') {
                let col = col.trim();
                let desc = desc.trim();
                if dataset.schema.iter().any(|c| c.name == col) {
                    descriptions.insert(col.to_string(), desc.to_string());
                }
            }
        }

        Ok(descriptions)
    }

    /// Performs semantic search by embedding the query and comparing against dataset embeddings.
    pub async fn semantic_search(
        &self,
        query: &str,
        datasets: &[Dataset],
    ) -> CatalogResult<Vec<(f64, Dataset)>> {
        // Build text representations for each dataset
        let dataset_texts: Vec<String> = datasets
            .iter()
            .map(|ds| {
                let cols: Vec<String> = ds
                    .schema
                    .iter()
                    .map(|c| format!("{}:{}", c.name, c.column_type))
                    .collect();
                format!(
                    "Dataset: {name}\nDescription: {desc}\nColumns: {cols}\nTags: {tags}",
                    name = ds.name,
                    desc = ds.description.as_deref().unwrap_or(""),
                    cols = cols.join(", "),
                    tags = ds.tags.join(", "),
                )
            })
            .collect();

        // Get embeddings
        let mut all_texts = vec![query.to_string()];
        all_texts.extend(dataset_texts.clone());

        let embeddings = self.embed(&all_texts).await?;

        if embeddings.len() < 2 {
            return Ok(vec![]);
        }

        let query_embed = &embeddings[0];
        let dataset_embeds = &embeddings[1..];

        // Compute cosine similarity
        let mut scored: Vec<(f64, usize)> = dataset_embeds
            .iter()
            .enumerate()
            .map(|(i, emb)| {
                let sim = cosine_similarity(query_embed, emb);
                (sim, i)
            })
            .collect();

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        Ok(scored
            .into_iter()
            .take(10)
            .map(|(score, i)| (score, datasets[i].clone()))
            .collect())
    }
}

fn cosine_similarity(a: &[f64], b: &[f64]) -> f64 {
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f64 = a.iter().map(|x| x * x).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| x * x).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 1e-6);
    }

    // Note: LLM-dependent tests require a running Ollama instance;
    // these are best run as integration tests or with mock servers.
}
