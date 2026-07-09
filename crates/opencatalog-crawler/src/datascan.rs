use opencatalog_core::error::CatalogResult;
use opencatalog_core::traits::CatalogStore;
use opencatalog_core::types::{DataSource, Dataset};
use regex::Regex;

type Classifier = (Vec<Regex>, String);

pub struct DataScanner;

impl DataScanner {
    pub async fn scan(
        &self,
        datasource: &DataSource,
        dataset: &Dataset,
        store: &dyn CatalogStore,
    ) -> CatalogResult<()> {
        tracing::info!(
            "Scanning dataset '{}' from datasource '{}'",
            dataset.name,
            datasource.name
        );

        let name_classifiers = build_name_classifiers();
        let type_classifiers = build_type_classifiers();

        let mut updated_columns = Vec::new();

        for column in &dataset.schema {
            let lower_name = column.name.to_lowercase();
            let lower_type = column.column_type.to_lowercase();

            let classification = classify_column(&lower_name, &lower_type, &name_classifiers, &type_classifiers);

            if let Some(class) = classification {
                let mut updated = column.clone();
                updated.classification = Some(class);

                let mut meta = updated.metadata.clone();
                meta.insert("scanned_by".into(), "datascan".into());
                meta.insert("scan_timestamp".into(), chrono::Utc::now().to_rfc3339());
                meta.insert("scan_source_type".into(), datasource.source_type.to_string());
                updated.metadata = meta;

                updated_columns.push(updated);
            }
        }

        for col in &updated_columns {
            store.update_column(col.clone()).await?;
        }

        tracing::info!(
            "Scan complete for '{}': classified {} of {} columns",
            dataset.name,
            updated_columns.len(),
            dataset.schema.len()
        );

        Ok(())
    }
}

fn build_name_classifiers() -> Vec<Classifier> {
    vec![
        (patterns(&["email", "e-mail"]), "pii.email".into()),
        (patterns(&["ssn", "social_security", "socialsecurity"]), "pii.ssn".into()),
        (patterns(&["phone", "telephone", "mobile", "cell"]), "pii.phone".into()),
        (patterns(&["credit_card", "cc_", "card_number", "ccnum"]), "pii.credit_card".into()),
        (patterns(&["password", "pwd", "secret"]), "pii.credential".into()),
        (patterns(&["name", "full_name", "first_name", "last_name", "given_name", "surname"]), "pii.name".into()),
        (patterns(&["address", "street", "city", "state", "zip", "postal"]), "pii.address".into()),
        (patterns(&["birth", "dob", "birth_date", "date_of_birth"]), "pii.birth_date".into()),
        (patterns(&["^ip$", "^ip_", "_ip$", "ip_address"]), "pii.ip".into()),
        (patterns(&["username", "user_name", "login"]), "pii.username".into()),
        (patterns(&["token", "auth_", "api_key", "api_secret", "apikey"]), "pii.auth_token".into()),
    ]
}

fn build_type_classifiers() -> Vec<(Vec<Regex>, Vec<Regex>, String)> {
    vec![
        (
            patterns(&["varchar\\(255\\)", "text", "character varying"]),
            patterns(&["email", "ssn", "password", "pwd", "secret", "phone", "credit", "ip", "address", "birth", "dob"]),
            "pii.sensitive_text".into(),
        ),
    ]
}

fn patterns(list: &[&str]) -> Vec<Regex> {
    list.iter().map(|p| Regex::new(&format!("(?i){p}")).unwrap()).collect()
}

fn classify_column(
    lower_name: &str,
    lower_type: &str,
    name_classifiers: &[Classifier],
    type_classifiers: &[(Vec<Regex>, Vec<Regex>, String)],
) -> Option<String> {
    for (name_pats, class) in name_classifiers {
        if name_pats.iter().any(|re| re.is_match(lower_name)) {
            return Some(class.clone());
        }
    }

    for (type_pats, name_pats, class) in type_classifiers {
        if type_pats.iter().any(|re| re.is_match(lower_type))
            && name_pats.iter().any(|re| re.is_match(lower_name))
        {
            return Some(class.clone());
        }
    }

    None
}