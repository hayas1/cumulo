//! ファセット（リスト）view。サイドバーで軸/値を選び、一覧として見せる枠。
//!
//! - `page`: ルートに割り当てる画面コンポーネント [`FacetView`]
//! - `sidebar`: 軸/値の絞り込みサイドバー（マップ view からも再利用する）

pub mod sidebar;
mod page;

pub use page::FacetView;
