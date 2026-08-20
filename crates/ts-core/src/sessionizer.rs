use crate::model::{ActivityInterval, EndReason, FocusSample};

const IDLE_THRESHOLD_MS: u64 = 300000;

#[derive(Debug, Default)]
pub struct Sessionizer {
    state: SessionState,
}

impl Sessionizer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, sample: FocusSample) -> Option<ActivityInterval> {
        let state = std::mem::take(&mut self.state);

        let (closed, next_state) = match state {
            SessionState::Empty => {
                if sample.idle_ms < IDLE_THRESHOLD_MS {
                    (
                        None,
                        SessionState::Open {
                            start_mono_ms: sample.mono_ms,
                            app: sample.app,
                            title: sample.title,
                        },
                    )
                } else {
                    (None, SessionState::Empty)
                }
            }
            SessionState::Open {
                app,
                title,
                start_mono_ms,
            } => {
                if sample.idle_ms >= IDLE_THRESHOLD_MS {
                    (
                        Some(ActivityInterval {
                            app,
                            title,
                            start_mono_ms,
                            end_mono_ms: sample
                                .mono_ms
                                .saturating_sub(sample.idle_ms)
                                .max(start_mono_ms),
                            end_reason: EndReason::Idle,
                        }),
                        SessionState::Idle,
                    )
                } else if app != sample.app || title != sample.title {
                    (
                        Some(ActivityInterval {
                            app,
                            title,
                            start_mono_ms,
                            end_mono_ms: sample.mono_ms,
                            end_reason: EndReason::FocusChange,
                        }),
                        SessionState::Open {
                            app: sample.app,
                            title: sample.title,
                            start_mono_ms: sample.mono_ms,
                        },
                    )
                } else {
                    (
                        None,
                        SessionState::Open {
                            start_mono_ms,
                            app,
                            title,
                        },
                    )
                }
            }
            SessionState::Idle => {
                if sample.idle_ms < IDLE_THRESHOLD_MS {
                    (
                        None,
                        SessionState::Open {
                            start_mono_ms: sample.mono_ms,
                            app: sample.app,
                            title: sample.title,
                        },
                    )
                } else {
                    (None, SessionState::Idle)
                }
            }
        };

        self.state = next_state;
        closed
    }
}

#[derive(Debug, PartialEq, Default)]
pub enum SessionState {
    #[default]
    Empty,
    Open {
        app: String,
        title: Option<String>,
        start_mono_ms: u64,
    },
    Idle,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_interval_when_focus_changes() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 0,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_2 = FocusSample {
            mono_ms: 1000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_3 = FocusSample {
            mono_ms: 2000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: 0,
        };

        // While the app and title don't change, nothing closes.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // The change of focus closes the previous interval.
        let closed = sessionizer.observe(sample_3);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 0,
                end_mono_ms: 2000,
                end_reason: EndReason::FocusChange,
            })
        );
    }

    #[test]
    fn close_idle_interval() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 0,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_2 = FocusSample {
            mono_ms: 60000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        // Did 60s/1min that it has been absent
        let sample_3 = FocusSample {
            mono_ms: 120000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 60000,
        };
        //  Did 180s/5min that it has been absent = stops counting the work.
        let sample_4 = FocusSample {
            mono_ms: 360000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: IDLE_THRESHOLD_MS,
        };

        // While the app is active, nothing closes.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);
        assert_eq!(sessionizer.observe(sample_3), None);

        let closed = sessionizer.observe(sample_4);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 0,
                end_mono_ms: 60000,
                end_reason: EndReason::Idle,
            })
        );
    }

    #[test]
    fn update_state_after_closing() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 0,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_2 = FocusSample {
            mono_ms: 1000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_3 = FocusSample {
            mono_ms: 2000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: 0,
        };

        // While the app and title don't change, nothing closes.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // The change of focus closes the previous interval.
        let closed = sessionizer.observe(sample_3);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 0,
                end_mono_ms: 2000,
                end_reason: EndReason::FocusChange,
            })
        );
    }

    #[test]
    fn close_idle_interval_and_focus_change_simultaneously() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 0,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_2 = FocusSample {
            mono_ms: 6000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        // Back to lunch and the first thing you do is switch screens.
        let sample_3 = FocusSample {
            mono_ms: 306000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: IDLE_THRESHOLD_MS,
        };

        // While the app and title don't change, nothing closes.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // The change of focus closes the previous interval.
        let closed = sessionizer.observe(sample_3);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 0,
                end_mono_ms: 6000,
                end_reason: EndReason::Idle,
            })
        );
    }

    #[test]
    fn do_not_open_interval_while_still_idle() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 0,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_2 = FocusSample {
            mono_ms: 6000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        // Back to lunch and the first thing you do is switch screens.
        let sample_3 = FocusSample {
            mono_ms: 306000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: IDLE_THRESHOLD_MS,
        };
        let sample_4 = FocusSample {
            mono_ms: 307000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: IDLE_THRESHOLD_MS + 1000,
        };
        let sample_5 = FocusSample {
            mono_ms: 308000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: 0,
        };
        let sample_6 = FocusSample {
            mono_ms: 309000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };

        // While the app and title don't change, nothing closes.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // The change of focus closes the previous interval.
        let closed = sessionizer.observe(sample_3);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 0,
                end_mono_ms: 6000,
                end_reason: EndReason::Idle,
            })
        );

        assert_eq!(sessionizer.observe(sample_4), None);
        assert_eq!(sessionizer.observe(sample_5), None);

        let closed = sessionizer.observe(sample_6);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Chrome".to_string(),
                title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
                start_mono_ms: 308000,
                end_mono_ms: 309000,
                end_reason: EndReason::FocusChange,
            })
        );
    }

    #[test]
    fn resumption_of_activity_with_non_zero_idle_time() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 0,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_2 = FocusSample {
            mono_ms: 6000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_3 = FocusSample {
            mono_ms: 306000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: IDLE_THRESHOLD_MS,
        };
        let sample_4 = FocusSample {
            mono_ms: 400000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 500,
        };
        let sample_5 = FocusSample {
            mono_ms: 405000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: 200,
        };

        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        let closed = sessionizer.observe(sample_3);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 0,
                end_mono_ms: 6000,
                end_reason: EndReason::Idle,
            })
        );

        assert_eq!(sessionizer.observe(sample_4), None);

        let closed = sessionizer.observe(sample_5);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 400000,
                end_mono_ms: 405000,
                end_reason: EndReason::FocusChange,
            })
        );
    }

    #[test]
    fn start_computer_with_open_app() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 0,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 400000,
        };

        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.state, SessionState::Empty);
    }

    #[test]
    fn impossible_idle_does_not_panic() {
        let mut sessionizer = Sessionizer::new();

        let sample_1 = FocusSample {
            mono_ms: 1000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 0,
        };
        let sample_2 = FocusSample {
            mono_ms: 2000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 500000,
        };

        sessionizer.observe(sample_1);
        let closed = sessionizer.observe(sample_2);

        assert_eq!(
            closed,
            Some(ActivityInterval {
                app: "Word".to_string(),
                title: Some("Petição A".to_string()),
                start_mono_ms: 1000,
                end_mono_ms: 1000,
                end_reason: EndReason::Idle,
            })
        );
    }
}
