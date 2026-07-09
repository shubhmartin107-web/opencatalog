use std::collections::HashMap;

use opencatalog_core::types::{ColumnLineageFacet, ColumnLineageInfo, ColumnLineageInput, OpenLineageEvent};
use uuid::Uuid;

/// Parses an OpenLineage event and extracts column-level lineage information.
pub fn extract_column_lineage(event: &OpenLineageEvent) -> Vec<ColumnLineageInfo> {
    let mut results = Vec::new();

    for output in &event.outputs {
        let output_name = &output.name;
        let output_ns = &output.namespace;

        if let Some(facets) = &output.facets
            && let Some(cl_facet_val) = facets.get("columnLineage")
            && let Ok(cl_facet) = serde_json::from_value::<ColumnLineageFacet>(cl_facet_val.clone())
        {
            for (target_col, field_info) in &cl_facet.fields {
                for input in &field_info.input_fields {
                    let trans_type = input
                        .transformations
                        .first()
                        .map(|t| t.trans_type.clone())
                        .unwrap_or_else(|| "DIRECT".into());

                    let trans_subtype = input
                        .transformations
                        .first()
                        .map(|t| t.subtype.clone())
                        .unwrap_or_else(|| "IDENTITY".into());

                    results.push(ColumnLineageInfo {
                        source_dataset: format!("{}:{}", input.namespace, input.name),
                        source_column: input.field.clone(),
                        target_dataset: format!("{output_ns}:{output_name}"),
                        target_column: target_col.clone(),
                        transformation_type: trans_type,
                        transformation_subtype: trans_subtype,
                        transformation_sql: None,
                    });
                }
            }
        }
    }

    results
}

/// Builds an OpenLineage event from column-level lineage information.
pub fn build_openlineage_event(
    job_namespace: &str,
    job_name: &str,
    producer: &str,
    inputs: Vec<(&str, &str, &str)>,
    outputs: Vec<(&str, &str, Vec<ColumnLineageInfo>)>,
) -> OpenLineageEvent {
    let mut output_datasets = Vec::new();

    for (output_ns, output_name, col_lineages) in &outputs {
        let mut fields: HashMap<String, opencatalog_core::types::ColumnLineageField> = HashMap::new();

        for lin in col_lineages {
            let input = ColumnLineageInput {
                namespace: lin.source_dataset.split(':').next().unwrap_or("").into(),
                name: lin
                    .source_dataset
                    .split(':')
                    .nth(1)
                    .unwrap_or(&lin.source_dataset)
                    .into(),
                field: lin.source_column.clone(),
                transformations: vec![opencatalog_core::types::ColumnLineageTransformation {
                    trans_type: lin.transformation_type.clone(),
                    subtype: lin.transformation_subtype.clone(),
                    description: None,
                    masking: None,
                }],
            };

            fields.entry(lin.target_column.clone()).or_insert_with(|| {
                opencatalog_core::types::ColumnLineageField {
                    input_fields: vec![],
                }
            });
            if let Some(f) = fields.get_mut(&lin.target_column) {
                f.input_fields.push(input);
            }
        }

        let facet = ColumnLineageFacet {
            schema: None,
            fields,
        };

        let mut facets = HashMap::new();
        facets.insert(
            "columnLineage".to_string(),
            serde_json::to_value(facet).unwrap_or_default(),
        );

        output_datasets.push(opencatalog_core::types::OpenLineageDatasetRef {
            namespace: output_ns.to_string(),
            name: output_name.to_string(),
            facets: Some(facets),
        });
    }

    let input_datasets: Vec<opencatalog_core::types::OpenLineageDatasetRef> = inputs
        .into_iter()
        .map(|(ns, name, _)| opencatalog_core::types::OpenLineageDatasetRef {
            namespace: ns.to_string(),
            name: name.to_string(),
            facets: None,
        })
        .collect();

    OpenLineageEvent {
        event_type: "COMPLETE".into(),
        event_time: chrono::Utc::now().to_rfc3339(),
        producer: producer.into(),
        schema_url: "https://openlineage.io/spec/1-1-0/OpenLineage.json".into(),
        job: opencatalog_core::types::OpenLineageJob {
            namespace: job_namespace.into(),
            name: job_name.into(),
            facets: None,
        },
        run: opencatalog_core::types::OpenLineageRun {
            run_id: Uuid::now_v7().to_string(),
            facets: None,
        },
        inputs: input_datasets,
        outputs: output_datasets,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_column_lineage() {
        let event = OpenLineageEvent {
            event_type: "COMPLETE".into(),
            event_time: "2025-01-01T00:00:00Z".into(),
            producer: "test".into(),
            schema_url: "".into(),
            job: opencatalog_core::types::OpenLineageJob {
                namespace: "test".into(),
                name: "test_job".into(),
                facets: None,
            },
            run: opencatalog_core::types::OpenLineageRun {
                run_id: "run-1".into(),
                facets: None,
            },
            inputs: vec![],
            outputs: vec![opencatalog_core::types::OpenLineageDatasetRef {
                namespace: "lakehouse".into(),
                name: "analytics.orders".into(),
                facets: Some(HashMap::from([(
                    "columnLineage".into(),
                    serde_json::json!({
                        "fields": {
                            "order_id": {
                                "inputFields": [{
                                    "namespace": "source_db",
                                    "name": "public.orders",
                                    "field": "id",
                                    "transformations": [{"type": "DIRECT", "subtype": "IDENTITY"}]
                                }]
                            }
                        }
                    }),
                )])),
            }],
        };

        let lineage = extract_column_lineage(&event);
        assert!(!lineage.is_empty());
        assert_eq!(lineage[0].target_column, "order_id");
        assert_eq!(lineage[0].source_column, "id");
    }
}