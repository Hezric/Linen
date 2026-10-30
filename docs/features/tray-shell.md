# Tray shell

`linen-tray` owns Windows tray menus and the single-instance activation route.
Consumers provide artwork, app identity, a pause callback and translated status;
their workers retain hardware and settings ownership. Login startup is external.

Every opening reads the icon's current rectangle and monitor work area. Icons
inside the taskbar open a centered, clamped 360-DIP flyout beside that edge.
An icon outside the taskbar, or an unavailable rectangle, selects the notification
corner. Horizontal taskbars use the right corner, or the left in RTL; vertical
taskbars use their lower end, or upper end in RTL. Corner flyouts enter from the
screen side; taskbar flyouts rise away from the bar. Exit reverses entry.

The flyout is an opaque undecorated tool window with DWM non-client rendering,
system shadow and rounded corners. The frontend does not draw a native-window
shadow. Height changes resize and reposition the complete window while preserving
its anchor. Work-area limits keep the footer reachable and the body scrollable.

Left-click toggles the flyout. Focus loss, mouse clicks outside its bounds,
Escape and close dismiss it and destroy the webview after the exit animation.
A focus-loss hide consumes a tray click within 300 ms, preventing hide/reopen
races. Opening and closing are serialized; initialization focus events cannot
dismiss a window before presentation. WebView2 creation yields synchronous shell
callbacks. Presentation waits for fonts and renderer frames.
Visibility is set through Tauri before focus is requested. Windows 11's XAML
overflow popup is explicitly hidden if it remains visible during corner opening.

Motion lasts 200 ms with ease-out timing. Native motion queries Windows client
area animation settings; UI motion follows live reduced-motion preferences.
Consumers refresh appearance once per second without querying hardware. Newer
Windows uses the undocumented `Explorer/Advanced/TaskbarSize` value (0 Default,
1 Small); absent values use legacy `TaskbarSi` (0 Small). Unknown values use
Default. Small artwork occupies 75% of the DPI-selected bitmap canvas.

Native tests cover lifecycle timing, all taskbar edges, negative monitor
coordinates, DPI, clamping, overflow and RTL, mode detection and image selection.
UI checks exercise the shared accessible markup; consuming apps retain their
behavior tests. Native placement, DWM composition, outside clicks, overflow
light-dismissal and visual motion also require direct Windows inspection.
