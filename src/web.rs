use web_sys::window;

pub fn open_url(url: &str) {
    if let Some(win) = window() {
        let _ = win.open_with_url_and_target(url, "_blank");
    }
}
