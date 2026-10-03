use crate::runtime::Runtime;
use pinmeter_core::fonts::FontCatalog;
use serde::Serialize;
use std::sync::Arc;
use tauri::{State, WebviewWindow};
use ts_rs::TS;

#[derive(Clone, Serialize, TS)]
pub struct FontStyleDto {
    pub id: String,
    pub name: String,
    pub weight: u16,
    pub slant: u8,
    pub stretch: u8,
    pub local_names: Vec<String>,
}
#[derive(Clone, Serialize, TS)]
pub struct FontFamilyDto {
    pub id: String,
    pub name: String,
    pub canonical_name: String,
    pub source: String,
    pub styles: Vec<FontStyleDto>,
}
#[derive(Clone, Serialize, TS)]
pub struct FontCatalogDto {
    pub families: Vec<FontFamilyDto>,
    pub system_available: bool,
    pub detail: String,
}
impl From<FontCatalog> for FontCatalogDto {
    fn from(value: FontCatalog) -> Self {
        Self {
            system_available: value.system_available,
            detail: value.detail,
            families: value
                .families
                .into_iter()
                .map(|f| FontFamilyDto {
                    id: f.id,
                    name: f.name,
                    canonical_name: f.canonical_name,
                    source: f.source,
                    styles: f
                        .styles
                        .into_iter()
                        .map(|s| FontStyleDto {
                            id: s.id,
                            name: s.name,
                            weight: s.weight,
                            slant: s.slant,
                            stretch: s.stretch,
                            local_names: s.local_names,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

pub fn catalog(runtime: &Runtime, refresh: bool) -> Result<FontCatalog, String> {
    let mut cached = runtime.font_catalog.lock().map_err(|e| e.to_string())?;
    if refresh || cached.is_none() {
        *cached = Some(pinmeter_platform::fonts::catalog());
    }
    Ok(cached.as_ref().unwrap().clone())
}

#[tauri::command]
pub async fn get_font_catalog(
    window: WebviewWindow,
    runtime: State<'_, Arc<Runtime>>,
    refresh: bool,
) -> Result<FontCatalogDto, String> {
    crate::commands::authorize(&window)?;
    let runtime = runtime.inner().clone();
    tauri::async_runtime::spawn_blocking(move || catalog(&runtime, refresh).map(Into::into))
        .await
        .map_err(|e| e.to_string())?
}

pub fn validate_change(
    runtime: &Runtime,
    next: &pinmeter_core::domain::Settings,
) -> Result<(), String> {
    next.validate()?;
    let changed = {
        let state = runtime.inner.lock().map_err(|e| e.to_string())?;
        let previous = &state.monitor.settings;
        previous.font_family != next.font_family || previous.font_style != next.font_style
    };
    // Enumeration is outside the monitoring lock. Unrelated saves retain an
    // unavailable preference so uninstalling a font cannot block other settings.
    if changed {
        pinmeter_core::fonts::validate_choice(
            &catalog(runtime, true)?,
            &next.font_family,
            &next.font_style,
        )?;
    }
    Ok(())
}
