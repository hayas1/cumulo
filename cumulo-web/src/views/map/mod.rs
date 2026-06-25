//! マップ view。枠（[`MapView`] / canvas / controls）とマップ固有の計算
//! （force / layout / lod / zoom）を同居させる。
//!
//! - `page`: ルートに割り当てる画面コンポーネント [`MapView`]
//! - `canvas`: SVG 描画コンポーネント
//! - `controls`: ズーム/件数バー
//! - `force` / `layout` / `lod` / `zoom`: 描画のための純粋計算（力学配置・LOD・ズーム）

pub mod canvas;
pub mod controls;
pub mod force;
pub mod layout;
pub mod lod;
mod page;
pub mod zoom;

pub use page::MapView;
