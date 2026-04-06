use leptos::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use crate::cube_bridge::{init_cube, dimension_to_face, face_to_dimension};

/// サイドバーのミニキューブコンポーネント
#[component]
pub fn CubeNav(
    active_dim: ReadSignal<String>,
    on_face_change: Callback<String>,
) -> impl IntoView {
    // Rc<RefCell<...>> で非Sendな値をWASM単一スレッド内で保持
    let cube_handle: Rc<RefCell<Option<crate::cube_bridge::CubeHandle>>> =
        Rc::new(RefCell::new(None));
    let face_closure: Rc<RefCell<Option<Closure<dyn Fn(u32)>>>> =
        Rc::new(RefCell::new(None));

    let node_ref = NodeRef::<leptos::html::Canvas>::new();

    let cube_ref = Rc::clone(&cube_handle);
    let closure_ref = Rc::clone(&face_closure);

    Effect::new(move |_| {
        if node_ref.get().is_some() {
            let handle = init_cube("mini-cube-canvas");
            let cb = on_face_change.clone();
            let closure = Closure::wrap(Box::new(move |face_idx: u32| {
                let dim_id = face_to_dimension(face_idx).to_string();
                cb.run(dim_id);
            }) as Box<dyn Fn(u32)>);
            handle.on_face_change(&closure);
            *cube_ref.borrow_mut() = Some(handle);
            *closure_ref.borrow_mut() = Some(closure);
        }
    });

    // active_dim が変わったらキューブを回転
    let cube_ref2 = Rc::clone(&cube_handle);
    Effect::new(move |_| {
        let dim = active_dim.get();
        if let Some(handle) = cube_ref2.borrow().as_ref() {
            let face = dimension_to_face(&dim);
            handle.rotate_to_face(face);
        }
    });

    let nav_items: Vec<(&str, &str)> = vec![
        ("vendor", "ベンダー面"),
        ("env", "環境面"),
        ("category", "カテゴリ面"),
    ];

    view! {
        <div class="cube-nav">
            <div class="cube-canvas-wrapper">
                <canvas
                    id="mini-cube-canvas"
                    node_ref=node_ref
                    width="140"
                    height="140"
                />
            </div>
            <div class="cube-nav-tabs">
                {nav_items.into_iter().map(|(id, label)| {
                    let id_str = id.to_string();
                    let id_str2 = id_str.clone();
                    let on_face = on_face_change.clone();
                    view! {
                        <button
                            class=move || {
                                if active_dim.get() == id_str {
                                    "cube-tab active"
                                } else {
                                    "cube-tab"
                                }
                            }
                            on:click=move |_| {
                                on_face.run(id_str2.clone());
                            }
                        >
                            {label}
                        </button>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}
