# In the Windows virtual machine: one smoke run of the catalog in the session of the logged-on
# user (through a scheduled task: a remote shell has no desktop), every page shown for 150 ms, with
# the frame times of the composition targets; copies the log to $Out.
#
#   run.ps1 -Name <name> -Out <directory on the shared volume> [-Env 'VAR=value VAR2=value'] [-Timeout 90]
#
# -Env must not be empty when given through prlctl, which drops an empty argument: pass a variable
# that does nothing, for example FERROUI_NOTHING=1.
param([Parameter(Mandatory)][string]$Name, [Parameter(Mandatory)][string]$Out, [string]$Env = '',
      [int]$Timeout = 90, [string]$Target = 'C:\ferroui-target', [string]$User = $env:USERNAME)
$local = Join-Path $Target 'vello-runs'; New-Item -ItemType Directory -Force -Path $local | Out-Null
$log = Join-Path $local "$Name.log"; $done = Join-Path $local "$Name.exit"
Remove-Item $done, $log -ErrorAction SilentlyContinue
$sets = ($Env -split ' ' | Where-Object { $_ } | ForEach-Object { "set $_" })
$exe = Join-Path $Target 'release\control-catalog-desktop.exe'
$wrapper = Join-Path $local "$Name.cmd"
Set-Content -Path $wrapper -Encoding ASCII -Value (@('@echo off', 'set FERROUI_FRAME_TIMES=1', 'set FERROUI_SMOKE_EXIT_MS=22000', 'set FERROUI_SMOKE_PAGES=150') + $sets + @("`"$exe`" > `"$log`" 2>&1", "echo %ERRORLEVEL% > `"$done`""))
$task = "FerroUI-vello-$Name"
cmd /c "schtasks /create /f /tn $task /tr `"cmd /c $wrapper`" /sc once /st 00:00 /ru $User /it > nul 2>&1"
cmd /c "schtasks /run /tn $task > nul 2>&1"
$waited = 0
while (-not (Test-Path $done) -and $waited -lt $Timeout) { Start-Sleep -Seconds 1; $waited++ }
cmd /c "schtasks /delete /f /tn $task > nul 2>&1"
if (Test-Path $log) { Copy-Item $log (Join-Path $Out "$Name.log") -Force }
if (Test-Path $done) { "$Name exit " + (Get-Content $done | Select-Object -First 1) } else { "$Name no result after $waited s" }
