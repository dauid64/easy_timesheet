#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "macos")]
pub use macos::MacOsMonitor;

use std::collections::VecDeque;

use ts_core::FocusSample;

pub struct FakeMonitor {
    pending: VecDeque<FocusSample>,
}

impl FakeMonitor {
    pub fn new(samples: Vec<FocusSample>) -> Self {
        FakeMonitor {
            pending: samples.into(),
        }
    }
}

impl ActivityMonitor for FakeMonitor {
    fn sample(&mut self) -> Option<FocusSample> {
        self.pending.pop_front()
    }
}

pub trait ActivityMonitor {
    fn sample(&mut self) -> Option<FocusSample>;
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use ts_core::{ActivityInterval, Sessionizer};

    use super::*;

    #[test]
    fn test_fake_monitor() {
        let samples = vec![
            FocusSample {
                mono_ms: 0,
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                idle_ms: 0,
            },
            FocusSample {
                mono_ms: 1000,
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                idle_ms: 0,
            },
            FocusSample {
                mono_ms: 2000,
                app: "Chrome".to_string(),
                title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
                idle_ms: 0,
            },
        ];

        let mut monitor = FakeMonitor::new(samples.clone());

        for expected_sample in samples {
            let sample = monitor.sample();
            assert_eq!(sample, Some(expected_sample));
        }

        // After all samples are consumed, the monitor should return None
        assert_eq!(monitor.sample(), None);
    }

    #[test]
    fn fake_monitor_feeds_sessionizer() {
        let samples = vec![
            FocusSample {
                mono_ms: 0,
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                idle_ms: 0,
            },
            FocusSample {
                mono_ms: 1000,
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                idle_ms: 0,
            },
            FocusSample {
                mono_ms: 2000,
                app: "Chrome".to_string(),
                title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
                idle_ms: 0,
            },
        ];

        let mut monitor = FakeMonitor::new(samples.clone());
        let mut sessionizer = Sessionizer::new();
        let mut closed = Vec::new();

        while let Some(sample) = monitor.sample() {
            if let Some(interval) = sessionizer.observe(sample) {
                closed.push(interval);
            }
        }

        assert_eq!(closed.len(), 1);
        assert_eq!(
            closed[0],
            ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 0,
                end_mono_ms: 2000,
                end_reason: ts_core::EndReason::FocusChange,
            }
        )
    }
}
