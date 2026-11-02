#requires -Version 7.0
param([Parameter(Mandatory)][string]$OutFile)
$ErrorActionPreference='Stop'
$desktop=Get-ChildItem -LiteralPath 'C:/Program Files/dotnet/shared/Microsoft.WindowsDesktop.App' -Directory|Sort-Object {[version]$_.Name} -Descending|Select-Object -First 1 -ExpandProperty FullName
foreach($assembly in 'WindowsBase.dll','UIAutomationTypes.dll','UIAutomationClient.dll'){Add-Type -Path (Join-Path $desktop $assembly)}
Add-Type -TypeDefinition @"
using System;using System.Runtime.InteropServices;
public static class G11TrayRead { [DllImport("user32.dll",CharSet=CharSet.Unicode)]public static extern IntPtr FindWindow(string c,string t); }
"@
$window=[G11TrayRead]::FindWindow('Shell_TrayWnd',$null)
if($window -eq [IntPtr]::Zero){throw 'No taskbar in this Windows session'}
$taskbar=[Windows.Automation.AutomationElement]::FromHandle($window)
$rect=$taskbar.Current.BoundingRectangle
$elements=$taskbar.FindAll([Windows.Automation.TreeScope]::Descendants,[Windows.Automation.PropertyCondition]::new([Windows.Automation.AutomationElement]::ControlTypeProperty,[Windows.Automation.ControlType]::Button))
$icons=@($elements|ForEach-Object {$c=$_.Current;if($c.ClassName -eq 'SystemTray.NormalButton'){@{Name=$c.Name;HelpText=$c.HelpText;Offscreen=$c.IsOffscreen;Class=$c.ClassName;X=$c.BoundingRectangle.X;Y=$c.BoundingRectangle.Y;Width=$c.BoundingRectangle.Width;Height=$c.BoundingRectangle.Height}}})
$entries=@(Get-ChildItem -LiteralPath 'HKCU:/Control Panel/NotifyIconSettings'|ForEach-Object {Get-ItemProperty -LiteralPath $_.PSPath}|Where-Object {$_.ExecutablePath -match '(earmuff|homing)\.exe$'}|Select-Object PSChildName,ExecutablePath,InitialTooltip,IsPromoted)
@{Host=$env:COMPUTERNAME;At=(Get-Date).ToString('o');Session=[Diagnostics.Process]::GetCurrentProcess().SessionId;Taskbar=@{Hwnd=$window.ToInt64();X=$rect.X;Y=$rect.Y;Width=$rect.Width;Height=$rect.Height};Icons=$icons;Entries=$entries;InputPerformed=$false}|ConvertTo-Json -Depth 7|Set-Content -LiteralPath $OutFile -Encoding utf8NoBOM
