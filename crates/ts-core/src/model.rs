#[derive(Debug)]
pub struct FocusSample {
    pub mono_ms: u64,          // relogio monotônico, contador que só cresce
    pub app: String,           // nome do app
    pub title: Option<String>, // título da aba
    pub idle_ms: u64,          // tempo ocioso
}

#[derive(Debug, PartialEq)]
pub enum EndReason {
    Idle,
    FocusChange,
}

#[derive(Debug, PartialEq)]
pub struct ActivityInterval {
    pub app: String,
    pub title: Option<String>,
    pub start_mono_ms: u64,
    pub end_mono_ms: u64,
    pub end_reason: EndReason,
}
