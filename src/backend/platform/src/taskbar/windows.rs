use super::{
    RetrySchedule,
    render::{Cell, Painter, layout, tooltip_lines},
};
use pinmeter_core::desktop::{DesktopIntent, DesktopStatus, DesktopSummary, rightmost_gap};
use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc::SyncSender,
    },
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::*,
        Graphics::Gdi::*,
        System::{Com::*, LibraryLoader::GetModuleHandleW, Registry::*},
        UI::{Accessibility::*, HiDpi::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
    core::{Interface, PCWSTR, w},
};

static LAYOUT_DIRTY: AtomicBool = AtomicBool::new(true);
static EXPLORER_PID: AtomicU32 = AtomicU32::new(0);
const WM_MOUSELEAVE: u32 = 0x02a3;
unsafe extern "system" fn changed(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    if matches!(
        event,
        EVENT_OBJECT_LOCATIONCHANGE
            | EVENT_OBJECT_CREATE
            | EVENT_OBJECT_DESTROY
            | EVENT_OBJECT_SHOW
            | EVENT_OBJECT_HIDE
            | EVENT_OBJECT_REORDER
    ) {
        let mut pid = 0;
        unsafe {
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
        }
        if pid != 0 && pid == EXPLORER_PID.load(Ordering::Relaxed) {
            LAYOUT_DIRTY.store(true, Ordering::Relaxed);
        }
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
fn status(out: &Mutex<DesktopStatus>, revision: u64, stage: &str, detail: impl Into<String>) {
    *out.lock().unwrap() = DesktopStatus {
        supported: stage != "unsupported",
        stage: stage.into(),
        detail: detail.into(),
        revision,
    };
}

pub(super) fn reveal_window_without_activation(handle: usize) -> Result<(), String> {
    let window = HWND(handle as *mut _);
    // SAFETY: the host supplies its live window handle on the owning UI thread.
    unsafe {
        if !IsWindow(Some(window)).as_bool() {
            return Err("主窗口句柄已失效".into());
        }
        let _ = ShowWindow(
            window,
            if IsIconic(window).as_bool() {
                SW_SHOWNOACTIVATE
            } else {
                SW_SHOWNA
            },
        );
        if !IsWindowVisible(window).as_bool() || IsIconic(window).as_bool() {
            return Err("无法恢复主窗口的可见状态".into());
        }
    }
    Ok(())
}
struct Ui {
    hwnd: HWND,
    tooltip: HWND,
    painter: Option<Painter>,
    tip_painter: Option<Painter>,
    summary: DesktopSummary,
    cells: Vec<Cell>,
    dark: bool,
    dpi: f32,
    hovered: Option<usize>,
    hover_since: Instant,
    sender: SyncSender<DesktopIntent>,
    paint_failed: bool,
}
impl Drop for Ui {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.tooltip);
            let _ = DestroyWindow(self.hwnd);
        }
    }
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            let create = &*(lp.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Ui;
        if !ptr.is_null() {
            let ui = &mut *ptr;
            match msg {
                WM_PAINT => {
                    let mut paint = PAINTSTRUCT::default();
                    BeginPaint(hwnd, &mut paint);
                    let tip = hwnd == ui.tooltip;
                    let painter = if tip {
                        ui.tip_painter.as_ref()
                    } else {
                        ui.painter.as_ref()
                    };
                    if let Some(painter) = painter {
                        ui.paint_failed |= painter
                            .draw(
                                &ui.summary,
                                &ui.cells,
                                ui.dark,
                                ui.hovered,
                                GetFocus() == hwnd,
                                tip,
                            )
                            .is_err();
                    }
                    let _ = EndPaint(hwnd, &paint);
                    return LRESULT(0);
                }
                WM_ERASEBKGND => return LRESULT(1),
                WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
                WM_MOUSEMOVE if hwnd == ui.hwnd => {
                    let x = (lp.0 as i16) as f32 * 96. / ui.dpi;
                    let y = ((lp.0 >> 16) as i16) as f32 * 96. / ui.dpi;
                    let hovered = ui
                        .cells
                        .iter()
                        .position(|c| x >= c.x && x < c.x + c.width && y >= c.y && y < c.y + 20.);
                    if hovered != ui.hovered {
                        ui.hovered = hovered;
                        ui.hover_since = Instant::now();
                        let _ = InvalidateRect(Some(hwnd), None, false);
                    }
                    let mut tracking = TRACKMOUSEEVENT {
                        cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                        dwFlags: TME_LEAVE,
                        hwndTrack: hwnd,
                        ..Default::default()
                    };
                    let _ = TrackMouseEvent(&mut tracking);
                    return LRESULT(0);
                }
                WM_MOUSELEAVE => {
                    ui.hovered = None;
                    let _ = ShowWindow(ui.tooltip, SW_HIDE);
                    let _ = InvalidateRect(Some(hwnd), None, false);
                    return LRESULT(0);
                }
                WM_LBUTTONUP if hwnd == ui.hwnd => {
                    if let Some(cell) = ui.hovered.and_then(|i| ui.cells.get(i)) {
                        let page = ui.summary.readings[cell.index].key;
                        let _ = ui.sender.try_send(DesktopIntent::Open(page));
                    }
                    return LRESULT(0);
                }
                WM_KEYUP if wp.0 == VK_RETURN.0 as usize || wp.0 == VK_SPACE.0 as usize => {
                    let _ = ui.sender.try_send(DesktopIntent::Open("overview"));
                    return LRESULT(0);
                }
                WM_SETFOCUS | WM_KILLFOCUS => {
                    let _ = InvalidateRect(Some(hwnd), None, false);
                }
                WM_CONTEXTMENU => {
                    let menu = CreatePopupMenu();
                    if let Ok(menu) = menu {
                        for (id, label) in [
                            (1, "打开总览"),
                            (2, "任务栏设置"),
                            (3, "隐藏任务栏读数"),
                            (4, "退出 Pinmeter"),
                        ] {
                            let text = wide(label);
                            let _ = AppendMenuW(menu, MF_STRING, id, PCWSTR(text.as_ptr()));
                        }
                        let mut point = POINT::default();
                        let _ = GetCursorPos(&mut point);
                        let _ = SetForegroundWindow(hwnd);
                        let selected = TrackPopupMenu(
                            menu,
                            TPM_RETURNCMD | TPM_RIGHTBUTTON,
                            point.x,
                            point.y,
                            Some(0),
                            hwnd,
                            None,
                        )
                        .0;
                        let action = match selected {
                            1 => Some(DesktopIntent::Open("overview")),
                            2 => Some(DesktopIntent::Open("settings")),
                            3 => Some(DesktopIntent::ToggleReadings),
                            4 => Some(DesktopIntent::Exit),
                            _ => None,
                        };
                        if let Some(action) = action {
                            let _ = ui.sender.try_send(action);
                        }
                        let _ = DestroyMenu(menu);
                    }
                    return LRESULT(0);
                }
                WM_NCDESTROY => {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }
}

fn shell_dark() -> bool {
    let mut value = 0u32;
    let mut size = 4;
    unsafe {
        let _ = RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut value as *mut u32).cast()),
            Some(&mut size),
        );
    }
    value == 0
}

#[derive(Clone, PartialEq)]
struct Geometry {
    parent: usize,
    rect: RECT,
    occupied: Vec<(i32, i32)>,
    dpi: u32,
    pid: u32,
    notify: RECT,
}

#[derive(Clone)]
struct DetectionError {
    detail: String,
    transient: bool,
}
impl From<String> for DetectionError {
    fn from(detail: String) -> Self {
        Self {
            detail,
            transient: false,
        }
    }
}
impl From<&str> for DetectionError {
    fn from(detail: &str) -> Self {
        Self::from(detail.to_owned())
    }
}
fn incomplete(detail: impl Into<String>) -> DetectionError {
    DetectionError {
        detail: detail.into(),
        transient: true,
    }
}
fn transient(error: windows::core::Error) -> DetectionError {
    incomplete(error.to_string())
}

const GEOMETRY_BUDGET: Duration = Duration::from_secs(1);

struct DetectionBudget<'a> {
    stop: &'a AtomicBool,
    deadline: Instant,
}
impl<'a> DetectionBudget<'a> {
    fn new(stop: &'a AtomicBool, now: Instant) -> Self {
        Self {
            stop,
            deadline: now + GEOMETRY_BUDGET,
        }
    }

    fn check(&self) -> Result<(), DetectionError> {
        self.check_at(Instant::now())
    }

    fn check_at(&self, now: Instant) -> Result<(), DetectionError> {
        if self.stop.load(Ordering::Acquire) {
            return Err(incomplete("任务栏检测已停止"));
        }
        if now >= self.deadline {
            return Err(incomplete("任务栏检测超出时间预算，稍后重新检测"));
        }
        Ok(())
    }
}

fn geometry(automation: &IUIAutomation, stop: &AtomicBool) -> Result<Geometry, DetectionError> {
    // Checks surround each COM call: the total traversal is bounded, but a
    // single provider call still depends on UI Automation's configured timeout.
    let budget = DetectionBudget::new(stop, Instant::now());
    budget.check()?;
    unsafe {
        let parent =
            FindWindowW(w!("Shell_TrayWnd"), None).map_err(|_| "Explorer 任务栏暂不可用")?;
        let mut pid = 0;
        GetWindowThreadProcessId(parent, Some(&mut pid));
        {
            use windows::Win32::System::Threading::*;
            let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
                .map_err(|_| "无法验证任务栏进程")?;
            let mut path = [0u16; 1024];
            let mut size = path.len() as u32;
            let result = QueryFullProcessImageNameW(
                process,
                PROCESS_NAME_WIN32,
                windows::core::PWSTR(path.as_mut_ptr()),
                &mut size,
            );
            let _ = CloseHandle(process);
            result.map_err(|_| "无法验证任务栏进程路径")?;
            if !String::from_utf16_lossy(&path[..size as usize])
                .to_ascii_lowercase()
                .ends_with("\\explorer.exe")
            {
                return Err("不支持第三方任务栏进程".into());
            }
        }
        EXPLORER_PID.store(pid, Ordering::Relaxed);
        if !IsWindowVisible(parent).as_bool() {
            return Err("任务栏暂时隐藏".into());
        }
        let mut rect = RECT::default();
        GetWindowRect(parent, &mut rect).map_err(|e| e.to_string())?;
        let mut monitor = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if !GetMonitorInfoW(
            MonitorFromWindow(parent, MONITOR_DEFAULTTONEAREST),
            &mut monitor,
        )
        .as_bool()
        {
            return Err("无法确定任务栏屏幕边界".into());
        }
        if rect.bottom.min(monitor.rcMonitor.bottom) - rect.top.max(monitor.rcMonitor.top)
            < rect.bottom - rect.top
        {
            return Err("任务栏暂时隐藏".into());
        }
        if rect.right - rect.left < rect.bottom - rect.top {
            return Err("暂不支持竖向任务栏".into());
        }
        budget.check()?;
        let root = automation.ElementFromHandle(parent).map_err(transient)?;
        budget.check()?;
        let frame_condition = automation
            .CreatePropertyCondition(
                UIA_AutomationIdPropertyId,
                &windows::Win32::System::Variant::VARIANT::from("TaskbarFrame"),
            )
            .map_err(transient)?;
        budget.check()?;
        let frame = root
            .FindFirst(TreeScope_Descendants, &frame_condition)
            .map_err(transient)?;
        budget.check()?;
        if frame.CurrentClassName().map_err(transient)? != "Taskbar.TaskbarFrameAutomationPeer" {
            return Err(incomplete("任务栏结构尚未就绪，正在重新检测"));
        }
        budget.check()?;
        let button_condition = automation
            .CreatePropertyCondition(
                UIA_ControlTypePropertyId,
                &windows::Win32::System::Variant::VARIANT::from(UIA_ButtonControlTypeId.0),
            )
            .map_err(transient)?;
        budget.check()?;
        let nodes = frame
            .FindAll(TreeScope_Descendants, &button_condition)
            .map_err(transient)?;
        budget.check()?;
        let count = nodes.Length().map_err(transient)?;
        budget.check()?;
        if count > 256 {
            return Err("任务栏元素过多，无法可靠定位".into());
        }
        let mut occupied = vec![];
        let mut buttons = 0;
        for i in 0..count {
            budget.check()?;
            let node = nodes.GetElement(i).map_err(transient)?;
            budget.check()?;
            let button = node.CurrentControlType().map_err(transient)? == UIA_ButtonControlTypeId;
            budget.check()?;
            let offscreen = node.CurrentIsOffscreen().map_err(transient)?.as_bool();
            budget.check()?;
            if button && !offscreen {
                let r = node.CurrentBoundingRectangle().map_err(transient)?;
                budget.check()?;
                if r.right > r.left && r.bottom > rect.top && r.top < rect.bottom {
                    occupied.push((r.left, r.right));
                    buttons += 1;
                }
            }
        }
        if buttons == 0 {
            return Err(incomplete("任务栏按钮暂不可读，正在重新检测"));
        }
        budget.check()?;
        // The notification container reserves all tray space, including blank areas.
        let notify_rect =
            if let Ok(notify) = FindWindowExW(Some(parent), None, w!("TrayNotifyWnd"), None) {
                let mut r = RECT::default();
                GetWindowRect(notify, &mut r).map_err(|e| e.to_string())?;
                occupied.push((r.left, r.right));
                r
            } else {
                return Err("无法确定系统通知区域".into());
            };
        budget.check()?;
        Ok(Geometry {
            parent: parent.0 as usize,
            rect,
            occupied,
            dpi: GetDpiForWindow(parent),
            pid,
            notify: notify_rect,
        })
    }
}

// Recheck native identity and bounds even when UIA is unavailable. A notification
// resize can invalidate a proposed layout without invalidating the visible child.
fn current_notification(g: &Geometry) -> Option<RECT> {
    unsafe {
        let parent = HWND(g.parent as *mut _);
        if FindWindowW(w!("Shell_TrayWnd"), None).ok() != Some(parent)
            || !IsWindowVisible(parent).as_bool()
            || GetDpiForWindow(parent) != g.dpi
        {
            return None;
        }
        let mut pid = 0;
        GetWindowThreadProcessId(parent, Some(&mut pid));
        let mut rect = RECT::default();
        if pid != g.pid || GetWindowRect(parent, &mut rect).is_err() || rect != g.rect {
            return None;
        }
        let notify = FindWindowExW(Some(parent), None, w!("TrayNotifyWnd"), None).ok()?;
        GetWindowRect(notify, &mut rect).ok()?;
        (rect.right > rect.left && rect.bottom > rect.top).then_some(rect)
    }
}

fn geometry_is_current(g: &Geometry) -> bool {
    current_notification(g) == Some(g.notify)
}

fn placement_fits(taskbar: RECT, notify: RECT, child: RECT, dpi: u32) -> bool {
    let margin = (4. * dpi as f32 / 96.) as i32;
    child.right > child.left
        && child.bottom > child.top
        && child.left >= taskbar.left
        && child.right <= taskbar.right
        && child.top >= taskbar.top
        && child.bottom <= taskbar.bottom
        && (child.right <= notify.left - margin || child.left >= notify.right + margin)
}

fn geometry_can_retain(g: &Geometry, hwnd: HWND) -> bool {
    let Some(notify) = current_notification(g) else {
        return false;
    };
    let mut child = RECT::default();
    unsafe {
        GetParent(hwnd).ok() == Some(HWND(g.parent as *mut _))
            && GetWindowRect(hwnd, &mut child).is_ok()
            && placement_fits(g.rect, notify, child, g.dpi)
    }
}

enum GeometryUpdate {
    Measured(Geometry),
    // Permission to keep the existing window only, never to lay out a new one.
    KeepVisible,
}

const MAX_OBSERVATION_AGE: Duration = Duration::from_secs(4);
// Freshness limits first placement and new layouts, not the lifetime of a visible
// window. A stalled UIA query must not blink a still-valid native placement.
fn measured_geometry(
    observation: &Observation,
    last_good: &mut Option<(Instant, Geometry)>,
    now: Instant,
    can_retain: bool,
    valid: impl Fn(&Geometry) -> bool,
    retained_valid: impl Fn(&Geometry) -> bool,
) -> Result<GeometryUpdate, String> {
    let (error, retryable) = match observation {
        Some((at, Ok(g))) => {
            if !valid(g) {
                ("任务栏位置已变化，正在重新检测".into(), true)
            } else if now.duration_since(*at) < MAX_OBSERVATION_AGE {
                *last_good = Some((*at, g.clone()));
                return Ok(GeometryUpdate::Measured(g.clone()));
            } else {
                ("正在检测任务栏位置".into(), true)
            }
        }
        Some((_, Err(error))) => (error.detail.clone(), error.transient),
        None => ("正在检测任务栏位置".into(), true),
    };
    if retryable
        && can_retain
        && let Some((_, g)) = last_good.as_ref()
        && retained_valid(g)
    {
        return Ok(GeometryUpdate::KeepVisible);
    }
    *last_good = None;
    Err(error)
}

pub fn run(
    latest: Arc<Mutex<Option<DesktopSummary>>>,
    out: Arc<Mutex<DesktopStatus>>,
    stop: Arc<AtomicBool>,
    sender: SyncSender<DesktopIntent>,
) {
    // SAFETY: the entire Win32/COM lifetime, including destruction, is thread-bound.
    unsafe {
        if CoInitializeEx(None, COINIT_MULTITHREADED).is_err() {
            status(&out, 0, "failed", "无法初始化桌面线程");
            return;
        }
        let result = run_inner(latest.clone(), &out, stop, sender);
        if let Err(error) = result {
            let revision = latest.lock().unwrap().as_ref().map_or(0, |s| s.revision);
            status(&out, revision, "failed", error);
        }
        CoUninitialize();
    }
}
unsafe fn run_inner(
    latest: Arc<Mutex<Option<DesktopSummary>>>,
    out: &Mutex<DesktopStatus>,
    stop: Arc<AtomicBool>,
    sender: SyncSender<DesktopIntent>,
) -> Result<(), String> {
    unsafe {
        let observer = Observer::start(latest.clone());
        let instance = GetModuleHandleW(None).map_err(|e| e.to_string())?;
        let class = w!("PinmeterTaskbarDisplay");
        let wc = WNDCLASSW {
            lpfnWndProc: Some(procedure),
            hInstance: instance.into(),
            lpszClassName: class,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            ..Default::default()
        };
        if RegisterClassW(&wc) == 0 {
            return Err("无法注册任务栏窗口".into());
        }
        let hook = SetWinEventHook(
            EVENT_OBJECT_CREATE,
            EVENT_OBJECT_LOCATIONCHANGE,
            None,
            Some(changed),
            0,
            0,
            WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
        );
        let _registration = Registration {
            instance: instance.into(),
            hook,
        };
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let mut ui: Option<Box<Ui>> = None;
        let mut parent = HWND::default();
        let mut next_layout = Instant::now();
        let mut last_config = None;
        let mut last_revision = None;
        let mut last_summary = None;
        let mut last_good = None;
        let mut last_observation = None;
        let mut last_placement = None;
        let mut last_geometry = None;
        while !stop.load(Ordering::Acquire) {
            let mut message = MSG::default();
            while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if let Some(error) = observer.failure() {
                return Err(error);
            }
            let summary = latest.lock().unwrap().clone();
            if let Some(summary) = summary {
                let revision = summary.revision;
                let enabled = summary.settings.enabled && !summary.settings.hidden;
                if !enabled {
                    if let Some(ui) = &ui {
                        let _ = ShowWindow(ui.hwnd, SW_HIDE);
                        let _ = ShowWindow(ui.tooltip, SW_HIDE);
                    }
                    status(
                        out,
                        revision,
                        if summary.settings.enabled {
                            "hidden"
                        } else {
                            "disabled"
                        },
                        if summary.settings.enabled {
                            "读数已隐藏，可从托盘恢复"
                        } else {
                            "任务栏显示已关闭"
                        },
                    );
                    last_config = None;
                    last_placement = None;
                    last_good = None;
                } else {
                    let config = (
                        summary.settings.clone(),
                        summary.font_family.clone(),
                        summary.font_style.clone(),
                    );
                    let changed = last_config.as_ref() != Some(&config);
                    let observation = observer.geometry.lock().unwrap().clone();
                    let observed_at = observation.as_ref().map(|(at, _)| *at);
                    if changed
                        || last_revision != Some(revision)
                        || observed_at != last_observation
                        || Instant::now() >= next_layout
                    {
                        last_observation = observed_at;
                        let can_retain = !changed
                            && last_geometry.is_some()
                            && ui.as_ref().is_some_and(|u| {
                                IsWindow(Some(u.hwnd)).as_bool()
                                    && IsWindowVisible(u.hwnd).as_bool()
                                    && !u.paint_failed
                            });
                        let measured = measured_geometry(
                            &observation,
                            &mut last_good,
                            Instant::now(),
                            can_retain,
                            geometry_is_current,
                            |g| ui.as_ref().is_some_and(|u| geometry_can_retain(g, u.hwnd)),
                        );
                        match measured {
                            Ok(GeometryUpdate::KeepVisible) => {
                                out.lock().unwrap().revision = revision;
                                next_layout = Instant::now() + Duration::from_millis(500);
                            }
                            Ok(GeometryUpdate::Measured(g))
                                if !changed
                                    && last_geometry.as_ref() == Some(&g)
                                    && ui.as_ref().is_some_and(|u| {
                                        IsWindow(Some(u.hwnd)).as_bool()
                                            && IsWindowVisible(u.hwnd).as_bool()
                                            && !u.paint_failed
                                            && u.dark == shell_dark()
                                    }) =>
                            {
                                out.lock().unwrap().revision = revision;
                                next_layout = Instant::now() + Duration::from_millis(500);
                            }
                            Ok(GeometryUpdate::Measured(g)) => {
                                last_geometry = Some(g.clone());
                                let new_parent = HWND(g.parent as *mut _);
                                if !AreDpiAwarenessContextsEqual(
                                    GetThreadDpiAwarenessContext(),
                                    GetWindowDpiAwarenessContext(new_parent),
                                )
                                .as_bool()
                                {
                                    if let Some(ui) = &ui {
                                        let _ = ShowWindow(ui.hwnd, SW_HIDE);
                                        let _ = ShowWindow(ui.tooltip, SW_HIDE);
                                    }
                                    status(
                                        out,
                                        revision,
                                        "unsupported",
                                        "任务栏 DPI 模式不同，未执行跨进程挂接",
                                    );
                                    next_layout = Instant::now() + Duration::from_secs(5);
                                } else {
                                    let recreate = ui.as_ref().is_none_or(|u| {
                                        !IsWindow(Some(u.hwnd)).as_bool() || parent != new_parent
                                    });
                                    if recreate {
                                        drop(ui.take());
                                        last_placement = None;
                                        let mut state = Box::new(Ui {
                                            hwnd: HWND::default(),
                                            tooltip: HWND::default(),
                                            painter: None,
                                            tip_painter: None,
                                            summary: summary.clone(),
                                            cells: vec![],
                                            dark: shell_dark(),
                                            dpi: g.dpi as f32,
                                            hovered: None,
                                            hover_since: Instant::now(),
                                            sender: sender.clone(),
                                            paint_failed: false,
                                        });
                                        let ptr = (&mut *state as *mut Ui).cast();
                                        state.hwnd = CreateWindowExW(
                                            WS_EX_NOACTIVATE | WS_EX_LAYERED,
                                            class,
                                            w!("Pinmeter 任务栏读数"),
                                            WS_CHILD | WS_TABSTOP,
                                            0,
                                            0,
                                            1,
                                            1,
                                            Some(new_parent),
                                            None,
                                            Some(instance.into()),
                                            Some(ptr),
                                        )
                                        .map_err(|e| e.to_string())?;
                                        state.tooltip = CreateWindowExW(
                                            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
                                            class,
                                            w!("Pinmeter 指标说明"),
                                            WS_POPUP,
                                            0,
                                            0,
                                            1,
                                            1,
                                            Some(state.hwnd),
                                            None,
                                            Some(instance.into()),
                                            Some(ptr),
                                        )
                                        .map_err(|e| e.to_string())?;
                                        parent = new_parent;
                                        ui = Some(state);
                                    }
                                    let state = ui.as_mut().unwrap();
                                    state.dpi = g.dpi as f32;
                                    state.summary = summary.clone();
                                    let dark = shell_dark();
                                    if state.dark != dark {
                                        state.dark = dark;
                                        last_placement = None;
                                    }
                                    let scale = state.dpi / 96.;
                                    if state.paint_failed
                                        || state.painter.as_ref().is_some_and(|p| {
                                            !p.uses_font(&summary.font_family, &summary.font_style)
                                        })
                                    {
                                        state.painter = None;
                                        state.tip_painter = None;
                                        state.paint_failed = false;
                                        last_placement = None;
                                    }
                                    if state.painter.is_none() {
                                        state.painter = Some(
                                            Painter::new(
                                                state.hwnd,
                                                1,
                                                1,
                                                state.dpi,
                                                &summary.font_family,
                                                &summary.font_style,
                                            )
                                            .map_err(|e| e.to_string())?,
                                        );
                                    }
                                    let painter = state.painter.as_mut().unwrap();
                                    let double = summary.settings.layout == "double"
                                        && ((g.rect.bottom - g.rect.top) as f32 / scale) >= 44.;
                                    let mut choice = None;
                                    for compact in [false, true] {
                                        let (cells, width, height) =
                                            layout(painter, &summary, double, compact)
                                                .map_err(|e| e.to_string())?;
                                        let width = (width * scale).ceil() as i32;
                                        let height = (height * scale).ceil() as i32;
                                        if height > g.rect.bottom - g.rect.top {
                                            continue;
                                        }
                                        if let Some(x) = rightmost_gap(
                                            g.rect.left,
                                            g.rect.right,
                                            &g.occupied,
                                            width,
                                            (4. * scale) as i32,
                                        ) {
                                            choice = Some((cells, x, width, height, compact));
                                            break;
                                        }
                                    }
                                    if let Some((cells, x, width, height, compact)) = choice {
                                        state.cells = cells;
                                        let y = (g.rect.bottom - g.rect.top - height) / 2;
                                        painter
                                            .resize(width as u32, height as u32, state.dpi)
                                            .map_err(|e| e.to_string())?;
                                        let placement = (
                                            g.parent,
                                            x - g.rect.left,
                                            y,
                                            width,
                                            height,
                                            g.dpi,
                                            summary.settings.clone(),
                                        );
                                        if last_placement.as_ref() != Some(&placement)
                                            || !IsWindowVisible(state.hwnd).as_bool()
                                        {
                                            SetWindowPos(
                                                state.hwnd,
                                                Some(HWND_TOP),
                                                x - g.rect.left,
                                                y,
                                                width,
                                                height,
                                                SWP_NOACTIVATE | SWP_SHOWWINDOW,
                                            )
                                            .map_err(|e| e.to_string())?;
                                            let _ = ShowWindow(state.tooltip, SW_HIDE);
                                            state.hovered = None;
                                            let _ = InvalidateRect(Some(state.hwnd), None, false);
                                            last_placement = Some(placement);
                                        }
                                        status(
                                            out,
                                            revision,
                                            "visible",
                                            if painter.font_fallback {
                                                "读数已显示；所选字体或样式不可用，暂用鸿蒙"
                                            } else if compact {
                                                "空间有限，当前显示所选指标的精简布局"
                                            } else {
                                                "任务栏读数已显示"
                                            },
                                        );
                                    } else {
                                        let _ = ShowWindow(state.hwnd, SW_HIDE);
                                        let _ = ShowWindow(state.tooltip, SW_HIDE);
                                        status(
                                            out,
                                            revision,
                                            "no_space",
                                            "任务栏空间不足，已保留托盘入口",
                                        );
                                    }
                                    next_layout = Instant::now() + Duration::from_millis(500);
                                }
                            }
                            Err(error) => {
                                last_geometry = None;
                                if let Some(ui) = &ui {
                                    let _ = ShowWindow(ui.hwnd, SW_HIDE);
                                    let _ = ShowWindow(ui.tooltip, SW_HIDE);
                                }
                                status(out, revision, "recovering", error);
                                next_layout = Instant::now() + Duration::from_millis(500);
                            }
                        }
                        last_config = Some(config);
                        last_revision = Some(revision);
                    }
                    if let Some(state) = ui.as_mut() {
                        if last_summary.as_ref() != Some(&summary) {
                            state.summary = summary.clone();
                            let accessible = wide(
                                &summary
                                    .readings
                                    .iter()
                                    .map(|r| {
                                        format!(
                                            "{} {} {}",
                                            r.display(),
                                            r.detail,
                                            r.temperature
                                                .as_ref()
                                                .map_or("", |t| t.detail.as_str())
                                        )
                                    })
                                    .collect::<Vec<_>>()
                                    .join("；"),
                            );
                            let _ = SetWindowTextW(state.hwnd, PCWSTR(accessible.as_ptr()));
                            if IsWindowVisible(state.hwnd).as_bool() {
                                let _ = InvalidateRect(Some(state.hwnd), None, false);
                            }
                            if IsWindowVisible(state.tooltip).as_bool() {
                                let _ = InvalidateRect(Some(state.tooltip), None, false);
                            }
                        }
                        if state.hovered.is_some()
                            && state.hover_since.elapsed() >= Duration::from_millis(600)
                            && IsWindowVisible(state.hwnd).as_bool()
                            && !IsWindowVisible(state.tooltip).as_bool()
                        {
                            let mut rect = RECT::default();
                            let _ = GetWindowRect(state.hwnd, &mut rect);
                            let scale = state.dpi / 96.;
                            let width = (560. * scale) as i32;
                            let height = ((12. + tooltip_lines(&state.summary).len() as f32 * 24.)
                                * scale) as i32;
                            let mut monitor = MONITORINFO {
                                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                                ..Default::default()
                            };
                            let _ = GetMonitorInfoW(
                                MonitorFromWindow(state.hwnd, MONITOR_DEFAULTTONEAREST),
                                &mut monitor,
                            );
                            let bounds = monitor.rcWork;
                            let x = (rect.right - width)
                                .max(bounds.left)
                                .min((bounds.right - width).max(bounds.left));
                            let y = if rect.top - height >= bounds.top {
                                rect.top - height - (8. * scale) as i32
                            } else {
                                rect.bottom + (8. * scale) as i32
                            };
                            if state.tip_painter.is_none() {
                                state.tip_painter = Painter::new(
                                    state.tooltip,
                                    width as u32,
                                    height as u32,
                                    state.dpi,
                                    &state.summary.font_family,
                                    &state.summary.font_style,
                                )
                                .ok();
                            }
                            if let Some(painter) = &mut state.tip_painter {
                                let _ = painter.resize(width as u32, height as u32, state.dpi);
                            }
                            let region = CreateRoundRectRgn(
                                0,
                                0,
                                width + 1,
                                height + 1,
                                (16. * scale) as i32,
                                (16. * scale) as i32,
                            );
                            SetWindowRgn(state.tooltip, Some(region), false);
                            let _ = SetWindowPos(
                                state.tooltip,
                                Some(HWND_TOPMOST),
                                x,
                                y.max(bounds.top),
                                width,
                                height,
                                SWP_NOACTIVATE | SWP_SHOWWINDOW,
                            );
                            let _ = InvalidateRect(Some(state.tooltip), None, false);
                        }
                    }
                }
                last_summary = Some(summary);
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        drop(ui);
        drop(observer);
        Ok(())
    }
}

struct Registration {
    instance: HINSTANCE,
    hook: HWINEVENTHOOK,
}
impl Drop for Registration {
    fn drop(&mut self) {
        unsafe {
            if !self.hook.is_invalid() {
                let _ = UnhookWinEvent(self.hook);
            }
            let _ = UnregisterClassW(w!("PinmeterTaskbarDisplay"), Some(self.instance));
        }
    }
}

// UI Automation must not query our own windows on their message thread.
// One observer owns COM and retry scheduling. The UI never delays a fresh result.
type Observation = Option<(Instant, Result<Geometry, DetectionError>)>;
struct Observer {
    geometry: Arc<Mutex<Observation>>,
    failure: Arc<Mutex<Option<String>>>,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl Observer {
    fn start(latest: Arc<Mutex<Option<DesktopSummary>>>) -> Self {
        let observed = Arc::new(Mutex::new(None));
        let result = observed.clone();
        let failure = Arc::new(Mutex::new(None));
        let failure_out = failure.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let stopping = stop.clone();
        let worker = std::thread::Builder::new()
            .name("pinmeter-taskbar-observer".into())
            .spawn(move || unsafe {
                if let Err(error) = CoInitializeEx(None, COINIT_MULTITHREADED).ok() {
                    *failure_out.lock().unwrap() =
                        Some(format!("任务栏检测线程初始化失败：{error}"));
                    return;
                }
                let work = (|| -> windows::core::Result<()> {
                    let automation: IUIAutomation =
                        CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)?;
                    let timeouts: IUIAutomation2 = automation.cast()?;
                    timeouts.SetConnectionTimeout(200)?;
                    timeouts.SetTransactionTimeout(200)?;
                    SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                    let mut next = Instant::now();
                    let mut next_event = Instant::now();
                    let mut retry = RetrySchedule::new(Instant::now());
                    while !stopping.load(Ordering::Acquire) {
                        let (enabled, revision) =
                            latest
                                .lock()
                                .unwrap()
                                .as_ref()
                                .map_or((false, 0), |summary| {
                                    (
                                        summary.settings.enabled && !summary.settings.hidden,
                                        summary.revision,
                                    )
                                });
                        let now = Instant::now();
                        let changed = retry.configure(enabled, revision, now);
                        if retry.due(now)
                            && (changed
                                || now >= next
                                || (now >= next_event
                                    && LAYOUT_DIRTY.swap(false, Ordering::Relaxed)))
                        {
                            let started = Instant::now();
                            let g = geometry(&automation, &stopping);
                            let completed = Instant::now();
                            if g.is_ok() {
                                retry.succeeded(completed);
                                next = completed + Duration::from_secs(2);
                            } else {
                                retry.failed(completed);
                                next = completed;
                            }
                            *result.lock().unwrap() = Some((started, g));
                            next_event = completed + Duration::from_millis(500);
                        }
                        std::thread::sleep(Duration::from_millis(100));
                    }
                    Ok(())
                })();
                if let Err(error) = work {
                    *failure_out.lock().unwrap() = Some(format!("任务栏检测失败：{error}"));
                }
                CoUninitialize();
            });
        let worker = match worker {
            Ok(worker) => Some(worker),
            Err(error) => {
                *failure.lock().unwrap() = Some(format!("无法启动任务栏检测线程：{error}"));
                None
            }
        };
        Self {
            geometry: observed,
            failure,
            stop,
            worker,
        }
    }

    fn failure(&self) -> Option<String> {
        observer_failure(
            self.worker
                .as_ref()
                .is_none_or(|worker| worker.is_finished()),
            self.failure.lock().unwrap().as_deref(),
        )
    }
}

fn observer_failure(finished: bool, detail: Option<&str>) -> Option<String> {
    finished.then(|| detail.unwrap_or("任务栏检测线程意外停止").into())
}
impl Drop for Observer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            // Complete pending accessibility messages while the observer unwinds COM.
            while !worker.is_finished() {
                unsafe {
                    let mut message = MSG::default();
                    while PeekMessageW(&mut message, None, 0, 0, PM_REMOVE).as_bool() {
                        let _ = TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detection_budget_limits_the_whole_traversal_and_honors_stop() {
        let stop = AtomicBool::new(false);
        let now = Instant::now();
        let budget = DetectionBudget::new(&stop, now);
        assert!(budget.check_at(now).is_ok());
        assert!(budget.check_at(now + GEOMETRY_BUDGET).is_err());
        stop.store(true, Ordering::Release);
        assert!(budget.check_at(now).is_err());
    }

    #[test]
    fn observer_exit_reaches_the_outer_recovery_with_its_cause() {
        assert!(observer_failure(false, Some("初始化失败")).is_none());
        assert_eq!(
            observer_failure(true, Some("初始化失败")).as_deref(),
            Some("初始化失败")
        );
        assert_eq!(
            observer_failure(true, None).as_deref(),
            Some("任务栏检测线程意外停止")
        );
    }

    fn sample() -> Geometry {
        Geometry {
            parent: 1,
            rect: RECT {
                left: 0,
                top: 1000,
                right: 1920,
                bottom: 1080,
            },
            occupied: vec![(0, 400), (1700, 1920)],
            dpi: 96,
            pid: 1,
            notify: RECT {
                left: 1700,
                top: 1000,
                right: 1920,
                bottom: 1080,
            },
        }
    }
    fn failure(at: Instant, transient: bool) -> Observation {
        Some((
            at,
            Err(DetectionError {
                detail: "injected detection failure".into(),
                transient,
            }),
        ))
    }

    #[test]
    fn prolonged_timeouts_keep_the_visible_position_until_detection_recovers() {
        let start = Instant::now();
        let mut good = None;
        let original = sample();
        assert!(
            measured_geometry(
                &Some((start, Ok(original.clone()))),
                &mut good,
                start,
                false,
                |_| true,
                |_| true
            )
            .is_ok()
        );
        for ms in (2000..=120_000).step_by(500) {
            let now = start + Duration::from_millis(ms);
            let result = measured_geometry(
                &failure(now, true),
                &mut good,
                now,
                true,
                |_| true,
                |_| true,
            )
            .unwrap();
            assert!(matches!(result, GeometryUpdate::KeepVisible));
            assert!(good.as_ref().unwrap().1 == original);
            assert_eq!(good.as_ref().unwrap().0, start);
        }
        let now = start + Duration::from_secs(121);
        // Replaying an old error also retains the window; it does not renew success.
        assert!(
            measured_geometry(
                &failure(start, true),
                &mut good,
                now,
                true,
                |_| true,
                |_| true
            )
            .is_ok()
        );
        assert_eq!(good.as_ref().unwrap().0, start);
        // A new successful result is accepted immediately, even after many failures.
        let mut moved = sample();
        moved.occupied.push((900, 1000));
        let recovered = measured_geometry(
            &Some((now, Ok(moved.clone()))),
            &mut good,
            now,
            false,
            |_| true,
            |_| true,
        )
        .unwrap();
        assert!(matches!(recovered, GeometryUpdate::Measured(g) if g == moved));
    }

    #[test]
    fn stalled_observations_keep_position_without_forging_a_new_success() {
        let start = Instant::now();
        let observed = Some((start, Ok(sample())));
        let mut good = None;
        for ms in [0, 500, 2000, 4000, 30_000, 120_000] {
            assert!(
                measured_geometry(
                    &observed,
                    &mut good,
                    start + Duration::from_millis(ms),
                    true,
                    |_| true,
                    |_| true
                )
                .is_ok()
            );
            assert_eq!(good.as_ref().unwrap().0, start);
        }
        assert!(
            measured_geometry(
                &None,
                &mut good,
                start + Duration::from_secs(121),
                true,
                |_| true,
                |_| true
            )
            .is_ok()
        );
        assert_eq!(good.as_ref().unwrap().0, start);
    }

    #[test]
    fn first_placement_requires_a_fresh_success_and_never_guesses_a_position() {
        let start = Instant::now();
        for observed in [None, failure(start, true), Some((start, Ok(sample())))] {
            let mut good = None;
            assert!(
                measured_geometry(
                    &observed,
                    &mut good,
                    start + MAX_OBSERVATION_AGE,
                    true,
                    |_| true,
                    |_| true
                )
                .is_err()
            );
            assert!(good.is_none());
        }
        let mut good = Some((start, sample()));
        // A new configuration must not revive a layout from an expired observation.
        assert!(
            measured_geometry(
                &Some((start, Ok(sample()))),
                &mut good,
                start + MAX_OBSERVATION_AGE,
                false,
                |_| true,
                |_| true
            )
            .is_err()
        );
        assert!(good.is_none());
    }

    #[test]
    fn invalid_environment_and_explicit_changes_discard_the_cached_position() {
        let start = Instant::now();
        for (transient, can_retain, native_valid) in [
            (false, true, true), // Unsupported structure.
            (true, false, true), // Disabled, config change, no space or hidden child.
            (true, true, false), // Explorer destroyed, moved, hidden, DPI/tray change.
        ] {
            let mut good = Some((start, sample()));
            assert!(
                measured_geometry(
                    &failure(start, transient),
                    &mut good,
                    start,
                    can_retain,
                    |_| native_valid,
                    |_| native_valid
                )
                .is_err()
            );
            assert!(good.is_none());
            // No resurrecting a discarded cache on a later transient error.
            assert!(
                measured_geometry(
                    &failure(start, true),
                    &mut good,
                    start,
                    true,
                    |_| true,
                    |_| true
                )
                .is_err()
            );
        }
        // Also reject a successful UIA result if native bounds changed during its query.
        let mut good = Some((start, sample()));
        assert!(
            measured_geometry(
                &Some((start, Ok(sample()))),
                &mut good,
                start,
                true,
                |_| false,
                |_| false
            )
            .is_err()
        );
        assert!(good.is_none());
    }

    #[test]
    fn notification_resize_retains_only_the_existing_safe_window() {
        let start = Instant::now();
        let g = sample();
        let child = RECT {
            left: 1200,
            right: 1600,
            top: 1004,
            bottom: 1076,
        };
        let mut notify = g.notify;
        let mut good = Some((start, g.clone()));
        // An outdated but successful UIA result previously hid the whole window.
        // Native notification changes alone do not authorize a new layout.
        for left in [1720, 1660, 1604, 1800] {
            notify.left = left;
            assert!(matches!(
                measured_geometry(
                    &Some((start, Ok(g.clone()))),
                    &mut good,
                    start,
                    true,
                    |_| false,
                    |g| placement_fits(g.rect, notify, child, g.dpi),
                ),
                Ok(GeometryUpdate::KeepVisible)
            ));
            assert_eq!(good.as_ref().unwrap().0, start);
        }
        // Crossing the reserved margin really invalidates the visible position.
        notify.left = 1603;
        assert!(
            measured_geometry(
                &failure(start, true),
                &mut good,
                start,
                true,
                |_| false,
                |g| placement_fits(g.rect, notify, child, g.dpi),
            )
            .is_err()
        );
        assert!(good.is_none());
        // No cached window means no guessed first placement, even in a safe gap.
        assert!(
            measured_geometry(
                &Some((start, Ok(g))),
                &mut good,
                start,
                true,
                |_| false,
                |_| true,
            )
            .is_err()
        );
    }

    #[test]
    fn incomplete_button_tree_keeps_visible_but_cannot_place_a_new_window() {
        let start = Instant::now();
        let observation = Some((start, Err(incomplete("任务栏按钮暂不可读"))));
        let mut good = Some((start, sample()));
        assert!(matches!(
            measured_geometry(&observation, &mut good, start, true, |_| true, |_| true),
            Ok(GeometryUpdate::KeepVisible)
        ));
        assert!(
            measured_geometry(&observation, &mut good, start, false, |_| true, |_| true).is_err()
        );
        assert!(good.is_none());
    }

    #[test]
    fn retention_requires_full_child_bounds_and_dpi_scaled_notification_margin() {
        let g = sample();
        let mut child = RECT {
            left: 1200,
            right: 1692,
            top: 1000,
            bottom: 1080,
        };
        assert!(placement_fits(g.rect, g.notify, child, 192));
        child.right += 1;
        assert!(!placement_fits(g.rect, g.notify, child, 192));
        assert!(placement_fits(g.rect, g.notify, child, 96));
        child.bottom += 1;
        assert!(!placement_fits(g.rect, g.notify, child, 96));
        child.bottom = child.top;
        assert!(!placement_fits(g.rect, g.notify, child, 96));
        child = RECT {
            left: -1,
            right: 500,
            top: 1000,
            bottom: 1080,
        };
        assert!(!placement_fits(g.rect, g.notify, child, 96));
    }

    #[test]
    #[ignore = "reads the current Explorer taskbar for 30 seconds; no windows are created"]
    fn live_observer_reads_without_embedding() {
        use pinmeter_core::desktop::TaskbarSettings;
        let latest = Arc::new(Mutex::new(Some(DesktopSummary {
            font_family: pinmeter_core::domain::default_font_family(),
            font_style: pinmeter_core::fonts::default_font_style(),
            session: "read-only-observer-probe".into(),
            cursor: 1,
            network_id: None,
            gpu_id: None,
            revision: 1,
            settings: TaskbarSettings {
                enabled: true,
                ..Default::default()
            },
            network: String::new(),
            readings: vec![],
        })));
        let previous_dpi =
            unsafe { SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
        let observer = Observer::start(latest);
        let start = Instant::now();
        let mut last = None;
        let mut good = None;
        let mut successes = 0;
        let mut failures = 0;
        let mut valid_positions = 0;
        while start.elapsed() < Duration::from_secs(30) {
            let observed = observer.geometry.lock().unwrap().clone();
            if let Some((at, result)) = &observed
                && last != Some(*at)
            {
                last = Some(*at);
                match result {
                    Ok(_) => successes += 1,
                    Err(error) => {
                        failures += 1;
                        eprintln!("observer error: {}", error.detail);
                    }
                }
                let resolved = measured_geometry(
                    &observed,
                    &mut good,
                    Instant::now(),
                    true,
                    geometry_is_current,
                    geometry_is_current,
                );
                valid_positions += usize::from(resolved.is_ok());
                eprintln!(
                    "observation at {:?}: query_ok={} position_available={}",
                    at.duration_since(start),
                    result.is_ok(),
                    resolved.is_ok()
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        drop(observer);
        unsafe {
            SetThreadDpiAwarenessContext(previous_dpi);
        }
        eprintln!(
            "live observer: {successes} successful, {failures} failed observations; shutdown completed"
        );
        assert!(
            successes > 0 && valid_positions > 0,
            "no valid native position detected on this machine"
        );
    }
}
