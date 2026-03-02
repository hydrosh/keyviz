use std::sync::Mutex;

use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindowBuilder};

use crate::app::state::AppState;
use crate::app::window::config_window;

const OVERLAY_WINDOW_PREFIX: &str = "overlay";

#[tauri::command]
pub fn log(message: String) {
    println!("[LOG] {}", message);
}

#[tauri::command]
pub fn set_toggle_shortcut(app: tauri::AppHandle, shortcut: Vec<String>) {
    let state = app.state::<Mutex<AppState>>();
    let mut app_state = state.lock().unwrap();
    app_state.toggle_shortcut = shortcut;
}

#[tauri::command]
pub fn set_main_window_monitor(app: tauri::AppHandle, monitor_name: String) {
    let state = app.state::<Mutex<AppState>>();

    if monitor_name == "all" {
        // Multi-monitor mode: show visualisation on every monitor
        {
            let mut app_state = state.lock().unwrap();
            if app_state.monitor_name == Some("all".to_string()) {
                return;
            }
            // Use (0, 0) offset so raw screen coords are emitted; each
            // overlay window subtracts its own position on the frontend.
            app_state.monitor_name = Some("all".to_string());
            app_state.monitor_scale = 1.0;
            app_state.monitor_position = (0, 0);
        }
        setup_all_monitors(&app);
        return;
    }

    // Single-monitor mode – tear down any extra overlay windows first.
    close_overlay_windows(&app);

    let mut app_state = state.lock().unwrap();
    if app_state.monitor_name == Some(monitor_name.clone()) {
        return;
    }

    if let Some(window) = app.get_webview_window("main") {
        let monitors = window.available_monitors().unwrap_or_default();
        let target_monitor = monitors.iter().find(|m| m.name() == Some(&monitor_name));

        if let Some(monitor) = target_monitor {
            let position = monitor.position();
            let size = monitor.size();
            let scale = monitor.scale_factor();

            // Update AppState
            app_state.monitor_name = Some(monitor_name.clone());
            app_state.monitor_scale = scale;
            app_state.monitor_position = (position.x, position.y);

            // Update window
            window
                .set_position(PhysicalPosition {
                    x: position.x,
                    y: position.y,
                })
                .unwrap_or(());
            window
                .set_size(PhysicalSize {
                    width: size.width,
                    height: size.height,
                })
                .unwrap_or(());
        }
    }
}

/// Close all dynamically-created overlay windows.
fn close_overlay_windows(app: &tauri::AppHandle) {
    for (label, window) in app.webview_windows() {
        if label.starts_with(OVERLAY_WINDOW_PREFIX) {
            let _ = window.close();
        }
    }
}

/// Position the main window on the first monitor and create transparent
/// overlay windows for every additional monitor.
fn setup_all_monitors(app: &tauri::AppHandle) {
    let Some(main_window) = app.get_webview_window("main") else {
        return;
    };

    let monitors = main_window.available_monitors().unwrap_or_default();
    if monitors.is_empty() {
        return;
    }

    // Close any leftover overlay windows before (re-)creating them.
    close_overlay_windows(app);

    let webview_url = tauri::WebviewUrl::App("index.html".into());

    for (i, monitor) in monitors.iter().enumerate() {
        let position = monitor.position();
        let size = monitor.size();

        if i == 0 {
            // Move the existing main window to the first monitor.
            // config_window was already called for this window on startup,
            // so only a reposition/resize is needed here.
            let _ = main_window.set_position(PhysicalPosition {
                x: position.x,
                y: position.y,
            });
            let _ = main_window.set_size(PhysicalSize {
                width: size.width,
                height: size.height,
            });
        } else {
            // Create a new transparent overlay window for each additional monitor.
            let label = format!("{}-{}", OVERLAY_WINDOW_PREFIX, i);
            if let Ok(overlay) = WebviewWindowBuilder::new(app, &label, webview_url.clone())
                .always_on_top(true)
                .skip_taskbar(true)
                .decorations(false)
                .transparent(true)
                .focused(false)
                .visible(false)
                .build()
            {
                // Set physical position/size after creation to avoid DPI scaling issues.
                let _ = overlay.set_position(PhysicalPosition {
                    x: position.x,
                    y: position.y,
                });
                let _ = overlay.set_size(PhysicalSize {
                    width: size.width,
                    height: size.height,
                });
                config_window(&overlay);
            }
        }
    }
}
