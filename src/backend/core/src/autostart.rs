/// System registration is separate from the saved user preference.
#[derive(Clone, Debug, Default)]
pub struct AutostartStatus {
    pub available: bool,
    pub enabled: Option<bool>,
    pub detail: String,
}

pub trait Autostart: Send + Sync {
    fn status(&self) -> AutostartStatus;
    /// Opaque platform registration, used only to undo an unsuccessful save.
    fn checkpoint(&self) -> Result<String, String>;
    fn restore(&self, checkpoint: &str) -> Result<(), String>;
    fn set_enabled(&self, enabled: bool) -> Result<(), String>;
}
