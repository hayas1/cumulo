use crate::model::*;
use gloo_storage::{LocalStorage, Storage};
use std::collections::HashMap;

const STORAGE_KEY: &str = "cumulo_store";

pub fn load_from_storage() -> AppStore {
    match LocalStorage::get::<AppStore>(STORAGE_KEY) {
        Ok(store) => store,
        Err(_) => default_app_store(),
    }
}

pub fn save_to_storage(store: &AppStore) {
    let _ = LocalStorage::set(STORAGE_KEY, store);
}

pub fn default_app_store() -> AppStore {
    AppStore {
        resources: default_resources(),
        dimensions: default_dimensions(),
        cube_config: CubeConfig::default(),
    }
}

fn default_dimensions() -> Vec<Dimension> {
    vec![
        Dimension {
            id: "vendor".to_string(),
            label: "ベンダー".to_string(),
            mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "vendor".to_string(),
                value_map: None,
            }],
            ordered_values: Some(vec![
                "AWS".to_string(),
                "GCP".to_string(),
                "Azure".to_string(),
            ]),
        },
        Dimension {
            id: "project_scope".to_string(),
            label: "プロジェクト / アカウント".to_string(),
            mappings: vec![
                TagMapping {
                    conditions: vec![("vendor".to_string(), "GCP".to_string())],
                    source_key: "project".to_string(),
                    value_map: None,
                },
                TagMapping {
                    conditions: vec![("vendor".to_string(), "AWS".to_string())],
                    source_key: "account".to_string(),
                    value_map: None,
                },
                TagMapping {
                    conditions: vec![("vendor".to_string(), "Azure".to_string())],
                    source_key: "subscription".to_string(),
                    value_map: None,
                },
                TagMapping {
                    conditions: vec![],
                    source_key: "project".to_string(),
                    value_map: None,
                },
            ],
            ordered_values: None,
        },
        Dimension {
            id: "env".to_string(),
            label: "環境".to_string(),
            mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "env".to_string(),
                value_map: Some({
                    let mut m = HashMap::new();
                    m.insert("production".to_string(), "prod".to_string());
                    m.insert("prd".to_string(), "prod".to_string());
                    m.insert("stg".to_string(), "staging".to_string());
                    m.insert("develop".to_string(), "dev".to_string());
                    m.insert("development".to_string(), "dev".to_string());
                    m
                }),
            }],
            ordered_values: Some(vec![
                "prod".to_string(),
                "staging".to_string(),
                "dev".to_string(),
            ]),
        },
        Dimension {
            id: "service".to_string(),
            label: "サービス / プロダクト".to_string(),
            mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "service".to_string(),
                value_map: None,
            }],
            ordered_values: None,
        },
        Dimension {
            id: "resource_type".to_string(),
            label: "リソース種別".to_string(),
            mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "resource_type".to_string(),
                value_map: None,
            }],
            ordered_values: None,
        },
    ]
}

fn default_resources() -> Vec<Resource> {
    vec![
        Resource {
            id: "res-001".to_string(),
            name: "auth-bigquery-prod".to_string(),
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "GCP".to_string());
                m.insert("project".to_string(), "gamma-proj".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "auth".to_string());
                m.insert("resource_type".to_string(), "BigQuery".to_string());
                m.insert("region".to_string(), "asia-northeast1".to_string());
                m.insert("team".to_string(), "platform".to_string());
                m
            },
            console_url:
                "https://console.cloud.google.com/bigquery?project=gamma-proj".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        Resource {
            id: "res-002".to_string(),
            name: "auth-bigquery-staging".to_string(),
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "GCP".to_string());
                m.insert("project".to_string(), "gamma-staging".to_string());
                m.insert("env".to_string(), "staging".to_string());
                m.insert("service".to_string(), "auth".to_string());
                m.insert("resource_type".to_string(), "BigQuery".to_string());
                m.insert("region".to_string(), "asia-northeast1".to_string());
                m.insert("team".to_string(), "platform".to_string());
                m
            },
            console_url:
                "https://console.cloud.google.com/bigquery?project=gamma-staging".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        Resource {
            id: "res-003".to_string(),
            name: "payment-rds-prod".to_string(),
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "AWS".to_string());
                m.insert("account".to_string(), "account-123".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "payment".to_string());
                m.insert("resource_type".to_string(), "RDS".to_string());
                m.insert("region".to_string(), "ap-northeast-1".to_string());
                m.insert("team".to_string(), "backend".to_string());
                m
            },
            console_url:
                "https://console.aws.amazon.com/rds/home?region=ap-northeast-1".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        Resource {
            id: "res-004".to_string(),
            name: "auth-aurora-prod".to_string(),
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "AWS".to_string());
                m.insert("account".to_string(), "account-456".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "auth".to_string());
                m.insert("resource_type".to_string(), "Aurora".to_string());
                m.insert("region".to_string(), "ap-northeast-1".to_string());
                m.insert("team".to_string(), "platform".to_string());
                m
            },
            console_url:
                "https://console.aws.amazon.com/rds/home?region=ap-northeast-1#databases"
                    .to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        Resource {
            id: "res-005".to_string(),
            name: "infra-cosmos-prod".to_string(),
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "Azure".to_string());
                m.insert("subscription".to_string(), "sub-789".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "infra".to_string());
                m.insert("resource_type".to_string(), "CosmosDB".to_string());
                m.insert("region".to_string(), "japaneast".to_string());
                m.insert("team".to_string(), "infra".to_string());
                m
            },
            console_url: "https://portal.azure.com/#blade/HubsExtension/BrowseResource/resourceType/Microsoft.DocumentDb%2FdatabaseAccounts".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ]
}
