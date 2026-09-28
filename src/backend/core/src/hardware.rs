use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HardwareItem {
    pub name: String,
    pub details: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HardwareSection {
    pub id: String,
    pub items: Vec<HardwareItem>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HardwareInventory {
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub system: Option<String>,
    pub system_detail: Option<String>,
    pub boot_at_ms: Option<f64>,
    pub sections: Vec<HardwareSection>,
}

/// One immutable inventory per application session. Slow work is performed outside this state.
#[derive(Clone, Debug)]
pub enum HardwareState {
    Loading,
    Ready(HardwareInventory),
    Failed(String),
    Unsupported,
}

impl HardwareInventory {
    pub fn normalize(&mut self) {
        if self.boot_at_ms.is_some_and(|v| !v.is_finite() || v <= 0.0) {
            self.boot_at_ms = None;
        }
        for section in &mut self.sections {
            section.items.retain(|item| !item.name.trim().is_empty());
            for item in &mut section.items {
                item.name = item.name.split_whitespace().collect::<Vec<_>>().join(" ");
                item.details.retain(|detail| !detail.trim().is_empty());
            }
        }
        if self.model.as_deref().is_some_and(|model| {
            [
                "system product name",
                "to be filled by o.e.m.",
                "default string",
            ]
            .contains(&model.trim().to_ascii_lowercase().as_str())
        }) {
            self.model = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn placeholder_and_unknown_values_are_not_real_hardware() {
        let mut inventory = HardwareInventory {
            model: Some("System Product Name".into()),
            boot_at_ms: Some(f64::NAN),
            sections: vec![HardwareSection {
                id: "cpu".into(),
                error: None,
                items: vec![
                    HardwareItem {
                        name: "  Ryzen   9  ".into(),
                        details: vec!["".into()],
                    },
                    HardwareItem::default(),
                ],
            }],
            ..Default::default()
        };
        inventory.normalize();
        assert!(inventory.model.is_none());
        assert!(inventory.boot_at_ms.is_none());
        assert_eq!(inventory.sections[0].items.len(), 1);
        assert_eq!(inventory.sections[0].items[0].name, "Ryzen 9");
    }
}
