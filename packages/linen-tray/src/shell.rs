use crate::{Appearance, Edge, Lifecycle, Phase, Placement, Rect, icon_index, placement};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use windows::{
    Win32::{
        Foundation::{COLORREF, HWND, LPARAM, POINT, RECT},
        Graphics::Dwm::*,
        UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
    },
    core::w,
};

pub type Artwork = &'static [(&'static str, [&'static [u8]; 5])];

pub struct Config {
    pub id: &'static str,
    pub title: &'static str,
    pub project: &'static str,
    pub artwork: Artwork,
    pub initial: &'static str,
}

struct Shell {
    config: Config,
    pause: MenuItem<tauri::Wry>,
    open: MenuItem<tauri::Wry>,
    quit: MenuItem<tauri::Wry>,
    clock: Instant,
    state: Mutex<Lifecycle>,
    geometry: Mutex<Option<Geometry>>,
    icon: Mutex<(String, bool, bool)>,
}

pub fn single_instance() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_single_instance::init(|app, _, _| open_flyout(app))
}

pub fn install(
    app: &AppHandle,
    config: Config,
    on_pause: impl Fn(&AppHandle) + Send + Sync + 'static,
) -> tauri::Result<()> {
    let zh = unsafe { windows::Win32::Globalization::GetUserDefaultUILanguage() & 0x3ff == 4 };
    let open = MenuItem::with_id(
        app,
        "open",
        format!("{} {}", if zh { "打开" } else { "Open" }, config.title),
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        "pause",
        if zh { "暂停 1 小时" } else { "Pause 1 hr" },
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(
        app,
        "quit",
        if zh { "退出" } else { "Quit" },
        true,
        None::<&str>,
    )?;
    let menu = Menu::with_items(app, &[&open, &pause, &quit])?;
    let image = icon_data(config.artwork, config.initial, false, false, 1.0)
        .ok_or_else(|| tauri::Error::AssetNotFound(config.initial.into()))?;
    TrayIconBuilder::with_id(config.id)
        .icon(Image::from_bytes(image)?)
        .tooltip(config.title)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "open" => open_flyout(app),
            "pause" => on_pause(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                toggle_flyout(tray.app_handle());
            }
        })
        .build(app)?;
    let initial = config.initial.to_owned();
    app.manage(Arc::new(Shell {
        config,
        pause,
        open,
        quit,
        clock: Instant::now(),
        state: Mutex::new(Lifecycle::default()),
        geometry: Mutex::new(None),
        icon: Mutex::new((initial, false, false)),
    }));
    Ok(())
}

fn shell(app: &AppHandle) -> Arc<Shell> {
    app.state::<Arc<Shell>>().inner().clone()
}

pub fn icon_data(
    artwork: Artwork,
    state: &str,
    light: bool,
    small: bool,
    scale: f64,
) -> Option<&'static [u8]> {
    let name = format!(
        "{}{}-{}",
        if small { "small-" } else { "" },
        state,
        if light { "light" } else { "dark" }
    );
    artwork
        .iter()
        .find(|(key, _)| *key == name)
        .map(|(_, sizes)| sizes[icon_index(scale)])
}

fn refresh_icon(app: &AppHandle, frame: usize) {
    let shared = shell(app);
    let (name, light, small) = shared.icon.lock().unwrap().clone();
    let name = if name == "reconnecting" {
        let frame = if animations_enabled() { frame % 3 } else { 0 };
        format!("reconnecting-{frame}")
    } else {
        name
    };
    let scale = app
        .tray_by_id(shared.config.id)
        .and_then(|t| t.rect().ok().flatten())
        .and_then(|r| {
            let p = r.position.to_physical::<i32>(1.0);
            app.monitor_from_point(p.x as f64, p.y as f64)
                .ok()
                .flatten()
        })
        .map_or(1.0, |m| m.scale_factor());
    if let (Some(tray), Some(data)) = (
        app.tray_by_id(shared.config.id),
        icon_data(shared.config.artwork, &name, light, small, scale),
    ) {
        match Image::from_bytes(data).and_then(|image| tray.set_icon(Some(image))) {
            Ok(()) => {}
            Err(e) => eprintln!("Tray icon: {e}"),
        }
    }
}

pub fn update_tray(
    app: &AppHandle,
    appearance: &Appearance,
    state: &str,
    paused: bool,
    minutes: u16,
    tooltip: &str,
) {
    let shared = shell(app);
    let animate = {
        let mut icon = shared.icon.lock().unwrap();
        let animate = state == "reconnecting" && icon.0 != "reconnecting";
        *icon = (
            state.into(),
            appearance.taskbar_light,
            appearance.taskbar_small,
        );
        animate
    };
    let zh = appearance.language == "zh";
    let duration = if minutes.is_multiple_of(60) {
        if zh {
            format!("暂停 {} 小时", minutes / 60)
        } else {
            format!("Pause {} hr", minutes / 60)
        }
    } else if zh {
        format!("暂停 {minutes} 分钟")
    } else {
        format!("Pause {minutes} min")
    };
    let _ = shared.pause.set_text(if paused {
        if zh { "立即恢复" } else { "Resume now" }
    } else {
        &duration
    });
    let _ = shared.open.set_text(format!(
        "{} {}",
        if zh { "打开" } else { "Open" },
        shared.config.title
    ));
    let _ = shared.quit.set_text(if zh { "退出" } else { "Quit" });
    if let Some(window) = app.get_webview_window("flyout")
        && let Ok(handle) = window.hwnd()
    {
        let _ = window.set_background_color(Some(background(appearance.dark)));
        let dark = i32::from(appearance.dark);
        unsafe {
            let _ = DwmSetWindowAttribute(
                HWND(handle.0),
                DWMWA_USE_IMMERSIVE_DARK_MODE,
                (&dark as *const i32).cast(),
                size_of_val(&dark) as u32,
            );
        }
    }
    if let Some(tray) = app.tray_by_id(shared.config.id) {
        let _ = tray.set_tooltip(Some(format!("{} — {tooltip}", shared.config.title)));
    }
    refresh_icon(app, (shared.clock.elapsed().as_millis() / 180) as usize);
    if animate {
        let app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            loop {
                std::thread::sleep(Duration::from_millis(180));
                let shared = shell(&app);
                if shared.icon.lock().unwrap().0 != "reconnecting" {
                    break;
                }
                let handle = app.clone();
                let _ = app.run_on_main_thread(move || {
                    refresh_icon(
                        &handle,
                        (shell(&handle).clock.elapsed().as_millis() / 180) as usize,
                    )
                });
            }
        });
    }
}

pub fn toggle_flyout(app: &AppHandle) {
    let shared = shell(app);
    let mut state = shared.state.lock().unwrap();
    let previous = state.phase;
    let phase = state.toggle(shared.clock.elapsed().as_millis() as u64);
    drop(state);
    if phase == previous {
        return;
    }
    match phase {
        Phase::Opening => create_flyout(app),
        Phase::Closing => close_window(app, previous == Phase::Open),
        _ => {}
    }
}

pub fn open_flyout(app: &AppHandle) {
    let shared = shell(app);
    let mut state = shared.state.lock().unwrap();
    if matches!(state.phase, Phase::Closing | Phase::Opening) {
        return;
    }
    if state.phase == Phase::Open {
        drop(state);
        if let Some(w) = app.get_webview_window("flyout") {
            let _ = w.set_focus();
        }
        return;
    }
    state.phase = Phase::Opening;
    drop(state);
    create_flyout(app);
}

fn background(dark: bool) -> tauri::window::Color {
    let value = if dark { 42 } else { 247 };
    tauri::window::Color(value, value, value, 255)
}

fn create_flyout(app: &AppHandle) {
    let app = app.clone();
    // WebView2 creation must yield the synchronous Windows tray event handler.
    tauri::async_runtime::spawn_blocking(move || {
        let shared = shell(&app);
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            *shared.geometry.lock().unwrap() = Some(Geometry::read(&app)?);
            let dark = crate::registry_light(w!("AppsUseLightTheme")) == 0;
            let window =
                WebviewWindowBuilder::new(&app, "flyout", WebviewUrl::App("index.html".into()))
                    .title(shared.config.title)
                    .inner_size(360.0, 400.0)
                    .decorations(false)
                    .transparent(false)
                    .theme(Some(if dark {
                        tauri::Theme::Dark
                    } else {
                        tauri::Theme::Light
                    }))
                    .background_color(background(dark))
                    .shadow(true)
                    .resizable(false)
                    .skip_taskbar(true)
                    .always_on_top(true)
                    .visible(false)
                    .build()?;
            style_window(&window)?;
            let handle = window.clone();
            window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                }
                if matches!(
                    event,
                    tauri::WindowEvent::Focused(false) | tauri::WindowEvent::CloseRequested { .. }
                ) {
                    let shared = shell(handle.app_handle());
                    let open = shared.state.lock().unwrap().phase == Phase::Open;
                    if open {
                        dismiss(
                            handle.app_handle(),
                            matches!(event, tauri::WindowEvent::Focused(false)),
                        );
                    }
                }
            });
            if shared.state.lock().unwrap().phase == Phase::Closing {
                window.destroy()?;
                shared.state.lock().unwrap().phase = Phase::Closed;
            }
            Ok(())
        })();
        if let Err(error) = result {
            shared.state.lock().unwrap().phase = Phase::Closed;
            eprintln!("Flyout: {error}");
        }
    });
}

fn style_window(window: &WebviewWindow) -> Result<(), Box<dyn std::error::Error>> {
    use windows::Win32::UI::Controls::MARGINS;
    let hwnd = HWND(window.hwnd()?.0);
    unsafe {
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(
            hwnd,
            GWL_EXSTYLE,
            (ex | WS_EX_TOOLWINDOW.0 as isize) & !(WS_EX_APPWINDOW.0 as isize),
        );
        let policy = DWMNCRP_ENABLED;
        let corners = DWMWCP_ROUND;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY,
            (&policy as *const DWMNCRENDERINGPOLICY).cast(),
            size_of_val(&policy) as u32,
        )?;
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&corners as *const DWM_WINDOW_CORNER_PREFERENCE).cast(),
            size_of_val(&corners) as u32,
        )?;
        DwmExtendFrameIntoClientArea(
            hwnd,
            &MARGINS {
                cxLeftWidth: 1,
                cxRightWidth: 1,
                cyTopHeight: 1,
                cyBottomHeight: 1,
            },
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Geometry {
    work: Rect,
    bar: Rect,
    icon: Option<Rect>,
    edge: Edge,
    rtl: bool,
    scale: f64,
}
unsafe extern "system" fn collect_taskbars(hwnd: HWND, data: LPARAM) -> windows::core::BOOL {
    let mut class = [0u16; 64];
    let length = unsafe { GetClassNameW(hwnd, &mut class) };
    let name = String::from_utf16_lossy(&class[..length as usize]);
    if matches!(name.as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok() {
            let bars = unsafe { &mut *(data.0 as *mut Vec<(HWND, Rect)>) };
            bars.push((hwnd, [rect.left, rect.top, rect.right, rect.bottom]));
        }
    }
    windows::core::BOOL(1)
}
impl Geometry {
    fn read(app: &AppHandle) -> tauri::Result<Self> {
        let shared = shell(app);
        let anchor = app
            .tray_by_id(shared.config.id)
            .and_then(|tray| tray.rect().ok().flatten());
        let monitor = if let Some(r) = &anchor {
            let p = r.position.to_physical::<i32>(1.0);
            app.monitor_from_point(p.x as f64, p.y as f64)?
        } else {
            app.primary_monitor()?
        }
        .ok_or_else(|| tauri::Error::AssetNotFound("No monitor".into()))?;
        let scale = monitor.scale_factor();
        let w = monitor.work_area();
        let p = monitor.position();
        let s = monitor.size();
        let work = [
            w.position.x,
            w.position.y,
            w.position.x + w.size.width as i32,
            w.position.y + w.size.height as i32,
        ];
        let full = [p.x, p.y, p.x + s.width as i32, p.y + s.height as i32];
        let differences = [
            work[1] - full[1],
            full[2] - work[2],
            full[3] - work[3],
            work[0] - full[0],
        ];
        let mut edge = if differences.iter().all(|v| *v == 0) {
            Edge::Bottom
        } else {
            match differences
                .iter()
                .enumerate()
                .max_by_key(|(_, v)| *v)
                .map(|(i, _)| i)
                .unwrap_or(2)
            {
                0 => Edge::Top,
                1 => Edge::Right,
                3 => Edge::Left,
                _ => Edge::Bottom,
            }
        };
        let mut bar = match edge {
            Edge::Top => [full[0], full[1], full[2], work[1]],
            Edge::Right => [work[2], full[1], full[2], full[3]],
            Edge::Bottom => [full[0], work[3], full[2], full[3]],
            Edge::Left => [full[0], full[1], work[0], full[3]],
        };
        let mut bars: Vec<(HWND, Rect)> = Vec::new();
        let _ = unsafe {
            EnumWindows(
                Some(collect_taskbars),
                LPARAM((&mut bars as *mut Vec<(HWND, Rect)>) as isize),
            )
        };
        let found = bars
            .into_iter()
            .find(|(_, r)| r[0] < full[2] && r[2] > full[0] && r[1] < full[3] && r[3] > full[1]);
        if let Some((_, rect)) = found {
            bar = rect;
            edge = if rect[2] - rect[0] > rect[3] - rect[1] {
                if (rect[1] + rect[3]) / 2 < (full[1] + full[3]) / 2 {
                    Edge::Top
                } else {
                    Edge::Bottom
                }
            } else if (rect[0] + rect[2]) / 2 < (full[0] + full[2]) / 2 {
                Edge::Left
            } else {
                Edge::Right
            };
        }
        let rtl = found.is_some_and(
            |(h, _)| unsafe { GetWindowLongPtrW(h, GWL_EXSTYLE) } & WS_EX_LAYOUTRTL.0 as isize != 0,
        );
        let icon = anchor.map(|r| {
            let p = r.position.to_physical::<i32>(scale);
            let s = r.size.to_physical::<u32>(scale);
            [p.x, p.y, p.x + s.width as i32, p.y + s.height as i32]
        });
        Ok(Self {
            work,
            bar,
            icon,
            edge,
            rtl,
            scale,
        })
    }
    fn place(self, height: f64) -> Placement {
        placement(
            self.work, self.bar, self.icon, self.edge, self.rtl, self.scale, height,
        )
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FlyoutLayout {
    limit: f64,
}

#[tauri::command]
pub async fn resize_flyout(app: AppHandle, height: f64) -> Result<FlyoutLayout, String> {
    if !height.is_finite() || height <= 0.0 {
        return Err("Invalid flyout height".into());
    }
    let shared = shell(&app);
    let geometry = shared
        .geometry
        .lock()
        .unwrap()
        .ok_or("Flyout geometry is unavailable")?;
    let window = app
        .get_webview_window("flyout")
        .ok_or("Flyout was closed")?;
    let placement = geometry.place(height);
    let [x, y, right, bottom] = placement.bounds;
    window
        .set_size(tauri::PhysicalSize::new(
            (right - x) as u32,
            (bottom - y) as u32,
        ))
        .map_err(|e| e.to_string())?;
    window
        .set_position(tauri::PhysicalPosition::new(x, y))
        .map_err(|e| e.to_string())?;
    Ok(FlyoutLayout {
        limit: (geometry.work[3] - geometry.work[1]) as f64 / geometry.scale - 24.0,
    })
}

pub fn animations_enabled() -> bool {
    let mut enabled = windows::core::BOOL(0);
    unsafe {
        SystemParametersInfoW(
            SPI_GETCLIENTAREAANIMATION,
            0,
            Some((&mut enabled as *mut windows::core::BOOL).cast()),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
        .is_ok()
            && enabled.as_bool()
    }
}

fn animate(window: &WebviewWindow, show: bool) -> Result<(), String> {
    let shared = shell(window.app_handle());
    let geometry = shared
        .geometry
        .lock()
        .unwrap()
        .ok_or("Flyout geometry is unavailable")?;
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    let mut bounds = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut bounds).map_err(|e| e.to_string())?;
    }
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } & !(WS_EX_LAYERED.0 as isize);
    unsafe {
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
    }
    let slide = geometry.place(400.0).slide;
    if show {
        bounds.left -= (slide[0] as f64 * geometry.scale).round() as i32;
        bounds.top -= (slide[1] as f64 * geometry.scale).round() as i32;
    }
    let start = Instant::now();
    loop {
        if show && shared.state.lock().unwrap().phase != Phase::Opening {
            return Ok(());
        }
        let t = if animations_enabled() {
            (start.elapsed().as_secs_f64() / 0.2).min(1.0)
        } else {
            1.0
        };
        let eased = 1.0 - (1.0 - t).powi(3);
        let distance = if show { 1.0 - eased } else { eased };
        unsafe {
            SetWindowPos(
                hwnd,
                None,
                bounds.left + (slide[0] as f64 * geometry.scale * distance).round() as i32,
                bounds.top + (slide[1] as f64 * geometry.scale * distance).round() as i32,
                0,
                0,
                SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
            )
            .map_err(|e| e.to_string())?;
        }
        unsafe {
            SetLayeredWindowAttributes(
                hwnd,
                COLORREF(0),
                (255.0 * (1.0 - distance)).round() as u8,
                LWA_ALPHA,
            )
            .map_err(|e| e.to_string())?;
        }
        if t >= 1.0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(8));
    }
    unsafe {
        if !show {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style);
    }
    Ok(())
}

#[tauri::command]
pub async fn prepare_flyout(app: AppHandle, height: f64) -> Result<(), String> {
    resize_flyout(app.clone(), height).await?;
    let window = app
        .get_webview_window("flyout")
        .ok_or("Flyout was closed")?;
    let hwnd = HWND(window.hwnd().map_err(|e| e.to_string())?.0);
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as isize);
        SetLayeredWindowAttributes(hwnd, COLORREF(0), 0, LWA_ALPHA).map_err(|e| e.to_string())?;
    }
    let geometry = shell(&app)
        .geometry
        .lock()
        .unwrap()
        .ok_or("Flyout geometry is unavailable")?;
    let slide = geometry.place(height).slide;
    let mut bounds = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut bounds).map_err(|e| e.to_string())?;
        // The first native frame must start at the same position as the entry animation.
        SetWindowPos(
            hwnd,
            None,
            bounds.left + (slide[0] as f64 * geometry.scale).round() as i32,
            bounds.top + (slide[1] as f64 * geometry.scale).round() as i32,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOZORDER,
        )
        .map_err(|e| e.to_string())?;
    }
    // Tauri must track visibility before its later focus request can activate the window.
    window.show().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn present_flyout(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let shared = shell(&app);
        if shared.state.lock().unwrap().phase != Phase::Opening {
            return Ok(());
        }
        let window = app
            .get_webview_window("flyout")
            .ok_or("Flyout was closed")?;
        let watch_window = window.clone();
        let watch_app = app.clone();
        // Desktop/taskbar clicks can leave focus unchanged, including during entry.
        tauri::async_runtime::spawn_blocking(move || {
            let shared = shell(&watch_app);
            let Ok(handle) = watch_window.hwnd() else {
                return;
            };
            let hwnd = HWND(handle.0);
            let pressed = || unsafe {
                GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0
                    || GetAsyncKeyState(VK_RBUTTON.0 as i32) < 0
            };
            let mut down = pressed();
            while matches!(
                shared.state.lock().unwrap().phase,
                Phase::Opening | Phase::Open
            ) {
                let current = pressed();
                if current && !down {
                    let mut point = POINT::default();
                    let mut bounds = RECT::default();
                    unsafe {
                        if GetCursorPos(&mut point).is_ok()
                            && GetWindowRect(hwnd, &mut bounds).is_ok()
                            && (point.x < bounds.left
                                || point.x >= bounds.right
                                || point.y < bounds.top
                                || point.y >= bounds.bottom)
                        {
                            dismiss(&watch_app, true);
                        }
                    }
                }
                down = current;
                std::thread::sleep(Duration::from_millis(16));
            }
        });
        animate(&window, true)?;
        if shared.state.lock().unwrap().phase != Phase::Opening {
            return Ok(());
        }
        if shared
            .geometry
            .lock()
            .unwrap()
            .is_some_and(|g| g.place(400.0).overflow)
        {
            // Windows 11 can leave its XAML overflow visible after a tray callback takes focus.
            unsafe {
                if let Ok(overflow) = FindWindowW(w!("TopLevelWindowForOverflowXamlIsland"), None)
                    && IsWindowVisible(overflow).as_bool()
                {
                    let _ = ShowWindow(overflow, SW_HIDE);
                }
            }
        }
        window.set_focus().map_err(|e| e.to_string())?;
        if !shared.state.lock().unwrap().present() {
            return Ok(());
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn dismiss(app: &AppHandle, focus: bool) {
    let shared = shell(app);
    let mut state = shared.state.lock().unwrap();
    if matches!(state.phase, Phase::Closed | Phase::Closing) {
        return;
    }
    let animate = state.phase == Phase::Open;
    state.dismiss(shared.clock.elapsed().as_millis() as u64, focus);
    drop(state);
    close_window(app, animate);
}

fn close_window(app: &AppHandle, animated: bool) {
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(window) = app.get_webview_window("flyout") {
            if animated && let Err(error) = animate(&window, false) {
                eprintln!("Flyout dismissal: {error}");
            }
            if let Err(error) = window.destroy() {
                eprintln!("Flyout destruction: {error}");
                return;
            }
            shell(&app).state.lock().unwrap().phase = Phase::Closed;
        }
    });
}

#[tauri::command]
pub fn dismiss_flyout(app: AppHandle) {
    dismiss(&app, false);
}

#[tauri::command]
pub fn open_project(app: AppHandle) -> Result<(), String> {
    use windows::{Win32::UI::Shell::ShellExecuteW, core::HSTRING};
    let result = unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(shell(&app).config.project),
            None,
            None,
            SW_SHOWNORMAL,
        )
    };
    if result.0 as isize <= 32 {
        Err("Could not open the project page".into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn artwork_selection_combines_state_theme_mode_and_dpi() {
        const SIZES: [&[u8]; 5] = [b"16", b"20", b"24", b"32", b"48"];
        const ART: Artwork = &[("small-paused-light", SIZES), ("protected-dark", SIZES)];
        assert_eq!(
            icon_data(ART, "paused", true, true, 1.5),
            Some(b"24".as_slice())
        );
        assert_eq!(
            icon_data(ART, "protected", false, false, 1.0),
            Some(b"16".as_slice())
        );
        assert_eq!(icon_data(ART, "paused", false, true, 1.5), None);
        assert_eq!(icon_data(ART, "unknown", true, true, 1.5), None);
    }
}
