//! category ドメインの web 層の値型（属性・ID・フィルタ）。色は shared の値型を使う。

use cumulo_model::{Category, Id};
use serde::{Deserialize, Serialize};

use crate::shared::Color;

/// 新規カテゴリに割り当てる既定色（category のポリシー。型は共有の Color）。
pub const DEFAULT_COLOR: Color = Color::rgb(0x88, 0x99, 0xaa);

/// Web 層が Category に付与するビジュアル属性。色は未指定（None）を取りうる。
/// `#[serde(flatten)]` で JSON にインライン展開され、color は hex 文字列として後方互換。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CategoryAttribute {
    #[serde(default, with = "crate::shared::color::hex_opt")]
    pub color: Option<Color>,
}

pub type CategoryId = Id<Category<CategoryAttribute>>;
/// 軸→値の絞り込み選択。web 層では CA を固定して扱う。
pub type Filters = cumulo_model::Filters<CategoryAttribute>;

#[cfg(test)]
mod tests {
    use super::CategoryAttribute;
    use crate::shared::Color;

    #[test]
    fn category_attribute_color_serializes_as_hex_string() {
        let attr = CategoryAttribute {
            color: Some(Color::rgb(0x22, 0xaa, 0x22)),
        };
        let json = serde_json::to_string(&attr).unwrap();
        assert_eq!(json, r##"{"color":"#22aa22"}"##);
        assert_eq!(
            serde_json::from_str::<CategoryAttribute>(&json).unwrap(),
            attr
        );
    }

    #[test]
    fn empty_color_string_is_unspecified() {
        // 既存データの空文字列は None（未指定）として読み、書き戻しも空文字列で後方互換
        let attr = serde_json::from_str::<CategoryAttribute>(r#"{"color":""}"#).unwrap();
        assert_eq!(attr.color, None);
        assert_eq!(serde_json::to_string(&attr).unwrap(), r#"{"color":""}"#);
    }

    #[test]
    fn color_round_trips_through_flattened_category() {
        // Category は attribute を #[serde(flatten)] するので、その経路でも壊れないこと
        use cumulo_model::Category;
        let c = Category {
            id: "gcp".try_into().unwrap(),
            label: "GCP".into(),
            parent: None,
            attribute: CategoryAttribute {
                color: Some(Color::rgb(0x22, 0xaa, 0x22)),
            },
        };
        let json = serde_json::to_string(&c).unwrap();
        assert!(json.contains(r##""color":"#22aa22""##));
        let back: Category<CategoryAttribute> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.attribute.color, Some(Color::rgb(0x22, 0xaa, 0x22)));
    }
}
