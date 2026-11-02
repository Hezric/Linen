#requires -Version 7.0
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class NativeScreenshot {
    [DllImport("user32.dll")] static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    public static void Save() {
        keybd_event(0x5b, 0, 0, UIntPtr.Zero);
        keybd_event(0x2c, 0, 0, UIntPtr.Zero);
        keybd_event(0x2c, 0, 2, UIntPtr.Zero);
        keybd_event(0x5b, 0, 2, UIntPtr.Zero);
    }
}
'@
$root = Join-Path $PWD 'material-probe-results'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$exe = Join-Path $PWD 'target/debug/examples/material_probe.exe'
$results = @()
function Capture-Screen($meta, $path) {
    $screenshots = Join-Path ([Environment]::GetFolderPath('MyPictures')) 'Screenshots'
    New-Item -ItemType Directory -Path $screenshots -Force | Out-Null
    $before = @(Get-ChildItem -LiteralPath $screenshots -Filter '*.png' -File | ForEach-Object FullName)
    [NativeScreenshot]::Save()
    $deadline = [DateTime]::UtcNow.AddSeconds(5)
    do {
        Start-Sleep -Milliseconds 100
        $files = @(Get-ChildItem -LiteralPath $screenshots -Filter '*.png' -File | Where-Object { $_.FullName -notin $before })
        if ($files.Count -gt 1) { throw 'Ambiguous native CI screenshot output' }
    } until ($files.Count -eq 1 -or [DateTime]::UtcNow -gt $deadline)
    if ($files.Count -ne 1) { throw 'Windows did not save the native CI screenshot' }
    Start-Sleep -Milliseconds 200
    $file = $files[0].FullName
    try {
        $full = [Drawing.Bitmap]::new($file)
        try {
            $bounds = [Drawing.Rectangle]::new([int]$meta.x, [int]$meta.y, [int]$meta.width, [int]$meta.height)
            $bitmap = $full.Clone($bounds, $full.PixelFormat)
            $bitmap.Save($path)
            $bitmap
        } finally { $full.Dispose() }
    } finally { Remove-Item -LiteralPath $file }
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
        Write-Output "Native background contrast: $theme control=$controlContrast material=$materialContrast fallback=$fallbackContrast"
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
