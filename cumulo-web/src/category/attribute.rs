use cumulo_model::{Category, Id};
use serde::{Deserialize, Serialize};

use crate::shared::Color;

pub const DEFAULT_COLOR: Color = Color::rgb(0x88, 0x99, 0xaa);

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct CategoryAttribute {
    #[serde(default, with = "crate::shared::color::hex_opt")]
    pub color: Option<Color>,
}

pub type CategoryId = Id<Category<CategoryAttribute>>;
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
        let attr = serde_json::from_str::<CategoryAttribute>(r#"{"color":""}"#).unwrap();
        assert_eq!(attr.color, None);
        assert_eq!(serde_json::to_string(&attr).unwrap(), r#"{"color":""}"#);
    }

    #[test]
    fn color_round_trips_through_flattened_category() {
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
