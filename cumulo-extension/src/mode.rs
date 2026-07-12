use leptos::prelude::*;

use crate::popup::PopupApp;

pub enum Mode {
    Popup,
    App,
}

impl Mode {
    pub fn resolve(pathname: &str) -> Self {
        if pathname.contains("popup") {
            Mode::Popup
        } else {
            Mode::App
        }
    }

    pub fn current() -> Self {
        let pathname = web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_default();
        Self::resolve(&pathname)
    }

    pub fn mount(self) {
        match self {
            Mode::Popup => {
                mount_to_body(PopupApp);
            }
            Mode::App => {
                mount_to_body(cumulo_web::RootLocalStore);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_path_resolves_to_popup() {
        assert!(matches!(Mode::resolve("/popup.html"), Mode::Popup));
    }

    #[test]
    fn index_path_resolves_to_app() {
        assert!(matches!(Mode::resolve("/index.html"), Mode::App));
    }

    #[test]
    fn root_path_resolves_to_app() {
        assert!(matches!(Mode::resolve("/"), Mode::App));
    }
}
