# Tray verification

Linen supplies the native shell and renderer consumed by Earmuff and Homing through
Cargo and npm Git tags. Verify the consumer's pinned Linen revision as well as the
application version. A browser preview proves layout and simulated interaction;
Windows recordings prove the composed result; registry readback proves identity.
Keep those claims separate.

## Safe test state

Start with the consumer's browser preview, which uses its simulated state rather
than the native audio or Wi-Fi backend. From an Earmuff or Homing checkout with its
locked dependencies installed, run `vp run dev`, then open
`http://127.0.0.1:1420/?lang=en&theme=light`. Run one consumer at a time because both
use port 1420. Also inspect `?lang=zh&theme=dark`. Homing additionally accepts
`state=connected`, `disconnected`, `reconnecting`, `paused` or `unreadable`, and
`view=settings`. Reload to reset the mock; its controls do not save user settings.
Record the server's PID and port and stop that exact process tree when finished.

There is no native `--test` or mock-backend launch switch. Redirecting `APPDATA`
is not enough to make a native launch safe: Earmuff can still change real volume,
and configured Homing can reconnect a real network. Use the browser for arbitrary
state changes. Inspect an already-running, authorized native application without
changing controls, or use a separately authorized disposable Windows test environment.
Keep production TOML, startup tasks and running/stopped state intact.

Portable consumers use an empty `portable` file beside the running executable.
Settings then live in `data/<App>.toml`, with WebView2 data in `data/webview`.
For Scoop, these paths are under `persist/<app>/runtime`; the executable's version
directory links to that persisted runtime. Resolve the effective file before any
settings comparison. A portable marker does not disable audio or network writes:
use the browser for simulated states and never start a second configured resident
as a test. Removing the marker selects the legacy APPDATA file again.

For an authorized migration, stop the exact resident, back up its entire APPDATA
directory under system temp, and copy its settings bytes into the portable location
only when the destination is absent. Preserve an existing portable file, including
an invalid one. Record source/destination hashes and parsed settings, then restart
only a previously running app through its existing startup task. Gate any configured
network consumer against its already-connected target before restarting. Retain
the old APPDATA directory and backup; never remove either to prove path selection.
Compare the chosen settings, task XML, resident path and tray identity after the
update. The hardware-free `runtime_probe`/`test-runtime.ps1` check demonstrates
portable path selection and settings retention across two version-directory links.

For hardware-independent native shell checks, Linen's `material_probe` example
creates its own window and accepts light/dark themes without changing the desktop
theme. Its source and frame sampler are under
[`packages/linen-tray/tests/material-probe/`](../packages/linen-tray/tests/material-probe/).
The `material` CI job builds/runs this example and retains `native-material`
artifacts. The CI controller changes transparency settings inside its disposable
runner; do not run that controller on a user's desktop. A probe does not replace
checking the actual consumer's native flyout.

## Reach and inspect each state

On Windows, first obtain authorization for visible control and screen recording.
Stop foreground input as soon as unrelated pointer or keyboard input indicates the
user has returned. Leave any Windows security or permission prompt to its owner.
For browser work use a browser automation tool; for the native flyout use the
platform's desktop automation or manual input.

| Outcome | Actions | Required evidence |
| --- | --- | --- |
| Entry and dismissal | Click the visible tray icon; dismiss with Escape; reopen; check outside-click dismissal separately. In preview, reload opens it and Escape dismisses it. | Recording from before entry until after exit; no flash, residual surface or cropped card. |
| Settings expansion | Open the flyout, click the Settings gear, wait for settling, click it again; include a rapid reversal. Touch no sliders, protection switches, pause buttons or network selectors on a live app. | Recording plus frame samples: smooth upward growth, fixed footer/bottom edge, no outer scrollbar or overlapping pages. |
| Native material | Inspect collapsed and expanded cards, entry/exit and reversals over a textured background in light and dark test environments. Use preview `theme` parameters for simulated coverage. | Native recording/still frames showing tint/blur within the rounded card and transparent shadow canvas outside it; no material flash or overhang during transitions. Browser screenshots alone cannot prove native blur. |
| Tooltip | Hover the tray icon without clicking; wait for Explorer's tooltip. | Screenshot of the actual tooltip showing only translated status, with no application name or dash. Inspect all required states/languages in the mock or an authorized isolated environment. UIA labels can include Windows-added spacing and are supporting evidence. |
| Update identity | Capture the promoted baseline, perform the authorized normal Scoop update, then capture again. | Same stable executable path, key and promotion, no new registration, and direct evidence that the exposed icon remains visible. |

Preserve the user's taskbar and display settings. Use preconfigured test environments
for light/dark, DPI, reduced motion and effects variants; record unavailable variants
as gaps instead of changing a live machine's theme, scaling or transparency.

## Record motion and measure pacing

Capture only the authorized display/window, keep recordings private, and retain
relevant frames with the owning task rather than committing desktop captures.
With a desktop automation recorder, check that no other recording is active, start
video capture, perform the sequence above, stop it in `finally`, and verify the
finalized file decodes. Inspect frames around entry, settings reversal and exit,
including the border, shadow and area outside the card. End the owned automation
session after collecting the evidence. A 30-fps video can expose flashes and jumps;
it cannot prove refresh-rate frame pacing.

Use the existing material-probe sampler for pacing evidence. It records
`requestAnimationFrame` timestamps, displayed card heights and native resize
start/completion times. Its independent pacing phase deliberately avoids waiting
for screenshot/marker acknowledgements. Read the emitted `*.pacing.json`: compare
height-change count and maximum change gap to the measured refresh interval using
the assertions in [`probe.js`](../packages/linen-tray/tests/material-probe/probe.js).
Report startup delay separately from gaps during animation. Preserve raw samples,
application/library revisions and the environment; do not infer smoothness from
native call count or average FPS alone. Consumer Playwright motion tests cover
mock geometry and simulated slow replies; they do not measure DWM presentation.

## Read identity before and after a Scoop update

Use PowerShell 7 and a .NET Desktop runtime in the same interactive session as
Explorer. From the Linen checkout, create a new evidence directory under system
temp and read the application by executable name (without `.exe`):

```powershell
#requires -Version 7.0
$tempRoot = [Environment]::GetEnvironmentVariable('TEMP', 'Machine')
if (!$tempRoot) { $tempRoot = [Environment]::GetEnvironmentVariable('TEMP', 'User') }
$work = Join-Path $tempRoot ('tray-check-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work -ErrorAction Stop | Out-Null
$app = 'Earmuff' # Or Homing, or another Linen consumer's executable name.
pwsh -NoProfile -File scripts/verify/read-tray-state.ps1 -App $app -OutFile (Join-Path $work 'before.json')
```

`Entries` reads `HKCU\Control Panel\NotifyIconSettings` and includes every path for
that executable name. Select the active fixed `persist/<app>/runtime/<app>.exe`
path from the result, not an obsolete version-directory entry. Record its key
(`PSChildName`), `ExecutablePath` and `IsPromoted`. `Icons` includes all normal tray
buttons because a status-only tooltip has no stable app-name prefix. Correlate the
translated status and bounds with a screenshot. A visible button has
`Offscreen=false`, positive bounds inside `Taskbar`, and is present in the actual
notification area; a promoted registry flag alone is insufficient.

For the promoted case, use the user's existing exposed baseline. Never drag an
icon out to manufacture it. If the baseline is unpromoted or inactive, preserve
that state and report that promoted retention was not covered.

After saving the baseline, stop only the application's identified resident process,
then run its authorized update through the normal Scoop channel. Keep the actual
version transition and Scoop log. A same-version force reinstall provides narrower
evidence than a real version upgrade. Restart only an application that was active,
using its existing startup manager; leave an inactive consumer stopped. Preserve
TOML and startup task contents and compare them afterward. A network-dependent
consumer must satisfy its own repository's safe-start conditions before restarting.
These verification scripts neither stop apps nor deploy them.

```powershell
#requires -Version 7.0
# Keep $work and $app from the baseline shell.
$baseline = Get-Content -LiteralPath (Join-Path $work 'before.json') -Raw | ConvertFrom-Json
$stable = @($baseline.Entries | Where-Object { $_.ExecutablePath -match '[\\/]persist[\\/][^\\/]+[\\/]runtime[\\/]' })
if ($stable.Count -ne 1) { throw 'Resolve the active stable path before continuing.' }
$stableExe = $stable[0].ExecutablePath
pwsh -NoProfile -File scripts/verify/read-tray-state.ps1 -App $app -OutFile (Join-Path $work 'after.json')
pwsh -NoProfile -File scripts/verify/compare-tray-state.ps1 -App $app -Before (Join-Path $work 'before.json') -After (Join-Path $work 'after.json') -ExecutablePath $stableExe -OutFile (Join-Path $work 'comparison.json')
```

The comparator fails if the stable key/path/promotion or the application's complete
registration set changes. It retains before/after taskbar buttons for direct
visibility assessment; it does not guess identity from a translated label. A
previously exposed icon moving into overflow is a defect, even if the registry
comparison passes. Capture its post-update visible position and actual tooltip.

SSH Session 0 cannot inspect the interactive taskbar. If remote verification is
already authorized, run the reader as a short-lived, non-elevated Task Scheduler
action with `InteractiveToken` for the existing user. Set a working directory the
user can read, suppress its console, limit execution time and remove the exact
owned task in `finally`. For an inaccessible SSH-created script, pass its trusted
contents through `-EncodedCommand` rather than changing ACLs. Transfer original
UTF-8 JSON files; console decoding can corrupt translated labels. Report registry
and interactive-session gaps separately.

## Default identity without changing user preferences

Run the isolated probe only when disposable local Scoop installation is authorized:

```powershell
#requires -Version 7.0
pwsh -NoProfile -File scripts/verify/default-tray-identity.ps1 -App $app -OutDirectory (Join-Path $work 'default-identity')
```

This compiles a hardware-free WinForms notification icon under a new random package
identity. It scopes `SCOOP_CACHE` to its own output directory, restores the prior
environment in `finally`, seeds its own archive cache and performs an actual Scoop
install/update from version 1 to 2. `result.json` proves distinct PE versions and
version-directory targets with the same persisted executable path, registration
key and default promotion. `owned.json` records exact package/cache/process
identities; `cleanup.json` confirms graceful exit and package/cache removal.
The app parameter labels the experiment; this probe does not execute or prove the
consumer binary. Pair it with the real consumer update above.

The probe leaves Explorer's registration cache untouched, including its disposable
entry. Each run uses a fresh identity; never delete a registration to reuse one.

## Cleanup and report

Stop the owned recorder, preview server and disposable processes; close owned tabs
and remove exact temporary tasks. Retain relevant JSON, logs, video and inspected
frames with the task, then remove its exact temporary directories after confirming
the resolved paths and ownership. Keep operation backups until their retention
condition is met. If the probe fails, use `owned.json` to recover its exact package
and processes; never sweep packages, processes or caches by name.

Read the user's `NotifyIconSettings` entries only: never write, delete or promote
them. Leave taskbar, network, volume, display and security settings unchanged.
Report checked revisions, application versions, recording/frame-sample paths,
before/after key/path/promotion and visibility, cleanup results and every untested
variant. Passing a registry comparison is not a claim about visual animation, and
passing mock tests is not a claim about native material.
