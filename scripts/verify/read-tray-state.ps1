#requires -Version 7.0
param(
    [Parameter(Mandatory)][ValidatePattern('^[a-zA-Z][a-zA-Z0-9_-]*$')][string]$App,
    [Parameter(Mandatory)][string]$OutFile
)
$ErrorActionPreference = 'Stop'
if (!$IsWindows) { throw 'Run in the interactive Windows session containing Explorer.' }
if (Test-Path -LiteralPath $OutFile) { throw 'Choose a new output file to preserve earlier evidence.' }
$desktop = Get-ChildItem -LiteralPath (Join-Path ${env:ProgramFiles} 'dotnet/shared/Microsoft.WindowsDesktop.App') -Directory |
    Sort-Object { [version]$_.Name } -Descending | Select-Object -First 1 -ExpandProperty FullName
foreach ($assembly in 'WindowsBase.dll', 'UIAutomationTypes.dll', 'UIAutomationClient.dll') {
    Add-Type -Path (Join-Path $desktop $assembly)
}
if (!('LinenTrayRead' -as [type])) {
    Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class LinenTrayRead {
    [DllImport("user32.dll", CharSet=CharSet.Unicode)]
    public static extern IntPtr FindWindow(string className, string title);
}
"@
}
$window = [LinenTrayRead]::FindWindow('Shell_TrayWnd', $null)
if ($window -eq [IntPtr]::Zero) { throw 'No taskbar in this Windows session.' }
$taskbar = [Windows.Automation.AutomationElement]::FromHandle($window)
$rect = $taskbar.Current.BoundingRectangle
$buttons = $taskbar.FindAll([Windows.Automation.TreeScope]::Descendants,
    [Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty,
        [Windows.Automation.ControlType]::Button))
$icons = @($buttons | ForEach-Object {
    $current = $_.Current
    if ($current.ClassName -eq 'SystemTray.NormalButton') {
        @{ Name=$current.Name; HelpText=$current.HelpText; Offscreen=$current.IsOffscreen; Class=$current.ClassName
           X=$current.BoundingRectangle.X; Y=$current.BoundingRectangle.Y
           Width=$current.BoundingRectangle.Width; Height=$current.BoundingRectangle.Height }
    }
})
$registry = 'HKCU:/Control Panel/NotifyIconSettings'
$entries = if (Test-Path -LiteralPath $registry) {
    @(Get-ChildItem -LiteralPath $registry | ForEach-Object { Get-ItemProperty -LiteralPath $_.PSPath } |
        Where-Object { [IO.Path]::GetFileName($_.ExecutablePath) -ieq "$App.exe" } |
        Select-Object PSChildName, ExecutablePath, InitialTooltip, IsPromoted)
} else { @() }
@{ App=$App; At=(Get-Date).ToString('o'); Session=[Diagnostics.Process]::GetCurrentProcess().SessionId
   Taskbar=@{ X=$rect.X; Y=$rect.Y; Width=$rect.Width; Height=$rect.Height }
   Icons=$icons; Entries=@($entries); InputPerformed=$false } |
    ConvertTo-Json -Depth 7 | Set-Content -LiteralPath $OutFile -Encoding utf8NoBOM
