# Tray update verification

Run this check before and after an authorized Scoop update. It reads Explorer's
actual taskbar buttons and the current user's notification-icon registrations;
it never moves the pointer, opens a flyout, or changes tray preferences.
Use PowerShell 7 in the same interactive Windows session as Explorer, with a
.NET Desktop runtime installed. Preserve the application's settings and startup
manager, and stop its exact resident process before updating.

```powershell
#requires -Version 7.0
$tempRoot = [Environment]::GetEnvironmentVariable('TEMP', 'Machine')
if (!$tempRoot) { $tempRoot = [Environment]::GetEnvironmentVariable('TEMP', 'User') }
$work = Join-Path $tempRoot 'tray-update-check'
New-Item -ItemType Directory -Path $work -ErrorAction Stop | Out-Null
./scripts/read-tray-state.ps1 -OutFile (Join-Path $work 'before.json')
```

`Icons` includes all normal notification-area buttons. Identify a consumer using
its translated status and before/after position; status-only labels have no stable
application-name prefix. In `Entries`, select each application's fixed `persist/runtime` path.
Record its `PSChildName`, `ExecutablePath`, and `IsPromoted`. In `Icons`, confirm
that a previously exposed icon is a `SystemTray.NormalButton`, has
`Offscreen=false`, and has nonempty bounds inside `Taskbar`. Preserve an
unpromoted or inactive baseline; do not drag an icon out merely to make a check
pass. An inactive application remains stopped.

Update through its normal Scoop channel, restart an active application through
its existing startup manager, then run the script again to `after.json` in the
same directory. The fixed registration key and path must be reused, with no new
entry for the application and the same promotion value. A previously exposed
icon must still appear among the visible taskbar buttons. Moving it into overflow
is a defect. UI Automation names can include an application label supplied by
Windows in addition to the application's status-only tooltip.

SSH runs in Session 0 and cannot inspect the interactive taskbar directly. Use a
short-lived, non-elevated Task Scheduler action with `InteractiveToken` for the
existing user. Keep its working directory in the user's profile, suppress the
helper console, use an execution time limit, record its exact identity, and
remove it in `finally`. If the interactive user cannot read an SSH-created TEMP
script, load the trusted script in the SSH process and pass its script block
through PowerShell's `-EncodedCommand`; keep the user's ACLs unchanged. Do not
substitute a new login, foreground input, or an overflow-menu click.

Keep before/after evidence with the owning task, and remove its temporary task
and output directory when the evidence has been retained. Report any unavailable
interactive session or unreadable button separately from registry observations.
