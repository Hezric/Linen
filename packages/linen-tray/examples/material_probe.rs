#[path = "../src/material.rs"]
mod material;
type Rect = [i32; 4];

use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::{
    Foundation::{COLORREF, HWND},
    Graphics::{
        Dwm::DwmFlush,
        Gdi::{CreateRectRgn, DeleteObject, HGDIOBJ, SetWindowRgn},
    },
    UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, LWA_ALPHA, SetLayeredWindowAttributes, SetWindowLongPtrW,
        WS_EX_LAYERED,
    },
};

#[derive(serde::Serialize)]
struct Layout {
    limit: f64,
    backdrop: bool,
}

#[tauri::command]
async fn resize_flyout(
    app: AppHandle,
    window: WebviewWindow,
    height: f64,
    reserve: Option<f64>,
    settled: Option<bool>,
) -> Result<Layout, String> {
    if !height.is_finite() || !(1.0..=575.0).contains(&height) {
        return Err("Invalid probe height".into());
    }
    if reserve.is_some() {
        material::suspend(&app).await?;
    }
    let (send, mut receive) = tauri::async_runtime::channel(1);
    app.run_on_main_thread(move || {
        let result = (|| -> Result<bool, String> {
            let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
            let scale = window.scale_factor().map_err(|e| e.to_string())?;
            let size = window.inner_size().map_err(|e| e.to_string())?;
            let px = |dip: f64| (dip * scale).round() as i32;
            let panel = [
                px(32.0),
                size.height as i32 - px(13.0) - (height * scale).ceil() as i32,
                size.width as i32 - px(13.0),
                size.height as i32 - px(13.0),
            ];
            let mode = std::env::args().nth(3).unwrap();
            let active = mode == "material"
                && material::update(hwnd, panel, scale, settled.unwrap_or(false));
            Ok(active)
        })();
        let _ = send.try_send(result);
    })
    .map_err(|e| e.to_string())?;
    Ok(Layout {
        limit: 575.0,
        backdrop: receive.recv().await.ok_or("CI host was closed")??,
    })
}

#[tauri::command]
fn show(window: WebviewWindow) -> Result<(), String> {
    window.show().map_err(|e| e.to_string())?;
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA).map_err(|e| e.to_string())?;
        let region = CreateRectRgn(
            0,
            (270.0 * scale).round() as i32,
            size.width as i32,
            size.height as i32,
        );
        if region.0.is_null() {
            return Err("CI host region allocation failed".into());
        }
        if SetWindowRgn(hwnd, Some(region), true) == 0 {
            let _ = DeleteObject(HGDIOBJ(region.0));
            return Err("CI host region failed".into());
        }
    }
    Ok(())
}

#[tauri::command]
async fn capture(window: WebviewWindow, stage: String, panel: [f64; 4]) -> Result<(), String> {
    let background = stage == "background";
    if background {
        window.hide().map_err(|e| e.to_string())?;
    }
    let position = window.inner_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        if background {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        unsafe { DwmFlush().map_err(|e| e.to_string())? };
        let marker = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
        let pending = marker.with_extension("pending");
        let ack = marker.with_extension("ack");
        std::fs::write(&pending, format!(
            "{{\"stage\":\"{stage}\",\"panel\":{panel:?},\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"scale\":{scale}}}",
            position.x, position.y, size.width, size.height
        )).map_err(|e| e.to_string())?;
        std::fs::rename(pending, &marker).map_err(|e| e.to_string())?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !ack.exists() {
            if std::time::Instant::now() > deadline {
                return Err("CI screenshot acknowledgement timed out".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::fs::remove_file(ack).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    if background {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn finish(app: AppHandle, error: Option<String>) {
    if let Some(error) = error {
        eprintln!("Material probe: {error}");
        app.exit(1);
    } else {
        app.exit(0);
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![resize_flyout, show, capture, finish])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "pattern", WebviewUrl::App("index.html".into()))
                .title("Linen CI background")
                .position(0.0, 0.0)
                .inner_size(900.0, 700.0)
                .decorations(false)
                .build()?;
            let script = format!(
                "window.probeStyle={:?};window.probeSource={:?};window.probeMode={:?};window.probeTheme={:?};{}",
                include_str!("../../linen-ui/style.css"),
                include_str!("../../linen-ui/index.js"),
                std::env::args().nth(3).unwrap(),
                std::env::args().nth(2).unwrap(),
                include_str!("../tests/material-probe/probe.js")
            );
            let host = WebviewWindowBuilder::new(app, "host", WebviewUrl::App("index.html".into()))
                .title("Linen CI material host")
                .position(100.0, 50.0)
                .inner_size(405.0, 620.0)
                .decorations(false)
                .shadow(false)
                .transparent(true)
                .background_color(tauri::window::Color(0, 0, 0, 0))
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(false)
                .initialization_script(script)
                .build()?;
            host.on_window_event(|event| {
                if matches!(event, tauri::WindowEvent::Destroyed) {
                    material::remove();
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!("tests/material-probe/tauri.conf.json"))
        .expect("material CI host failed");
}
