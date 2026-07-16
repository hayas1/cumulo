//! Browser-driven end-to-end harness. The whole crate is gated behind the
//! `browser` feature so `cargo test --all` never compiles Chromium tooling;
//! run the scenarios with `cargo test -p cumulo-e2e --features browser`.
#![cfg(feature = "browser")]

mod chrome;
mod session;
mod site;

pub use session::{DropZone, Session};
