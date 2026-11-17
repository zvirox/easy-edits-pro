// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Control server and authenticated long-lived headless daemon (AF9).
//!
//! The binary entry point lives in `main.rs`; the router, UDS server,
//! authentication, budget, session isolation, and audit modules are
//! exposed here so integration tests and embedders can mount them directly.

pub mod api;
pub mod audit;
pub mod auth;
pub mod budget;
pub mod guard;
pub mod handlers;
pub mod project;
pub mod provenance;
pub mod runner;
pub mod server;
pub mod session;
pub mod uds;

pub use audit::{AuditLogger, AuditRecord, compute_argument_hash};
pub use auth::{AuthClaims, AuthError, AuthManager};
pub use budget::{BudgetError, RateLimiter, TimeBudget};
pub use project::{ProjectError, project_close, project_open, project_save, project_status};
pub use provenance::{ProvenanceError, validate_path_containment, validate_untrusted_media_text};
pub use runner::{
    ByoEndpoint, EndpointError, JobRecord, JobState, RunnerConfig, RunnerError, RunnerService,
    RunnerStatus, WorkerBudget, WorkerError, WorkerLease, WorkerUsage,
};
pub use server::{ControlServer, DaemonConfig};
pub use session::{DaemonSession, SessionError, SessionManager};
pub use uds::UdsServer;
