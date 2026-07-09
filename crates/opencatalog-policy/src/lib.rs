use opencatalog_core::error::CatalogResult;
use opencatalog_core::types::*;
use sha2::Digest;

/// Evaluates policies against a query/role context and returns applicable transformations.
pub struct PolicyEvaluator;

impl PolicyEvaluator {
    pub fn evaluate(
        policies: &[Policy],
        request: &PolicyEvalRequest,
    ) -> CatalogResult<PolicyEvalResult> {
        let mut matched = Vec::new();
        let mut transforms = Vec::new();

        for policy in policies {
            if !policy.enabled {
                continue;
            }

            for rule in &policy.rules {
                // Check if rule applies to the requester's role
                if !rule.roles.is_empty() && !rule.roles.iter().any(|r| r == &request.role) {
                    continue;
                }

                // Check dataset pattern
                let dataset = request.dataset.as_deref().unwrap_or("");
                let dataset_re = match glob_to_regex(&rule.dataset_pattern) {
                    Ok(re) => re,
                    Err(_) => continue,
                };

                if !dataset_re.is_match(dataset) {
                    continue;
                }

                // Check column pattern
                if let Some(ref col_pattern) = rule.column_pattern {
                    let col_re = match glob_to_regex(col_pattern) {
                        Ok(re) => re,
                        Err(_) => continue,
                    };
                    let matching_cols: Vec<&String> = request
                        .columns
                        .iter()
                        .filter(|c| col_re.is_match(c))
                        .collect();

                    if matching_cols.is_empty() {
                        continue;
                    }

                    for col in matching_cols {
                        transforms.push(ColumnTransform {
                            dataset: dataset.to_string(),
                            column: col.clone(),
                            action: rule.action.clone(),
                        });
                    }
                } else {
                    // Apply to all columns
                    for col in &request.columns {
                        transforms.push(ColumnTransform {
                            dataset: dataset.to_string(),
                            column: col.clone(),
                            action: rule.action.clone(),
                        });
                    }
                }

                matched.push(policy.clone());
            }
        }

        Ok(PolicyEvalResult {
            matched_policies: matched,
            transformations: transforms,
        })
    }

    /// Applies a masking action to a value in-place.
    pub fn apply_mask(value: &str, method: &MaskMethod) -> String {
        match method {
            MaskMethod::Redact => "***REDACTED***".into(),
            MaskMethod::Hash => {
                let hash = sha2::Sha256::digest(value.as_bytes());
                hex::encode(hash)
            }
            MaskMethod::Nullify => String::new(),
            MaskMethod::Partial(visible) => {
                if value.len() <= *visible {
                    value.to_string()
                } else {
                    let visible_part: String = value.chars().take(*visible).collect();
                    let masked: String = "*".repeat(value.len().saturating_sub(*visible));
                    format!("{visible_part}{masked}")
                }
            }
            MaskMethod::Tokenize => {
                let hash = sha2::Sha256::digest(value.as_bytes());
                format!("tok_{}", &hex::encode(hash)[..12])
            }
            MaskMethod::Sha256 { salt } => {
                let salted = match salt {
                    Some(s) => format!("{value}{s}"),
                    None => value.to_string(),
                };
                let hash = sha2::Sha256::digest(salted.as_bytes());
                hex::encode(hash)
            }
            MaskMethod::Mask { character, count } => {
                std::iter::repeat_n(*character, *count).collect()
            }
            MaskMethod::Custom(s) => s.clone(),
        }
    }
}

fn glob_to_regex(pattern: &str) -> Result<regex::Regex, regex::Error> {
    let escaped = regex::escape(pattern);
    let re_str = format!("^{}$", escaped.replace(r"\*", ".*").replace(r"\?", "."));
    regex::Regex::new(&re_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_masking_policy_evaluation() {
        let policy = Policy {
            id: uuid::Uuid::nil(),
            name: "Mask PII".into(),
            description: None,
            policy_type: PolicyType::Masking,
            rules: vec![PolicyRule {
                dataset_pattern: "*.customers".into(),
                column_pattern: Some("email".into()),
                condition: None,
                action: PolicyAction::Mask(MaskMethod::Redact),
                roles: vec!["analyst".into()],
            }],
            enabled: true,
            priority: 100,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let req = PolicyEvalRequest {
            query: "SELECT email FROM customers".into(),
            dataset: Some("analytics.customers".into()),
            columns: vec!["email".into(), "name".into()],
            role: "analyst".into(),
        };

        let result = PolicyEvaluator::evaluate(&[policy], &req).unwrap();
        assert_eq!(result.transformations.len(), 1);
        assert_eq!(result.transformations[0].column, "email");
    }

    #[test]
    fn test_mask_methods() {
        assert_eq!(PolicyEvaluator::apply_mask("test@email.com", &MaskMethod::Redact), "***REDACTED***");
        assert_eq!(PolicyEvaluator::apply_mask("test@email.com", &MaskMethod::Nullify), "");
        assert_eq!(
            PolicyEvaluator::apply_mask("test@email.com", &MaskMethod::Partial(4)),
            "test**********"
        );
    }
}
