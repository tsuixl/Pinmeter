use tauri::{PhysicalPosition, PhysicalSize, WebviewWindow};

pub fn apply_theme(window: &WebviewWindow, preference: &str) -> tauri::Result<()> {
    let theme = match preference {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    };
    window.set_theme(theme)
}

pub fn activate(window: &WebviewWindow) -> tauri::Result<()> {
    window.unminimize()?;
    window.show()?;
    ensure_visible(window, false)?;
    window.set_focus()
}

/// Framework-specific resource hint; sampling and confirmed state stay in Rust.
pub fn set_webview_background(window: &WebviewWindow, background: bool) {
    #[cfg(windows)]
    {
        use webview2_com::Microsoft::Web::WebView2::Win32::{
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
            COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL, ICoreWebView2_19,
        };
        use windows_core::Interface;
        let current_window = window.clone();
        if let Err(error) = window.with_webview(move |webview| {
            // This closure runs on the WebView UI thread. Recheck after dispatch.
            let inactive = current_window.is_minimized().unwrap_or(background)
                || !current_window.is_visible().unwrap_or(!background);
            let result = (|| -> windows_core::Result<()> {
                // SAFETY: controller access and COM calls remain on its owning UI thread.
                let core: ICoreWebView2_19 =
                    unsafe { webview.controller().CoreWebView2()? }.cast()?;
                unsafe {
                    core.SetMemoryUsageTargetLevel(if inactive {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW
                    } else {
                        COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL
                    })
                }
            })();
            if let Err(error) = result {
                eprintln!("WebView memory hint unavailable: {error}");
            }
        }) {
            eprintln!("WebView memory hint dispatch failed: {error}");
        }
    }
    #[cfg(not(windows))]
    let _ = (window, background);
}

/// Recover only when the title area is inaccessible, or on an explicit user action.
pub fn ensure_visible(window: &WebviewWindow, force: bool) -> tauri::Result<()> {
    if window.is_minimized()? || window.is_maximized()? {
        return Ok(());
    }
    let monitors = window.available_monitors()?;
    let position = window.outer_position()?;
    let size = window.outer_size()?;
    let reachable = monitors.iter().any(|monitor| {
        let area = monitor.work_area();
        title_reachable(
            (position.x, position.y, size.width),
            (
                area.position.x,
                area.position.y,
                area.size.width,
                area.size.height,
            ),
        )
    });
    if reachable && !force {
        return Ok(());
    }
    let monitor = window
        .primary_monitor()?
        .or_else(|| monitors.first().cloned());
    if let Some(monitor) = monitor {
        let area = monitor.work_area();
        let inner = window.inner_size()?;
        let border_x = size.width.saturating_sub(inner.width);
        let border_y = size.height.saturating_sub(inner.height);
        if size.width > area.size.width || size.height > area.size.height {
            window.set_size(PhysicalSize::new(
                inner.width.min(area.size.width.saturating_sub(border_x)),
                inner.height.min(area.size.height.saturating_sub(border_y)),
            ))?;
        }
        let width = size.width.min(area.size.width);
        let height = size.height.min(area.size.height);
        window.set_position(PhysicalPosition::new(
            area.position.x + ((area.size.width - width) / 2) as i32,
            area.position.y + ((area.size.height - height) / 2) as i32,
        ))?;
    }
    Ok(())
}

fn title_reachable(window: (i32, i32, u32), area: (i32, i32, u32, u32)) -> bool {
    let (x, y, width) = (window.0 as i64, window.1 as i64, window.2 as i64);
    let (left, top, right, bottom) = (
        area.0 as i64,
        area.1 as i64,
        area.0 as i64 + area.2 as i64,
        area.1 as i64 + area.3 as i64,
    );
    y >= top && y + 32 <= bottom && (x + width).min(right) - x.max(left) >= 100
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disconnected_and_negative_coordinate_monitors() {
        assert!(!title_reachable((4000, 50, 880), (0, 0, 1920, 1080)));
        assert!(!title_reachable((50, -50, 880), (0, 0, 1920, 1080)));
        assert!(title_reachable((-1500, 50, 880), (-1920, 0, 1920, 1080)));
    }
}
