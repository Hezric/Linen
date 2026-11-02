using System;
using System.Runtime.InteropServices;

public static class DwmReference {
    [StructLayout(LayoutKind.Sequential)] struct Rect { public int left, top, right, bottom; }
    [StructLayout(LayoutKind.Sequential)] struct Margins { public int left, right, top, bottom; }
    [StructLayout(LayoutKind.Sequential)] struct Point { public int x, y; }
    [StructLayout(LayoutKind.Sequential)] struct Message { public IntPtr window; public uint message; public IntPtr wParam, lParam; public uint time; public Point point; public uint extra; }
    delegate IntPtr WindowProcedure(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    static readonly WindowProcedure procedure = Handle;
    static IntPtr pattern, host;
    [DllImport("user32.dll", CharSet = CharSet.Unicode, SetLastError = true)] static extern IntPtr CreateWindowExW(uint exStyle, string name, string title, uint style, int x, int y, int width, int height, IntPtr parent, IntPtr menu, IntPtr instance, IntPtr param);
    [DllImport("user32.dll")] static extern bool DestroyWindow(IntPtr window);
    [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr window);
    [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr window, IntPtr dc);
    [DllImport("user32.dll")] static extern int FillRect(IntPtr dc, ref Rect rect, IntPtr brush);
    [DllImport("user32.dll")] static extern bool ValidateRect(IntPtr window, IntPtr rect);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] static extern IntPtr SetWindowLongPtrW(IntPtr window, int index, IntPtr value);
    [DllImport("user32.dll")] static extern IntPtr DefWindowProcW(IntPtr window, uint message, IntPtr wParam, IntPtr lParam);
    [DllImport("user32.dll")] static extern bool PeekMessageW(out Message message, IntPtr window, uint min, uint max, uint remove);
    [DllImport("user32.dll")] static extern bool TranslateMessage(ref Message message);
    [DllImport("user32.dll")] static extern IntPtr DispatchMessageW(ref Message message);
    [DllImport("gdi32.dll")] static extern IntPtr CreateSolidBrush(uint color);
    [DllImport("gdi32.dll")] static extern bool DeleteObject(IntPtr value);
    [DllImport("dwmapi.dll")] static extern int DwmExtendFrameIntoClientArea(IntPtr window, ref Margins margins);
    [DllImport("dwmapi.dll")] static extern int DwmSetWindowAttribute(IntPtr window, int attribute, ref int value, int size);
    [DllImport("dwmapi.dll")] static extern int DwmGetWindowAttribute(IntPtr window, int attribute, out int value, int size);
    [DllImport("dwmapi.dll")] static extern int DwmFlush();

    static IntPtr Handle(IntPtr window, uint message, IntPtr wParam, IntPtr lParam) {
        if (message == 0x000f) { ValidateRect(window, IntPtr.Zero); return IntPtr.Zero; }
        if (message == 0x0014) return new IntPtr(1);
        return DefWindowProcW(window, message, wParam, lParam);
    }
    public static void Start() {
        // The reference uses only Win32/DWM, with no Tauri, WebView or Composition visual.
        pattern = CreateWindowExW(8, "STATIC", "CI native pattern", 0x90000000, 0, 0, 900, 700, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
        if (pattern == IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        SetWindowLongPtrW(pattern, -4, Marshal.GetFunctionPointerForDelegate(procedure));
        var dc = GetDC(pattern);
        var brushes = new IntPtr[] { CreateSolidBrush(0xccccff), CreateSolidBrush(0xffddcc), CreateSolidBrush(0x000040), CreateSolidBrush(0x402000) };
        try {
            for (int y = 0; y < 700; y += 4) for (int x = 0; x < 900; x += 4) {
                var rect = new Rect { left = x, top = y, right = x + 4, bottom = y + 4 };
                if (FillRect(dc, ref rect, brushes[(x >= 350 ? 2 : 0) + ((x / 4 + y / 4) % 2)]) == 0) throw new System.ComponentModel.Win32Exception();
            }
        } finally { ReleaseDC(pattern, dc); foreach (var brush in brushes) DeleteObject(brush); }
        host = CreateWindowExW(0x00200008, "STATIC", "CI DWM system backdrop reference", 0x10cf0000, 100, 50, 405, 620, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero, IntPtr.Zero);
        if (host == IntPtr.Zero) throw new System.ComponentModel.Win32Exception();
        SetWindowLongPtrW(host, -4, Marshal.GetFunctionPointerForDelegate(procedure));
        var margins = new Margins { left = -1, right = -1, top = -1, bottom = -1 };
        Marshal.ThrowExceptionForHR(DwmExtendFrameIntoClientArea(host, ref margins));
        SetBackdrop(1);
    }
    public static int SetBackdrop(int type) {
        int dark = 0;
        Marshal.ThrowExceptionForHR(DwmSetWindowAttribute(host, 20, ref dark, 4));
        Marshal.ThrowExceptionForHR(DwmSetWindowAttribute(host, 38, ref type, 4));
        Message message;
        while (PeekMessageW(out message, IntPtr.Zero, 0, 0, 1)) { TranslateMessage(ref message); DispatchMessageW(ref message); }
        Marshal.ThrowExceptionForHR(DwmFlush());
        int actual;
        Marshal.ThrowExceptionForHR(DwmGetWindowAttribute(host, 38, out actual, 4));
        if (actual != type || !IsWindowVisible(host)) throw new Exception("DWM reference was not applied to a visible window");
        return actual;
    }
    public static void Close() {
        if (host != IntPtr.Zero) { DestroyWindow(host); host = IntPtr.Zero; }
        if (pattern != IntPtr.Zero) { DestroyWindow(pattern); pattern = IntPtr.Zero; }
    }
}
