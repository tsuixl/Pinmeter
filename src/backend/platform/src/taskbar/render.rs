use pinmeter_core::desktop::DesktopSummary;
use windows::{
    Win32::{
        Foundation::*,
        Graphics::{
            Direct2D::{Common::*, *},
            DirectWrite::*,
            Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM,
            Gdi::*,
        },
        UI::WindowsAndMessaging::{
            NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
            SystemParametersInfoW, ULW_ALPHA, UpdateLayeredWindow,
        },
    },
    core::{Interface, PCWSTR, Result, w},
};
use windows_numerics::Vector2;

const FONT: &[u8] = include_bytes!("../../assets/taskbar/Geist.ttf");
const HARMONY_FONT: &[u8] =
    include_bytes!("../../../../shared/fonts/harmonyos-sans-sc/HarmonyOS_Sans_SC_Regular.ttf");

pub struct Painter {
    target: ID2D1DCRenderTarget,
    hwnd: HWND,
    surface: Surface,
    write: IDWriteFactory5,
    _font_loader: Option<FontLoader>,
    font_family: String,
    format: IDWriteTextFormat,
    typography: IDWriteTypography,
    tokens: serde_json::Value,
}
impl Painter {
    pub fn new(hwnd: HWND, width: u32, height: u32, dpi: f32, font_family: &str) -> Result<Self> {
        // SAFETY: all COM graphics objects remain on the owning native UI thread.
        unsafe {
            let factory: ID2D1Factory = D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;
            let write: IDWriteFactory5 = DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED)?;
            let (family, bytes) = match font_family {
                "harmonyos_sans_sc" => ("HarmonyOS Sans SC".to_owned(), Some(HARMONY_FONT)),
                "geist" => ("Geist".to_owned(), Some(FONT)),
                "system" => {
                    let mut metrics = NONCLIENTMETRICSW {
                        cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
                        ..Default::default()
                    };
                    SystemParametersInfoW(
                        SPI_GETNONCLIENTMETRICS,
                        metrics.cbSize,
                        Some((&mut metrics as *mut NONCLIENTMETRICSW).cast()),
                        SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
                    )?;
                    let name = &metrics.lfMessageFont.lfFaceName;
                    let end = name.iter().position(|c| *c == 0).unwrap_or(name.len());
                    (String::from_utf16_lossy(&name[..end]), None)
                }
                _ => return Err(E_INVALIDARG.into()),
            };
            let (collection, font_loader) = if let Some(bytes) = bytes {
                let loader = write.CreateInMemoryFontFileLoader()?;
                write.RegisterFontFileLoader(&loader)?;
                let font_loader = FontLoader {
                    write: write.clone(),
                    loader: loader.clone(),
                };
                let file = loader.CreateInMemoryFontFileReference(
                    &write,
                    bytes.as_ptr().cast(),
                    bytes.len() as u32,
                    None,
                )?;
                let builder = write.CreateFontSetBuilder()?;
                builder.AddFontFile(&file)?;
                let set = builder.CreateFontSet()?;
                let collection: IDWriteFontCollection =
                    write.CreateFontCollectionFromFontSet(&set)?.cast()?;
                (Some(collection), Some(font_loader))
            } else {
                (None, None)
            };
            let tokens: serde_json::Value = serde_json::from_str(include_str!(
                "../../../../shared/design-tokens/taskbar.json"
            ))
            .expect("checked Sakani tokens");
            let format = write.CreateTextFormat(
                PCWSTR(
                    family
                        .encode_utf16()
                        .chain(Some(0))
                        .collect::<Vec<_>>()
                        .as_ptr(),
                ),
                collection.as_ref(),
                DWRITE_FONT_WEIGHT_NORMAL,
                DWRITE_FONT_STYLE_NORMAL,
                DWRITE_FONT_STRETCH_NORMAL,
                tokens["fontSize"].as_f64().unwrap() as f32,
                w!("zh-CN"),
            )?;
            format.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
            format.SetParagraphAlignment(DWRITE_PARAGRAPH_ALIGNMENT_CENTER)?;
            let typography = write.CreateTypography()?;
            typography.AddFontFeature(DWRITE_FONT_FEATURE {
                nameTag: DWRITE_FONT_FEATURE_TAG_TABULAR_FIGURES,
                parameter: 1,
            })?;
            let target = factory.CreateDCRenderTarget(&D2D1_RENDER_TARGET_PROPERTIES {
                dpiX: dpi,
                dpiY: dpi,
                pixelFormat: D2D1_PIXEL_FORMAT {
                    format: DXGI_FORMAT_B8G8R8A8_UNORM,
                    alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                },
                ..Default::default()
            })?;
            let surface = Surface::new(width, height)?;
            // Transparent glyph edges must not depend on an opaque ClearType backdrop.
            target.SetTextAntialiasMode(D2D1_TEXT_ANTIALIAS_MODE_GRAYSCALE);
            target.BindDC(
                surface.dc,
                &RECT {
                    left: 0,
                    top: 0,
                    right: width as i32,
                    bottom: height as i32,
                },
            )?;
            Ok(Self {
                target,
                hwnd,
                surface,
                write,
                _font_loader: font_loader,
                font_family: font_family.into(),
                format,
                typography,
                tokens,
            })
        }
    }
    pub fn uses_font(&self, font_family: &str) -> bool {
        self.font_family == font_family
    }
    pub fn resize(&mut self, width: u32, height: u32, dpi: f32) -> Result<()> {
        unsafe {
            self.target.SetDpi(dpi, dpi);
            if self.surface.width != width || self.surface.height != height {
                let surface = Surface::new(width, height)?;
                self.target.BindDC(
                    surface.dc,
                    &RECT {
                        left: 0,
                        top: 0,
                        right: width as i32,
                        bottom: height as i32,
                    },
                )?;
                self.surface = surface;
            }
            Ok(())
        }
    }
    fn color(&self, dark: bool, key: &str) -> D2D1_COLOR_F {
        let hex = self.tokens[if dark { "dark" } else { "light" }][key]
            .as_str()
            .unwrap()
            .trim_start_matches('#');
        let n = u32::from_str_radix(hex, 16).unwrap();
        D2D1_COLOR_F {
            r: ((n >> 16) & 255) as f32 / 255.,
            g: ((n >> 8) & 255) as f32 / 255.,
            b: (n & 255) as f32 / 255.,
            a: 1.,
        }
    }
    pub fn measure(&self, text: &str) -> Result<f32> {
        unsafe {
            let layout = self.write.CreateTextLayout(
                &text.encode_utf16().collect::<Vec<_>>(),
                &self.format,
                2000.,
                40.,
            )?;
            layout.SetTypography(
                &self.typography,
                DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: text.encode_utf16().count() as u32,
                },
            )?;
            let mut metrics = DWRITE_TEXT_METRICS::default();
            layout.GetMetrics(&mut metrics)?;
            Ok(metrics.widthIncludingTrailingWhitespace)
        }
    }
    fn text(&self, text: &str, bounds: [f32; 4], color: D2D1_COLOR_F, right: bool) -> Result<()> {
        let [x, y, width, height] = bounds;
        unsafe {
            let text: Vec<_> = text.encode_utf16().collect();
            let layout = self
                .write
                .CreateTextLayout(&text, &self.format, width, height)?;
            layout.SetTypography(
                &self.typography,
                DWRITE_TEXT_RANGE {
                    startPosition: 0,
                    length: text.len() as u32,
                },
            )?;
            layout.SetTextAlignment(if right {
                DWRITE_TEXT_ALIGNMENT_TRAILING
            } else {
                DWRITE_TEXT_ALIGNMENT_LEADING
            })?;
            let brush = self.target.CreateSolidColorBrush(&color, None)?;
            self.target.DrawTextLayout(
                Vector2 { X: x, Y: y },
                &layout,
                &brush,
                D2D1_DRAW_TEXT_OPTIONS_CLIP,
            );
            Ok(())
        }
    }
    pub fn draw(
        &self,
        summary: &DesktopSummary,
        cells: &[Cell],
        dark: bool,
        hover: Option<usize>,
        focused: bool,
        tooltip: bool,
    ) -> Result<()> {
        self.paint(summary, cells, dark, hover, focused, tooltip)?;
        unsafe {
            UpdateLayeredWindow(
                self.hwnd,
                None,
                None,
                Some(&SIZE {
                    cx: self.surface.width as i32,
                    cy: self.surface.height as i32,
                }),
                Some(self.surface.dc),
                Some(&POINT { x: 0, y: 0 }),
                COLORREF(0),
                Some(&BLENDFUNCTION {
                    BlendOp: AC_SRC_OVER as u8,
                    BlendFlags: 0,
                    SourceConstantAlpha: 255,
                    AlphaFormat: AC_SRC_ALPHA as u8,
                }),
                ULW_ALPHA,
            )
        }
    }
    fn paint(
        &self,
        summary: &DesktopSummary,
        cells: &[Cell],
        dark: bool,
        hover: Option<usize>,
        focused: bool,
        tooltip: bool,
    ) -> Result<()> {
        unsafe {
            self.target.BeginDraw();
            // Only the separate tooltip owns a surface; the taskbar supplies our backdrop.
            let background = if tooltip {
                self.color(dark, "bg-inverse")
            } else {
                D2D1_COLOR_F::default()
            };
            self.target.Clear(Some(&background));
        }
        let result = (|| {
            if tooltip {
                let fg = self.color(dark, "fg-on-inverse");
                for (i, line) in tooltip_lines(summary).iter().enumerate() {
                    self.text(line, [12., 6. + i as f32 * 24., 536., 24.], fg, false)?;
                }
            } else {
                for (i, cell) in cells.iter().enumerate() {
                    let r = &summary.readings[cell.index];
                    let fg = self.color(dark, "fg-default");
                    let muted = self.color(
                        dark,
                        if hover == Some(i) {
                            "fg-default"
                        } else {
                            "fg-muted"
                        },
                    );
                    // Lucide arrow-down / arrow-up: same 24x24 line geometry at 14px.
                    if cell.index < 2 {
                        let brush = unsafe { self.target.CreateSolidColorBrush(&muted, None)? };
                        let k = 14. / 24.;
                        let px = cell.x;
                        let py = cell.y + 3.;
                        let points = if cell.index == 0 {
                            [
                                (12., 5., 12., 19.),
                                (5., 12., 12., 19.),
                                (19., 12., 12., 19.),
                            ]
                        } else {
                            [(12., 19., 12., 5.), (5., 12., 12., 5.), (19., 12., 12., 5.)]
                        };
                        for (a, b, c, d) in points {
                            unsafe {
                                self.target.DrawLine(
                                    Vector2 {
                                        X: px + a * k,
                                        Y: py + b * k,
                                    },
                                    Vector2 {
                                        X: px + c * k,
                                        Y: py + d * k,
                                    },
                                    &brush,
                                    2. * k,
                                    None,
                                )
                            };
                        }
                    } else {
                        self.text(
                            r.label,
                            [cell.x, cell.y, cell.label_width, 20.],
                            muted,
                            false,
                        )?;
                    }
                    // A fixed right-aligned value slot keeps % and temperature aligned
                    // across CPU/GPU rows, including one-, two- and three-digit loads.
                    let value_width = cell.value_width;
                    if r.show_value {
                        self.text(
                            &r.text,
                            [cell.x + cell.label_width, cell.y, value_width, 20.],
                            fg,
                            true,
                        )?;
                        self.text(
                            &r.unit,
                            [
                                cell.x
                                    + cell.label_width
                                    + value_width
                                    + if cell.index < 2 { 4. } else { 0. },
                                cell.y,
                                cell.unit_width,
                                20.,
                            ],
                            muted,
                            false,
                        )?;
                    }
                    if let Some(temperature) = &r.temperature {
                        self.text(
                            &temperature.formatted(),
                            [
                                cell.x + cell.label_width + value_width + cell.unit_width + 4.,
                                cell.y,
                                cell.temperature_width,
                                20.,
                            ],
                            muted,
                            false,
                        )?;
                    }
                    if !r.normal {
                        self.text("!", [cell.x + cell.width - 8., cell.y, 8., 20.], fg, false)?;
                    }
                }
                if focused {
                    let brush = unsafe {
                        self.target
                            .CreateSolidColorBrush(&self.color(dark, "border-focus"), None)?
                    };
                    let size = unsafe { self.target.GetSize() };
                    unsafe {
                        self.target.DrawRectangle(
                            &D2D_RECT_F {
                                left: 1.,
                                top: 1.,
                                right: size.width - 1.,
                                bottom: size.height - 1.,
                            },
                            &brush,
                            1.,
                            None,
                        )
                    };
                }
            }
            Ok(())
        })();
        let end = unsafe { self.target.EndDraw(None, None) };
        result.and(end)
    }
}
struct Surface {
    #[cfg(test)]
    bits: *const u8,
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    width: u32,
    height: u32,
}
impl Surface {
    fn new(width: u32, height: u32) -> Result<Self> {
        unsafe {
            let dc = CreateCompatibleDC(None);
            if dc.is_invalid() {
                return Err(windows::core::Error::from_thread());
            }
            let info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width as i32,
                    biHeight: -(height as i32),
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut bits = std::ptr::null_mut();
            let bitmap = match CreateDIBSection(Some(dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
            {
                Ok(bitmap) => bitmap,
                Err(error) => {
                    let _ = DeleteDC(dc);
                    return Err(error);
                }
            };
            let previous = SelectObject(dc, bitmap.into());
            Ok(Self {
                #[cfg(test)]
                bits: bits.cast(),
                dc,
                bitmap,
                previous,
                width,
                height,
            })
        }
    }
}
impl Drop for Surface {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.previous);
            let _ = DeleteObject(self.bitmap.into());
            let _ = DeleteDC(self.dc);
        }
    }
}
struct FontLoader {
    write: IDWriteFactory5,
    loader: IDWriteInMemoryFontFileLoader,
}
impl Drop for FontLoader {
    fn drop(&mut self) {
        unsafe {
            let _ = self.write.UnregisterFontFileLoader(&self.loader);
        }
    }
}
#[derive(Clone)]
pub struct Cell {
    pub index: usize,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub label_width: f32,
    pub value_width: f32,
    pub unit_width: f32,
    pub temperature_width: f32,
}
pub fn tooltip_lines(summary: &DesktopSummary) -> Vec<String> {
    let mut lines = if summary.settings.network {
        vec![summary.network.clone()]
    } else {
        vec![]
    };
    for index in pinmeter_core::desktop::summary_groups(summary, false)
        .into_iter()
        .flatten()
    {
        let reading = &summary.readings[index];
        lines.push(reading.display());
        lines.push(reading.detail.clone());
        if let Some(temperature) = &reading.temperature {
            lines.push(temperature.detail.clone());
        }
    }
    lines
}
pub fn layout(
    painter: &Painter,
    summary: &DesktopSummary,
    double: bool,
    compact: bool,
) -> Result<(Vec<Cell>, f32, f32)> {
    let mut groups = pinmeter_core::desktop::summary_groups(summary, compact);
    if !double {
        groups = groups.into_iter().flatten().map(|i| vec![i]).collect();
    }
    let two_rows = double && groups.iter().any(|g| g.len() > 1);
    let value = painter.measure("888.8")?.ceil();
    let unit = painter.measure("GB/s")?.ceil();
    let percent = painter.measure("100")?.ceil();
    let percent_unit = painter.measure("%")?.ceil();
    let label = painter
        .measure("CPU")?
        .max(painter.measure("GPU")?)
        .max(painter.measure("内存")?)
        .ceil()
        + 4.;
    let temperature = painter
        .measure("(-50°C*)")?
        .max(painter.measure("(150°C*)")?)
        .ceil();
    let mut cells = vec![];
    let mut x = 8.;
    for group in groups {
        let any_value = group.iter().any(|i| summary.readings[*i].show_value);
        let mut group_width: f32 = 0.;
        for (row, index) in group.into_iter().enumerate() {
            let label_width = if index < 2 { 22. } else { label };
            let value_width = if !any_value {
                0.
            } else if index < 2 {
                value
            } else {
                percent
            };
            let unit_width = if !any_value {
                0.
            } else if index < 2 {
                unit
            } else {
                percent_unit
            };
            let temperature_width = if summary.readings[index].temperature.is_some() {
                temperature
            } else {
                0.
            };
            let width = label_width
                + value_width
                + unit_width
                + 12.
                + if index < 2 {
                    4.
                } else if temperature_width > 0. {
                    4. + temperature_width
                } else {
                    0.
                };
            group_width = group_width.max(width);
            cells.push(Cell {
                index,
                x,
                y: 2. + row as f32 * 20.,
                width,
                label_width,
                value_width,
                unit_width,
                temperature_width,
            });
        }
        x += group_width + 12.;
    }
    Ok((cells, x - 4., if two_rows { 44. } else { 24. }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pinmeter_core::desktop::{SummaryReading, SummaryTemperature, TaskbarSettings};

    fn font_scales() -> impl Iterator<Item = (&'static str, f32)> {
        ["harmonyos_sans_sc", "geist", "system"]
            .into_iter()
            .flat_map(|font| [96., 144., 192.].map(|dpi| (font, dpi)))
    }

    #[test]
    fn native_font_selection_loads_real_faces_and_rejects_unknown_values() {
        for font in ["harmonyos_sans_sc", "geist", "system"] {
            let painter = Painter::new(HWND::default(), 1, 1, 96., font).unwrap();
            assert!(painter.uses_font(font));
            let mut name =
                vec![0; unsafe { painter.format.GetFontFamilyNameLength() } as usize + 1];
            unsafe {
                painter.format.GetFontFamilyName(&mut name).unwrap();
            }
            let family = String::from_utf16_lossy(&name[..name.len() - 1]);
            match font {
                "harmonyos_sans_sc" => assert_eq!(family, "HarmonyOS Sans SC"),
                "geist" => assert_eq!(family, "Geist"),
                _ => assert!(!family.is_empty()),
            }
            assert!(painter.measure("内存 100% CPU 57°C").unwrap() > 0.);
            if font == "harmonyos_sans_sc" {
                let widths: Vec<_> = (0..10)
                    .map(|digit| painter.measure(&digit.to_string()).unwrap())
                    .collect();
                assert!(widths.iter().all(|width| (width - widths[0]).abs() < 0.01));
            }
        }
        assert!(Painter::new(HWND::default(), 1, 1, 96., "unknown").is_err());
    }

    fn pixels(painter: &Painter) -> Vec<[u8; 4]> {
        // EndDraw has completed; the owning DIB stays alive and is not being painted.
        unsafe {
            GdiFlush().unwrap();
            std::slice::from_raw_parts(
                painter.surface.bits.cast::<[u8; 4]>(),
                (painter.surface.width * painter.surface.height) as usize,
            )
            .to_vec()
        }
    }

    fn example() -> DesktopSummary {
        DesktopSummary {
            session: "render-test".into(),
            cursor: 0,
            revision: 0,
            network_id: None,
            gpu_id: None,
            settings: TaskbarSettings::default(),
            font_family: pinmeter_core::domain::default_font_family(),
            network: "Example".into(),
            readings: [
                ("network", "3.2"),
                ("network", "128.0"),
                ("cpu", "100"),
                ("memory", "46"),
                ("gpu", "100"),
            ]
            .into_iter()
            .map(|(key, text)| SummaryReading {
                show_value: true,
                valid_at_ms: None,
                key,
                label: key,
                text: text.into(),
                unit: "%".into(),
                detail: "Example data".into(),
                normal: true,
                temperature: matches!(key, "cpu" | "gpu").then(|| SummaryTemperature {
                    text: "150".into(),
                    normal: true,
                    detail: "Example temperature".into(),
                    valid_at_ms: None,
                    alternative_sensor: true,
                }),
            })
            .collect(),
        }
    }
    #[test]
    fn transparent_readings_keep_glyph_alpha_and_clear_previous_frames() {
        let summary = example();
        for (font, dpi) in font_scales() {
            let mut painter = Painter::new(HWND::default(), 1, 1, dpi, font).unwrap();
            for double in [true, false] {
                let (cells, width, height) = layout(&painter, &summary, double, false).unwrap();
                painter
                    .resize(
                        (width * dpi / 96.).ceil() as u32,
                        (height * dpi / 96.).ceil() as u32,
                        dpi,
                    )
                    .unwrap();
                for dark in [true, false] {
                    for hover in [None, Some(0)] {
                        painter
                            .paint(&summary, &cells, dark, hover, false, false)
                            .unwrap();
                        let frame = pixels(&painter);
                        let width = painter.surface.width as usize;
                        assert!(
                            frame[..width].iter().all(|p| *p == [0; 4]),
                            "top padding must be transparent"
                        );
                        assert!(
                            frame.iter().step_by(width).all(|p| *p == [0; 4]),
                            "side padding must be transparent"
                        );
                        assert!(
                            frame.iter().filter(|p| p[3] == 0).count() > frame.len() / 2,
                            "hover must not fill the cell"
                        );
                        assert!(frame.iter().any(|p| p[3] == 255), "text must remain opaque");
                        assert!(
                            frame.iter().any(|p| p[3] > 0 && p[3] < 255),
                            "glyph edges must be antialiased"
                        );
                        assert!(
                            frame.iter().all(|p| p[..3].iter().all(|c| *c <= p[3])),
                            "colors must be premultiplied"
                        );
                    }
                    painter
                        .paint(&summary, &cells, dark, None, true, false)
                        .unwrap();
                    assert!(
                        pixels(&painter).iter().filter(|p| p[3] == 0).count()
                            > pixels(&painter).len() / 2
                    );
                    painter
                        .paint(&summary, &[], dark, None, false, false)
                        .unwrap();
                    assert!(
                        pixels(&painter).iter().all(|p| *p == [0; 4]),
                        "redraw must erase old text and focus"
                    );
                    painter
                        .paint(&summary, &cells, dark, None, false, true)
                        .unwrap();
                    assert!(
                        pixels(&painter).iter().all(|p| p[3] == 255),
                        "tooltip must retain its separate surface"
                    );
                }
            }
        }
    }
    #[test]
    fn metric_combinations_fit_without_overlap_or_temperature_width_changes() {
        let painter = Painter::new(HWND::default(), 1, 1, 96., "harmonyos_sans_sc").unwrap();
        let mut summary = example();
        for mask in 0..8 {
            summary.settings.cpu = mask & 1 != 0;
            summary.settings.gpu = mask & 2 != 0;
            summary.settings.cpu_temperature = summary.settings.cpu;
            summary.settings.gpu_temperature = summary.settings.gpu;
            summary.settings.memory = mask & 4 != 0;
            for double in [true, false] {
                for compact in [false, true] {
                    let (cells, width, height) =
                        layout(&painter, &summary, double, compact).unwrap();
                    assert_eq!(
                        cells.len(),
                        if compact {
                            2
                        } else {
                            2 + (mask as u32).count_ones() as usize
                        }
                    );
                    for (i, cell) in cells.iter().enumerate() {
                        assert!(
                            cell.x >= 0. && cell.x + cell.width <= width && cell.y + 20. <= height
                        );
                        for other in &cells[i + 1..] {
                            assert!(
                                cell.y != other.y
                                    || cell.x + cell.width <= other.x
                                    || other.x + other.width <= cell.x
                            );
                        }
                        if cell.temperature_width > 0. {
                            for text in ["(0°C)", "(—°C)", "(-50°C*)", "(150°C*)"] {
                                assert!(painter.measure(text).unwrap() <= cell.temperature_width);
                            }
                        }
                    }
                    if mask == 7 && double && !compact {
                        let cpu = cells
                            .iter()
                            .find(|c| summary.readings[c.index].key == "cpu")
                            .unwrap();
                        let gpu = cells
                            .iter()
                            .find(|c| summary.readings[c.index].key == "gpu")
                            .unwrap();
                        assert_eq!(cpu.x, gpu.x);
                        assert_eq!(gpu.y - cpu.y, 20.);
                    }
                }
            }
        }
    }
    #[test]
    fn percent_and_temperature_pixels_stay_aligned_across_digit_counts() {
        for (font, dpi) in font_scales() {
            let scale = dpi / 96.;
            let mut summary = example();
            let mut painter = Painter::new(HWND::default(), 1, 1, dpi, font).unwrap();
            let (cells, width, height) = layout(&painter, &summary, true, false).unwrap();
            painter
                .resize(
                    (width * scale).ceil() as u32,
                    (height * scale).ceil() as u32,
                    dpi,
                )
                .unwrap();
            let cpu = cells
                .iter()
                .find(|c| summary.readings[c.index].key == "cpu")
                .unwrap();
            let gpu = cells
                .iter()
                .find(|c| summary.readings[c.index].key == "gpu")
                .unwrap();
            let left = ((cpu.x + cpu.label_width + cpu.value_width) * scale).ceil() as usize;
            let right = ((cpu.x + cpu.width - 12.) * scale).floor() as usize;
            let region = |frame: &[[u8; 4]], cell: &Cell| {
                let top = (cell.y * scale).round() as usize;
                let stride = painter.surface.width as usize;
                (top..top + (20. * scale).round() as usize)
                    .flat_map(|y| frame[y * stride + left..y * stride + right].iter().copied())
                    .collect::<Vec<_>>()
            };
            for text in ["2", "18", "100"] {
                summary.readings[cpu.index].text = text.into();
                summary.readings[gpu.index].text = "100".into();
                painter
                    .paint(&summary, &cells, true, None, false, false)
                    .unwrap();
                let frame = pixels(&painter);
                let cpu_suffix = region(&frame, cpu);
                assert!(cpu_suffix.iter().any(|pixel| pixel[3] > 0));
                assert_eq!(
                    cpu_suffix,
                    region(&frame, gpu),
                    "percent and temperature must share columns for {text}% versus 100%"
                );
            }
        }
    }
}
