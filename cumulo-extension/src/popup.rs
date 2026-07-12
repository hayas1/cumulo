use std::collections::{HashMap, HashSet};

use js_sys::Reflect;
use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

use cumulo_model::Forest;
use cumulo_web::{CategoryId, Client, Platform, LOCAL_STORE};

use crate::clip::Clip;

#[wasm_bindgen(inline_js = "export async function active_tab() { \
    const [t] = await chrome.tabs.query({ active: true, currentWindow: true }); \
    return t ? { url: t.url ?? '', title: t.title ?? '' } : null; \
}")]
extern "C" {
    #[wasm_bindgen(catch)]
    async fn active_tab() -> Result<JsValue, JsValue>;
}

#[component]
pub fn PopupApp() -> impl IntoView {
    let client = Client::new(&LOCAL_STORE);
    let bipartite = client.read();
    let title = RwSignal::new(String::new());
    let url = RwSignal::new(String::new());
    let dims = RwSignal::new(HashMap::<CategoryId, CategoryId>::new());
    let added = RwSignal::new(false);

    spawn_local(async move {
        if let Ok(tab) = active_tab().await {
            let field = |k: &str| {
                Reflect::get(&tab, &JsValue::from_str(k))
                    .ok()
                    .and_then(|v| v.as_string())
            };
            if let Some(u) = field("url") {
                url.set(u);
            }
            if let Some(t) = field("title") {
                title.set(t);
            }
        }
    });

    let add = move || {
        let clip = Clip {
            id: Platform::new_resource_id(),
            title: title.get_untracked(),
            url: url.get_untracked(),
            categories: dims.get_untracked().into_values().collect(),
            created_at: Platform::now_iso(),
        };
        client.update(|s| s.catalog.push(clip.into_resource()));
        added.set(true);
    };

    view! {
        <div class="popup" style="width:320px;padding:12px;display:flex;flex-direction:column;gap:8px;box-sizing:border-box">
            <input
                class="form-input"
                type="text"
                placeholder="タイトル"
                prop:value=move || title.get()
                on:input=move |ev| {
                    added.set(false);
                    title.set(event_target_value(&ev));
                }
            />
            <input
                class="form-input"
                type="text"
                placeholder="https://..."
                prop:value=move || url.get()
                on:input=move |ev| {
                    added.set(false);
                    url.set(event_target_value(&ev));
                }
            />

            {move || {
                let s = bipartite.get();
                s.taxonomy
                    .roots()
                    .into_iter()
                    .map(|root| {
                        let root_id = root.id.clone();
                        let root_label = if root.label.is_empty() {
                            root.id.to_string()
                        } else {
                            root.label.clone()
                        };
                        let options = s
                            .taxonomy
                            .dfs_order(&root.id, &HashSet::new())
                            .into_iter()
                            .filter_map(|(id, depth, _)| {
                                let n = s.taxonomy.node(&id)?;
                                let label = if n.label.is_empty() {
                                    id.to_string()
                                } else {
                                    n.label.clone()
                                };
                                let indent = "\u{00a0}\u{00a0}".repeat(depth);
                                Some(view! {
                                    <option value=id.to_string()>{format!("{indent}{label}")}</option>
                                })
                            })
                            .collect::<Vec<_>>();
                        let on_pick = root_id.clone();
                        view! {
                            <div
                                class="popup-row"
                                style="display:flex;align-items:center;gap:8px"
                            >
                                <span style="min-width:5rem;font-size:0.85rem;color:#666">
                                    {root_label}
                                </span>
                                <select
                                    class="form-input"
                                    style="flex:1"
                                    on:change=move |ev| {
                                        let v = event_target_value(&ev);
                                        let key = on_pick.clone();
                                        if v.is_empty() {
                                            dims.update(|d| {
                                                d.remove(&key);
                                            });
                                        } else if let Ok(val) = CategoryId::try_from(v.as_str()) {
                                            dims.update(|d| {
                                                d.insert(key, val);
                                            });
                                        }
                                    }
                                >
                                    <option value="">"(なし)"</option>
                                    {options}
                                </select>
                            </div>
                        }
                    })
                    .collect::<Vec<_>>()
            }}

            <button
                class="form-save-btn"
                on:click=move |_| add()
            >
                "+ cumulo に追加"
            </button>

            <Show when=move || added.get()>
                <span style="color:#2e7d32;font-size:0.85rem">"追加しました ✓"</span>
            </Show>

            <a
                href="index.html"
                target="_blank"
                rel="noopener"
                style="font-size:0.85rem;color:#3578c5;text-decoration:none"
            >
                "cumulo を開く →"
            </a>
        </div>
    }
}
