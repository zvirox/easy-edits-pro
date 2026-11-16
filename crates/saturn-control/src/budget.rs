// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2025-2026 zvirox and the Easy Edits Pro authors
// Also available under the EEP commercial licence — see LICENSING.md
//! Bounded rate and time budgets for daemon sessions (AF9).

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "error", content = "message")]
pub enum BudgetError {
    #[error("RateLimitExceeded: rate limit of {limit} requests per minute exceeded")]
    RateLimitExceeded { limit: u32 },
    #[error("BudgetExceeded: session time budget of {limit_secs}s exceeded")]
    BudgetExceeded { limit_secs: u64 },
}

impl BudgetError {
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::RateLimitExceeded { .. } => "RateLimitExceeded",
            Self::BudgetExceeded { .. } => "BudgetExceeded",
        }
    }
}

/// Sliding-window rate limiter per session.
#[derive(Clone, Debug)]
pub struct RateLimiter {
    pub max_requests_per_minute: u32,
    window_start: Instant,
    request_count: u32,
}

impl RateLimiter {
    pub fn new(max_requests_per_minute: u32) -> Self {
        Self {
            max_requests_per_minute,
            window_start: Instant::now(),
            request_count: 0,
        }
    }

    pub fn check(&mut self) -> Result<(), BudgetError> {
        let now = Instant::now();
        if now.duration_since(self.window_start) >= Duration::from_secs(60) {
            self.window_start = now;
            self.request_count = 0;
        }

        if self.request_count >= self.max_requests_per_minute {
            return Err(BudgetError::RateLimitExceeded {
                limit: self.max_requests_per_minute,
            });
        }

        self.request_count += 1;
        Ok(())
    }

    pub fn current_count(&self) -> u32 {
        self.request_count
    }
}

/// Wall-clock session time budget enforcement.
#[derive(Clone, Debug)]
pub struct TimeBudget {
    pub budget: Option<Duration>,
    pub start_time: Instant,
}

impl TimeBudget {
    pub fn new(budget_secs: Option<u64>) -> Self {
        Self {
            budget: budget_secs.map(Duration::from_secs),
            start_time: Instant::now(),
        }
    }

    pub fn check(&self) -> Result<(), BudgetError> {
        if let Some(budget) = self.budget {
            if self.start_time.elapsed() > budget {
                return Err(BudgetError::BudgetExceeded {
                    limit_secs: budget.as_secs(),
                });
            }
        }
        Ok(())
    }

    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_up_to_limit_and_rejects_excess() {
        let mut limiter = RateLimiter::new(3);
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());
        assert!(limiter.check().is_ok());

        let err = limiter.check().unwrap_err();
        assert_eq!(err, BudgetError::RateLimitExceeded { limit: 3 });
        assert_eq!(err.error_code(), "RateLimitExceeded");
    }

    #[test]
    fn time_budget_allows_within_budget_and_rejects_after_expiry() {
        let budget = TimeBudget {
            budget: Some(Duration::from_millis(50)),
            start_time: Instant::now() - Duration::from_millis(100),
        };
        let err = budget.check().unwrap_err();
        assert_eq!(err, BudgetError::BudgetExceeded { limit_secs: 0 });
        assert_eq!(err.error_code(), "BudgetExceeded");
    }

    #[test]
    fn unconstrained_budget_always_succeeds() {
        let budget = TimeBudget::new(None);
        assert!(budget.check().is_ok());
    }
}
