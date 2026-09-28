//! Recovery is restricted to the unique identity assigned before spawning our helper.
use std::{
    thread,
    time::{Duration, Instant},
};
use windows::{
    Win32::{
        Foundation::{
            ERROR_ACTIVE_CONNECTIONS, ERROR_MORE_DATA, ERROR_SUCCESS, ERROR_WMI_INSTANCE_NOT_FOUND,
        },
        System::{
            Com::CoCreateGuid,
            Diagnostics::Etw::{
                CONTROLTRACE_HANDLE, ControlTraceW, EVENT_TRACE_CONTROL_QUERY,
                EVENT_TRACE_CONTROL_STOP, EVENT_TRACE_PROPERTIES, WNODE_FLAG_TRACED_GUID,
            },
        },
    },
    core::{GUID, PCWSTR},
};

pub(super) struct OwnedSession {
    id: GUID,
    name: Vec<u16>,
}

#[repr(C)]
struct Properties {
    header: EVENT_TRACE_PROPERTIES,
    name: [u16; 512],
}

impl Properties {
    fn new(id: GUID) -> Self {
        let mut value = Self {
            header: EVENT_TRACE_PROPERTIES::default(),
            name: [0; 512],
        };
        value.header.Wnode.BufferSize = std::mem::size_of::<Self>() as u32;
        value.header.Wnode.Guid = id;
        value.header.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
        value.header.LoggerNameOffset = std::mem::offset_of!(Self, name) as u32;
        value
    }
}

impl OwnedSession {
    pub(super) fn new() -> Result<Self, String> {
        let id = unsafe { CoCreateGuid() }.map_err(|error| error.to_string())?;
        let name = format!("Pinmeter-network-{:032x}", id.to_u128())
            .encode_utf16()
            .chain(Some(0))
            .collect();
        Ok(Self { id, name })
    }

    pub(super) fn argument(&self) -> String {
        format!("{:032x}", self.id.to_u128())
    }

    /// Call only after the helper has exited: it must not create another session after recovery.
    pub(super) fn ensure_stopped(&self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let mut properties = Properties::new(self.id);
            let error = unsafe {
                ControlTraceW(
                    CONTROLTRACE_HANDLE::default(),
                    PCWSTR(self.name.as_ptr()),
                    &mut properties.header,
                    EVENT_TRACE_CONTROL_QUERY,
                )
            };
            if error == ERROR_WMI_INSTANCE_NOT_FOUND {
                return Ok(());
            }
            if error != ERROR_SUCCESS {
                return Err(format!("ETW query failed: Windows {}", error.0));
            }
            if properties.header.Wnode.Guid != self.id {
                return Err(
                    "ETW identity differs; refusing to stop another owner's session".into(),
                );
            }
            // Query writes offsets and flags; never pass its mutated properties back to STOP.
            let mut properties = Properties::new(self.id);
            let error = unsafe {
                ControlTraceW(
                    CONTROLTRACE_HANDLE::default(),
                    PCWSTR(self.name.as_ptr()),
                    &mut properties.header,
                    EVENT_TRACE_CONTROL_STOP,
                )
            };
            if error == ERROR_WMI_INSTANCE_NOT_FOUND {
                return Ok(());
            }
            if error != ERROR_SUCCESS
                && error != ERROR_MORE_DATA
                && error != ERROR_ACTIVE_CONNECTIONS
            {
                return Err(format!("ETW stop failed: Windows {}", error.0));
            }
            if Instant::now() >= deadline {
                return Err("ETW session still present after stop".into());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Diagnostics::Etw::{EVENT_TRACE_FLAG_NETWORK_TCPIP, StartTraceW};

    #[test]
    #[ignore = "requires an elevated Windows test process; creates real network ETW sessions"]
    fn real_etw_recovery_checks_identity_and_preserves_other_session() {
        let ours = OwnedSession::new().unwrap();
        let other = OwnedSession::new().unwrap();
        let start = |owned: &OwnedSession| {
            let mut properties = Properties::new(owned.id);
            properties.header.Wnode.ClientContext = 1;
            properties.header.BufferSize = 64;
            properties.header.MinimumBuffers = 2;
            properties.header.MaximumBuffers = 4;
            properties.header.LogFileMode = 0x02000100;
            properties.header.EnableFlags = EVENT_TRACE_FLAG_NETWORK_TCPIP;
            let mut handle = CONTROLTRACE_HANDLE::default();
            let error = unsafe {
                StartTraceW(
                    &mut handle,
                    PCWSTR(owned.name.as_ptr()),
                    &mut properties.header,
                )
            };
            assert_eq!(error, ERROR_SUCCESS);
        };
        // Also clean up if an assertion panics, without ever enumerating foreign sessions.
        struct Cleanup<'a>(&'a OwnedSession, &'a OwnedSession);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = self.0.ensure_stopped();
                let _ = self.1.ensure_stopped();
            }
        }
        let _cleanup = Cleanup(&ours, &other);
        start(&ours);
        start(&other);
        let impostor = OwnedSession {
            id: ours.id,
            name: other.name.clone(),
        };
        assert!(
            impostor
                .ensure_stopped()
                .unwrap_err()
                .contains("identity differs")
        );
        ours.ensure_stopped().unwrap();
        ours.ensure_stopped().unwrap();
        let mut properties = Properties::new(other.id);
        assert_eq!(
            unsafe {
                ControlTraceW(
                    CONTROLTRACE_HANDLE::default(),
                    PCWSTR(other.name.as_ptr()),
                    &mut properties.header,
                    EVENT_TRACE_CONTROL_QUERY,
                )
            },
            ERROR_SUCCESS
        );
        other.ensure_stopped().unwrap();
    }
}
