use pinmeter_core::fonts::{FontCatalog, builtin_catalog};

pub fn catalog() -> FontCatalog {
    let mut catalog = builtin_catalog();
    #[cfg(target_os = "windows")]
    match windows_fonts::enumerate() {
        Ok((mut families, default_name)) => {
            if let Some(default) = families.iter().find(|f| {
                f.canonical_name.eq_ignore_ascii_case(&default_name) || f.name == default_name
            }) {
                catalog.families[2].styles = default.styles.clone();
                catalog.families[2].canonical_name = default.canonical_name.clone();
            }
            families.sort_by_cached_key(|f| f.name.to_lowercase());
            catalog.detail = format!("已读取 {} 个 Windows 字体家族", families.len());
            catalog.families.extend(families);
            catalog.system_available = true;
        }
        Err(error) => catalog.detail = format!("系统字体读取失败，仍可使用内置字体：{error}"),
    }
    #[cfg(not(target_os = "windows"))]
    {
        catalog.detail = "当前平台暂不支持列出系统字体，可使用内置字体和系统默认".into();
    }
    catalog
}

#[cfg(target_os = "windows")]
mod windows_fonts {
    use pinmeter_core::fonts::{FontFamily, FontStyle};
    use windows::{
        Win32::{Graphics::DirectWrite::*, UI::WindowsAndMessaging::*},
        core::{BOOL, PCWSTR, Result, w},
    };

    pub fn system_family_name() -> Result<String> {
        let mut metrics = NONCLIENTMETRICSW {
            cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        // SAFETY: the correctly sized structure remains alive throughout the call.
        unsafe {
            SystemParametersInfoW(
                SPI_GETNONCLIENTMETRICS,
                metrics.cbSize,
                Some((&mut metrics as *mut NONCLIENTMETRICSW).cast()),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )?;
        }
        let name = &metrics.lfMessageFont.lfFaceName;
        Ok(String::from_utf16_lossy(
            &name[..name.iter().position(|c| *c == 0).unwrap_or(name.len())],
        ))
    }

    fn localized(strings: &IDWriteLocalizedStrings, locale: &str) -> Result<String> {
        // SAFETY: DirectWrite owns the strings and fills the allocated UTF-16 buffer.
        unsafe {
            let mut index = 0;
            let mut exists = BOOL::default();
            let locale: Vec<_> = locale.encode_utf16().chain(Some(0)).collect();
            strings.FindLocaleName(PCWSTR(locale.as_ptr()), &mut index, &mut exists)?;
            if !exists.as_bool() {
                strings.FindLocaleName(w!("en-us"), &mut index, &mut exists)?;
            }
            if !exists.as_bool() {
                index = 0;
            }
            let len = strings.GetStringLength(index)?;
            let mut text = vec![0; len as usize + 1];
            strings.GetString(index, &mut text)?;
            Ok(String::from_utf16_lossy(&text[..len as usize]))
        }
    }

    pub fn enumerate() -> Result<(Vec<FontFamily>, String)> {
        // SAFETY: all COM objects are local to this call; only plain metadata leaves it.
        unsafe {
            let factory: IDWriteFactory = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let mut collection = None;
            factory.GetSystemFontCollection(&mut collection, true)?;
            let collection = collection.expect("successful DirectWrite collection");
            let mut families = vec![];
            for i in 0..collection.GetFontFamilyCount() {
                let family = collection.GetFontFamily(i)?;
                let names = family.GetFamilyNames()?;
                let canonical = localized(&names, "en-us")?;
                let name = localized(&names, "zh-cn")?;
                let id = format!("installed:{canonical}");
                if !pinmeter_core::fonts::valid_family(&id) {
                    continue;
                }
                let mut styles = vec![];
                for j in 0..family.GetFontCount() {
                    let font = family.GetFont(j)?;
                    if font.GetSimulations() != DWRITE_FONT_SIMULATIONS_NONE {
                        continue;
                    }
                    let mut face = FontStyle::new(
                        localized(&font.GetFaceNames()?, "en-us")?,
                        font.GetWeight().0 as u16,
                        font.GetStyle().0 as u8,
                        font.GetStretch().0 as u8,
                    );
                    if styles.iter().any(|s: &FontStyle| s.id == face.id) {
                        continue;
                    }
                    for kind in [
                        DWRITE_INFORMATIONAL_STRING_POSTSCRIPT_NAME,
                        DWRITE_INFORMATIONAL_STRING_FULL_NAME,
                    ] {
                        let mut info = None;
                        let mut exists = BOOL::default();
                        font.GetInformationalStrings(kind, &mut info, &mut exists)?;
                        if let Some(info) = info.filter(|_| exists.as_bool()) {
                            for locale in ["en-us", "zh-cn"] {
                                let local = localized(&info, locale)?;
                                if !local.is_empty() && !face.local_names.contains(&local) {
                                    face.local_names.push(local);
                                }
                            }
                        }
                    }
                    if face.local_names.is_empty() {
                        face.local_names.push(format!("{canonical} {}", face.name));
                    }
                    styles.push(face);
                }
                styles.sort_by_key(|s| (s.stretch, s.slant, s.weight));
                if !styles.is_empty() {
                    families.push(FontFamily {
                        id,
                        name,
                        canonical_name: canonical,
                        source: "installed".into(),
                        styles,
                    });
                }
            }
            Ok((families, system_family_name()?))
        }
    }
}

#[cfg(target_os = "windows")]
pub(crate) use windows_fonts::system_family_name;

#[cfg(all(test, target_os = "windows"))]
mod tests {
    #[test]
    fn directwrite_returns_installed_families_and_non_synthetic_styles() {
        let catalog = super::catalog();
        assert!(catalog.system_available, "{}", catalog.detail);
        assert!(catalog.families.iter().any(|f| f.source == "installed"));
        for family in &catalog.families {
            let mut ids = std::collections::HashSet::new();
            for style in &family.styles {
                assert!(ids.insert(&style.id));
                assert_eq!(
                    pinmeter_core::fonts::style_axes(&style.id),
                    Some((style.weight, style.slant, style.stretch))
                );
                if family.source == "installed" {
                    assert!(!style.local_names.is_empty());
                }
            }
        }
    }
}
