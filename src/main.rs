mod components;
mod cube_bridge;
mod model;
mod storage;

use leptos::prelude::*;
use crate::components::cube_nav::CubeNav;
use crate::components::detail_panel::DetailPanel;
use crate::components::slice_grid::SliceGrid;
use crate::model::Resource;
use crate::storage::{load_store, save_store};

#[component]
fn App() -> impl IntoView {
    let (store, set_store) = signal(load_store());
    let (selected_resource, set_selected_resource) = signal::<Option<Resource>>(None);

    // axis_z の変化を通知するシグナル（CubeNavの面と同期）
    let active_dim = RwSignal::new(store.get_untracked().cube_config.axis_z.clone());

    // 面が変わったら axis_z を更新し LocalStorage に保存
    let on_face_change = Callback::new(move |dim_id: String| {
        active_dim.set(dim_id.clone());
        set_store.update(|s| {
            s.cube_config.axis_z = dim_id;
        });
        let _ = save_store(&store.get_untracked());
    });

    view! {
        <div class="app-layout">
            // サイドバー
            <aside class="sidebar">
                <div class="sidebar-logo">
                    <span class="logo-text">"cumulo"</span>
                    <span class="logo-sub">"cloud navigator"</span>
                </div>
                <CubeNav
                    active_dim=active_dim.read_only()
                    on_face_change=on_face_change
                />
            </aside>

            // メインエリア
            <main class="main-area">
                <header class="main-header">
                    <h1>"Cloud Resource Explorer"</h1>
                    <span class="resource-count">
                        {move || store.get().resources.len()} " resources"
                    </span>
                </header>
                <SliceGrid
                    store=store
                    selected_resource=set_selected_resource
                />
            </main>

            // 詳細パネル
            <DetailPanel
                selected=selected_resource
                store=store
            />
        </div>
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(App);
}
