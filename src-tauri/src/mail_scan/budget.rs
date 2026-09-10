//! Per-run spend control and circuit breaker (spec §8.3).
//!
//! Both exist to bound what a single Scan press can cost. They fail in opposite
//! directions on purpose:
//!
//! - Hitting the **call cap** is a normal, successful outcome — the run ends
//!   `completed` with `budget_exhausted` set and the user gets a Continue button.
//! - Tripping the **breaker** is a failure — five consecutive transient errors means
//!   the endpoint is dead, and continuing would drain a rate-limit quota to learn
//!   nothing. The run ends `failed` with `E_LLM_UNAVAILABLE`.

/// Default per-run cap. High enough for a 90-day first scan with pass-1 batching,
/// low enough that a runaway loop is bounded.
pub const DEFAULT_MAX_CALLS: u32 = 600;

/// Consecutive transient failures before the breaker opens.
pub const BREAKER_THRESHOLD: u32 = 5;

#[derive(Debug, Clone, Copy)]
pub struct BudgetConfig {
    pub max_calls: u32,
    pub breaker_threshold: u32,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_calls: DEFAULT_MAX_CALLS,
            breaker_threshold: BREAKER_THRESHOLD,
        }
    }
}

/// Why no further call may be issued.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetStop {
    /// Call cap reached. Run ends `completed`; already-scored items are kept.
    Exhausted,
    /// Breaker open after repeated transient failures.
    Unavailable(String),
}

#[derive(Debug, Clone)]
pub struct Budget {
    config: BudgetConfig,
    calls_used: u32,
    consecutive_failures: u32,
    stop: Option<BudgetStop>,
}

impl Budget {
    pub fn new(config: BudgetConfig) -> Self {
        Self {
            config,
            calls_used: 0,
            consecutive_failures: 0,
            stop: None,
        }
    }

    pub fn calls_used(&self) -> u32 {
        self.calls_used
    }

    /// Claim one call. Reserving *before* the request means a request that never
    /// returns still counts against the cap.
    pub fn reserve(&mut self) -> Result<(), BudgetStop> {
        if let Some(stop) = &self.stop {
            return Err(stop.clone());
        }
        if self.calls_used >= self.config.max_calls {
            let stop = BudgetStop::Exhausted;
            self.stop = Some(stop.clone());
            return Err(stop);
        }
        self.calls_used += 1;
        Ok(())
    }

    pub fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    /// Record a transient failure. Returns the stop reason if this one opened the
    /// breaker. Non-transient failures must not be reported here — a bad model id
    /// fails the run on its own and should not need five tries to do it.
    pub fn record_transient_failure(&mut self, detail: &str) -> Option<BudgetStop> {
        self.consecutive_failures += 1;
        if self.consecutive_failures >= self.config.breaker_threshold && self.stop.is_none() {
            let stop = BudgetStop::Unavailable(format!(
                "{} consecutive provider failures; last: {detail}",
                self.consecutive_failures
            ));
            self.stop = Some(stop.clone());
            return self.stop.clone();
        }
        None
    }

    /// Production learns this from [`Self::reserve`]; tests assert on it directly.
    #[cfg(test)]
    pub fn is_stopped(&self) -> bool {
        self.stop.is_some()
    }
}

/// Pre-run cost preview (spec §8.3): what the user is asked to approve.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CallEstimate {
    pub listings: u32,
    pub pass1_calls: u32,
    pub pass2_calls: u32,
    pub enrich_calls: u32,
    pub total_calls: u32,
    pub over_budget: bool,
}

/// Estimate calls for `listings` new items. `pass_rate` is the share expected to clear
/// the pass-1 gate — a coarse prior, shown as an estimate and never billed against.
pub fn estimate_calls(
    listings: u32,
    batch_size: usize,
    pass_rate: f64,
    max_calls: u32,
) -> CallEstimate {
    let batch = batch_size.max(1) as u32;
    let pass1_calls = listings.div_ceil(batch);
    let pass2_calls = (f64::from(listings) * pass_rate.clamp(0.0, 1.0)).round() as u32;
    // Enrichment runs only on what survives pass 2; assume most of it does.
    let enrich_calls = pass2_calls;
    let total_calls = pass1_calls + pass2_calls + enrich_calls;
    CallEstimate {
        listings,
        pass1_calls,
        pass2_calls,
        enrich_calls,
        total_calls,
        over_budget: total_calls > max_calls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserve_stops_at_the_cap() {
        let mut b = Budget::new(BudgetConfig {
            max_calls: 2,
            ..Default::default()
        });
        assert!(b.reserve().is_ok());
        assert!(b.reserve().is_ok());
        assert_eq!(b.reserve().unwrap_err(), BudgetStop::Exhausted);
        assert_eq!(b.calls_used(), 2, "a refused call must not be charged");
    }

    #[test]
    fn exhaustion_is_sticky() {
        let mut b = Budget::new(BudgetConfig {
            max_calls: 1,
            ..Default::default()
        });
        b.reserve().unwrap();
        assert!(b.reserve().is_err());
        // Even after a success is recorded, the cap does not reopen mid-run.
        b.record_success();
        assert_eq!(b.reserve().unwrap_err(), BudgetStop::Exhausted);
    }

    #[test]
    fn breaker_opens_after_five_consecutive_failures() {
        let mut b = Budget::new(BudgetConfig::default());
        for _ in 0..4 {
            assert!(b.record_transient_failure("503").is_none());
        }
        let stop = b.record_transient_failure("503").expect("fifth must trip it");
        assert!(matches!(stop, BudgetStop::Unavailable(_)));
        assert!(b.is_stopped());
        assert!(matches!(b.reserve().unwrap_err(), BudgetStop::Unavailable(_)));
    }

    #[test]
    fn a_success_resets_the_failure_streak() {
        let mut b = Budget::new(BudgetConfig::default());
        for _ in 0..4 {
            b.record_transient_failure("503");
        }
        b.record_success();
        for _ in 0..4 {
            assert!(b.record_transient_failure("503").is_none());
        }
        assert!(!b.is_stopped(), "intermittent failures must not trip the breaker");
    }

    #[test]
    fn estimate_shows_the_batching_saving() {
        let e = estimate_calls(118, 10, 0.34, 600);
        assert_eq!(e.pass1_calls, 12, "118 listings batched by 10");
        assert_eq!(e.pass2_calls, 40);
        assert_eq!(e.total_calls, 12 + 40 + 40);
        assert!(!e.over_budget);
    }

    #[test]
    fn estimate_flags_a_run_that_would_blow_the_cap() {
        let e = estimate_calls(5000, 10, 0.5, 600);
        assert!(e.over_budget);
    }
}
