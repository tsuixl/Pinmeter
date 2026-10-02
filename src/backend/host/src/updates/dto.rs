use pinmeter_core::updates::{ReleaseNotes, UpdateStage};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ReleaseSectionDto {
    pub title: String,
    pub items: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct ReleaseNotesDto {
    pub version: String,
    pub date: String,
    pub summary: String,
    pub sections: Vec<ReleaseSectionDto>,
}
impl From<&ReleaseNotes> for ReleaseNotesDto {
    fn from(r: &ReleaseNotes) -> Self {
        Self {
            version: r.version.clone(),
            date: r.date.clone(),
            summary: r.summary.clone(),
            sections: r
                .sections
                .iter()
                .map(|s| ReleaseSectionDto {
                    title: s.title.clone(),
                    items: s.items.clone(),
                })
                .collect(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct UpdateSnapshotDto {
    pub revision: String,
    pub current_version: String,
    pub installation: String,
    pub can_install: bool,
    pub stage: String,
    pub detail: String,
    pub progress: Option<u8>,
    pub target: Option<ReleaseNotesDto>,
    pub target_history: Vec<ReleaseNotesDto>,
    pub history: Vec<ReleaseNotesDto>,
    pub unread: Vec<ReleaseNotesDto>,
    pub automatic_check: bool,
    pub last_check_ms: Option<f64>,
    pub dismissed_version: Option<String>,
    pub manual_request: String,
    pub confirmation_required: bool,
    pub release_url: String,
}
pub fn stage_name(stage: UpdateStage) -> &'static str {
    match stage {
        UpdateStage::Idle => "idle",
        UpdateStage::Checking => "checking",
        UpdateStage::Available => "available",
        UpdateStage::Downloading => "downloading",
        UpdateStage::Ready => "ready",
        UpdateStage::Preparing => "preparing",
        UpdateStage::Failed => "failed",
    }
}
