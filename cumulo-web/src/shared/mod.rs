//! サイト横断で共有するもの（モジュール名は暫定）。
//! 値型（[`color`]）と、複数機能から使うコンポーネント（palette / settings_modal）。

pub mod color;
pub mod palette;
pub mod settings_modal;

pub use color::Color;
