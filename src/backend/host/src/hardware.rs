use pinmeter_core::hardware::{HardwareInventory, HardwareState};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HardwareItemDto {
    pub name: String,
    pub details: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HardwareSectionDto {
    pub id: String,
    pub items: Vec<HardwareItemDto>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, TS)]
pub struct HardwareInfoDto {
    pub status: String,
    pub detail: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub system: Option<String>,
    pub system_detail: Option<String>,
    pub boot_at_ms: Option<f64>,
    pub sections: Vec<HardwareSectionDto>,
}
pub fn snapshot(state: &HardwareState) -> HardwareInfoDto {
    let empty = HardwareInventory::default();
    let (status, detail, inventory) = match state {
        HardwareState::Loading => ("loading", "正在读取硬件信息…", &empty),
        HardwareState::Ready(inventory) => ("ready", "本次启动时读取", inventory),
        HardwareState::Failed(error) => ("failed", error.as_str(), &empty),
        HardwareState::Unsupported => ("unsupported", "此平台暂未接入硬件清单", &empty),
    };
    HardwareInfoDto {
        status: status.into(),
        detail: detail.into(),
        manufacturer: inventory.manufacturer.clone(),
        model: inventory.model.clone(),
        system: inventory.system.clone(),
        system_detail: inventory.system_detail.clone(),
        boot_at_ms: inventory.boot_at_ms,
        sections: inventory
            .sections
            .iter()
            .map(|section| HardwareSectionDto {
                id: section.id.clone(),
                error: section.error.clone(),
                items: section
                    .items
                    .iter()
                    .map(|item| HardwareItemDto {
                        name: item.name.clone(),
                        details: item.details.clone(),
                    })
                    .collect(),
            })
            .collect(),
    }
}
