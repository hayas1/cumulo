use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// クラウドリソース（物理的な実体）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    pub id: String,
    pub name: String,
    pub raw_tags: HashMap<String, String>,
    pub console_url: String,
    pub created_at: String,
}

/// 論理軸の定義（キューブの軸として使う抽象概念）
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Dimension {
    pub id: String,
    pub label: String,
    pub tag_mappings: Vec<TagMapping>,
}

/// raw_tags のどのキーをこのDimensionとして解釈するかのマッピング
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TagMapping {
    pub conditions: Vec<(String, String)>,
    pub source_key: String,
    pub value_map: Option<HashMap<String, String>>,
}

/// キューブの表示設定
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CubeConfig {
    pub axis_x: String,
    pub axis_y: String,
    pub axis_z: String,
    pub filters: Vec<(String, String)>,
}

/// LocalStorageに保存するルートデータ構造
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AppStore {
    pub resources: Vec<Resource>,
    pub dimensions: Vec<Dimension>,
    pub cube_config: CubeConfig,
}

/// グリッドのセル（X/Y軸の組み合わせ）にマップされたリソースリスト
pub type GridCell = Vec<Resource>;

/// グリッド全体: (x_value, y_value) -> resources
pub type SliceGrid = HashMap<(String, String), GridCell>;

/// Dimensionに対してResourceの値を解決する
pub fn resolve_dimension(resource: &Resource, dimension: &Dimension) -> Option<String> {
    for mapping in &dimension.tag_mappings {
        let all_match = mapping
            .conditions
            .iter()
            .all(|(k, v)| resource.raw_tags.get(k).map(|s| s.as_str()) == Some(v.as_str()));
        if all_match {
            let raw_value = resource.raw_tags.get(&mapping.source_key)?;
            return Some(match &mapping.value_map {
                Some(map) => map
                    .get(raw_value)
                    .cloned()
                    .unwrap_or_else(|| raw_value.clone()),
                None => raw_value.clone(),
            });
        }
    }
    None
}

/// スライスグリッドを構築する
/// z_value: 第3軸（奥行き）の現在の値でフィルタリング
pub fn build_slice_grid(
    resources: &[Resource],
    dimensions: &[Dimension],
    axis_x: &str,
    axis_y: &str,
    axis_z: &str,
    z_value: &str,
    filters: &[(String, String)],
) -> SliceGrid {
    let dim_x = dimensions.iter().find(|d| d.id == axis_x);
    let dim_y = dimensions.iter().find(|d| d.id == axis_y);
    let dim_z = dimensions.iter().find(|d| d.id == axis_z);

    let mut grid: SliceGrid = HashMap::new();

    for resource in resources {
        // フィルター条件チェック
        let passes_filters = filters.iter().all(|(dim_id, expected)| {
            if let Some(dim) = dimensions.iter().find(|d| d.id == dim_id.as_str()) {
                resolve_dimension(resource, dim)
                    .map(|v| v == *expected)
                    .unwrap_or(false)
            } else {
                true
            }
        });
        if !passes_filters {
            continue;
        }

        // 第3軸のフィルタリング
        if let Some(dz) = dim_z {
            let zv = resolve_dimension(resource, dz);
            if zv.as_deref() != Some(z_value) {
                continue;
            }
        }

        let x_val = dim_x
            .and_then(|d| resolve_dimension(resource, d))
            .unwrap_or_default();
        let y_val = dim_y
            .and_then(|d| resolve_dimension(resource, d))
            .unwrap_or_default();

        if x_val.is_empty() || y_val.is_empty() {
            continue;
        }

        grid.entry((x_val, y_val))
            .or_default()
            .push(resource.clone());
    }

    grid
}

/// 指定したDimensionの全ユニーク値を返す
pub fn dimension_values(resources: &[Resource], dimension: &Dimension) -> Vec<String> {
    let mut values: Vec<String> = resources
        .iter()
        .filter_map(|r| resolve_dimension(r, dimension))
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    values.sort();
    values
}

// ---------- デフォルトデータ ----------

pub fn default_dimensions() -> Vec<Dimension> {
    let env_value_map: HashMap<String, String> = [
        ("production", "prod"),
        ("prd", "prod"),
        ("staging", "stg"),
        ("development", "dev"),
        ("develop", "dev"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();

    vec![
        Dimension {
            id: "vendor".to_string(),
            label: "ベンダー".to_string(),
            tag_mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "vendor".to_string(),
                value_map: None,
            }],
        },
        Dimension {
            id: "project_scope".to_string(),
            label: "プロジェクト/アカウント".to_string(),
            tag_mappings: vec![
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
        },
        Dimension {
            id: "env".to_string(),
            label: "環境".to_string(),
            tag_mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "env".to_string(),
                value_map: Some(env_value_map),
            }],
        },
        Dimension {
            id: "category".to_string(),
            label: "カテゴリ".to_string(),
            tag_mappings: vec![TagMapping {
                conditions: vec![],
                source_key: "category".to_string(),
                value_map: None,
            }],
        },
    ]
}

pub fn default_cube_config() -> CubeConfig {
    CubeConfig {
        axis_x: "vendor".to_string(),
        axis_y: "env".to_string(),
        axis_z: "category".to_string(),
        filters: vec![],
    }
}

pub fn sample_resources() -> Vec<Resource> {
    let now = "2025-01-01T00:00:00Z".to_string();

    fn tags(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    vec![
        Resource {
            id: "r001".to_string(),
            name: "alpha-s3-backup".to_string(),
            raw_tags: tags(&[
                ("vendor", "AWS"),
                ("account", "alpha-aws"),
                ("env", "prod"),
                ("region", "ap-northeast-1"),
                ("category", "Storage"),
                ("team", "infra"),
            ]),
            console_url:
                "https://s3.console.aws.amazon.com/s3/buckets/alpha-backup?region=ap-northeast-1"
                    .to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r002".to_string(),
            name: "alpha-rds-main".to_string(),
            raw_tags: tags(&[
                ("vendor", "AWS"),
                ("account", "alpha-aws"),
                ("env", "prod"),
                ("region", "ap-northeast-1"),
                ("category", "Database"),
                ("team", "backend"),
            ]),
            console_url:
                "https://ap-northeast-1.console.aws.amazon.com/rds/home?region=ap-northeast-1"
                    .to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r003".to_string(),
            name: "alpha-ec2-web".to_string(),
            raw_tags: tags(&[
                ("vendor", "AWS"),
                ("account", "alpha-aws"),
                ("env", "prod"),
                ("region", "ap-northeast-1"),
                ("category", "Compute"),
                ("team", "backend"),
            ]),
            console_url:
                "https://ap-northeast-1.console.aws.amazon.com/ec2/home?region=ap-northeast-1"
                    .to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r004".to_string(),
            name: "alpha-ec2-staging".to_string(),
            raw_tags: tags(&[
                ("vendor", "AWS"),
                ("account", "alpha-aws"),
                ("env", "stg"),
                ("region", "ap-northeast-1"),
                ("category", "Compute"),
                ("team", "backend"),
            ]),
            console_url:
                "https://ap-northeast-1.console.aws.amazon.com/ec2/home?region=ap-northeast-1"
                    .to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r005".to_string(),
            name: "gamma-bigquery".to_string(),
            raw_tags: tags(&[
                ("vendor", "GCP"),
                ("project", "gamma-proj"),
                ("env", "prod"),
                ("region", "asia-northeast1"),
                ("category", "Database"),
                ("team", "data-platform"),
            ]),
            console_url: "https://console.cloud.google.com/bigquery?project=gamma-proj".to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r006".to_string(),
            name: "gamma-gcs-archive".to_string(),
            raw_tags: tags(&[
                ("vendor", "GCP"),
                ("project", "gamma-proj"),
                ("env", "prod"),
                ("region", "asia-northeast1"),
                ("category", "Storage"),
                ("team", "data-platform"),
            ]),
            console_url:
                "https://console.cloud.google.com/storage/browser?project=gamma-proj".to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r007".to_string(),
            name: "gamma-cloudrun-api".to_string(),
            raw_tags: tags(&[
                ("vendor", "GCP"),
                ("project", "gamma-proj"),
                ("env", "prod"),
                ("region", "asia-northeast1"),
                ("category", "Compute"),
                ("team", "backend"),
            ]),
            console_url:
                "https://console.cloud.google.com/run?project=gamma-proj".to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r008".to_string(),
            name: "gamma-cloudrun-dev".to_string(),
            raw_tags: tags(&[
                ("vendor", "GCP"),
                ("project", "gamma-proj"),
                ("env", "dev"),
                ("region", "asia-northeast1"),
                ("category", "Compute"),
                ("team", "backend"),
            ]),
            console_url:
                "https://console.cloud.google.com/run?project=gamma-proj".to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r009".to_string(),
            name: "beta-blob-assets".to_string(),
            raw_tags: tags(&[
                ("vendor", "Azure"),
                ("subscription", "beta-sub"),
                ("env", "prod"),
                ("region", "japaneast"),
                ("category", "Storage"),
                ("team", "frontend"),
            ]),
            console_url: "https://portal.azure.com/#blade/Microsoft_Azure_Storage/StorageAccountMenuBlade/overview/subscriptionId/beta-sub".to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r010".to_string(),
            name: "beta-cosmosdb".to_string(),
            raw_tags: tags(&[
                ("vendor", "Azure"),
                ("subscription", "beta-sub"),
                ("env", "prod"),
                ("region", "japaneast"),
                ("category", "Database"),
                ("team", "backend"),
            ]),
            console_url: "https://portal.azure.com/#blade/Microsoft_Azure_Storage/CosmosDB/subscriptionId/beta-sub".to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r011".to_string(),
            name: "alpha-sqs-events".to_string(),
            raw_tags: tags(&[
                ("vendor", "AWS"),
                ("account", "alpha-aws"),
                ("env", "prod"),
                ("region", "ap-northeast-1"),
                ("category", "Messaging"),
                ("team", "backend"),
            ]),
            console_url: "https://ap-northeast-1.console.aws.amazon.com/sqs/v2/home?region=ap-northeast-1".to_string(),
            created_at: now.clone(),
        },
        Resource {
            id: "r012".to_string(),
            name: "gamma-pubsub-pipeline".to_string(),
            raw_tags: tags(&[
                ("vendor", "GCP"),
                ("project", "gamma-proj"),
                ("env", "prod"),
                ("region", "asia-northeast1"),
                ("category", "Messaging"),
                ("team", "data-platform"),
            ]),
            console_url: "https://console.cloud.google.com/cloudpubsub?project=gamma-proj".to_string(),
            created_at: now.clone(),
        },
    ]
}

pub fn default_app_store() -> AppStore {
    AppStore {
        resources: sample_resources(),
        dimensions: default_dimensions(),
        cube_config: default_cube_config(),
    }
}
