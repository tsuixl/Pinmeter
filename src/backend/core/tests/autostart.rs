use pinmeter_core::{
    application::Monitor,
    autostart::{Autostart, AutostartStatus},
    domain::Settings,
    ports::SettingsRepository,
};
use std::sync::Mutex;

#[derive(Default)]
struct Registration {
    value: Mutex<String>,
    calls: Mutex<Vec<bool>>,
    fail_apply: bool,
    fail_restore: bool,
}
impl Autostart for Registration {
    fn status(&self) -> AutostartStatus {
        AutostartStatus {
            available: true,
            enabled: Some(!self.value.lock().unwrap().is_empty()),
            detail: String::new(),
        }
    }
    fn checkpoint(&self) -> Result<String, String> {
        Ok(self.value.lock().unwrap().clone())
    }
    fn restore(&self, checkpoint: &str) -> Result<(), String> {
        if self.fail_restore {
            return Err("restore failed".into());
        }
        *self.value.lock().unwrap() = checkpoint.into();
        Ok(())
    }
    fn set_enabled(&self, enabled: bool) -> Result<(), String> {
        self.calls.lock().unwrap().push(enabled);
        *self.value.lock().unwrap() = if enabled {
            "new path".into()
        } else {
            String::new()
        };
        if self.fail_apply {
            Err("registration failed".into())
        } else {
            Ok(())
        }
    }
}
#[derive(Default)]
struct Repository {
    value: Mutex<Option<Settings>>,
    fail: bool,
}
impl SettingsRepository for Repository {
    fn load(&self) -> Result<Option<Settings>, String> {
        Ok(self.value.lock().unwrap().clone())
    }
    fn save(&self, value: &Settings) -> Result<(), String> {
        if self.fail {
            return Err("save failed".into());
        }
        *self.value.lock().unwrap() = Some(value.clone());
        Ok(())
    }
}

#[test]
fn enable_disable_and_reset_persist_without_touching_registration_for_other_settings() {
    let mut monitor = Monitor::new(Settings::default(), None);
    let registration = Registration::default();
    let repository = Repository::default();
    assert!(!monitor.settings.autostart);
    for enabled in [true, false, true] {
        let mut next = monitor.settings.clone();
        next.autostart = enabled;
        monitor
            .update_settings(next, monitor.settings.revision, &repository, &registration)
            .unwrap();
        assert_eq!(monitor.autostart.enabled, Some(enabled));
        assert_eq!(repository.load().unwrap().unwrap().autostart, enabled);
    }
    let mut next = monitor.settings.clone();
    next.theme = "dark".into();
    monitor
        .update_settings(next, monitor.settings.revision, &repository, &registration)
        .unwrap();
    assert_eq!(registration.calls.lock().unwrap().len(), 3);
    monitor
        .update_settings(
            Settings::default(),
            monitor.settings.revision,
            &repository,
            &registration,
        )
        .unwrap();
    assert!(!monitor.settings.autostart);
    assert!(registration.value.lock().unwrap().is_empty());
}

#[test]
fn conflicts_and_invalid_settings_have_no_system_effects() {
    let mut monitor = Monitor::new(Settings::default(), None);
    let registration = Registration::default();
    let repository = Repository::default();
    let mut next = Settings {
        autostart: true,
        ..Default::default()
    };
    assert!(
        monitor
            .update_settings(next.clone(), 1, &repository, &registration)
            .is_err()
    );
    next.interval_ms = 7;
    assert!(
        monitor
            .update_settings(next, 0, &repository, &registration)
            .is_err()
    );
    assert!(registration.calls.lock().unwrap().is_empty());
}

#[test]
fn save_or_registration_failure_restores_the_exact_previous_registration() {
    for (fail_save, fail_apply) in [(true, false), (false, true)] {
        for previous in ["", "old executable path and disabled trigger"] {
            let mut monitor = Monitor::new(Settings::default(), None);
            let registration = Registration {
                value: Mutex::new(previous.into()),
                fail_apply,
                ..Default::default()
            };
            let repository = Repository {
                fail: fail_save,
                ..Default::default()
            };
            let next = Settings {
                autostart: true,
                ..Default::default()
            };
            assert!(
                monitor
                    .update_settings(next, 0, &repository, &registration)
                    .is_err()
            );
            assert_eq!(*registration.value.lock().unwrap(), previous);
            assert_eq!(monitor.settings, Settings::default());
            assert!(repository.load().unwrap().is_none());
        }
    }
}

#[test]
fn rollback_failure_is_reported_with_actual_system_state() {
    let mut monitor = Monitor::new(Settings::default(), None);
    let registration = Registration {
        fail_restore: true,
        ..Default::default()
    };
    let repository = Repository {
        fail: true,
        ..Default::default()
    };
    let next = Settings {
        autostart: true,
        ..Default::default()
    };
    let error = monitor
        .update_settings(next, 0, &repository, &registration)
        .unwrap_err();
    assert!(error.contains("restore failed"));
    assert!(!monitor.settings.autostart);
    assert_eq!(monitor.autostart.enabled, Some(true));
}
