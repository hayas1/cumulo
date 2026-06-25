//! 画面の「枠（view）」。各 view は表示する枠の構築に集中し、resource / category などの
//! ドメインは「その枠へ自分をどう見せるか」に集中する、という分担を意図する。
//!
//! 設計メモ: 将来 view やプレゼンテーションが増えたら、枠ごとに描画 trait
//! （例: `MapPresentable` / `ListRowPresentable`）を定義し、各ドメイン型がそれを実装して
//! 差し込む形へ寄せる。枠が interface を所有しドメイン型が実装側になるので、
//! ドメイン型が全ての枠を知る逆依存を避けられる。
//! 現状は枠ごとにプレゼンテーション実装が 1 つなので trait は導入していない（YAGNI）。

pub mod facet;
pub mod map;
