use std::cell::{Cell, RefCell};
use windows::{
    System::{DispatcherQueue, DispatcherQueueController},
    UI::{
        Composition::{
            CompositionRoundedRectangleGeometry, Compositor, Desktop::DesktopWindowTarget,
            SpriteVisual,
        },
        ViewManagement::{AccessibilitySettings, UISettings},
    },
    Win32::{
        Foundation::HWND,
        Graphics::Dwm::{DWMWA_USE_HOSTBACKDROPBRUSH, DwmSetWindowAttribute},
        System::WinRT::{
            Composition::ICompositorDesktopInterop, CreateDispatcherQueueController,
            DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT, DispatcherQueueOptions,
        },
    },
    core::Interface,
};

thread_local! {
    static SUSPENDED: Cell<bool> = const { Cell::new(true) };
    static QUEUE: RefCell<Option<DispatcherQueueController>> = const { RefCell::new(None) };
    static SURFACE: RefCell<Option<(isize, windows::core::Result<Material>)>> = const { RefCell::new(None) };
}

struct Material {
    _target: DesktopWindowTarget,
    visual: SpriteVisual,
    shape: CompositionRoundedRectangleGeometry,
}

impl Material {
    fn new(hwnd: HWND) -> windows::core::Result<Self> {
        if DispatcherQueue::GetForCurrentThread().is_err() {
            let queue = unsafe {
                CreateDispatcherQueueController(DispatcherQueueOptions {
                    dwSize: size_of::<DispatcherQueueOptions>() as u32,
                    threadType: DQTYPE_THREAD_CURRENT,
                    apartmentType: DQTAT_COM_NONE,
                })?
            };
            QUEUE.with(|slot| *slot.borrow_mut() = Some(queue));
        }
        let enabled = 1i32;
        unsafe {
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_HOSTBACKDROPBRUSH,
                (&enabled as *const i32).cast(),
                size_of_val(&enabled) as u32,
            )?;
        }
        let compositor = Compositor::new()?;
        let target = unsafe {
            compositor
                .cast::<ICompositorDesktopInterop>()?
                .CreateDesktopWindowTarget(hwnd, false)?
        };
        let visual = compositor.CreateSpriteVisual()?;
        visual.SetBrush(&compositor.CreateHostBackdropBrush()?)?;
        let shape = compositor.CreateRoundedRectangleGeometry()?;
        visual.SetClip(&compositor.CreateGeometricClipWithGeometry(&shape)?)?;
        target.SetRoot(&visual)?;
        Ok(Self {
            _target: target,
            visual,
            shape,
        })
    }

    fn update(&self, panel: crate::Rect, scale: f64) -> windows::core::Result<()> {
        let mut size = self.visual.Size()?;
        size.X = (panel[2] - panel[0]) as f32;
        size.Y = (panel[3] - panel[1]) as f32;
        self.visual.SetSize(size)?;
        self.shape.SetSize(size)?;
        let mut radius = self.shape.CornerRadius()?;
        radius.X = (8.0 * scale) as f32;
        radius.Y = radius.X;
        self.shape.SetCornerRadius(radius)?;
        let mut offset = self.visual.Offset()?;
        offset.X = panel[0] as f32;
        offset.Y = panel[1] as f32;
        self.visual.SetOffset(offset)?;
        self.visual.SetIsVisible(true)
    }
}

// Confine the desktop brush to the card; the fixed host also contains transparent shadow space.
pub(crate) fn update(hwnd: HWND, panel: crate::Rect, scale: f64, settled: bool) -> bool {
    if settled {
        SUSPENDED.with(|flag| flag.set(false));
    }
    if SUSPENDED.with(Cell::get) {
        return false;
    }
    let enabled = UISettings::new()
        .and_then(|settings| settings.AdvancedEffectsEnabled())
        .unwrap_or(false)
        && !AccessibilitySettings::new()
            .and_then(|settings| settings.HighContrast())
            .unwrap_or(true);
    SURFACE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if !enabled {
            if let Some((_, Ok(surface))) = slot.as_ref() {
                let _ = surface.visual.SetIsVisible(false);
            }
            return false;
        }
        if slot
            .as_ref()
            .is_none_or(|(owner, _)| *owner != hwnd.0 as isize)
        {
            let surface = Material::new(hwnd);
            if let Err(error) = &surface {
                eprintln!("Flyout backdrop: {error}");
            }
            *slot = Some((hwnd.0 as isize, surface));
        }
        let Some((_, Ok(surface))) = slot.as_ref() else {
            return false;
        };
        if let Err(error) = surface.update(panel, scale) {
            eprintln!("Flyout backdrop layout: {error}");
            let _ = surface.visual.SetIsVisible(false);
            return false;
        }
        true
    })
}

pub(crate) async fn suspend(app: &tauri::AppHandle) -> Result<(), String> {
    let (send, mut receive) = tauri::async_runtime::channel(1);
    app.run_on_main_thread(move || {
        SUSPENDED.with(|flag| flag.set(true));
        let commit = SURFACE.with(|slot| {
            let slot = slot.borrow();
            if let Some((_, Ok(surface))) = slot.as_ref() {
                surface.visual.SetIsVisible(false)?;
                surface._target.Compositor()?.RequestCommitAsync().map(Some)
            } else {
                Ok(None)
            }
        });
        let _ = send.try_send(commit);
    })
    .map_err(|error| error.to_string())?;
    let commit = receive
        .recv()
        .await
        .ok_or("Backdrop suspension was interrupted")?
        .map_err(|error| error.to_string())?;
    if let Some(commit) = commit {
        commit.await.map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub(crate) fn remove() {
    SUSPENDED.with(|flag| flag.set(true));
    SURFACE.with(|slot| *slot.borrow_mut() = None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::{
        Win32::{
            System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize},
            UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WINDOW_EX_STYLE, WS_POPUP},
        },
        core::w,
    };

    #[test]
    fn desktop_brush_is_clipped_to_the_visible_card() {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok().unwrap() };
        let hwnd = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Linen material CI"),
                WS_POPUP,
                0,
                0,
                405,
                620,
                None,
                None,
                None,
                None,
            )
            .unwrap()
        };
        let checked = (|| -> windows::core::Result<()> {
            let surface = Material::new(hwnd)?;
            surface.update([32, 307, 392, 607], 1.0)?;
            assert_eq!(surface.visual.Size()?.X, 360.0);
            assert_eq!(surface.visual.Size()?.Y, 300.0);
            assert_eq!(surface.visual.Offset()?.Y, 307.0);
            assert_eq!(surface.shape.CornerRadius()?.X, 8.0);
            surface.update([48, 460, 588, 910], 1.5)?;
            assert_eq!(surface.visual.Size()?.X, 540.0);
            assert_eq!(surface.shape.Size()?.Y, 450.0);
            assert_eq!(surface.visual.Offset()?.Y, 460.0);
            assert_eq!(surface.shape.CornerRadius()?.X, 12.0);
            Ok(())
        })();
        QUEUE.with(|slot| *slot.borrow_mut() = None);
        unsafe {
            DestroyWindow(hwnd).unwrap();
            CoUninitialize();
        }
        checked.unwrap();
    }
}
