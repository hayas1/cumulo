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
        map_config: MapConfig::default(),
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
                "Datadog".to_string(),
                "Sentry".to_string(),
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
        Dimension {
            id: "team".to_string(),
            label: "チーム".to_string(),
            mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "team".to_string(),
                value_map: None,
            }],
            ordered_values: None,
        },
    ]
}

fn default_resources() -> Vec<Resource> {
    vec![
        Resource {
            id: "r1".to_string(),
            name: "auth-bigquery-prod".to_string(),
            freq: 12,
            parent_id: None,
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
            console_url: "https://console.cloud.google.com/bigquery?project=gamma-proj"
                .to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![
                ChildResource {
                    id: "r1c1".to_string(),
                    name: "dataset_events".to_string(),
                    freq: 8,
                    console_url:
                        "https://console.cloud.google.com/bigquery?project=gamma-proj&d=events"
                            .to_string(),
                },
                ChildResource {
                    id: "r1c2".to_string(),
                    name: "dataset_users".to_string(),
                    freq: 4,
                    console_url:
                        "https://console.cloud.google.com/bigquery?project=gamma-proj&d=users"
                            .to_string(),
                },
            ],
        },
        Resource {
            id: "r2".to_string(),
            name: "auth-bigquery-staging".to_string(),
            freq: 5,
            parent_id: None,
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
            console_url: "https://console.cloud.google.com/bigquery?project=gamma-staging"
                .to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![],
        },
        Resource {
            id: "r3".to_string(),
            name: "payment-rds-prod".to_string(),
            freq: 8,
            parent_id: None,
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "AWS".to_string());
                m.insert("account".to_string(), "acct-123".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "payment".to_string());
                m.insert("resource_type".to_string(), "RDS".to_string());
                m.insert("region".to_string(), "ap-northeast-1".to_string());
                m.insert("team".to_string(), "backend".to_string());
                m
            },
            console_url: "https://console.aws.amazon.com/rds".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![],
        },
        Resource {
            id: "r4".to_string(),
            name: "auth-aurora-prod".to_string(),
            freq: 15,
            parent_id: None,
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "AWS".to_string());
                m.insert("account".to_string(), "acct-456".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "auth".to_string());
                m.insert("resource_type".to_string(), "Aurora".to_string());
                m.insert("region".to_string(), "ap-northeast-1".to_string());
                m.insert("team".to_string(), "platform".to_string());
                m
            },
            console_url: "https://console.aws.amazon.com/rds".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![
                ChildResource {
                    id: "r4c1".to_string(),
                    name: "db_auth_main".to_string(),
                    freq: 12,
                    console_url:
                        "https://console.aws.amazon.com/rds/home#database:id=auth-main"
                            .to_string(),
                },
                ChildResource {
                    id: "r4c2".to_string(),
                    name: "db_auth_read".to_string(),
                    freq: 3,
                    console_url:
                        "https://console.aws.amazon.com/rds/home#database:id=auth-read"
                            .to_string(),
                },
            ],
        },
        Resource {
            id: "r5".to_string(),
            name: "infra-cosmos-prod".to_string(),
            freq: 3,
            parent_id: None,
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
            console_url: "https://portal.azure.com/".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![],
        },
        Resource {
            id: "r6".to_string(),
            name: "auth-datadog-apm".to_string(),
            freq: 20,
            parent_id: None,
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "Datadog".to_string());
                m.insert("project".to_string(), "dd-main".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "auth".to_string());
                m.insert("resource_type".to_string(), "APM".to_string());
                m.insert("region".to_string(), "global".to_string());
                m.insert("team".to_string(), "platform".to_string());
                m
            },
            console_url: "https://app.datadoghq.com/apm/services/auth".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![],
        },
        Resource {
            id: "r7".to_string(),
            name: "payment-sentry".to_string(),
            freq: 7,
            parent_id: None,
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "Sentry".to_string());
                m.insert("project".to_string(), "sentry-pay".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "payment".to_string());
                m.insert("resource_type".to_string(), "ErrorTrack".to_string());
                m.insert("region".to_string(), "global".to_string());
                m.insert("team".to_string(), "backend".to_string());
                m
            },
            console_url: "https://sentry.io/".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![],
        },
        Resource {
            id: "r11".to_string(),
            name: "analytics-bigquery".to_string(),
            freq: 11,
            parent_id: None,
            attrs: {
                let mut m = HashMap::new();
                m.insert("vendor".to_string(), "GCP".to_string());
                m.insert("project".to_string(), "analytics-proj".to_string());
                m.insert("env".to_string(), "prod".to_string());
                m.insert("service".to_string(), "analytics".to_string());
                m.insert("resource_type".to_string(), "BigQuery".to_string());
                m.insert("region".to_string(), "us-central1".to_string());
                m.insert("team".to_string(), "data".to_string());
                m
            },
            console_url:
                "https://console.cloud.google.com/bigquery?project=analytics-proj".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            children: vec![ChildResource {
                id: "r11c1".to_string(),
                name: "dataset_metrics".to_string(),
                freq: 9,
                console_url:
                    "https://console.cloud.google.com/bigquery?project=analytics-proj&d=metrics"
                        .to_string(),
            }],
        },
    ]
}
