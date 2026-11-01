# Tray shell

`linen-tray` owns Windows tray menus and the single-instance activation route.
Consumers provide artwork, app identity, a pause callback and translated status;
their workers retain hardware and settings ownership. Login startup is external.

Every opening uses the tray icon to select its monitor, then docks a clamped
360-DIP flyout eight DIP inside that monitor's work-area bottom-right corner.
The visible outer stroke, rather than the transparent host or blurred shadow, defines
the eight-DIP gap. The current taskbar rectangle corrects its edge of the reported
work area when shell updates lag a resize. Consumers refresh unchanged open panels
on their existing one-second UI tick; changes to monitor, DPI or work area resolve
a new host destination, and every reopening reads geometry again.
Regular and overflow icons, taskbar edges and RTL use the same anchor and enter
horizontally from the right. Exit reverses entry.

The native host is a fixed transparent tool window covering the work-area height.
The shared renderer draws the visible panel, border, shadow and rounded corners,
and animates its height upward around one screen-fixed footer. This keeps the
visible boundary and content in the same composed frame; see the
[rendering decision](../adr/0001-renderer-owned-flyout-shape.md).
Native regions include the complete current/target transition envelope and its
shadow. The driver reserves that envelope before growing the DOM and tightens it
after the latest target settles; outside-click checks use the current panel rectangle.
The remaining canvas is excluded from the window region. Entry and exit move the
fixed host horizontally. Browser previews share the panel anchor and motion.
Height targets follow asynchronous content changes, and the incoming page fades
in without a cloned outgoing page. Ordinary panel content stays free of transient
outer scrollbars. Work-area limits keep the footer reachable; genuinely constrained
content retains its existing internal scrolling.

Left-click toggles the flyout. Focus loss, mouse clicks outside its bounds,
Escape and close dismiss it and destroy the webview after the exit animation.
A focus-loss hide consumes a tray click within 300 ms, preventing hide/reopen
races. Opening and closing are serialized; initialization focus events cannot
dismiss a window before presentation. WebView2 creation yields synchronous shell
callbacks. Presentation waits for fonts and renderer frames. The window and WebView2 start
with transparent native backgrounds and the system app theme, avoiding a default
white frame before content paints. The renderer follows live appearance updates. The accessible flyout begins with its status card, without
an app-name heading. The first layout completes without CSS transitions. Consumers add the body class
`presented` after `present_flyout` resolves to enable later UI transitions.
The renderer prepares at the invisible destination before native motion places
the first composed panel frame at the entry origin. DWM display transitions are disabled
so they cannot scale a window that the shell is already sliding. Region updates and
motion serialize around the resolved destination; motion never infers that
destination from an in-flight window rectangle.
Visibility is set through Tauri before focus is requested. Windows 11's XAML
overflow popup is explicitly hidden if it remains visible during corner opening.

Native motion lasts 250 ms with cubic ease-out timing and fully opaque content.
UI content transitions last 200 ms. Native motion queries Windows client
area animation settings; UI motion follows live reduced-motion preferences.
Windows UISettings supplies the system accent; ColorValuesChanged updates the consuming workers and open flyout. Shared controls, focus underlines and outline icons use that accent, while filled protection shields use a lighter-weight tonal variant. Blue defaults apply only before the system snapshot or when the native color read fails. Consumers refresh appearance once per second without querying hardware. Newer
Windows uses the undocumented `Explorer/Advanced/TaskbarSize` value (0 Default,
1 Small); absent values use legacy `TaskbarSi` (0 Small). Unknown values use
Default. Small artwork occupies 75% of the DPI-selected bitmap canvas.

Native tests cover lifecycle timing, all taskbar edges, negative monitor
coordinates, DPI, clamping, overflow and RTL, mode detection and image selection.
Shared headings, rows, settings and footer use matching extra left and right
insets relative to the unchanged status card. Its outer left and right margins
remain equal. The 360-DIP visible panel width and icon geometry stay fixed; long row
names retain ellipsis. Text inputs reserve a two-pixel bottom border in every state. Focus changes its
color to the accent without moving the text or adding a full outline; invalid
input uses the error color. Other controls retain the shared keyboard focus ring.
UI checks exercise the shared accessible markup; consuming apps retain their
behavior tests. Native placement, transparency, input regions, outside clicks, overflow
light-dismissal and visual motion also require direct Windows inspection.
