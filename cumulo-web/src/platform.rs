use cumulo_model::{Category, Id, Resource};
use js_sys::Array;
use serde::{Deserialize, Serialize};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

/// Web 層が Resource に付与する値。
/// `#[serde(flatten)]` で JSON にインライン展開されるため、既存データと後方互換。
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct ResourceAttribute {
    pub console_url: String,
    pub created_at: Option<String>,
    pub freq: u32,
}

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
pub type ResourceId = Id<Resource<ResourceAttribute, CategoryAttribute>>;
/// 軸→値の絞り込み選択。web 層では CA を固定して扱う。
pub type Filters = cumulo_model::Filters<CategoryAttribute>;

/// ブラウザ固有の副作用（ID 生成、色生成、ダウンロード、URL 開放）をまとめる。
/// js_sys / web_sys を使うため core クレートには含めない。
pub struct Platform;

impl Platform {
    pub fn new_node_id() -> CategoryId {
        let n = (js_sys::Math::random() * 1e15) as u64;
        // "node" プレフィックスを付けるので空文字列にはならない
        format!("node{n:x}").try_into().unwrap()
    }

    pub fn new_resource_id() -> ResourceId {
        let n = (js_sys::Math::random() * 1e15) as u64;
        // "r" プレフィックスを付けるので空文字列にはならない
        format!("r{n:x}").try_into().unwrap()
    }

    pub fn new_resource() -> Resource<ResourceAttribute, CategoryAttribute> {
        Resource {
            id: Self::new_resource_id(),
            label: None,
            parent: None,
            categories: Vec::new(),
            attribute: ResourceAttribute::default(),
        }
    }

    pub fn random_color() -> Color {
        const PALETTE: &[&str] = &[
            "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e", "#10b981", "#14b8a6",
            "#06b6d4", "#3b82f6", "#6366f1", "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e",
        ];
        let idx = (js_sys::Math::random() * PALETTE.len() as f64) as usize;
        Color::from_hex(PALETTE[idx.min(PALETTE.len() - 1)]).expect("palette entries are valid hex")
    }

    /// leptos_router の `base` に渡す path prefix を返す。
    /// 値はビルド時の env `CUMULO_BASE_PATH` から取る。
    /// trunk は `--public-url` を cargo ビルドへ渡さず、また自身の `TRUNK_*` env は
    /// cargo 子プロセスへ素通ししないため、router base 用には独立した env を使う。
    pub fn router_base() -> &'static str {
        Self::normalize_base(option_env!("CUMULO_BASE_PATH"))
    }

    /// `<A href>` に渡す絶対パスを返す。`route` は `path!("/facet")` 等のルート定義に対応する。
    /// leptos_router の `<A>` は絶対パス（先頭 '/'）を base 前置せず素通しし、相対パスは
    /// 現在ルートからの不安定な解決になるため、base を自前で前置した絶対パスを組む。
    pub fn href(route: &str) -> String {
        Self::join_base(Self::router_base(), route)
    }

    /// base と route を結合する純粋ロジック。
    /// 不変条件: base は末尾スラッシュなし（normalize_base 保証）、route は先頭スラッシュあり。
    fn join_base(base: &str, route: &str) -> String {
        format!("{base}{route}")
    }

    /// public_url を router base にできる形へ整える。
    /// 末尾スラッシュは router base として不正なので除く。
    /// ローカル（trunk serve, public_url 未指定 or "/"）では "" を返し、Router は base なしとして扱う。
    fn normalize_base(public_url: Option<&str>) -> &str {
        match public_url {
            Some(url) if url != "/" && !url.is_empty() => url.trim_end_matches('/'),
            _ => "",
        }
    }

    pub fn now_iso() -> String {
        js_sys::Date::new_0()
            .to_iso_string()
            .as_string()
            .unwrap_or_default()
    }

    pub fn open_url(url: &str) {
        if let Some(win) = web_sys::window() {
            let _ = win.open_with_url_and_target(url, "_blank");
        }
    }

    pub fn trigger_download(filename: &str, content: &str) {
        let arr = Array::new();
        arr.push(&JsValue::from_str(content));
        let opts = BlobPropertyBag::new();
        opts.set_type("application/json");
        let blob = Blob::new_with_str_sequence_and_options(&arr, &opts).unwrap();
        let url = Url::create_object_url_with_blob(&blob).unwrap();
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let a: HtmlAnchorElement = document.create_element("a").unwrap().dyn_into().unwrap();
        a.set_href(&url);
        a.set_download(filename);
        let body = document.body().unwrap();
        body.append_child(&a).unwrap();
        a.click();
        body.remove_child(&a).unwrap();
        Url::revoke_object_url(&url).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::{CategoryAttribute, Color, Platform};

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

    #[test]
    fn 末尾スラッシュを除いた_path_prefix_を返す() {
        assert_eq!(Platform::normalize_base(Some("/cumulo/")), "/cumulo");
    }

    #[test]
    fn スラッシュなしはそのまま返す() {
        assert_eq!(Platform::normalize_base(Some("/cumulo")), "/cumulo");
    }

    #[test]
    fn ローカルの_未指定_空_ルートは_base_なし() {
        // trunk serve はアセットを "/" 配信するので base は付けない
        assert_eq!(Platform::normalize_base(None), "");
        assert_eq!(Platform::normalize_base(Some("")), "");
        assert_eq!(Platform::normalize_base(Some("/")), "");
    }

    #[test]
    fn href_は_base_を前置した絶対パスを返す() {
        // 絶対パスなら現在地に依らず base 配下の同じルートへ解決される
        assert_eq!(Platform::join_base("/cumulo", "/facet"), "/cumulo/facet");
        assert_eq!(Platform::join_base("/cumulo", "/"), "/cumulo/");
    }

    #[test]
    fn href_は_base_なしでもルートを保つ() {
        assert_eq!(Platform::join_base("", "/facet"), "/facet");
        assert_eq!(Platform::join_base("", "/"), "/");
    }
}
