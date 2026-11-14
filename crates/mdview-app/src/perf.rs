//! Lightweight frame-time telemetry for the 120 Hz budget.

use std::collections::VecDeque;
use std::time::Duration;

/// Rolling frame-time window. A 120 Hz frame has 8.33 ms of budget.
pub struct FrameMetrics {
    samples: VecDeque<Duration>,
    frames: u64,
}

impl Default for FrameMetrics {
    fn default() -> Self {
        Self {
            samples: VecDeque::with_capacity(120),
            frames: 0,
        }
    }
}

impl FrameMetrics {
    /// Record one CPU frame.
    pub fn record(&mut self, duration: Duration) {
        self.samples.push_back(duration);
        self.frames += 1;
        if self.samples.len() >= 120 {
            let p50 = percentile(&self.samples, 0.50);
            let p95 = percentile(&self.samples, 0.95);
            let max = self.samples.iter().copied().max().unwrap_or_default();
            let over = self
                .samples
                .iter()
                .filter(|sample| sample.as_secs_f64() * 1_000.0 > 8.33)
                .count();
            log::info!(
                "perf frames={} p50={:.3}ms p95={:.3}ms max={:.3}ms over_8.33ms={over}",
                self.frames,
                p50.as_secs_f64() * 1_000.0,
                p95.as_secs_f64() * 1_000.0,
                max.as_secs_f64() * 1_000.0
            );
            self.samples.clear();
        }
    }
}

fn percentile(samples: &VecDeque<Duration>, percent: f64) -> Duration {
    let mut sorted = samples.iter().copied().collect::<Vec<_>>();
    sorted.sort();
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let index = ((sorted.len() - 1) as f64 * percent).round() as usize;
    sorted[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_are_stable() {
        let metrics = FrameMetrics {
            samples: (1..=100).map(Duration::from_millis).collect(),
            frames: 100,
        };
        assert_eq!(percentile(&metrics.samples, 0.50), Duration::from_millis(51));
        assert_eq!(percentile(&metrics.samples, 0.95), Duration::from_millis(95));
    }
}
