use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen(module = "/public/cube.js")]
extern "C" {
    pub type CubeHandle;

    #[wasm_bindgen(js_name = "initCube")]
    pub fn init_cube(canvas_id: &str) -> CubeHandle;

    #[wasm_bindgen(method, js_name = "rotateToFace")]
    pub fn rotate_to_face(this: &CubeHandle, face_index: u32);

    #[wasm_bindgen(method, js_name = "onFaceChange")]
    pub fn on_face_change(this: &CubeHandle, callback: &Closure<dyn Fn(u32)>);

    #[wasm_bindgen(method, js_name = "destroy")]
    pub fn destroy(this: &CubeHandle);
}

/// キューブの面インデックス → Dimension IDの対応
/// face 0: vendor面（正面）
/// face 1: env面（上面）
/// face 2: category面（右面）
/// face 3: vendor面（背面）
/// face 4: env面（下面）
/// face 5: category面（左面）
pub fn face_to_dimension(face_index: u32) -> &'static str {
    match face_index % 3 {
        0 => "vendor",
        1 => "env",
        2 => "category",
        _ => "vendor",
    }
}

pub fn dimension_to_face(dim_id: &str) -> u32 {
    match dim_id {
        "vendor" => 0,
        "env" => 1,
        "category" => 2,
        _ => 0,
    }
}

/// JS の window.open を呼び出す
pub fn open_url(url: &str) {
    if let Some(window) = web_sys::window() {
        let _ = window.open_with_url_and_target(url, "_blank");
    }
}

#[allow(dead_code)]
pub fn console_log(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}
