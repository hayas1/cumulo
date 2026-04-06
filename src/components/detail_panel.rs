use leptos::prelude::*;
use crate::model::{Resource, AppStore, resolve_dimension};
use crate::cube_bridge::open_url;

#[component]
pub fn DetailPanel(
    selected: ReadSignal<Option<Resource>>,
    store: ReadSignal<AppStore>,
) -> impl IntoView {
    view! {
        <div class="detail-panel">
            {move || match selected.get() {
                None => view! {
                    <div class="detail-empty">
                        <p>"リソースを選択してください"</p>
                    </div>
                }.into_any(),
                Some(resource) => {
                    let s = store.get();
                    let dims: Vec<(String, Option<String>)> = s.dimensions.iter().map(|dim| {
                        let val = resolve_dimension(&resource, dim);
                        (dim.label.clone(), val)
                    }).collect();

                    let console_url = resource.console_url.clone();
                    let console_url2 = console_url.clone();
                    let resource_name = resource.name.clone();

                    // ベンダー色クラス
                    let vendor_class = resource.raw_tags
                        .get("vendor")
                        .map(|v| format!("detail-vendor vendor-{}", v.to_lowercase()))
                        .unwrap_or_else(|| "detail-vendor".to_string());

                    view! {
                        <div class="detail-content">
                            <div class=vendor_class>
                                {resource.raw_tags.get("vendor").cloned().unwrap_or_default()}
                            </div>
                            <h2 class="detail-name">{resource_name}</h2>

                            <div class="detail-tags">
                                {dims.into_iter().map(|(label, value)| {
                                    view! {
                                        <div class="detail-row">
                                            <span class="detail-key">{label}</span>
                                            <span class="detail-val">
                                                {value.unwrap_or_else(|| "—".to_string())}
                                            </span>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>

                            // raw_tags も全部表示
                            <details class="raw-tags-section">
                                <summary>"Raw Tags"</summary>
                                <div class="raw-tags">
                                    {resource.raw_tags.iter().map(|(k, v)| {
                                        let k = k.clone();
                                        let v = v.clone();
                                        view! {
                                            <div class="raw-tag-row">
                                                <span class="raw-tag-key">{k}</span>
                                                <span class="raw-tag-val">{v}</span>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            </details>

                            <button
                                class="console-jump-btn"
                                on:click=move |_| {
                                    open_url(&console_url2);
                                }
                            >
                                "コンソールへジャンプ ↗"
                            </button>
                        </div>
                    }.into_any()
                }
            }}
        </div>
    }
}
