//! Control server library surface.
//!
//! The binary entry point lives in `main.rs`; the router and handlers are
//! exposed here so integration tests and embedders can mount them directly.

pub mod handlers;
pub mod server;
