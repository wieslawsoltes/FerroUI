#requires -Version 7.0
[CmdletBinding()]
param(
    [ValidateSet('All','Memory','HarnessSelfTest')][string]$Section = 'All',
    [ValidateRange(6,1000)][int]$MemoryCycles = 20,
    [ValidateRange(1,1000)][int]$StressCycles = 20,
    [ValidateRange(1,1000)][int]$RaceCycles = 20,
    [ValidateRange(1,100)][int]$EndedRestartRuns = 5,
    [string]$OutputDirectory,
    [string]$TargetDirectory = $env:CARGO_TARGET_DIR,
    [string]$Node = 'node',
    [string]$WslTargetDirectory = '/tmp/ferroui-portable-media-verification',
    [switch]$IncludeWsl
)
$ErrorActionPreference = 'Stop'
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
if (!$OutputDirectory) { $OutputDirectory = Join-Path (Split-Path $repo) ('windows-verification-' + (Get-Date -Format 'yyyyMMdd-HHmmss')) }
$output = [IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) { throw "Use a fresh output directory: $output" }
[IO.Directory]::CreateDirectory($output) | Out-Null
if (!$TargetDirectory) { $TargetDirectory = Join-Path $repo 'target' }
$TargetDirectory = [IO.Path]::GetFullPath($TargetDirectory)
$oldTarget = $env:CARGO_TARGET_DIR
$oldEvidence = $env:FERROUI_MEDIA_EVIDENCE
$records = [Collections.Generic.List[object]]::new()
$memory = [Collections.Generic.List[object]]::new()
$report = [ordered]@{ status='running'; section=$Section; revision=(& git -C $repo rev-parse HEAD); started_utc=[DateTime]::UtcNow.ToString('o'); output=$output; steps=$records; memory=$memory; binary_sha256=$null; failure=$null }
$changedPaths=@(& git -C $repo diff --name-only HEAD; & git -C $repo ls-files --others --exclude-standard)
$report.source_changes=@($changedPaths | Sort-Object -Unique | ForEach-Object {
    $path=Join-Path $repo $_
    if (Test-Path -LiteralPath $path -PathType Leaf) { @{path=$_;sha256=(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash} }
})
function Save-Report {
    $report | ConvertTo-Json -Depth 12 | Set-Content -LiteralPath (Join-Path $output 'report.json') -Encoding utf8
    $lines=[Collections.Generic.List[string]]::new()
    $lines.Add("# Windows media verification: $($report.status)")
    $lines.Add(''); $lines.Add("Revision: $($report.revision). Binary SHA-256: $($report.binary_sha256)")
    if ($report.failure) { $lines.Add(''); $lines.Add("Failure: $($report.failure)") }
    $lines.Add('');$lines.Add('| Step | Result | Elapsed ms |');$lines.Add('| --- | --- | ---: |')
    foreach ($entry in $records) { $lines.Add("| $($entry.name) | $($entry.status) | $($entry.elapsed_ms) |") }
    $lines.Add('');$lines.Add('Memory deltas are final minus cycle 5; requested Rust bytes and default-heap figures overlap and must not be added. MAX means unavailable. These are observations, not proof of leak freedom.')
    $lines.Add('');$lines.Add('| Workload | Rust live bytes Δ | Default heap allocated Δ | Default heap committed Δ | Private bytes Δ | Local GPU bytes Δ | Handles Δ |')
    $lines.Add('| --- | ---: | ---: | ---: | ---: | ---: | ---: |')
    foreach ($m in $memory) {
        $delta=@('rust_live_bytes','default_heap_allocated','default_heap_committed','private_bytes','local_gpu_bytes','handles') | ForEach-Object {
            if ($m.last[$_] -eq [ulong]::MaxValue -or $m.baseline[$_] -eq [ulong]::MaxValue) {'unavailable'} else { [long]$m.last[$_] - [long]$m.baseline[$_] }
        }
        $lines.Add("| $($m.name) | $($delta -join ' | ') |")
    }
    $lines | Set-Content -LiteralPath (Join-Path $output 'report.md') -Encoding utf8
}
function Run-Step([string]$Name,[string]$Exe,[string[]]$Arguments,[string[]]$Markers=@(),[int]$TimeoutSeconds=180,[int]$ExpectedExit=0) {
    Write-Host "Running $Name"
    $info = [Diagnostics.ProcessStartInfo]::new()
    $info.FileName=$Exe; $info.WorkingDirectory=$repo
    $info.UseShellExecute=$false; $info.CreateNoWindow=$true
    $info.RedirectStandardOutput=$true; $info.RedirectStandardError=$true
    foreach ($argument in $Arguments) { $info.ArgumentList.Add($argument) }
    $entry=[ordered]@{name=$Name; exe=$Exe; arguments=$Arguments; status='running';elapsed_ms=$null;exit_code=$null}
    $records.Add($entry); Save-Report
    $stdout=[IO.FileStream]::new((Join-Path $output "$Name.stdout.log"),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::ReadWrite)
    $stderr=[IO.FileStream]::new((Join-Path $output "$Name.stderr.log"),[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::ReadWrite)
    $process=[Diagnostics.Process]::new(); $process.StartInfo=$info
    $timer=[Diagnostics.Stopwatch]::StartNew()
    try {
        if (!$process.Start()) { throw "Failed to start $Name" }
        $outTask=$process.StandardOutput.BaseStream.CopyToAsync($stdout)
        $errTask=$process.StandardError.BaseStream.CopyToAsync($stderr)
        $timedOut=$false
        while (!$process.WaitForExit(500)) {
            if ($timer.Elapsed.TotalSeconds -gt $TimeoutSeconds) { $process.Kill($true); $process.WaitForExit(); $timedOut=$true; break }
        }
        $outTask.GetAwaiter().GetResult(); $errTask.GetAwaiter().GetResult()
        $entry.exit_code=$process.ExitCode
        if ($timedOut) { throw "$Name timed out" }
        if ($process.ExitCode -ne $ExpectedExit) { throw "$Name exit $($process.ExitCode), expected $ExpectedExit" }
    } finally {
        $stdout.Dispose(); $stderr.Dispose(); $process.Dispose()
        $entry.elapsed_ms=$timer.ElapsedMilliseconds
    }
    $text=[IO.File]::ReadAllText((Join-Path $output "$Name.stdout.log")) + [IO.File]::ReadAllText((Join-Path $output "$Name.stderr.log"))
    foreach ($marker in $Markers) { if (!$text.Contains($marker)) { throw "$Name missing marker: $marker" } }
    if ($text -match '(?m)^(CAMERA_STRESS_FAILED|VIDEO_PIXEL_FAILED|LIFECYCLE_STRESS_FAILED|.*_TIMEOUT\b)') { throw "$Name reported a failure" }
    $entry.status='passed'; Save-Report
    return $text
}
function Verify-Soak([string]$Text,[int]$Cycles) {
    $samples=[regex]::Matches($Text,'(?m)^CAMERA_STRESS_SAMPLE .+$')
    if ($samples.Count -ne $Cycles) { throw "Expected $Cycles retirement checkpoints, got $($samples.Count)" }
    $cycle=1
    foreach ($sample in $samples) {
        if ($sample.Value -notmatch "^CAMERA_STRESS_SAMPLE cycle=$cycle\s" -or $sample.Value.Trim() -notmatch 'frames=0 reads=0 quarantine=0 native=\(0, 0, 0\)$') { throw "Missing/out-of-order/nonzero retirement checkpoint: $sample" }
        $cycle++
    }
    $owners=[regex]::Matches($Text,'(?m)^CAMERA_NATIVE_OWNERS cycle=(\d+) owners=\[0, 0, 0, 0, 0\]\s*$')
    if ($owners.Count -ne $Cycles) { throw "Expected $Cycles zero native ownership checkpoints, got $($owners.Count)" }
    for ($i=0; $i -lt $Cycles; $i++) {
        if ([int]$owners[$i].Groups[1].Value -ne $i+1) { throw 'Out-of-order native ownership checkpoint' }
    }
}
function Fields([string]$Line) {
    $result=[ordered]@{}
    foreach ($match in [regex]::Matches($Line,'(\w+)=(\d+)')) { $result[$match.Groups[1].Value]=[ulong]$match.Groups[2].Value }
    return $result
}
function Memory-Summary([string]$Text,[string]$Name,[int]$Cycles) {
    Verify-Soak $Text $Cycles
    $samples=[regex]::Matches($Text,'(?m)^CAMERA_MEMORY_SAMPLE .+$')
    $process=[regex]::Matches($Text,'(?m)^CAMERA_STRESS_SAMPLE .+$')
    if ($samples.Count -ne $Cycles) { throw 'Missing memory attribution samples' }
    $baseline=Fields $samples[4].Value; $last=Fields $samples[$samples.Count-1].Value
    $baseProcess=Fields $process[4].Value; $lastProcess=Fields $process[$process.Count-1].Value
    foreach ($key in $baseProcess.Keys) { $baseline[$key]=$baseProcess[$key]; $last[$key]=$lastProcess[$key] }
    $memory.Add([ordered]@{name=$Name; baseline=$baseline; last=$last}); Save-Report
}
try {
    $env:CARGO_TARGET_DIR=$TargetDirectory
    # Prove that process success alone cannot turn missing evidence into a pass.
    $pwsh=(Get-Process -Id $PID).Path
    $null=Run-Step 'harness-positive' $pwsh @('-NoProfile','-Command','Write-Output "ARGUMENT WITH SPACES"; exit 0') @('ARGUMENT WITH SPACES') 15
    $null=Run-Step 'harness-expected-exit' $pwsh @('-NoProfile','-Command','Write-Output "EXPECTED_REJECTION"; exit 7') @('EXPECTED_REJECTION') 15 7
    foreach ($negative in @('marker','exit','timeout')) {
        $detected=$false
        $expected=@{marker='missing marker:';exit='exit 9, expected 0';timeout='timed out'}[$negative]
        try {
            switch ($negative) {
                'marker' { $null=Run-Step 'harness-missing-marker' $pwsh @('-NoProfile','-Command','exit 0') @('REQUIRED_BUT_ABSENT') 15 }
                'exit' { $null=Run-Step 'harness-wrong-exit' $pwsh @('-NoProfile','-Command','exit 9') @() 15 }
                'timeout' { $null=Run-Step 'harness-timeout' $pwsh @('-NoProfile','-Command','Start-Sleep -Seconds 10') @() 1 }
            }
        } catch {
            if (!$_.Exception.Message.Contains($expected)) { throw }
            $detected=$true; $records[$records.Count-1].status='passed_negative_control'
        }
        if (!$detected) { throw "Harness failed to detect $negative negative control" }
    }
    if ($Section -eq 'HarnessSelfTest') { $report.status='passed'; Write-Host "Harness checks passed: $(Join-Path $output 'report.md')"; return }
    $null=Run-Step 'build-audit' 'cargo' @('build','--locked','-p','portable-media','--features','memory-audit','--target','x86_64-pc-windows-msvc') @() 1200
    $app=Join-Path $output 'portable-media-audit.exe'
    Copy-Item -LiteralPath (Join-Path $TargetDirectory 'x86_64-pc-windows-msvc/debug/portable-media.exe') -Destination $app
    $report.binary_sha256=(Get-FileHash -LiteralPath $app -Algorithm SHA256).Hash
    foreach ($quiet in @($false,$true)) {
        foreach ($png in @($true,$false)) {
            $name="memory-quiet-$quiet-png-$png".ToLowerInvariant()
            $env:FERROUI_MEDIA_EVIDENCE=if ($png) { Join-Path $output "$name-images" } else { $null }
            $arguments=@('--camera','--synthetic-camera','--camera-smoke','--camera-recovery-smoke','--renderer',"--camera-stress-cycles=$MemoryCycles")
            if ($quiet) { $arguments+='--quiet-diagnostics' }
            if (!$png) { $arguments+='--no-png-diagnostics' }
            $text=Run-Step $name $app $arguments @("CAMERA_STRESS_PASS cycles=$MemoryCycles",'CAMERA_MEMORY_SAMPLE') ($MemoryCycles*90+60)
            Memory-Summary $text $name $MemoryCycles
        }
    }
    foreach ($workload in @('capture','capture-native','reset','idle')) {
        $env:FERROUI_MEDIA_EVIDENCE=$null
        $text=Run-Step "memory-isolate-$workload" $app @('--camera','--synthetic-camera','--camera-smoke','--camera-recovery-smoke','--renderer',"--camera-stress-cycles=$MemoryCycles",'--quiet-diagnostics','--no-png-diagnostics',"--memory-workload=$workload") @("CAMERA_STRESS_PASS cycles=$MemoryCycles",'CAMERA_MEMORY_SAMPLE') ($MemoryCycles*90+60)
        Memory-Summary $text "isolate-$workload" $MemoryCycles
    }
    if ($Section -eq 'All') {
        $env:FERROUI_MEDIA_EVIDENCE=Join-Path $output 'video-images'
        $null=Run-Step 'native-tests' 'cargo' @('test','--locked','-p','ferroui-media-windows','--features','controls,native-frame-tests,synthetic-camera','--target','x86_64-pc-windows-msvc','--lib','--','--nocapture') @('test result: ok.','NATIVE_MP4_FILE_PASS','NATIVE_ANGLE_ALIAS_PASS','NATIVE_D3D11_FRAMES_PASS','NATIVE_ANGLE_CONTEXT_CHURN_PASS') 1200
        $null=Run-Step 'display-owner-tests' 'cargo' @('test','--locked','-p','ferroui-win32','--target','x86_64-pc-windows-msvc','--lib','angle_display_lifetime') @('5 passed; 0 failed') 1200
        $null=Run-Step 'egl-tests' 'cargo' @('test','--locked','-p','ferroui-opengl','--target','x86_64-pc-windows-msvc','--lib','egl::') @('test result: ok.','externally_leased_display_does_not_terminate_another_wrappers_native_display ... ok','failed_context_initialization_releases_native_context_without_terminating_display ... ok') 1200
        $null=Run-Step 'allocator-tests' 'cargo' @('test','--locked','-p','portable-media','--features','memory-audit','--target','x86_64-pc-windows-msvc','memory_audit::tests') @('1 passed; 0 failed') 1200
        $null=Run-Step 'source-contract-tests' 'cargo' @('test','--locked','-p','ferroui-media-video','--target','x86_64-pc-windows-msvc','--lib','native_source::tests') @('test result: ok.','canceled_native_write_keeps_its_slot_and_removes_late_output ... ok','rapid_replacement_cannot_accumulate_filesystem_workers ... ok') 1200
        $null=Run-Step 'video-control-tests' 'cargo' @('test','--locked','-p','ferroui-media-video-controls','--target','x86_64-pc-windows-msvc','--lib') @('test result: ok.','ended_restart_keeps_polling_until_native_play_is_observed ... ok') 1200
        $fixtures=Join-Path $output 'fixtures'
        $null=Run-Step 'fixtures' $Node @('tests/media/make-windows-geometry-fixtures.mjs',$fixtures)
        foreach ($crop in @($false,$true)) {
            foreach ($orientation in 0..7) {
                $suffix=if ($crop) { '-crop' } else { '' }
                $arguments=@((Join-Path $fixtures "orientation-$orientation$suffix.mp4"),'--renderer','--video-pixel-smoke','--video-lifecycle-pixels',"--orientation=$orientation")
                if ($crop) { $arguments+='--crop-smoke' }
                $null=Run-Step "video-$orientation$suffix" $app $arguments @("VIDEO_PIXEL_SMOKE_PASS orientation=$orientation crop=$($crop.ToString().ToLowerInvariant())",'VIDEO_PRESENTATION_READY phase=recovered','VIDEO_LIFECYCLE_PIXELS_PASS phase=seek','VIDEO_LIFECYCLE_PIXELS_PASS phase=recovered') 90
            }
        }
        $null=Run-Step 'lifecycle-races' $app @('tests/media/fixtures/avc-aac.mp4',"--lifecycle-stress-cycles=$RaceCycles",'--renderer','--synthetic-camera') @("LIFECYCLE_STRESS_PASS cycles=$RaceCycles") ($RaceCycles*60+60)
        $env:FERROUI_MEDIA_EVIDENCE=$null
        $text=Run-Step 'camera-soak' $app @('--camera','--synthetic-camera','--camera-smoke','--camera-recovery-smoke','--renderer',"--camera-stress-cycles=$StressCycles",'--quiet-diagnostics','--no-png-diagnostics') @("CAMERA_STRESS_PASS cycles=$StressCycles") ($StressCycles*90+60)
        Verify-Soak $text $StressCycles
        $null=Run-Step 'default-build' 'cargo' @('build','--locked','-p','portable-media','--target','x86_64-pc-windows-msvc') @() 1200
        $default=Join-Path $TargetDirectory 'x86_64-pc-windows-msvc/debug/portable-media.exe'
        $null=Run-Step 'default-device-recovery' $default @('tests/media/fixtures/avc-aac.mp4','--resource','--smoke','--renderer','--device-recovery-smoke') @('DEVICE_RECOVERY_PASS','NATIVE_MEDIA_SMOKE_PASS retained_frames=0') 90
        foreach ($iteration in 1..$EndedRestartRuns) {
            $null=Run-Step "default-ended-restart-$iteration" $default @('tests/media/fixtures/avc-aac.mp4','--resource','--smoke','--renderer','--ended-restart-smoke') @('ENDED_RESTART_COMPLETED','NATIVE_MEDIA_SMOKE_PASS retained_frames=0') 90
        }
        $null=Run-Step 'default-rejection' $default @('--camera','--synthetic-camera') @('requires a Windows build') 30 1
        if ($IncludeWsl) {
            # No Windows path interpolation into Bash; pass it as a positional argument.
            $linuxRepo=(& wsl -d Ubuntu --exec wslpath -a $repo).Trim()
            $null=Run-Step 'wsl-build' 'wsl' @('-d','Ubuntu','--exec','bash','-lc','cd "$1" && CARGO_TARGET_DIR="$2" CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo build --locked -p portable-media','verify-windows',$linuxRepo,$WslTargetDirectory) @() 1800
            $null=Run-Step 'wsl-video-control-tests' 'wsl' @('-d','Ubuntu','--exec','bash','-lc','cd "$1" && CARGO_TARGET_DIR="$2" CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked -p ferroui-media-video-controls --lib','verify-windows',$linuxRepo,$WslTargetDirectory) @('test result: ok.','ended_restart_keeps_polling_until_native_play_is_observed ... ok') 1800
            $null=Run-Step 'wsl-egl-tests' 'wsl' @('-d','Ubuntu','--exec','bash','-lc','cd "$1" && CARGO_TARGET_DIR="$2" CARGO_BUILD_JOBS=2 CARGO_PROFILE_DEV_DEBUG=0 cargo test --locked -p ferroui-opengl --lib egl::','verify-windows',$linuxRepo,$WslTargetDirectory) @('test result: ok.','failed_context_initialization_releases_native_context_without_terminating_display ... ok') 1800
        }
    }
    $report.status='passed'
    Write-Host "Verification passed: $(Join-Path $output 'report.json')"
} catch {
    $report.status='failed'; $report.failure=$_.Exception.Message
    if ($records.Count) { $records[$records.Count-1].status='failed' }
    Write-Error -ErrorRecord $_ -ErrorAction Continue
} finally {
    $env:CARGO_TARGET_DIR=$oldTarget; $env:FERROUI_MEDIA_EVIDENCE=$oldEvidence
    $report.finished_utc=[DateTime]::UtcNow.ToString('o'); Save-Report
}
if ($report.status -ne 'passed') { exit 1 }
