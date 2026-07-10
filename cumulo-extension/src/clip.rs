//! ブラウザのタブから取り込む「クリップ」を Resource へ射影する。
//! ブラウザ副作用（id 生成・現在時刻）は呼び出し側から値で受け取り、変換自体は純粋に保つ。

use cumulo_model::Resource;
use cumulo_web::{CategoryAttribute, CategoryId, ResourceAttribute, ResourceId};

/// popup が集めたタブ情報＋選択カテゴリ。`into_resource` で Catalog に足す Resource になる。
pub struct Clip {
    pub id: ResourceId,
    pub title: String,
    pub url: String,
    pub categories: Vec<CategoryId>,
    pub created_at: String,
}

impl Clip {
    pub fn into_resource(self) -> Resource<ResourceAttribute, CategoryAttribute> {
        let title = self.title.trim();
        Resource {
            id: self.id,
            // 空タイトルは None にして、モデル側の「カテゴリ値から自動生成」に委ねる。
            label: (!title.is_empty()).then(|| title.to_string()),
            parent: None,
            categories: self.categories,
            attribute: ResourceAttribute {
                console_url: self.url,
                created_at: Some(self.created_at),
                freq: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rid(s: &str) -> ResourceId {
        s.try_into().unwrap()
    }
    fn cid(s: &str) -> CategoryId {
        s.try_into().unwrap()
    }

    #[test]
    fn maps_tab_and_categories_into_resource() {
        let r = Clip {
            id: rid("r1"),
            title: "Example".into(),
            url: "https://example.com".into(),
            categories: vec![cid("gcp"), cid("prod")],
            created_at: "2026-07-05T00:00:00Z".into(),
        }
        .into_resource();

        assert_eq!(r.id, rid("r1"));
        assert_eq!(r.label, Some("Example".to_string()));
        assert_eq!(r.parent, None);
        assert_eq!(r.categories, vec![cid("gcp"), cid("prod")]);
        assert_eq!(r.attribute.console_url, "https://example.com");
        assert_eq!(
            r.attribute.created_at,
            Some("2026-07-05T00:00:00Z".to_string())
        );
        assert_eq!(r.attribute.freq, 0);
    }

    #[test]
    fn blank_title_yields_no_label() {
        let r = Clip {
            id: rid("r1"),
            title: "   ".into(),
            url: "https://example.com".into(),
            categories: vec![],
            created_at: "t".into(),
        }
        .into_resource();
        assert_eq!(r.label, None);
    }
}
