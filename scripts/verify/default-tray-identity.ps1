#requires -Version 7.0
param(
    [Parameter(Mandatory)][ValidatePattern('^[a-zA-Z][a-zA-Z0-9_-]*$')][string]$App,
    [Parameter(Mandatory)][string]$OutDirectory
)
$ErrorActionPreference = 'Stop'
if (!$IsWindows) { throw 'Run in an interactive Windows session with Scoop and .NET Framework.' }
if (Test-Path -LiteralPath $OutDirectory) { throw 'Choose a new directory for the disposable probe.' }
$work = (New-Item -ItemType Directory -Path $OutDirectory -ErrorAction Stop).FullName
$name = 'tray-verify-' + $App.ToLowerInvariant() + '-' + [guid]::NewGuid().ToString('N').Substring(0, 12)
$scoopRoot = if ($env:SCOOP) { [IO.Path]::GetFullPath($env:SCOOP) } else { Join-Path $env:USERPROFILE 'scoop' }
$scoop = Join-Path $scoopRoot 'shims/scoop.ps1'
$appRoot = Join-Path $scoopRoot "apps/$name"
$persistRoot = Join-Path $scoopRoot "persist/$name"
$exe = Join-Path $persistRoot 'runtime/Probe.exe'
$manifest = Join-Path $work "$name.json"
$cacheRoot = (New-Item -ItemType Directory -Path (Join-Path $work 'cache')).FullName
if (!(Test-Path -LiteralPath $scoop) -or (Test-Path -LiteralPath $appRoot) -or (Test-Path -LiteralPath $persistRoot)) {
    throw 'Scoop is unavailable or the disposable identity already exists.'
}
$source = @"
using System;using System.IO;using System.Drawing;using System.Windows.Forms;using System.Reflection;using System.Diagnostics;
[assembly: AssemblyVersion("VERSION.0.0.0")]
static class Probe {
 [STAThread]static void Main(string[] args) {
  using(var icon=new NotifyIcon())using(var timer=new Timer()) {
   icon.Icon=SystemIcons.Information;icon.Text="Disposable tray verification";icon.Visible=true;
   File.WriteAllLines(args[0],new[]{Process.GetCurrentProcess().Id.ToString(),Process.GetCurrentProcess().MainModule.FileName,Assembly.GetExecutingAssembly().GetName().Version.ToString()});
   var until=DateTime.UtcNow.AddSeconds(30);timer.Interval=100;
   timer.Tick+=(s,e)=>{if(File.Exists(args[1])||DateTime.UtcNow>=until)Application.Exit();};timer.Start();Application.Run();
   icon.Visible=false;
  }
 }
}
"@
$owned = @{ Name=$name; AppRoot=$appRoot; PersistRoot=$persistRoot; CacheFiles=@(); Processes=@() }
$process = $null
$results = @()
function Save-Owned {
    $owned | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $work 'owned.json') -Encoding utf8NoBOM
}
function Entries {
    if (!(Test-Path -LiteralPath 'HKCU:/Control Panel/NotifyIconSettings')) { return @() }
    @(Get-ChildItem -LiteralPath 'HKCU:/Control Panel/NotifyIconSettings' |
        ForEach-Object { Get-ItemProperty -LiteralPath $_.PSPath } |
        Where-Object ExecutablePath -IEQ $exe | Select-Object PSChildName, ExecutablePath, InitialTooltip, IsPromoted)
}
function Stop-Probe {
    if ($null -ne $script:process) {
        [IO.File]::WriteAllText($script:stopFile, 'stop')
        if (!$script:process.WaitForExit(10000)) { throw 'Disposable probe failed to stop; see owned.json.' }
        if ($script:process.ExitCode -ne 0) { throw 'Disposable probe failed.' }
        $script:process.Dispose()
        $script:process = $null
    }
}
Save-Owned
$previousCache = $env:SCOOP_CACHE
try {
$env:SCOOP_CACHE = $cacheRoot
try {
    if ((Entries).Count) { throw 'Probe identity is not fresh.' }
    foreach ($version in '1', '2') {
        $build = (New-Item -ItemType Directory -Path (Join-Path $work "version-$version")).FullName
        $code = Join-Path $build 'Probe.cs'
        $binary = Join-Path $build 'Probe.exe'
        [IO.File]::WriteAllText($code, $source.Replace('VERSION', $version), [Text.UTF8Encoding]::new($false))
        & (Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319/csc.exe') /nologo /target:winexe /r:System.Windows.Forms.dll /r:System.Drawing.dll "/out:$binary" $code
        if ($LASTEXITCODE) { throw 'Probe compilation failed.' }
        $archive = Join-Path $work "probe-$version.zip"
        Compress-Archive -LiteralPath $binary -DestinationPath $archive
        $url = ([uri]$archive).AbsoluteUri
        $cache = Join-Path $cacheRoot ("$name#$version#" + ($url -replace '[^\w\.\-]+', '_'))
        if (Test-Path -LiteralPath $cache) { throw 'Probe cache collision.' }
        $owned.CacheFiles += $cache
        Save-Owned
        Copy-Item -LiteralPath $archive -Destination $cache
        @{ version=$version; description='Disposable notification identity verification'; license='Unlicense'; url=$url
           hash=(Get-FileHash -LiteralPath $archive).Hash; persist='runtime'
           post_install='Copy-Item -LiteralPath "$dir\Probe.exe" -Destination "$persist_dir\runtime\Probe.exe" -Force -ErrorAction Stop' } |
            ConvertTo-Json | Set-Content -LiteralPath $manifest -Encoding utf8NoBOM
        $log = Join-Path $work "scoop-$version.log"
        if ($version -eq '1') { & $scoop install $manifest --no-update-scoop *> $log }
        else { & $scoop update $name --independent *> $log }
        if ($LASTEXITCODE) { throw "Probe Scoop operation failed; see $log" }
        if ((Get-Item -LiteralPath $exe).VersionInfo.FileVersion -ne "$version.0.0.0") { throw 'Wrong probe binary.' }
        $marker = Join-Path $work "$version.txt"
        $stopFile = Join-Path $work "$version.stop"
        $process = Start-Process -FilePath $exe -ArgumentList ('"' + $marker + '"'), ('"' + $stopFile + '"') -WindowStyle Hidden -PassThru
        $owned.Processes += @{ Pid=$process.Id; Started=$process.StartTime.ToString('o'); Path=$exe }
        Save-Owned
        $deadline = [datetime]::UtcNow.AddSeconds(8)
        do { Start-Sleep -Milliseconds 200; $entries = Entries }
        while ((!(Test-Path -LiteralPath $marker) -or $entries.Count -ne 1) -and [datetime]::UtcNow -lt $deadline)
        if (!(Test-Path -LiteralPath $marker) -or $entries.Count -ne 1) { throw 'Explorer did not create exactly one probe registration.' }
        $actual = [IO.File]::ReadAllLines($marker)
        if ($actual[1] -ine $exe -or $entries[0].IsPromoted -eq 1) { throw 'Probe path or default promotion unexpected.' }
        $results += @{ PackageVersion=$version; CurrentTarget=(Get-Item -LiteralPath "$appRoot/current").Target
                       ProcessPath=$actual[1]; FileVersion=$actual[2]; Tray=$entries[0] }
        Stop-Probe
    }
    if ($results[0].Tray.PSChildName -ne $results[1].Tray.PSChildName -or
        $results[0].Tray.IsPromoted -ne $results[1].Tray.IsPromoted -or
        $results[0].CurrentTarget -eq $results[1].CurrentTarget) { throw 'Default identity changed across the Scoop update.' }
    @{ App=$App; Results=$results; SameEntry=$true; DefaultPromotionUnchanged=$true
       Method='Disposable WinForms identity; distinct PE versions and Scoop version directories; not the consumer binary.' } |
        ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $work 'result.json') -Encoding utf8NoBOM
} finally {
    Stop-Probe
    if (Test-Path -LiteralPath $appRoot) {
        & $scoop uninstall $name --purge *> (Join-Path $work 'uninstall.log')
        if ($LASTEXITCODE) { throw 'Probe uninstall failed; see owned.json.' }
    }
    if ((Test-Path -LiteralPath $appRoot) -or (Test-Path -LiteralPath $persistRoot)) { throw 'Probe package remains; see owned.json.' }
    foreach ($cache in $owned.CacheFiles) {
        if (![IO.Path]::GetFullPath($cache).StartsWith([IO.Path]::GetFullPath($cacheRoot) + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) { throw 'Unsafe probe cache path.' }
        if (Test-Path -LiteralPath $cache) { Remove-Item -LiteralPath $cache -Force }
    }
    if (Get-ChildItem -LiteralPath $cacheRoot -Force) { throw 'Probe cache has unexpected files; see owned.json.' }
    Remove-Item -LiteralPath $cacheRoot
    @{ PackageRemoved=$true; ProcessExited=$true; RegistryRetainedReadOnly=$true } |
        ConvertTo-Json | Set-Content -LiteralPath (Join-Path $work 'cleanup.json') -Encoding utf8NoBOM
}
} finally {
    $env:SCOOP_CACHE = $previousCache
}
