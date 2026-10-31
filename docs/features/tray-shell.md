# Tray shell

`linen-tray` owns Windows tray menus and the single-instance activation route.
Consumers provide artwork, app identity, a pause callback and translated status;
their workers retain hardware and settings ownership. Login startup is external.

Every opening reads the icon's current rectangle and monitor work area. Icons
inside the taskbar open a centered, clamped 360-DIP flyout beside that edge.
An icon outside the taskbar, or an unavailable rectangle, selects the notification
corner. Horizontal taskbars use the right corner, or the left in RTL; vertical
taskbars use their lower end, or upper end in RTL. Corner flyouts enter from the
screen side; taskbar flyouts slide a full window away from the bar. Exit reverses entry.

The flyout is an opaque undecorated tool window with DWM non-client rendering,
system shadow and rounded corners. The frontend does not draw a native-window
shadow. Height changes resize and reposition the complete window while preserving
its anchor. Work-area limits keep the footer reachable and the body scrollable.

Left-click toggles the flyout. Focus loss, mouse clicks outside its bounds,
Escape and close dismiss it and destroy the webview after the exit animation.
A focus-loss hide consumes a tray click within 300 ms, preventing hide/reopen
races. Opening and closing are serialized; initialization focus events cannot
dismiss a window before presentation. WebView2 creation yields synchronous shell
callbacks. Presentation waits for fonts and renderer frames. The window and WebView2 start
with the system app theme and the same opaque background as the UI, avoiding
a default white frame before content paints. Appearance refresh keeps the native
background in sync. The accessible flyout begins with its status card, without
an app-name heading. The first layout completes without CSS transitions. Consumers add the body class
`presented` after `present_flyout` resolves to enable later UI transitions.
The renderer prepares at the invisible destination before native motion places
the first opaque frame at the entry origin. DWM display transitions are disabled
so they cannot scale a window that the shell is already sliding. Resize and
motion serialize around the resolved destination; motion never infers that
destination from an in-flight window rectangle.
Visibility is set through Tauri before focus is requested. Windows 11's XAML
overflow popup is explicitly hidden if it remains visible during corner opening.

Native motion lasts 250 ms with cubic ease-out timing and fully opaque content.
UI content transitions last 200 ms. Native motion queries Windows client
area animation settings; UI motion follows live reduced-motion preferences.
Consumers refresh appearance once per second without querying hardware. Newer
Windows uses the undocumented `Explorer/Advanced/TaskbarSize` value (0 Default,
1 Small); absent values use legacy `TaskbarSi` (0 Small). Unknown values use
Default. Small artwork occupies 75% of the DPI-selected bitmap canvas.

Native tests cover lifecycle timing, all taskbar edges, negative monitor
coordinates, DPI, clamping, overflow and RTL, mode detection and image selection.
Shared rows, headings, settings, the status card and footer leave eight extra
DIP on their right side. Left insets, icon geometry and the 360-DIP flyout
width stay fixed; long row names retain ellipsis. The pause-duration input uses
a neutral border and the shared keyboard focus ring, without an additional
accent underline; invalid values use an error-colored border.
UI checks exercise the shared accessible markup; consuming apps retain their
behavior tests. Native placement, DWM composition, outside clicks, overflow
light-dismissal and visual motion also require direct Windows inspection.
