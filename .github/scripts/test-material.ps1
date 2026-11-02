#requires -Version 7.0
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
public static class LayeredCapture {
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int index);
    [DllImport("user32.dll", EntryPoint = "SystemParametersInfoW", CharSet = CharSet.Unicode, SetLastError = true)] public static extern bool GetWallpaper(uint action, uint length, System.Text.StringBuilder path, uint flags);
    [DllImport("user32.dll", EntryPoint = "SystemParametersInfoW", CharSet = CharSet.Unicode, SetLastError = true)] public static extern bool SetWallpaper(uint action, uint length, string path, uint flags);
    [DllImport("user32.dll")] static extern IntPtr GetDC(IntPtr window);
    [DllImport("user32.dll")] static extern int ReleaseDC(IntPtr window, IntPtr dc);
    [DllImport("gdi32.dll", SetLastError = true)] static extern bool BitBlt(IntPtr target, int x, int y, int width, int height, IntPtr source, int sx, int sy, uint operation);
    public static void Copy(IntPtr target, int width, int height, int x, int y) {
        var source = GetDC(IntPtr.Zero);
        if (source == IntPtr.Zero) throw new Win32Exception();
        try {
            if (!BitBlt(target, 0, 0, width, height, source, x, y, 0x40CC0020)) throw new Win32Exception();
        } finally { ReleaseDC(IntPtr.Zero, source); }
    }
}
'@
$root = Join-Path $PWD 'material-probe-results'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$exe = Join-Path $PWD 'target/debug/examples/material_probe.exe'
$results = @()
$originalWallpaper = [Text.StringBuilder]::new(32768)
if (![LayeredCapture]::GetWallpaper(0x73, $originalWallpaper.Capacity, $originalWallpaper, 0)) { throw 'Cannot read CI wallpaper' }
$wallpaper = Join-Path $root 'wallpaper.bmp'
$width = [LayeredCapture]::GetSystemMetrics(0)
$height = [LayeredCapture]::GetSystemMetrics(1)
$fixture = [Drawing.Bitmap]::new($width, $height)
$paint = [Drawing.Graphics]::FromImage($fixture)
$red = [Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(128, 240, 0, 0))
$blue = [Drawing.SolidBrush]::new([Drawing.Color]::FromArgb(128, 0, 128, 255))
try {
    $paint.Clear([Drawing.Color]::White)
    $paint.FillRectangle([Drawing.Brushes]::Black, 350, 0, $width - 350, $height)
    for ($y = 0; $y -lt $height; $y += 4) {
        for ($x = 0; $x -lt $width; $x += 4) {
            $brush = if (([int]($x / 4) + [int]($y / 4)) % 2) { $blue } else { $red }
            $paint.FillRectangle($brush, $x, $y, 4, 4)
        }
    }
    $fixture.Save($wallpaper, [Drawing.Imaging.ImageFormat]::Bmp)
} finally { $paint.Dispose(); $fixture.Dispose(); $red.Dispose(); $blue.Dispose() }
function Capture-Screen($meta, $path) {
    $bitmap = [Drawing.Bitmap]::new([int]$meta.width, [int]$meta.height)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    # CopyFromScreen rejects the documented SRCCOPY | CAPTUREBLT combination.
    $dc = $graphics.GetHdc()
    try { [LayeredCapture]::Copy($dc, $bitmap.Width, $bitmap.Height, $meta.x, $meta.y) } finally { $graphics.ReleaseHdc($dc); $graphics.Dispose() }
    $bitmap.Save($path)
    $bitmap
}
function Difference($a, $b) {
    [Math]::Abs([int]$a.R - $b.R) + [Math]::Abs([int]$a.G - $b.G) + [Math]::Abs([int]$a.B - $b.B)
}
function Roughness($bitmap, $scale) {
    $sum = 0.0; $count = 0
    for ($y = [int](400 * $scale); $y -lt [int](460 * $scale); $y++) {
        for ($x = [int](100 * $scale); $x -lt [int](220 * $scale); $x++) {
            $sum += Difference $bitmap.GetPixel($x, $y) $bitmap.GetPixel($x + 1, $y)
            $count++
        }
    }
    $sum / $count
}
function Background-Contrast($bitmap, $scale) {
    $left = 0.0; $right = 0.0; $count = 0
    for ($y = 400; $y -lt 460; $y++) {
        for ($x = 100; $x -lt 160; $x++) {
            $a = $bitmap.GetPixel([int]($x * $scale), [int]($y * $scale))
            $b = $bitmap.GetPixel([int](($x + 180) * $scale), [int]($y * $scale))
            $left += [int]$a.R + $a.G + $a.B
            $right += [int]$b.R + $b.G + $b.B
            $count++
        }
    }
    ($left - $right) / $count
}
try {
if (![LayeredCapture]::SetWallpaper(0x14, 0, $wallpaper, 3)) { throw 'Cannot set the disposable CI wallpaper fixture' }
foreach ($theme in 'light', 'dark') {
    $shots = @{}
    $metadata = @{}
    try {
        foreach ($mode in 'transparent', 'material', 'fallback') {
            $marker = Join-Path $root "$theme-$mode.json"
            $stdout = Join-Path $root "$theme-$mode.stdout.log"
            $stderr = Join-Path $root "$theme-$mode.stderr.log"
            $process = Start-Process -FilePath $exe -ArgumentList @($marker, $theme, $mode) -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            try {
                $deadline = [DateTime]::UtcNow.AddSeconds(120)
                while (!$process.HasExited -or (Test-Path -LiteralPath $marker)) {
                    if ([DateTime]::UtcNow -gt $deadline) { throw "Material CI capture timed out: $theme/$mode" }
                    if (!(Test-Path -LiteralPath $marker)) { Start-Sleep -Milliseconds 10; continue }
                    $meta = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
                    $key = "$mode-$($meta.stage)"
                    $shots[$key] = Capture-Screen $meta (Join-Path $root "$theme-$key.png")
                    $metadata[$key] = $meta
                    Copy-Item -LiteralPath $marker -Destination (Join-Path $root "$theme-$key.json")
                    Remove-Item -LiteralPath $marker
                    [IO.File]::WriteAllText([IO.Path]::ChangeExtension($marker, 'ack'), '')
                }
                $process.WaitForExit()
                if ($process.ExitCode -ne 0) {
                    throw "Material CI host failed: $theme/$mode; exit=$($process.ExitCode); $(Get-Content -LiteralPath $stderr -Raw)"
                }
                foreach ($stage in 'initial', 'background', 'shrink-settled', 'grow-settled') {
                    if (!$shots.ContainsKey("$mode-$stage")) { throw "Missing probe capture: $theme/$mode/$stage" }
                }
                foreach ($phase in 'shrink', 'grow') {
                    if (!($shots.Keys -like "$mode-$phase-frame-*")) { throw "Missing transition frames: $theme/$mode/$phase" }
                }
            } finally {
                if (!$process.HasExited) { taskkill.exe /PID $process.Id /T /F | Out-Null }
                if (!$process.WaitForExit(5000)) { throw "Owned CI process tree did not exit: $($process.Id)" }
                $process.Dispose()
            }
        }
        $scale = $metadata['material-initial'].scale
        $control = Roughness $shots['transparent-initial'] $scale
        $material = Roughness $shots['material-initial'] $scale
        $fallback = Roughness $shots['fallback-initial'] $scale
        if ($control -lt 1 -or $material -ge $control * 0.5) {
            throw "HostBackdrop did not visibly blur behind WebView/layered host: $theme control=$control material=$material fallback=$fallback"
        }
        $controlContrast = Background-Contrast $shots['transparent-initial'] $scale
        $materialContrast = Background-Contrast $shots['material-initial'] $scale
        $fallbackContrast = Background-Contrast $shots['fallback-initial'] $scale
        if ($controlContrast -lt 10 -or $materialContrast -lt $controlContrast * 0.25 -or [Math]::Abs($fallbackContrast) -gt 1) {
            throw "Material lost the background's coarse color signal: $theme control=$controlContrast material=$materialContrast fallback=$fallbackContrast"
        }
        $outsideChanges = 0
        $frames = 0
        foreach ($key in @($shots.Keys | Where-Object { $_ -like 'material-*' -and $_ -ne 'material-background' })) {
            $meta = $metadata[$key]
            $bitmap = $shots[$key]
            $background = $shots['material-background']
            $changes = 0
            for ($y = 0; $y -lt $bitmap.Height; $y += 4) {
                for ($x = 0; $x -lt $bitmap.Width; $x += 4) {
                    # Exclude the CSS shadow when looking for a stale native card above the DOM.
                    if ($x -ge ($meta.panel[0] - 32) * $scale -and $x -le ($meta.panel[2] + 32) * $scale -and $y -ge ($meta.panel[1] - 32) * $scale -and $y -le ($meta.panel[3] + 32) * $scale) { continue }
                    if ((Difference $bitmap.GetPixel($x, $y) $background.GetPixel($x, $y)) -gt 6) { $changes++ }
                }
            }
            if ($changes -gt 2) { throw "Material escaped DOM card/shadow envelope: $theme/$key; $changes samples" }
            $outsideChanges += $changes
            if ($key -like '*-frame-*') { $frames++ }
        }
        $edgeChanges = 0
        foreach ($stage in 'initial', 'shrink-settled', 'grow-settled') {
            $meta = $metadata["material-$stage"]
            $a = $shots["material-$stage"]
            $b = $shots["transparent-$stage"]
            for ($y = 270; $y -lt 620; $y++) {
                for ($x = 0; $x -lt 405; $x++) {
                    $cx = [Math]::Clamp($x, $meta.panel[0] + 8, $meta.panel[2] - 8)
                    $cy = [Math]::Clamp($y, $meta.panel[1] + 8, $meta.panel[3] - 8)
                    if ([Math]::Pow($x - $cx, 2) + [Math]::Pow($y - $cy, 2) -le 100) { continue }
                    $px = [int]($x * $scale); $py = [int]($y * $scale)
                    if ((Difference $a.GetPixel($px, $py) $b.GetPixel($px, $py)) -gt 6) { $edgeChanges++ }
                }
            }
        }
        if ($edgeChanges -gt 2) { throw "Material escaped rounded card: $theme; $edgeChanges pixels" }
        $results += @{ theme = $theme; transparentRoughness = $control; materialRoughness = $material; fallbackRoughness = $fallback; controlContrast = $controlContrast; materialContrast = $materialContrast; fallbackContrast = $fallbackContrast; outsideChanges = $outsideChanges; edgeChanges = $edgeChanges; transitionFrames = $frames }
    } finally {
        foreach ($bitmap in $shots.Values) { $bitmap.Dispose() }
    }
}
$results | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'results.json') -Encoding utf8
} finally {
    if (![LayeredCapture]::SetWallpaper(0x14, 0, $originalWallpaper.ToString(), 3)) { throw 'Cannot restore the original CI wallpaper' }
}
