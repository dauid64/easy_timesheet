const FIVE_MINUTES_IN_MILLIS: u64 = 300000;

#[derive(Debug, Default)]
struct Sessionizer {
    state: SessionState,
}

impl Sessionizer {
    fn new() -> Self {
        Sessionizer {
            ..Default::default()
        }
    }

    fn observe(&mut self, sample: FocusSample) -> Option<ActivityInterval> {
        let state = std::mem::take(&mut self.state);

        let (closed, next_state) = match state {
            SessionState::Empty => (
                None,
                SessionState::Open {
                    start_mono_ms: sample.mono_ms,
                    app: sample.app,
                    title: sample.title,
                },
            ),
            SessionState::Open {
                app,
                title,
                start_mono_ms,
            } => {
                if sample.idle_ms >= FIVE_MINUTES_IN_MILLIS {
                    (
                        Some(ActivityInterval {
                            app,
                            title,
                            start_mono_ms,
                            end_mono_ms: sample.mono_ms - sample.idle_ms,
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
                if sample.idle_ms < FIVE_MINUTES_IN_MILLIS {
                    (
                        None,
                        SessionState::Open {
                            start_mono_ms: sample.mono_ms,
                            app: sample.app.clone(),
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
enum SessionState {
    #[default]
    Empty,
    Open {
        app: String,
        title: Option<String>,
        start_mono_ms: u64,
    },
    Idle,
}

#[derive(Debug)]
struct FocusSample {
    mono_ms: u64,          // relogio monotônico, contador que só cresce
    app: String,           // nome do app
    title: Option<String>, // título da aba
    idle_ms: u64,          // tempo ocioso
}

#[derive(Debug, PartialEq)]
enum EndReason {
    Idle,
    FocusChange,
}

#[derive(Debug, PartialEq)]
struct ActivityInterval {
    app: String,
    title: Option<String>,
    start_mono_ms: u64,
    end_mono_ms: u64,
    end_reason: EndReason,
}

#[cfg(test)]
mod tests {
    use std::assert_eq;

    use super::*;

    #[test]
    fn fecha_intervalo_quando_o_foco_muda() {
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

        // Enquanto app e título não mudam, nada fecha.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // A troca de foco fecha o intervalo anterior.
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
    fn fecha_intervalo_ocioso() {
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
        // Faz 60s/1min que está ausente
        let sample_3 = FocusSample {
            mono_ms: 120000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: 60000,
        };
        // Faz 180s/5min que está ausente = para de contar o trabalho.
        let sample_4 = FocusSample {
            mono_ms: 360000,
            app: "Word".to_string(),
            title: Some("Petição A".to_string()),
            idle_ms: FIVE_MINUTES_IN_MILLIS,
        };

        // Enquanto app está ativo, nada fecha.
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
    fn estado_atualizado_apos_fechar() {
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

        // Enquanto app e título não mudam, nada fecha.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // A troca de foco fecha o intervalo anterior.
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
    fn ocioso_e_mudanca_de_foco_simultaneamente() {
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
        // Voltou do almoço e a primeira coisa que faz é trocar de tela.
        let sample_3 = FocusSample {
            mono_ms: 306000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: FIVE_MINUTES_IN_MILLIS,
        };

        // Enquanto app e título não mudam, nada fecha.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // A troca de foco fecha o intervalo anterior.
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
    fn nao_abre_intervalo_enquanto_continua_ocioso() {
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
        // Voltou do almoço e a primeira coisa que faz é trocar de tela.
        let sample_3 = FocusSample {
            mono_ms: 306000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: FIVE_MINUTES_IN_MILLIS,
        };
        let sample_4 = FocusSample {
            mono_ms: 307000,
            app: "Chrome".to_string(),
            title: Some("PJe - 0801234-56.2024.8.26.0100".to_string()),
            idle_ms: FIVE_MINUTES_IN_MILLIS + 1000,
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

        // Enquanto app e título não mudam, nada fecha.
        assert_eq!(sessionizer.observe(sample_1), None);
        assert_eq!(sessionizer.observe(sample_2), None);

        // A troca de foco fecha o intervalo anterior.
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
}
