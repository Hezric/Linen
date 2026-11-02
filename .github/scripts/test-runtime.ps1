#requires -Version 7.0
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:RUNNER_TEMP ('linen-runtime-' + [Guid]::NewGuid().ToString('N'))
$runtime = Join-Path $root 'persist/runtime'
New-Item -ItemType Directory -Path $runtime -Force | Out-Null
try {
    foreach ($version in 'one', 'two') {
        $dir = Join-Path $root $version
        New-Item -ItemType Directory -Path $dir | Out-Null
        Copy-Item -LiteralPath 'target/debug/examples/runtime_probe.exe' -Destination (Join-Path $dir 'runtime_probe.exe')
        New-Item -ItemType Junction -Path (Join-Path $dir 'runtime') -Target $runtime | Out-Null
        Copy-Item -LiteralPath (Join-Path $dir 'runtime_probe.exe') -Destination (Join-Path $runtime 'runtime_probe.exe') -Force
        $output = Join-Path $root "$version.txt"
        $process = Start-Process -FilePath (Join-Path $dir 'runtime_probe.exe') -ArgumentList $output -WindowStyle Hidden -PassThru
        try {
            if (!$process.WaitForExit(10000) -or $process.ExitCode -ne 0) { throw 'Launcher failed' }
            $deadline = [DateTime]::UtcNow.AddSeconds(10)
            while (!(Test-Path -LiteralPath $output)) {
                if ([DateTime]::UtcNow -gt $deadline) { throw 'Runtime did not report its path' }
                Start-Sleep -Milliseconds 10
            }
            $pidText, $actual = [IO.File]::ReadAllLines($output)
            if ($actual.Replace('\\?\', '') -ine (Join-Path $runtime 'runtime_probe.exe')) { throw "Unexpected image path: $actual" }
            try { $child = [Diagnostics.Process]::GetProcessById([int]$pidText) } catch [ArgumentException] { $child = $null }
            if ($child) {
                try {
                    if (!$child.WaitForExit(10000)) { throw 'Runtime did not exit' }
                } finally {
                    if (!$child.HasExited) { $child.Kill($true); $child.WaitForExit() }
                    $child.Dispose()
                }
            }
            Write-Output "$version image path: $actual"
        } finally {
            if (!$process.HasExited) { $process.Kill($true); $process.WaitForExit() }
            $process.Dispose()
        }
    }
    if ([IO.File]::ReadAllLines((Join-Path $root 'one.txt'))[1] -cne [IO.File]::ReadAllLines((Join-Path $root 'two.txt'))[1]) { throw 'Update changed the runtime image path' }
} finally {
    foreach ($version in 'one', 'two') {
        $link = Join-Path $root "$version/runtime"
        if (Test-Path -LiteralPath $link) { [IO.Directory]::Delete($link) }
    }
    if (![IO.Path]::GetFullPath($root).StartsWith([IO.Path]::GetFullPath($env:RUNNER_TEMP) + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Cleanup escaped the runner temp directory' }
    Remove-Item -LiteralPath $root -Recurse -Force
}
