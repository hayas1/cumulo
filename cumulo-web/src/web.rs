use crate::model::DimensionNode;
use web_sys::window;

pub fn open_url(url: &str) {
    if let Some(win) = window() {
        let _ = win.open_with_url_and_target(url, "_blank");
    }
}

impl DimensionNode {
    pub fn new_id() -> String {
        let n = (js_sys::Math::random() * 1e15) as u64;
        format!("node{n:x}")
    }

    pub fn random_color() -> String {
        const PALETTE: &[&str] = &[
            "#ef4444", "#f97316", "#f59e0b", "#eab308", "#84cc16", "#22c55e", "#10b981", "#14b8a6",
            "#06b6d4", "#3b82f6", "#6366f1", "#8b5cf6", "#a855f7", "#d946ef", "#ec4899", "#f43f5e",
        ];
        let idx = (js_sys::Math::random() * PALETTE.len() as f64) as usize;
        PALETTE[idx.min(PALETTE.len() - 1)].to_string()
    }
}
