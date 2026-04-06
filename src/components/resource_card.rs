use crate::model::Resource;
use leptos::*;
use web_sys::window;

fn open_url(url: String) {
    if let Some(win) = window() {
        let _ = win.open_with_url_and_target(&url, "_blank");
    }
}

fn vendor_css_class(vendor: &str) -> &'static str {
    match vendor {
        "AWS" => "vendor-aws",
        "GCP" => "vendor-gcp",
        "Azure" => "vendor-azure",
        _ => "vendor-other",
    }
}

/// 表示優先度の高いキーを順序付きで返す
fn display_attr_keys() -> &'static [&'static str] {
    &[
        "vendor",
        "project",
        "account",
        "subscription",
        "env",
        "service",
        "resource_type",
        "region",
        "team",
    ]
}

#[component]
pub fn ResourceCard(resource: Resource, highlighted: Signal<bool>) -> impl IntoView {
    let vendor = resource
        .attrs
        .get("vendor")
        .cloned()
        .unwrap_or_default();
    let vendor_class = vendor_css_class(&vendor);

    // 表示優先度の高いキーを先に、残りをアルファベット順で
    let mut ordered_attrs: Vec<(String, String)> = Vec::new();
    for key in display_attr_keys() {
        if let Some(val) = resource.attrs.get(*key) {
            ordered_attrs.push((key.to_string(), val.clone()));
        }
    }
    for (k, v) in &resource.attrs {
        if !display_attr_keys().contains(&k.as_str()) {
            ordered_attrs.push((k.clone(), v.clone()));
        }
    }

    let console_url = resource.console_url.clone();

    view! {
        <div
            class="resource-card"
            class=("highlighted", move || highlighted.get())
        >
            <div class="resource-card-header">
                <span class="resource-name">{resource.name.clone()}</span>
                <span class=format!("vendor-badge {}", vendor_class)>{vendor.clone()}</span>
            </div>
            <div class="resource-attrs">
                {ordered_attrs.into_iter().filter(|(k, _)| k != "vendor").map(|(k, v)| {
                    view! {
                        <span class="attr-tag">
                            <span>{k}</span>
                            {v}
                        </span>
                    }
                }).collect::<Vec<_>>()}
            </div>
            <div class="resource-card-footer">
                <button
                    class="console-link"
                    on:click=move |_| open_url(console_url.clone())
                >
                    "コンソールへ →"
                </button>
            </div>
        </div>
    }
}
