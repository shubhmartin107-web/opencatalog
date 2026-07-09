use std::collections::HashMap;

use opencatalog_core::types::*;
use uuid::Uuid;

/// Generates suggested glossary terms from dataset schema using heuristics.
/// Scans column names for PII indicators and common business terms.
pub fn suggest_glossary_terms(dataset: &Dataset) -> Vec<GlossaryTerm> {
    let mut suggestions = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for col in &dataset.schema {
        let lower = col.name.to_lowercase();

        let suggestions_for_col: Vec<(&str, &str, &str)> = match () {
            _ if lower.contains("email") => {
                vec![("Customer Email", "Email address of a customer", "Customer Data")]
            }
            _ if lower.contains("phone") || lower.contains("telephone") => {
                vec![("Phone Number", "Telephone contact number", "Customer Data")]
            }
            _ if lower.contains("ssn") || lower.contains("social_security") => {
                vec![("Social Security Number", "Government-issued SSN identifier", "PII")]
            }
            _ if lower.contains("credit") || lower.contains("card") => {
                vec![("Payment Card", "Credit or debit card details", "Financial")]
            }
            _ if lower.contains("address") || lower.contains("street") => {
                vec![("Physical Address", "Postal or street address", "Customer Data")]
            }
            _ if lower.contains("name") && !lower.contains("username") => {
                vec![("Person Name", "Full name of an individual", "Customer Data")]
            }
            _ if lower.contains("salary") || lower.contains("compensation") => {
                vec![("Salary", "Employee compensation information", "HR")]
            }
            _ if lower.contains("birth") || lower.contains("dob") => {
                vec![("Date of Birth", "Individual's date of birth", "PII")]
            }
            _ if lower == "id" || lower.ends_with("_id") => {
                vec![("Identifier", "Unique identifier for a record", "Technical")]
            }
            _ if lower.contains("created_at") || lower.contains("updated_at") => {
                vec![("Timestamp", "System-generated timestamp", "Technical")]
            }
            _ => vec![],
        };

        for (name, desc, domain) in suggestions_for_col {
            if seen.insert(name) {
                suggestions.push(GlossaryTerm {
                    id: Uuid::nil(),
                    name: name.into(),
                    description: desc.into(),
                    short_description: None,
                    domain: Some(domain.into()),
                    synonyms: vec![],
                    related_term_ids: vec![],
                    custom_properties: HashMap::new(),
                    status: TermStatus::Draft,
                    created_at: chrono::Utc::now(),
                    updated_at: chrono::Utc::now(),
                });
            }
        }
    }

    suggestions
}

pub fn map_term_to_columns(
    term: &GlossaryTerm,
    dataset: &Dataset,
) -> Vec<TermMapping> {
    let lower_name = term.name.to_lowercase();
    let mut mappings = Vec::new();

    for col in &dataset.schema {
        let col_lower = col.name.to_lowercase();
        let should_map = term.synonyms.iter().any(|s| col_lower.contains(&s.to_lowercase()))
            || col_lower.contains(&lower_name)
            || lower_name.contains(&col_lower);

        if should_map {
            mappings.push(TermMapping {
                id: Uuid::nil(),
                term_id: term.id,
                dataset_id: dataset.id,
                column_id: Some(col.id),
                created_at: chrono::Utc::now(),
            });
        }
    }

    mappings
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn make_test_dataset() -> Dataset {
        Dataset {
            id: Uuid::nil(),
            data_source_id: Uuid::nil(),
            name: "test.customers".into(),
            physical_name: "test.customers".into(),
            dataset_type: DatasetType::Table,
            version: 1,
            metadata: std::collections::HashMap::new(),
            schema: vec![
                Column {
                    id: Uuid::nil(),
                    dataset_id: Uuid::nil(),
                    name: "email".into(),
                    column_type: "string".into(),
                    description: None,
                    is_nullable: true,
                    is_primary_key: false,
                    is_foreign_key: false,
                    ordinal_position: 0,
                    classification: None,
                    tags: vec![],
                    glossary_term_id: None,
                    metadata: std::collections::HashMap::new(),
                },
                Column {
                    id: Uuid::nil(),
                    dataset_id: Uuid::nil(),
                    name: "ssn".into(),
                    column_type: "string".into(),
                    description: None,
                    is_nullable: true,
                    is_primary_key: false,
                    is_foreign_key: false,
                    ordinal_position: 1,
                    classification: None,
                    tags: vec![],
                    glossary_term_id: None,
                    metadata: std::collections::HashMap::new(),
                },
            ],
            description: None,
            tags: vec![],
            classification: None,
            location: None,
            row_count: None,
            last_crawled_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_suggest_glossary_terms() {
        let ds = make_test_dataset();
        let terms = suggest_glossary_terms(&ds);
        assert!(terms.iter().any(|t| t.name == "Customer Email"));
        assert!(terms.iter().any(|t| t.name == "Social Security Number"));
    }
}
