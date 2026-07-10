//! 拡張が提示する UI サーフェス（mode）。同じ wasm を各エントリ HTML が読み込み、
//! どの mode として body に mount するかを location から解決する。
//! いまは popup（Web クリッパー）と全画面アプリの 2 つだけ。増える mode はここに並べる。

use leptos::prelude::*;

use crate::popup::PopupApp;

/// 拡張の mount 先。エントリ HTML（popup.html / index.html）に 1:1 で対応する。
pub enum Mode {
    /// action.default_popup=popup.html。開いているページをクリップする Web クリッパー。
    Popup,
    /// index.html。cumulo-web をそのまま全画面で開くアプリ本体。
    App,
}

impl Mode {
    /// エントリのパスから mode を決める。popup.html だけ Popup、他は App に倒す。
    /// 副作用を持たない純粋な写像にして、location 取得（[`Mode::current`]）と分離する。
    pub fn resolve(pathname: &str) -> Self {
        if pathname.contains("popup") {
            Mode::Popup
        } else {
            Mode::App
        }
    }

    /// いま読み込まれているページの mode。location.pathname から解決する。
    pub fn current() -> Self {
        let pathname = web_sys::window()
            .and_then(|w| w.location().pathname().ok())
            .unwrap_or_default();
        Self::resolve(&pathname)
    }

    /// この mode の view を body に mount する。全 mode を同じ形（component を mount）で並べる。
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
