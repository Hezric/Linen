#requires -Version 7.0
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$root = Join-Path $PWD 'material-probe-results'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$exe = Join-Path $PWD 'target/debug/examples/material_probe.exe'
$results = @()
foreach ($theme in 'light', 'dark') {
    $shots = @{}
    try {
        foreach ($mode in 'transparent', 'material', 'fallback') {
            $marker = Join-Path $root "$theme-$mode.json"
            $stdout = Join-Path $root "$theme-$mode.stdout.log"
            $stderr = Join-Path $root "$theme-$mode.stderr.log"
            $process = Start-Process -FilePath $exe -ArgumentList @($marker, $theme, $mode) -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            try {
                $deadline = [DateTime]::UtcNow.AddSeconds(30)
                while (!(Test-Path -LiteralPath $marker)) {
                    if ($process.HasExited -or [DateTime]::UtcNow -gt $deadline) {
                        throw "Material CI host did not become ready: $theme/$mode; $(Get-Content -LiteralPath $stderr -Raw)"
                    }
                    Start-Sleep -Milliseconds 100
                }
                $meta = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
                if ($mode -eq 'material' -and !$meta.active) { throw 'Native material was unavailable' }
                $bitmap = [Drawing.Bitmap]::new($meta.width, $meta.height)
                $graphics = [Drawing.Graphics]::FromImage($bitmap)
                try { $graphics.CopyFromScreen($meta.x, $meta.y, 0, 0, $bitmap.Size) } finally { $graphics.Dispose() }
                $bitmap.Save((Join-Path $root "$theme-$mode.png"))
                $shots[$mode] = $bitmap
            } finally {
                if (!$process.HasExited) { taskkill.exe /PID $process.Id /T /F | Out-Null }
                if (!$process.WaitForExit(5000)) { throw "Owned CI process tree did not exit: $($process.Id)" }
                $process.Dispose()
            }
        }
        $scale = $meta.scale
        function Roughness($bitmap) {
            $sum = 0.0; $count = 0
            for ($y = [int](400 * $scale); $y -lt [int](460 * $scale); $y++) {
                for ($x = [int](100 * $scale); $x -lt [int](220 * $scale); $x++) {
                    $a = $bitmap.GetPixel($x, $y); $b = $bitmap.GetPixel($x + 1, $y)
                    $sum += [Math]::Abs($a.R - $b.R) + [Math]::Abs($a.G - $b.G) + [Math]::Abs($a.B - $b.B)
                    $count++
                }
            }
            $sum / $count
        }
        $control = Roughness $shots.transparent
        $material = Roughness $shots.material
        if ($control -lt 1 -or $material -ge $control * 0.5) {
            throw "HostBackdrop did not visibly blur behind WebView/layered host: $theme control=$control material=$material"
        }
        $outsideChanges = 0
        for ($y = [int](330 * $scale); $y -lt [int](550 * $scale); $y++) {
            $x = [int](5 * $scale)
            if ($shots.material.GetPixel($x, $y).ToArgb() -ne $shots.transparent.GetPixel($x, $y).ToArgb()) { $outsideChanges++ }
        }
        if ($outsideChanges -gt 2) { throw "Material escaped the visible card: $outsideChanges pixels" }
        $results += @{ theme = $theme; transparentRoughness = $control; materialRoughness = $material; outsideChanges = $outsideChanges }
    } finally {
        foreach ($bitmap in $shots.Values) { $bitmap.Dispose() }
    }
}
$results | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $root 'results.json') -Encoding utf8
