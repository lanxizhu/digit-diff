use std::{thread, time::Duration};

use tauri::{Manager, PhysicalSize, Size, Window};
use window_vibrancy::*;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
async fn animate_window_size(window: Window, width: f64, height: f64) -> Result<(), String> {
    let current_size = window.inner_size().unwrap();
    let start_width = current_size.width as f64;

    let start_height = current_size.height as f64;

    let steps = 30; // 动画步数
    let duration_ms = 300; // 总时长
    let step_duration = duration_ms / steps;

    for i in 1..=steps {
        let new_width = start_width + (width - start_width) * (i as f64 / steps as f64);
        let new_height = start_height + (height - start_height) * (i as f64 / steps as f64);

        let _ = window.set_size(Size::Physical(PhysicalSize {
            width: new_width.round() as u32,
            height: new_height.round() as u32,
        }));

        thread::sleep(Duration::from_millis(step_duration));
    }

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            {
                // apply_liquid_glass(&window, NSGlassEffectViewStyle::Clear, None, Some(26.0))
                //     .expect(
                //         "Unsupported platform! 'apply_liquid_glass' is only supported on macOS 26+",
                //     );
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                    .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
            }

            #[cfg(target_os = "windows")]
            apply_mica(&window, Some((18, 18, 18, 125)))
                .expect("Unsupported platform! 'apply_mica' is only supported on Windows");

            Ok(())
        })
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![animate_window_size])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
