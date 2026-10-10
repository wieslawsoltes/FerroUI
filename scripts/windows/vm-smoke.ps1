<#
.SYNOPSIS
  Builds the Windows platform backend on this machine, runs its tests and the smoke run of its
  example in each rendering mode, and writes a plain-text report.

.DESCRIPTION
  For a Windows machine without a person: a virtual machine or a build host
  (docs/porting/win32-platform.md, section 10.4). The script installs nothing. It checks for the
  Rust toolchain, the MSVC build tools and LLVM (libclang, which the build of ANGLE needs) and says
  what is missing; then it builds ferroui-win32 and its example for the architecture of the
  machine with the build output in the directory given, runs the tests of the crate, and runs
  `win32_window --smoke` once for each rendering mode.

  A window needs a desktop. A process started by a service or by the tools of a virtual machine
  host runs in session 0, which has none: the smoke runs then show what works without a desktop
  and fail the rest. With -InteractiveUser the smoke runs are started in the session of that
  logged-on user through a scheduled task that exists for the length of the run.

.PARAMETER TargetDir
  The directory of the build output (CARGO_TARGET_DIR). Required: the output of a full build is
  several gigabytes, and it is the caller who knows which disk has the room. It has to be on a
  disk of the machine: on a folder shared from a virtual machine host rustc fails to build its
  archives ("failed to remove temporary directory: The parameter is incorrect").

.PARAMETER Source
  The root of the repository. Default: two directories above this script.

.PARAMETER Report
  The file the report is written to. Default: vm-smoke-report.txt in the target directory.

.PARAMETER LogDir
  The directory of the logs of the steps. Default: vm-smoke-logs beside the report (so a report on
  a shared folder has its logs there, where the host reads them).

.PARAMETER SmokeArguments
  Further arguments of the smoke runs, for example --angle-probe.

.PARAMETER Modes
  The rendering modes of the smoke runs. Default: software, and angle when ANGLE is built.

.PARAMETER NoAngle
  Build without the default feature `angle` only (no ANGLE, no LLVM needed).

.PARAMETER Jobs
  The number of parallel jobs of cargo. Default: the choice of cargo.

.PARAMETER InteractiveUser
  The name of a logged-on user in whose session the smoke runs are started.

.PARAMETER CargoHome, RustupHome, LibclangPath
  Where the toolchain and libclang are, when the environment of the process does not say.

.PARAMETER Desktop
  Also build ferroui-desktop and run its example hello_window for two seconds (Skia: a published
  binary of Skia for the target is needed, or Skia is built from source, which takes an hour):
  with the default options, with each rendering mode alone, and with ANGLE on the UI thread.
  Then build and run upstream's integration tests of windows (tests\FerroUI.IntegrationTests.Win32:
  windows of the window class on the desktop, in every state and with every kind of decorations),
  the test of what a window presents once more with each rendering mode alone, and the picture run
  of the catalog (control-catalog-desktop with FERROUI_SMOKE_SCREENSHOTS) in both rendering modes;
  the pictures are copied to vm-smoke-screenshots beside the report.

.PARAMETER AllowUnmarkedSource
  Run although the sources carry no mark of a completed sync. By default the script refuses a
  source tree without the file .vm-sync-commit (written last by scripts/windows/vm-sync.sh, with
  the commit that was synced) or with the file .vm-sync-in-progress (a sync that is running or was
  interrupted): a run once built a tree that was half synced. A checkout with Git needs no mark:
  its commit is asked of Git.

.PARAMETER HelloRepeat
  How many times the run of hello_window with the default options is made (to look for a failure
  that does not happen every time). Default: 1.

.EXAMPLE
  powershell -NoProfile -ExecutionPolicy Bypass -File scripts\windows\vm-smoke.ps1 -TargetDir D:\ferroui-target -Jobs 3

.EXAMPLE
  With the sources on a folder shared from a virtual machine host: start the script through the
  drive letter the share is mapped to (the letter `net use` shows for the share; it is not the same
  on every machine or after every start), never through its UNC path, which cmd refuses as a
  working directory. <S> stands for that letter:

  powershell -NoProfile -ExecutionPolicy Bypass -File <S>:\ferroui-vm-windows\src\scripts\windows\vm-smoke.ps1 -TargetDir C:\ferroui-target -Jobs 3 -InteractiveUser <user> -Desktop -Report <S>:\ferroui-vm-windows\vm-smoke-report.txt
#>
param(
    [Parameter(Mandatory = $true)][string]$TargetDir,
    [string]$Source = '',
    [string]$Report = '',
    [string]$LogDir = '',
    [string]$SmokeArguments = '',
    [string[]]$Modes = @(),
    [switch]$NoAngle,
    [int]$Jobs = 0,
    [string]$InteractiveUser = '',
    [string]$CargoHome = '',
    [string]$RustupHome = '',
    [string]$LibclangPath = '',
    [switch]$Desktop,
    [int]$HelloRepeat = 1,
    [switch]$SkipBuild,
    [switch]$SkipTests,
    [switch]$AllowUnmarkedSource
)

# Native tools write progress to the error stream; that is not a failure.
$ErrorActionPreference = 'Continue'

if ($CargoHome) { $env:CARGO_HOME = $CargoHome }
if ($RustupHome) { $env:RUSTUP_HOME = $RustupHome }
if ($LibclangPath) { $env:LIBCLANG_PATH = $LibclangPath }
if ($env:CARGO_HOME -and (Test-Path (Join-Path $env:CARGO_HOME 'bin'))) {
    $env:Path = (Join-Path $env:CARGO_HOME 'bin') + ';' + $env:Path
}

# The root of the repository: two directories above this script. ($PSScriptRoot is empty while the
# parameters are bound when the script is started with -File from a mapped drive.)
if (-not $Source) {
    $scriptPath = $MyInvocation.MyCommand.Path
    if (-not $scriptPath) { $scriptPath = $PSCommandPath }
    $Source = Split-Path -Parent (Split-Path -Parent (Split-Path -Parent $scriptPath))
}

New-Item -ItemType Directory -Force -Path $TargetDir | Out-Null
$TargetDir = (Resolve-Path $TargetDir).Path
if (-not $Report) { $Report = Join-Path $TargetDir 'vm-smoke-report.txt' }
if (-not $LogDir) { $LogDir = Join-Path (Split-Path -Parent $Report) 'vm-smoke-logs' }
$logs = $LogDir
New-Item -ItemType Directory -Force -Path $logs | Out-Null

$lines = New-Object System.Collections.Generic.List[string]
$failed = New-Object System.Collections.Generic.List[string]
function Say([string]$text) {
    $lines.Add($text)
    # Not the output stream: a function that reports would return the text with its result.
    Write-Host $text
    [System.IO.File]::WriteAllLines($Report, $lines)
}

# Runs a command line through cmd with both streams in a log; returns the exit code.
function Run([string]$name, [string]$commandLine) {
    $log = Join-Path $logs "$name.log"
    $started = Get-Date
    cmd /c "$commandLine > `"$log`" 2>&1" | Out-Null
    $code = $LASTEXITCODE
    $seconds = [int]((Get-Date) - $started).TotalSeconds
    Say ("{0}: exit code {1}, {2} s, log {3}" -f $name, $code, $seconds, $log)
    return $code
}

function Tail([string]$name, [int]$count, [string]$pattern = '') {
    $log = Join-Path $logs "$name.log"
    if (-not (Test-Path $log)) { return }
    $text = Get-Content $log
    if ($pattern) { $text = $text | Where-Object { $_ -match $pattern } }
    $text | Select-Object -Last $count | ForEach-Object { Say "    $_" }
}

Say "FerroUI Windows platform backend: build, tests and smoke runs"
Say ("date: {0}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'))
$os = Get-CimInstance Win32_OperatingSystem
Say ("system: {0} {1}, {2}" -f $os.Caption, $os.Version, $env:PROCESSOR_ARCHITECTURE)
$session = [System.Diagnostics.Process]::GetCurrentProcess().SessionId
Say ("this process: user {0}, session {1}, interactive {2}" -f [System.Security.Principal.WindowsIdentity]::GetCurrent().Name, $session, [Environment]::UserInteractive)
Say "source: $Source"
if ($Source.StartsWith('\\')) {
    # The steps run through cmd, which refuses a UNC path as its working directory.
    Say "MISSING: the source is a UNC path. Start the script through the drive letter the share is mapped to (the letter 'net use' shows for it)."
    exit 2
}
Say "target directory: $TargetDir"

# ---- which sources these are ------------------------------------------------------------------
# A tree on a shared folder is copied there from the host (scripts/windows/vm-sync.sh), and the
# machine has no Git: the copy says which commit it is, in a file that is written when the copy
# is complete. A tree that is being copied, or whose copy was interrupted, is not built.
$syncCommit = Join-Path $Source '.vm-sync-commit'
$syncRunning = Join-Path $Source '.vm-sync-in-progress'
if (Test-Path $syncRunning) {
    Say ("REFUSED: the sources are being synced, or a sync was interrupted ({0}, {1}). Sync again and start the script when the sync has ended." -f $syncRunning, ((Get-Content $syncRunning -ErrorAction SilentlyContinue) -join ' '))
    exit 3
}
if (Test-Path $syncCommit) {
    Say ("sources: commit {0} (synced {1})" -f ((Get-Content $syncCommit) -join ' '), (Get-Item $syncCommit).LastWriteTime.ToString('yyyy-MM-dd HH:mm:ss'))
} elseif ((Test-Path (Join-Path $Source '.git')) -and (Get-Command git.exe -ErrorAction SilentlyContinue)) {
    $head = (cmd /c "git -C `"$Source`" rev-parse HEAD 2>&1") -join ' '
    $dirty = @(cmd /c "git -C `"$Source`" status --porcelain 2>&1").Count
    Say "sources: commit $head of a checkout ($dirty changed file(s))"
} elseif ($AllowUnmarkedSource) {
    Say "sources: UNKNOWN COMMIT: the tree has no mark of a completed sync (.vm-sync-commit); run because of -AllowUnmarkedSource"
} else {
    Say "REFUSED: the sources have no mark of a completed sync ($syncCommit). Sync them with scripts/windows/vm-sync.sh on the host, which writes the commit there when it is done, or pass -AllowUnmarkedSource."
    exit 3
}

# ---- nobody else builds into the target directory ---------------------------------------------
# cargo holds the file .cargo-lock of a profile directory open while it builds into it. A second
# cargo would wait for it without a word and then build on top of what the first one left, which
# a report cannot tell from its own build: the script does not start then.
$heldLocks = New-Object System.Collections.Generic.List[string]
$lockFiles = @(Get-ChildItem -Path $TargetDir -Filter '.cargo-lock' -Recurse -Depth 2 -Force -ErrorAction SilentlyContinue)
foreach ($lockFile in $lockFiles) {
    try {
        # No sharing: fails while any other process has the file open.
        $handle = [System.IO.File]::Open($lockFile.FullName, [System.IO.FileMode]::Open, [System.IO.FileAccess]::ReadWrite, [System.IO.FileShare]::None)
        $handle.Close()
    } catch {
        $heldLocks.Add($lockFile.FullName)
    }
}
if ($heldLocks.Count -gt 0) {
    $builders = @(Get-Process -Name cargo, rustc -ErrorAction SilentlyContinue | ForEach-Object { "{0} (process {1}, started {2})" -f $_.ProcessName, $_.Id, $_.StartTime.ToString('HH:mm:ss') })
    Say ("REFUSED: another build holds the lock of the target directory: {0}. Running: {1}. Wait for it to end, or stop it, and start the script again." -f ($heldLocks -join ', '), ($builders -join '; '))
    exit 4
}
Say ("build lock: free ({0} lock file(s) looked at)" -f $lockFiles.Count)

# ---- what the build needs ---------------------------------------------------------------------
Say ""
Say "-- tools"
$missing = New-Object System.Collections.Generic.List[string]

$cargo = Get-Command cargo.exe -ErrorAction SilentlyContinue
if ($cargo) {
    Say ("cargo: {0}" -f (cmd /c "cargo --version 2>&1"))
    Say ("rustc: {0}" -f ((cmd /c "rustc -vV 2>&1") -join '; '))
} else {
    $missing.Add("the Rust toolchain (rustup from https://rustup.rs with the MSVC host of this architecture; cargo.exe is not on the path and CARGO_HOME is '$env:CARGO_HOME')")
}

$vswhere = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio\Installer\vswhere.exe'
$msvc = @()
if (Test-Path $vswhere) {
    $msvc = @(& $vswhere -products * -latest -property installationPath 2>$null)
}
foreach ($root in @('C:\BuildTools', 'C:\BuildTools2026')) {
    if (Test-Path (Join-Path $root 'VC\Tools\MSVC')) { $msvc += $root }
}
$msvc = @($msvc | Where-Object { $_ } | Select-Object -Unique)
if ($msvc.Count -gt 0) {
    foreach ($root in $msvc) {
        $versions = (Get-ChildItem (Join-Path $root 'VC\Tools\MSVC') -ErrorAction SilentlyContinue | ForEach-Object { $_.Name }) -join ', '
        Say "MSVC build tools: $root (VC tools $versions)"
    }
} else {
    $missing.Add("the MSVC build tools (Visual Studio Build Tools with the C++ workload for this architecture, and a Windows SDK)")
}

$libclang = $null
foreach ($directory in @($env:LIBCLANG_PATH, (Join-Path $env:ProgramFiles 'LLVM\bin'))) {
    if ($directory -and (Test-Path (Join-Path $directory 'libclang.dll'))) { $libclang = $directory; break }
}
$angle = -not $NoAngle
if ($libclang) {
    $env:LIBCLANG_PATH = $libclang
    $clang = Join-Path $libclang 'clang.exe'
    $version = if (Test-Path $clang) { (cmd /c "`"$clang`" --version 2>&1" | Select-Object -First 1) } else { 'clang.exe not found beside it' }
    Say "LLVM (libclang): $libclang ($version)"
} elseif ($angle) {
    $missing.Add("LLVM for libclang.dll (the build of ANGLE generates bindings with it: an LLVM release for this architecture, or pass -NoAngle to build without the rendering mode ANGLE)")
}

if ($missing.Count -gt 0) {
    Say ""
    Say "MISSING: the build cannot start. Install:"
    foreach ($item in $missing) { Say "  - $item" }
    exit 2
}

$env:CARGO_TARGET_DIR = $TargetDir
$env:CARGO_PROFILE_DEV_DEBUG = '0'
$env:CARGO_PROFILE_TEST_DEBUG = '0'
$env:CARGO_INCREMENTAL = '0'
$env:RUST_BACKTRACE = '1'
$jobsArgument = if ($Jobs -gt 0) { "-j$Jobs" } else { '' }
Set-Location $Source

# ---- build ------------------------------------------------------------------------------------
Say ""
Say "-- build"
if (-not $SkipBuild) {
    # First without the default feature `angle`: the backend with software rendering alone.
    if ((Run 'build' "cargo build --locked $jobsArgument -p ferroui-win32 --no-default-features --examples") -ne 0) {
        $failed.Add('build')
        Tail 'build' 40
    }
    if ($angle) {
        if ((Run 'build-angle' "cargo build --locked $jobsArgument -p ferroui-win32 --examples") -ne 0) {
            $failed.Add('build with ANGLE')
            Tail 'build-angle' 60
            $angle = $false
        }
    }
}

# ---- tests ------------------------------------------------------------------------------------
Say ""
Say "-- tests"
if (-not $SkipTests -and -not $failed.Contains('build')) {
    $testFeatures = if ($angle) { '' } else { '--no-default-features' }
    if ((Run 'test' "cargo test --locked $jobsArgument -p ferroui-win32 $testFeatures --no-fail-fast") -ne 0) { $failed.Add('tests') }
    Tail 'test' 6 'test result|FAILED|panicked'
}

# ---- smoke runs -------------------------------------------------------------------------------
if ($Modes.Count -eq 0) {
    $Modes = @('software')
    if ($angle) { $Modes += 'angle' }
}
$example = Join-Path $TargetDir 'debug\examples\win32_window.exe'

# Starts a command line in the session of a logged-on user and waits for it: a scheduled task
# that is created, run once and deleted.
function Run-Interactive([string]$name, [string]$commandLine, [int]$timeout = 180) {
    # The files of the task are on the disk of the machine: a drive that is mapped for the
    # account of this process (a folder shared from a virtual machine host) need not exist in the
    # session of the user. The log is copied to the log directory afterwards.
    $local = Join-Path $TargetDir 'vm-smoke-interactive'
    New-Item -ItemType Directory -Force -Path $local | Out-Null
    $log = Join-Path $local "$name.log"
    $done = Join-Path $local "$name.exit"
    Remove-Item $done, $log -ErrorAction SilentlyContinue
    $wrapper = Join-Path $local "$name.cmd"
    Set-Content -Path $wrapper -Encoding ASCII -Value @(
        '@echo off',
        "set RUST_BACKTRACE=1",
        "$commandLine > `"$log`" 2>&1",
        "echo %ERRORLEVEL% > `"$done`""
    )
    $task = "FerroUI-vm-smoke-$name"
    cmd /c "schtasks /create /f /tn $task /tr `"cmd /c $wrapper`" /sc once /st 00:00 /ru $InteractiveUser /it > nul 2>&1" | Out-Null
    if ($LASTEXITCODE -ne 0) { Say "${name}: the scheduled task could not be created for user $InteractiveUser"; return 1 }
    cmd /c "schtasks /run /tn $task > nul 2>&1" | Out-Null
    $waited = 0
    while (-not (Test-Path $done) -and $waited -lt $timeout) { Start-Sleep -Seconds 1; $waited++ }
    cmd /c "schtasks /delete /f /tn $task > nul 2>&1" | Out-Null
    if (Test-Path $log) { Copy-Item $log (Join-Path $logs "$name.log") -Force }
    if (-not (Test-Path $done)) { Say "${name}: no result after $waited s (is $InteractiveUser logged on?)"; return 1 }
    $code = [int]((Get-Content $done | Select-Object -First 1).Trim())
    Say ("{0}: exit code {1}, in the session of {2}, log {3}" -f $name, $code, $InteractiveUser, (Join-Path $logs "$name.log"))
    return $code
}

Say ""
Say "-- smoke runs"
if (-not (Test-Path $example)) {
    Say "the example was not built: $example"
    $failed.Add('smoke runs (no example)')
} else {
    foreach ($mode in $Modes) {
        $name = "smoke-$mode"
        $commandLine = "`"$example`" --smoke --rendering $mode $SmokeArguments"
        $code = if ($InteractiveUser) { Run-Interactive $name $commandLine } else { Run $name $commandLine }
        if ($code -ne 0) { $failed.Add("smoke run ($mode)") }
        Tail $name 200 '^\[(FAIL|info| ok )\]|^smoke run|panicked|^Windows |^Screens|^  '
    }
    # Through ANGLE with each composition mode that presents through a surface of its own.
    if ($Modes -contains 'angle') {
        foreach ($composition in @('dcomp', 'winui', 'dxgi')) {
            $name = "smoke-angle-$composition"
            $commandLine = "`"$example`" --smoke --rendering angle --composition $composition"
            $code = if ($InteractiveUser) { Run-Interactive $name $commandLine } else { Run $name $commandLine }
            if ($code -ne 0) { $failed.Add("smoke run (angle, $composition)") }
            Tail $name 200 '^\[(FAIL|info| ok )\]|^smoke run|panicked|^Windows |^Screens|^  '
        }
    }
}

# ---- the desktop entry point ------------------------------------------------------------------
if ($Desktop) {
    Say ""
    Say "-- ferroui-desktop"
    if ((Run 'desktop-build' "cargo build --locked $jobsArgument -p ferroui-desktop --example hello_window") -ne 0) {
        $failed.Add('ferroui-desktop build')
        Tail 'desktop-build' 40
    } else {
        $hello = Join-Path $TargetDir 'debug\examples\hello_window.exe'
        # The default options (ANGLE first, the render thread), then each rendering mode alone,
        # and ANGLE on the UI thread: which of them works says where a failure is.
        $variants = @(
            @('default', ''),
            @('software', 'set FERROUI_SMOKE_RENDERING=software&& '),
            @('angle', 'set FERROUI_SMOKE_RENDERING=angle&& '),
            @('angle-ui-thread', 'set FERROUI_SMOKE_RENDERING=angle&& set FERROUI_SMOKE_RENDER_ON_UI_THREAD=1&& ')
        )
        for ($i = 2; $i -le $HelloRepeat; $i++) { $variants += ,@("default-$i", '') }
        foreach ($variant in $variants) {
            $name = "hello-window-$($variant[0])"
            $commandLine = "set FERROUI_SMOKE_EXIT_MS=2000&& set FERROUI_SMOKE_LOG=1&& $($variant[1])`"$hello`""
            $code = if ($InteractiveUser) { Run-Interactive $name $commandLine } else { Run $name $commandLine }
            if ($code -ne 0) { $failed.Add("hello_window ($($variant[0]))") }
            # A repetition that passed is one line of the report.
            if ($code -ne 0 -or $variant[0] -notmatch '^default-') { Tail $name 30 '^(?!WARN: driver_utils)' }
        }
    }

    # Upstream's integration tests of windows: one binary without the test harness, which
    # prints every test with its result. It needs a desktop, like the smoke runs.
    Say ""
    Say "-- integration tests of windows"
    if ((Run 'integration-build' "cargo test --locked $jobsArgument -p ferroui-integration-tests-win32 --test integration --no-run") -ne 0) {
        $failed.Add('integration tests build')
        Tail 'integration-build' 40
    } else {
        $binary = Get-ChildItem (Join-Path $TargetDir 'debug\deps') -Filter 'integration-*.exe' |
            Sort-Object LastWriteTime -Descending | Select-Object -First 1
        if (-not $binary) {
            Say "the test binary was not found under $TargetDir\debug\deps"
            $failed.Add('integration tests (no binary)')
        } else {
            $commandLine = "`"$($binary.FullName)`""
            $code = if ($InteractiveUser) { Run-Interactive 'integration' $commandLine 900 } else { Run 'integration' $commandLine }
            if ($code -ne 0) { $failed.Add('integration tests') }
            Tail 'integration' 80 'FAILED|^test result|^running|panicked|^failures|^    '
            # What a window presents reaches the edges of its client area, with each rendering
            # mode alone (the run above had the default options).
            foreach ($mode in @('software', 'angle')) {
                $name = "integration-presented-$mode"
                $commandLine = "set FERROUI_SMOKE_RENDERING=$mode&& `"$($binary.FullName)`" presented_frame_tests"
                $code = if ($InteractiveUser) { Run-Interactive $name $commandLine 300 } else { Run $name $commandLine }
                if ($code -ne 0) { $failed.Add("presented frames ($mode)") }
                Tail $name 20 'FAILED|^test |^test result|panicked|^    '
            }
            foreach ($composition in @('dcomp', 'winui', 'dxgi')) {
                $name = "integration-presented-angle-$composition"
                $commandLine = "set FERROUI_SMOKE_RENDERING=angle&& set FERROUI_SMOKE_COMPOSITION=$composition&& `"$($binary.FullName)`" presented_frame_tests"
                $code = if ($InteractiveUser) { Run-Interactive $name $commandLine 300 } else { Run $name $commandLine }
                if ($code -ne 0) { $failed.Add("presented frames (angle, $composition)") }
                Tail $name 20 'FAILED|^test |^test result|panicked|^    '
            }
        }
    }

    # The catalog: its desktop host shows the home page and a handful of others and writes two
    # pictures of each (the frame of the framework, and the client area of the window as the
    # system composed it), once with software rendering and once through ANGLE. The pictures
    # are copied beside the report.
    Say ""
    Say "-- pictures of the catalog"
    if ((Run 'catalog-build' "cargo build --locked $jobsArgument -p control-catalog-desktop") -ne 0) {
        $failed.Add('catalog build')
        Tail 'catalog-build' 40
    } else {
        $catalog = Join-Path $TargetDir 'debug\control-catalog-desktop.exe'
        $pictures = Join-Path $TargetDir 'vm-smoke-screenshots'
        # The third run is of a window whose client area extends into its frame (the title bar
        # and the caption buttons drawn by the framework), through ANGLE, with the page of the
        # window customizations among its pages.
        # The runs named winui-* are through Windows.UI.Composition with a window that asks for
        # that transparency level (acrylic blur, mica): what is behind the window shows through,
        # blurred, where the system has the effect. The capture of the window shows what the
        # system composed of the window itself; the backdrop is the desktop's, so the picture
        # to look at is the screen of the machine.
        foreach ($mode in @('software', 'angle', 'extended', 'dcomp', 'winui', 'winui-acrylic', 'winui-mica')) {
            $name = "catalog-$mode"
            $directory = Join-Path $pictures $mode
            New-Item -ItemType Directory -Force -Path $directory | Out-Null
            Get-ChildItem $directory -Filter '*.png' -ErrorAction SilentlyContinue | Remove-Item -Force
            $commandLine = "set FERROUI_SMOKE_PAGES=2500&& set FERROUI_SMOKE_EXIT_MS=120000&& set FERROUI_SMOKE_RENDERING=$mode&& set FERROUI_SMOKE_SCREENSHOTS=$directory&& `"$catalog`""
            if ($mode -eq 'extended') {
                $commandLine = "set FERROUI_SMOKE_PAGES=2500&& set FERROUI_SMOKE_EXIT_MS=120000&& set FERROUI_SMOKE_RENDERING=angle&& set FERROUI_SMOKE_EXTEND_CLIENT_AREA=1&& set FERROUI_SMOKE_SCREENSHOT_PAGES=Home,Window Customizations,Buttons&& set FERROUI_SMOKE_SCREENSHOTS=$directory&& `"$catalog`""
            }
            if ($mode -like 'winui-*') {
                $level = $mode.Substring(6)
                $commandLine = "set FERROUI_SMOKE_PAGES=2500&& set FERROUI_SMOKE_EXIT_MS=120000&& set FERROUI_SMOKE_RENDERING=angle&& set FERROUI_SMOKE_COMPOSITION=winui&& set FERROUI_SMOKE_TRANSPARENCY=$level&& set FERROUI_SMOKE_SCREENSHOT_PAGES=Home,Window Customizations,Buttons&& set FERROUI_SMOKE_SCREENSHOTS=$directory&& `"$catalog`""
            }
            if ($mode -eq 'dcomp' -or $mode -eq 'winui') {
                # Presented through a composition mode, with the page that sets the transparency
                # level of the window among the pages.
                $commandLine = "set FERROUI_SMOKE_PAGES=2500&& set FERROUI_SMOKE_EXIT_MS=120000&& set FERROUI_SMOKE_RENDERING=angle&& set FERROUI_SMOKE_COMPOSITION=$mode&& set FERROUI_SMOKE_SCREENSHOT_PAGES=Home,Window Customizations,Buttons&& set FERROUI_SMOKE_SCREENSHOTS=$directory&& `"$catalog`""
            }
            $code = if ($InteractiveUser) { Run-Interactive $name $commandLine 600 } else { Run $name $commandLine }
            if ($code -ne 0) { $failed.Add("catalog ($mode)") }
            $count = @(Get-ChildItem $directory -Filter '*.png' -ErrorAction SilentlyContinue).Count
            Say "    $count picture(s) in $directory"
            if ($count -eq 0) { $failed.Add("catalog pictures ($mode)") }
            Tail $name 40 'content reaches|colour\(s\)|panicked|error|moved|extended|transparency'
            $copy = Join-Path (Join-Path (Split-Path -Parent $Report) 'vm-smoke-screenshots') $mode
            New-Item -ItemType Directory -Force -Path $copy | Out-Null
            Copy-Item (Join-Path $directory '*.png') $copy -Force -ErrorAction SilentlyContinue
        }
    }
}

Say ""
if ($failed.Count -eq 0) {
    Say "RESULT: passed"
    exit 0
}
Say ("RESULT: failed: {0}" -f ($failed -join '; '))
exit 1
