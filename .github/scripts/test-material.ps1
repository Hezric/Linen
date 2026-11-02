#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = Join-Path $PWD 'material-probe-results'
New-Item -ItemType Directory -Path $root -Force | Out-Null
$exe = Join-Path $PWD 'target/debug/examples/material_probe.exe'
$results = @()
$personalize = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Themes\Personalize'
$transparencyBefore = (Get-ItemProperty -LiteralPath $personalize -Name EnableTransparency -ErrorAction SilentlyContinue).EnableTransparency
try {
    if (!(Test-Path -LiteralPath $personalize)) { New-Item -Path $personalize -Force | Out-Null }
    Set-ItemProperty -LiteralPath $personalize -Name EnableTransparency -Type DWord -Value 1
    foreach ($theme in 'light', 'dark') {
        foreach ($mode in 'material', 'fallback') {
            $marker = Join-Path $root "$theme-$mode.json"
            $stdout = Join-Path $root "$theme-$mode.stdout.log"
            $stderr = Join-Path $root "$theme-$mode.stderr.log"
            $records = @()
            $process = Start-Process -FilePath $exe -ArgumentList @($marker, $theme, $mode) -PassThru -WindowStyle Hidden -RedirectStandardOutput $stdout -RedirectStandardError $stderr
            try {
                $deadline = [DateTime]::UtcNow.AddSeconds(120)
                while (!$process.HasExited -or (Test-Path -LiteralPath $marker)) {
                    if ([DateTime]::UtcNow -gt $deadline) { throw "Material API probe timed out: $theme/$mode" }
                    if (!(Test-Path -LiteralPath $marker)) { Start-Sleep -Milliseconds 10; continue }
                    $meta = Get-Content -LiteralPath $marker -Raw | ConvertFrom-Json
                    $records += $meta
                    if ($meta.stage -in @('initial','shrink-frame-2','grow-frame-2','shrink-settled','grow-settled')) {
                        Add-Type -AssemblyName System.Drawing
                        $image = [Drawing.Bitmap]::new([int]$meta.width, [int]$meta.height)
                        $graphics = [Drawing.Graphics]::FromImage($image)
                        try {
                            $graphics.CopyFromScreen([int]$meta.x, [int]$meta.y, 0, 0, $image.Size)
                            $image.Save((Join-Path $root "$theme-$mode-$($meta.stage).png"))
                        } finally {
                            $graphics.Dispose()
                            $image.Dispose()
                        }
                    }
                    Copy-Item -LiteralPath $marker -Destination (Join-Path $root "$theme-$mode-$($meta.stage).json")
                    Remove-Item -LiteralPath $marker
                    [IO.File]::WriteAllText([IO.Path]::ChangeExtension($marker, 'ack'), '')
                }
                $process.WaitForExit()
                if ($process.ExitCode -ne 0) {
                    throw "Material API host failed: $theme/$mode; exit=$($process.ExitCode); $(Get-Content -LiteralPath $stderr -Raw)"
                }
                foreach ($stage in 'initial', 'shrink-settled', 'grow-settled') {
                    if ($stage -notin $records.stage) { throw "Missing API frame: $theme/$mode/$stage" }
                }
                foreach ($phase in 'shrink', 'grow') {
                    if (!($records.stage -like "$phase-frame-*")) { throw "Missing transition API frames: $theme/$mode/$phase" }
                }
                $results += @{theme = $theme; mode = $mode; frames = $records}
            } finally {
                if (!$process.HasExited) { taskkill.exe /PID $process.Id /T /F | Out-Null }
                if (!$process.WaitForExit(5000)) { throw "Owned CI process tree did not exit: $($process.Id)" }
                $process.Dispose()
            }
        }
    }
    @{ verification = 'native-api'; operatingSystem = [Runtime.InteropServices.RuntimeInformation]::OSDescription; transparencyBefore = $transparencyBefore; transparencyConfigured = 1; probes = $results; materialVisualConfirmation = 'required after installation' } | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $root 'results.json') -Encoding utf8
} finally {
    if ($null -ne $transparencyBefore) { Set-ItemProperty -LiteralPath $personalize -Name EnableTransparency -Value $transparencyBefore }
    else { Remove-ItemProperty -LiteralPath $personalize -Name EnableTransparency -ErrorAction SilentlyContinue }
}
