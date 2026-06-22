//! category ドメインの web 層の値型（色・属性・ID・フィルタ）。

use cumulo_model::{Category, Id};
use serde::{Deserialize, Serialize};

/// RGBA 色（各チャンネル 0–255）。内部は r,g,b,a で保持し、JSON / HTML color input /
/// SVG・CSS の境界では `#rrggbb`（不透明）または `#rrggbbaa` の hex 文字列に相互変換する。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    /// CSS hex（`#rgb` / `#rrggbb` / `#rrggbbaa`、`#` 省略可）をパースする。不正なら None。
    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if !s.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
        match s.len() {
            // #rgb 省略形は各桁を 2 倍に展開（0xf→0xff）
            3 => {
                let nib = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).ok().map(|v| v * 17);
                Some(Color::rgb(nib(0)?, nib(1)?, nib(2)?))
            }
            6 => Some(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    /// CSS hex 文字列。不透明なら `#rrggbb`、半透明なら `#rrggbbaa`。
    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

/// 新規カテゴリに割り当てる既定色。
pub const DEFAULT_COLOR: Color = Color::rgb(0x88, 0x99, 0xaa);

/// CategoryAttribute.color 用 serde。JSON では hex 文字列（未指定は空文字列）として表し、
/// 既存データと後方互換を保つ。内部の Option<Color> との橋渡し。
mod color_field {
    use super::Color;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &Option<Color>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&value.map(Color::to_hex).unwrap_or_default())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Color>, D::Error> {
        let s = String::deserialize(d)?;
        Ok(if s.is_empty() {
            None
        } else {
            Color::from_hex(&s)
        })
    }
}

/// Web 層が Category に付与するビジュアル属性。色は未指定（None）を取りうる。
/// `#[serde(flatten)]` で JSON にインライン展開され、color は hex 文字列として後方互換。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CategoryAttribute {
    #[serde(default, with = "color_field")]
    pub color: Option<Color>,
}

pub type CategoryId = Id<Category<CategoryAttribute>>;
/// 軸→値の絞り込み選択。web 層では CA を固定して扱う。
pub type Filters = cumulo_model::Filters<CategoryAttribute>;

#[cfg(test)]
mod tests {
    use super::{CategoryAttribute, Color};

    #[test]
    fn hex_round_trips_for_opaque_and_alpha() {
        let opaque = Color::from_hex("#22aa22").unwrap();
        assert_eq!(opaque, Color::rgb(0x22, 0xaa, 0x22));
        assert_eq!(opaque.to_hex(), "#22aa22"); // 不透明は #rrggbb

        let alpha = Color::from_hex("#22aa2280").unwrap();
        assert_eq!(alpha, Color::rgba(0x22, 0xaa, 0x22, 0x80));
        assert_eq!(alpha.to_hex(), "#22aa2280"); // 半透明は #rrggbbaa
    }

    #[test]
    fn hex_accepts_short_form_and_rejects_garbage() {
        // #rgb は各桁 2 倍に展開
        assert_eq!(Color::from_hex("#0af"), Some(Color::rgb(0x00, 0xaa, 0xff)));
        assert_eq!(Color::from_hex("zzzzzz"), None);
        assert_eq!(Color::from_hex(""), None);
    }

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
