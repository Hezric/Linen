#requires -Version 7.0
param(
    [Parameter(Mandatory)][ValidatePattern('^[a-zA-Z][a-zA-Z0-9_-]*$')][string]$App,
    [Parameter(Mandatory)][string]$Before,
    [Parameter(Mandatory)][string]$After,
    [Parameter(Mandatory)][string]$ExecutablePath,
    [Parameter(Mandatory)][string]$OutFile
)
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath $OutFile) { throw 'Choose a new output file to preserve earlier evidence.' }
$prior = Get-Content -LiteralPath $Before -Raw | ConvertFrom-Json
$current = Get-Content -LiteralPath $After -Raw | ConvertFrom-Json
foreach ($snapshot in $prior, $current) {
    if ($snapshot.App -ine $App -or $snapshot.InputPerformed -ne $false) { throw 'Mismatched or non-read-only snapshot.' }
}
$old = @($prior.Entries | Where-Object ExecutablePath -IEQ $ExecutablePath)
$new = @($current.Entries | Where-Object ExecutablePath -IEQ $ExecutablePath)
if ($old.Count -ne 1 -or $new.Count -ne 1) { throw 'Expected exactly one registration for the stable executable in both snapshots.' }
function Registration($entry) {
    @($entry.PSChildName, $entry.ExecutablePath.ToLowerInvariant(), $entry.IsPromoted) | ConvertTo-Json -Compress
}
$oldAll = @($prior.Entries | ForEach-Object { Registration $_ } | Sort-Object)
$newAll = @($current.Entries | ForEach-Object { Registration $_ } | Sort-Object)
if ((Registration $old[0]) -cne (Registration $new[0]) -or
    ($oldAll | ConvertTo-Json -Compress) -cne ($newAll | ConvertTo-Json -Compress)) {
    throw 'Notification registration, executable path or promotion changed.'
}
@{ App=$App; Before=$prior.At; After=$current.At; Entry=$new[0]; SameRegistrations=$true
   BeforeTaskbar=$prior.Taskbar; AfterTaskbar=$current.Taskbar
   BeforeButtons=$prior.Icons; AfterButtons=$current.Icons
   VisibilityAssessment='Correlate the translated status and bounds with screenshots; registration alone does not prove visibility.' } |
    ConvertTo-Json -Depth 8 | Set-Content -LiteralPath $OutFile -Encoding utf8NoBOM
