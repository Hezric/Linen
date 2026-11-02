#![windows_subsystem = "windows"]

#[path = "../src/material.rs"]
mod material;
type Rect = [i32; 4];

use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::{
    Foundation::{COLORREF, HWND},
    Graphics::{Dwm::DwmFlush, Gdi::{CreateRectRgn, SetWindowRgn}},
    UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, LWA_ALPHA, SetLayeredWindowAttributes,
        SetWindowLongPtrW, WS_EX_LAYERED,
    },
};

#[tauri::command]
async fn ready(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    material::suspend(&app).await?;
    let (send, mut receive) = tauri::async_runtime::channel(1);
    let host = window.clone();
    app.run_on_main_thread(move || {
        let result = (|| -> Result<bool, String> {
            let hwnd = HWND(host.hwnd().map_err(|e| e.to_string())?.0);
            let scale = host.scale_factor().map_err(|e| e.to_string())?;
            let size = host.inner_size().map_err(|e| e.to_string())?;
            let px = |dip: f64| (dip * scale).round() as i32;
            let panel = [px(32.0), size.height as i32 - px(313.0), size.width as i32 - px(13.0), size.height as i32 - px(13.0)];
            let mode = std::env::args().nth(3).unwrap();
            let active = mode == "material" && material::update(hwnd, panel, scale, true);
            host.show().map_err(|e| e.to_string())?;
            unsafe {
                let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
                SetLayeredWindowAttributes(hwnd, COLORREF(0), 255, LWA_ALPHA).map_err(|e| e.to_string())?;
                let region = CreateRectRgn(0, px(270.0), size.width as i32, size.height as i32);
                if SetWindowRgn(hwnd, Some(region), true) == 0 {
                    return Err("CI host region failed".into());
                }
            }
            Ok(active)
        })();
        let _ = send.try_send(result);
    })
    .map_err(|e| e.to_string())?;
    let active = receive.recv().await.ok_or("CI host was closed")??;
    window.eval(&format!(
        "document.querySelector('.flyout').dataset.backdrop = '{}'; requestAnimationFrame(()=>requestAnimationFrame(()=>window.__TAURI_INTERNALS__.invoke('painted',{{active:{active}}})));",
        if std::env::args().nth(3).unwrap() == "fallback" { "false" } else { "true" }
    )).map_err(|e| e.to_string())
}

#[tauri::command]
fn painted(window: WebviewWindow, active: bool) -> Result<(), String> {
    let position = window.inner_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    unsafe { DwmFlush().map_err(|e| e.to_string())?; }
    std::fs::write(std::env::args().nth(1).unwrap(), format!(
        "{{\"active\":{active},\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"scale\":{scale}}}",
        position.x, position.y, size.width, size.height
    )).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![ready, painted])
        .setup(|app| {
            WebviewWindowBuilder::new(app, "pattern", WebviewUrl::App("index.html".into()))
                .title("Linen CI background")
                .position(0.0, 0.0)
                .inner_size(900.0, 700.0)
                .decorations(false)
                .build()?;
            let dark = std::env::args().nth(2).unwrap() == "dark";
            let script = format!(
                "addEventListener('DOMContentLoaded',()=>{{const style=document.createElement('style');style.textContent={:?};document.head.append(style);document.documentElement.dataset.theme='{}';document.body.className='native';document.body.innerHTML='<main id=app><section class=flyout style=\"height:300px\"></section></main>';window.__TAURI_INTERNALS__.invoke('ready');}});",
                include_str!("../../linen-ui/style.css"), if dark { "dark" } else { "light" }
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
