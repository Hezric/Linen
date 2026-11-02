mod material {
    include!("../src/material.rs");

    pub(super) fn inspect(hwnd: HWND, dom: [f64; 4], scale: f64, settled: bool) -> String {
        use windows::UI::Composition::{CompositionBackdropBrush, CompositionGeometricClip};
        use windows::Win32::System::WinRT::Composition::IDesktopWindowTargetInterop;

        SURFACE.with(|slot| {
            let slot = slot.borrow();
            let Some((owner, Ok(surface))) = slot.as_ref() else {
                return "null".into();
            };
            assert_eq!(*owner, hwnd.0 as isize);
            let target = surface._target.cast::<IDesktopWindowTargetInterop>().unwrap();
            assert_eq!(unsafe { target.Hwnd().unwrap() }, hwnd);
            let root = surface._target.Root().unwrap().cast::<SpriteVisual>().unwrap();
            assert_eq!(root, surface.visual);
            assert!(root.Brush().unwrap().cast::<CompositionBackdropBrush>().is_ok());
            let clip = root.Clip().unwrap().cast::<CompositionGeometricClip>().unwrap();
            let shape = clip
                .Geometry()
                .unwrap()
                .cast::<CompositionRoundedRectangleGeometry>()
                .unwrap();
            assert_eq!(shape, surface.shape);
            let size = root.Size().unwrap();
            let clip_size = shape.Size().unwrap();
            assert_eq!([clip_size.X, clip_size.Y], [size.X, size.Y]);
            let radius = shape.CornerRadius().unwrap();
            assert_eq!([radius.X, radius.Y], [(8.0 * scale) as f32; 2]);
            let offset = root.Offset().unwrap();
            let visible = root.IsVisible().unwrap();
            assert!(visible, "material stays visible throughout the transition");
            let bounds = [offset.X, offset.Y, offset.X + size.X, offset.Y + size.Y];
            if settled {
                for (actual, expected) in bounds.into_iter().zip(dom) {
                    assert!((actual as f64 - expected * scale).abs() <= 1.0);
                }
            } else {
                assert!(bounds[0] as f64 >= dom[0] * scale - 1.0);
                assert!(bounds[1] as f64 >= dom[1] * scale - 1.0);
                assert!(bounds[2] as f64 <= dom[2] * scale + 1.0);
                assert!(bounds[3] as f64 <= dom[3] * scale + 1.0);
            }
            format!(
                "{{\"visible\":{visible},\"bounds\":{bounds:?},\"clipSize\":[{},{}],\"radius\":[{},{}]}}",
                clip_size.X, clip_size.Y, radius.X, radius.Y
            )
        })
    }
}
type Rect = [i32; 4];

use tauri::{AppHandle, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use windows::Win32::{
    Foundation::{COLORREF, HWND},
    Graphics::{
        Dwm::{DWMWA_CLOAKED, DwmGetWindowAttribute},
        Gdi::{CreateRectRgn, DeleteObject, HGDIOBJ, SetWindowRgn},
    },
    UI::WindowsAndMessaging::{
        GWL_EXSTYLE, GetWindowLongPtrW, IsWindowVisible, LWA_ALPHA, SetLayeredWindowAttributes,
        SetWindowLongPtrW, WS_EX_LAYERED,
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
) -> Result<Layout, String> {
    if !height.is_finite() || !(1.0..=575.0).contains(&height) {
        return Err("Invalid probe height".into());
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
            let active = mode == "material" && material::update(hwnd, panel, scale);
            Ok(active)
        })();
        let _ = send.try_send(result);
    })
    .map_err(|e| e.to_string())?;
    let backdrop = receive.recv().await.ok_or("CI host was closed")??;
    material::commit(&app).await?;
    Ok(Layout {
        limit: 575.0,
        backdrop,
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
async fn record_frame(
    app: AppHandle,
    window: WebviewWindow,
    stage: String,
    panel: [f64; 4],
) -> Result<(), String> {
    let position = window.inner_position().map_err(|e| e.to_string())?;
    let size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    let layered = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } & WS_EX_LAYERED.0 as isize != 0;
    let visible = unsafe { IsWindowVisible(hwnd).as_bool() };
    let mut cloaked = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            (&mut cloaked as *mut u32).cast(),
            size_of_val(&cloaked) as u32,
        )
        .map_err(|e| e.to_string())?;
    }
    if !layered {
        return Err("Probe lost its layered host style".into());
    }
    if !visible || cloaked != 0 {
        return Err(format!(
            "Probe is not presented: visible={visible} cloaked={cloaked}"
        ));
    }
    let (send, mut receive) = tauri::async_runtime::channel(1);
    let host = window.clone();
    let settled = stage == "initial" || stage.ends_with("-settled");
    app.run_on_main_thread(move || {
        let hwnd = HWND(host.hwnd().unwrap().0);
        let surface = material::inspect(hwnd, panel, scale, settled);
        let effects = windows::UI::ViewManagement::UISettings::new()
            .and_then(|settings| settings.AdvancedEffectsEnabled())
            .unwrap_or(false);
        let _ = send.try_send((surface, effects));
    })
    .map_err(|e| e.to_string())?;
    let (surface, effects) = receive
        .recv()
        .await
        .ok_or("API inspection was interrupted")?;
    if std::env::args().nth(3).unwrap() == "material" && surface == "null" {
        return Err("The real WebView host has no attached backdrop".into());
    }
    tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
        let marker = std::path::PathBuf::from(std::env::args().nth(1).unwrap());
        let pending = marker.with_extension("pending");
        let ack = marker.with_extension("ack");
        std::fs::write(&pending, format!(
            "{{\"stage\":\"{stage}\",\"layered\":{layered},\"visible\":{visible},\"cloaked\":{cloaked},\"advancedEffects\":{effects},\"surface\":{surface},\"panel\":{panel:?},\"x\":{},\"y\":{},\"width\":{},\"height\":{},\"scale\":{scale}}}",
            position.x, position.y, size.width, size.height
        )).map_err(|e| e.to_string())?;
        std::fs::rename(pending, &marker).map_err(|e| e.to_string())?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !ack.exists() {
            if std::time::Instant::now() > deadline {
                return Err("CI frame acknowledgement timed out".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        std::fs::remove_file(ack).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
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
        .invoke_handler(tauri::generate_handler![resize_flyout, show, record_frame, finish])
        .setup(|app| {
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
