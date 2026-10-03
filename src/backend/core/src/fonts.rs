//! Font metadata and preferences; no operating-system or rendering types.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontStyle {
    pub id: String,
    pub name: String,
    pub weight: u16,
    pub slant: u8,
    pub stretch: u8,
    pub local_names: Vec<String>,
}
impl FontStyle {
    pub fn new(name: impl Into<String>, weight: u16, slant: u8, stretch: u8) -> Self {
        Self {
            id: format!("{weight}:{slant}:{stretch}"),
            name: name.into(),
            weight,
            slant,
            stretch,
            local_names: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FontFamily {
    pub id: String,
    pub name: String,
    pub canonical_name: String,
    pub source: String,
    pub styles: Vec<FontStyle>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FontCatalog {
    pub families: Vec<FontFamily>,
    pub system_available: bool,
    pub detail: String,
}

pub fn default_font_style() -> String {
    "auto".into()
}

pub fn style_axes(style: &str) -> Option<(u16, u8, u8)> {
    if style == "auto" {
        return Some((400, 0, 5));
    }
    let parts: Vec<_> = style.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let weight: u16 = parts[0].parse().ok()?;
    let slant: u8 = parts[1].parse().ok()?;
    let stretch: u8 = parts[2].parse().ok()?;
    ((1..=1000).contains(&weight)
        && slant <= 2
        && (1..=9).contains(&stretch)
        && style == format!("{weight}:{slant}:{stretch}"))
    .then_some((weight, slant, stretch))
}

pub fn valid_family(id: &str) -> bool {
    matches!(id, "harmonyos_sans_sc" | "geist" | "system")
        || id.strip_prefix("installed:").is_some_and(|name| {
            !name.trim().is_empty()
                && name == name.trim()
                && name.chars().count() <= 256
                && !name.chars().any(char::is_control)
        })
}

pub fn validate_choice(catalog: &FontCatalog, family: &str, style: &str) -> Result<(), String> {
    let font = catalog
        .families
        .iter()
        .find(|f| f.id == family)
        .ok_or("所选字体已不可用，请刷新字体列表")?;
    if style != "auto" && !font.styles.iter().any(|s| s.id == style) {
        return Err("该字体没有所选样式，请重新选择".into());
    }
    Ok(())
}

pub fn builtin_catalog() -> FontCatalog {
    let style = |name, weight| FontStyle::new(name, weight, 0, 5);
    FontCatalog {
        families: vec![
            FontFamily {
                id: "harmonyos_sans_sc".into(),
                name: "HarmonyOS Sans SC（内置默认）".into(),
                canonical_name: "HarmonyOS Sans SC".into(),
                source: "builtin".into(),
                styles: vec![
                    style("Regular", 400),
                    style("Medium", 500),
                    style("Bold", 700),
                ],
            },
            FontFamily {
                id: "geist".into(),
                name: "Geist（内置）".into(),
                canonical_name: "Geist".into(),
                source: "builtin".into(),
                styles: [
                    "Thin",
                    "ExtraLight",
                    "Light",
                    "Regular",
                    "Medium",
                    "SemiBold",
                    "Bold",
                    "ExtraBold",
                    "Black",
                ]
                .into_iter()
                .enumerate()
                .map(|(i, name)| style(name, (i as u16 + 1) * 100))
                .collect(),
            },
            FontFamily {
                id: "system".into(),
                name: "系统默认".into(),
                canonical_name: String::new(),
                source: "system".into(),
                styles: vec![],
            },
        ],
        system_available: false,
        detail: String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installed_names_and_real_styles_are_distinct_from_arbitrary_css() {
        assert!(valid_family("installed:微软雅黑"));
        for invalid in ["installed:", "installed: bad", "installed:a\nbody", "other"] {
            assert!(!valid_family(invalid));
        }
        for invalid in ["500", "0:0:5", "400:3:5", "400:0:10", "0400:0:5"] {
            assert!(style_axes(invalid).is_none());
        }
        assert_eq!(style_axes("500:2:3"), Some((500, 2, 3)));
        let catalog = builtin_catalog();
        assert!(validate_choice(&catalog, "harmonyos_sans_sc", "500:0:5").is_ok());
        assert!(validate_choice(&catalog, "harmonyos_sans_sc", "500:2:5").is_err());
        assert!(validate_choice(&catalog, "installed:Missing", "auto").is_err());
    }
}
