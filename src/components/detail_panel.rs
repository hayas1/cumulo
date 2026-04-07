use crate::model::AppStore;
use leptos::*;
use web_sys::window;

fn open_url(url: String) {
    if let Some(win) = window() {
        let _ = win.open_with_url_and_target(&url, "_blank");
    }
}

#[component]
pub fn DetailPanel(
    store: ReadSignal<AppStore>,
    selected_id: RwSignal<Option<String>>,
) -> impl IntoView {
    let resource = create_memo(move |_| {
        let id = selected_id.get()?;
        let s = store.get();
        s.resources.iter().find(|r| r.id == id).cloned()
    });

    view! {
        <Show when=move || resource.get().is_some()>
            <div class="detail-panel">
                {move || {
                    resource
                        .get()
                        .map(|r| {
                            let url = r.console_url.clone();
                            let freq = r.freq;

                            let mut attrs_sorted: Vec<_> = r
                                .attrs
                                .iter()
                                .map(|(k, v)| (k.clone(), v.clone()))
                                .collect();
                            attrs_sorted.sort_by_key(|(k, _)| k.clone());

                            // children を2つのクロージャ用に事前クローン
                            let children = r.children.clone();
                            let children_check = children.clone();

                            view! {
                                // ヘッダー
                                <div class="detail-header">
                                    <div class="detail-name">{r.name.clone()}</div>
                                    <button
                                        class="detail-close"
                                        on:click=move |_| selected_id.set(None)
                                    >
                                        "×"
                                    </button>
                                </div>

                                // 属性一覧
                                <div class="detail-body">
                                    <div class="detail-section-title">"属性"</div>
                                    <div class="detail-attrs">
                                        {attrs_sorted
                                            .into_iter()
                                            .map(|(k, v)| {
                                                view! {
                                                    <div class="detail-attr-row">
                                                        <span class="detail-attr-key">{k}</span>
                                                        <span class="detail-attr-val">{v}</span>
                                                    </div>
                                                }
                                            })
                                            .collect::<Vec<_>>()}
                                    </div>

                                    // 子リソース
                                    <Show when=move || !children_check.is_empty()>
                                        <div class="detail-section-title">"子リソース"</div>
                                        <div class="detail-children">
                                            {children
                                                .iter()
                                                .map(|c| {
                                                    let curl = c.console_url.clone();
                                                    let cfreq = c.freq;
                                                    view! {
                                                        <div class="child-row">
                                                            <span class="child-name">
                                                                {c.name.clone()}
                                                            </span>
                                                            <span class="child-freq">
                                                                "×" {cfreq}
                                                            </span>
                                                            <button
                                                                class="child-jump"
                                                                on:click=move |_| {
                                                                    open_url(curl.clone())
                                                                }
                                                            >
                                                                "→"
                                                            </button>
                                                        </div>
                                                    }
                                                })
                                                .collect::<Vec<_>>()}
                                        </div>
                                    </Show>
                                </div>

                                // フッター: コンソールジャンプ
                                <div class="detail-footer">
                                    <span class="detail-freq">
                                        "アクセス頻度: " {freq}
                                    </span>
                                    <button
                                        class="console-jump-btn"
                                        on:click=move |_| open_url(url.clone())
                                    >
                                        "コンソールへ →"
                                    </button>
                                </div>
                            }
                        })
                }}
            </div>
        </Show>
    }
}
