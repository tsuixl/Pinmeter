/// Current minute in the host's local timezone. Never fall back to UTC silently.
pub fn local_minute() -> Option<u16> {
    #[cfg(target_os = "windows")]
    {
        // SAFETY: GetLocalTime has no pointer parameters and returns an initialized SYSTEMTIME.
        let time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        Some(time.wHour * 60 + time.wMinute)
    }
    #[cfg(not(target_os = "windows"))]
    {
        None
    }
}
