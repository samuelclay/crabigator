#[allow(dead_code)]
pub enum AppEvent {
    PtyOutput(Vec<u8>),
    GitRefresh,
    Quit,
}
