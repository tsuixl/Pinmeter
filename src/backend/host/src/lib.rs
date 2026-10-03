mod bridges;
mod commands;
pub mod contracts;
mod desktop;
mod diagnostics;
mod exit;
mod hardware;
mod ip;
mod network_control;
pub mod presenters;
mod runtime;
mod settings;
mod startup_window;
mod updates;

pub fn run() {
    use tauri::Manager;
    let mut context = tauri::generate_context!();
    for window in &mut context.config_mut().app.windows {
        if window.label == "main" {
            window.decorations = cfg!(target_os = "macos");
            #[cfg(target_os = "macos")]
            {
                window.title_bar_style = tauri::TitleBarStyle::Overlay;
                window.hidden_title = true;
                window.traffic_light_position =
                    Some(tauri::utils::config::LogicalPosition { x: 12.0, y: 16.0 });
            }
        }
    }
    let app = tauri::Builder::default()
        .manage(diagnostics::Diagnostics::default())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(runtime) = app.try_state::<std::sync::Arc<runtime::Runtime>>() {
                runtime.startup_visibility.lock().unwrap().reveal();
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = bridges::activate(&window);
            }
        }))
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(
                    tauri_plugin_window_state::StateFlags::POSITION
                        | tauri_plugin_window_state::StateFlags::SIZE
                        | tauri_plugin_window_state::StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .setup(|app| {
            let updates = updates::Updates::new(app.handle())?;
            app.manage(updates.clone());
            updates.start(app.handle().clone());
            if let Some(window) = app.get_webview_window("main") {
                let _ = bridges::ensure_visible(&window, false);
            }
            let repository = std::sync::Arc::new(pinmeter_platform::settings::FileSettings::new(
                app.path().app_config_dir()?.join("settings.json"),
            ));
            let runtime = runtime::Runtime::new(repository);
            if let Some(window) = app.get_webview_window("main") {
                let theme = runtime.inner.lock().unwrap().monitor.settings.theme.clone();
                bridges::apply_theme(&window, &theme)?;
            }
            app.manage(runtime.clone());
            runtime.start(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "main"
                && let tauri::WindowEvent::CloseRequested { api, .. } = event
                && let Some(runtime) = window
                    .app_handle()
                    .try_state::<std::sync::Arc<runtime::Runtime>>()
                && !runtime.is_stopped()
            {
                api.prevent_close();
                runtime.inner().request_close(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            diagnostics::prepare_diagnostics,
            diagnostics::export_diagnostics,
            startup_window::complete_window_startup,
            updates::get_update_state,
            updates::check_app_update,
            updates::download_app_update,
            updates::install_app_update,
            updates::update_update_preference,
            updates::open_update_downloads,
            commands::get_hardware_info,
            commands::temperature_driver_missing,
            commands::install_temperature_driver,
            commands::get_app_exit_state,
            commands::resolve_app_close,
            commands::minimize_to_tray,
            commands::request_app_exit,
            commands::cancel_app_exit,
            commands::release_all_network_control,
            commands::get_monitor_state,
            commands::set_ip_view_active,
            commands::refresh_ip,
            commands::refresh_ip_checks,
            commands::set_app_network_monitoring,
            commands::change_network_control,
            commands::subscribe_monitor,
            commands::unsubscribe_monitor,
            commands::ack_monitor_batch,
            commands::get_history,
            commands::update_settings,
            commands::perform_desktop_action,
            commands::get_desktop_platform
        ])
        .build(context)
        .expect("Pinmeter desktop runtime failed");
    app.run(|app, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = &event {
            let runtime = app.state::<std::sync::Arc<runtime::Runtime>>();
            if !runtime.is_stopped() {
                api.prevent_exit();
                runtime.inner().begin_exit(app, false);
            }
        }
        if matches!(event, tauri::RunEvent::Exit) {
            app.state::<std::sync::Arc<runtime::Runtime>>().stop();
        }
    });
}
