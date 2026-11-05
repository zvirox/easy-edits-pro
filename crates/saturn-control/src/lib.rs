// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Control server library surface.
//!
//! The binary entry point lives in `main.rs`; the router and handlers are
//! exposed here so integration tests and embedders can mount them directly.

pub mod handlers;
pub mod server;
