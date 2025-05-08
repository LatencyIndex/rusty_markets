use rand::{rngs::SmallRng, Rng, SeedableRng};
use tokio::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub struct ExpBackoffConfig {
    pub wait_min: Duration,
    pub wait_max: Duration,
}

/// Self-contained exponential backoff with jitter.
pub struct ExpBackoff {
    config: ExpBackoffConfig,
    attempts: u32,
    last_fail: Option<Instant>,
    // Time to wait after last_fail.
    // Stored so that it does not change within an attempt due to the jitter rng.
    wait_total: Duration,
    rng: SmallRng,
}

impl ExpBackoff {
    pub fn new(config: ExpBackoffConfig) -> Self {
        Self {
            config,
            attempts: 0,
            last_fail: None,
            wait_total: Duration::ZERO,
            rng: SmallRng::from_rng(&mut rand::rng()),
        }
    }
    /// Number of failed attempts since last success.
    pub fn attempt_count(&self) -> u32 {
        self.attempts
    }
    // Generate a jittered wait duration for the current nb. of attempts.
    fn gen_total_wait(&mut self) -> Duration {
        if self.attempts == 0 {
            // No wait on initial attempt
            Duration::ZERO
        } else {
            // Double wait time for each failed attempt
            let dt = self
                .config
                .wait_min
                .saturating_mul(2_u32.saturating_pow(self.attempts))
                .min(self.config.wait_max);
            // On attempt 1, dt = 2^1 * wait_min, dt/2 = wait_min
            // so even after jitter, it is always the case that wait >= wait_min
            self.rng.random_range((dt / 2)..=dt)
        }
    }
    /// Wait for the start of the next attempt, based on when the previous attempt started.
    pub async fn wait(&mut self) {
        // Only wait on subsequent attempts, otherwise start immediately
        if let Some(remaining_wait) = self.remaining_wait() {
            tokio::time::sleep(remaining_wait).await;
        }
    }
    /// Mark the current moment as the start of an attempt, increment the attempt counter,
    /// and set the remaining wait to measure to the start of the next attempt.
    pub fn start_attempt(&mut self) {
        // Record attempt as if it had failed, since it will just be reset in case of success.
        self.last_fail = Some(Instant::now());
        self.attempts = self.attempts.saturating_add(1);
        self.wait_total = self.gen_total_wait();
    }
    /// Exact behavior of tokio::time::sleep(Duration::zero) is not well specified,
    /// so None is returned for cases where no wait is needed, so that
    /// sleep can be omitted more ergonomically.
    pub fn remaining_wait(&self) -> Option<Duration> {
        self.last_fail.map(|last_fail| {
            // time::Instant lacks saturating operations, so we prefer to express the wait in terms of time::Duration.
            let wait_elapsed = last_fail.elapsed();
            self.wait_total.saturating_sub(wait_elapsed)
        })
    }
    /// To be called when an attempt succeeds, and the backoff should be reset.
    pub fn reset(&mut self) {
        self.attempts = 0;
        self.last_fail = None;
    }
}
