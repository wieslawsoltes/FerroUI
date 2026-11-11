#requires -Version 7.2
param(
    [Parameter(Mandatory)][string]$SkiaExecutable,
    [Parameter(Mandatory)][string]$NativeExecutable,
    [string]$OutputDirectory = 'target/gauntlet-renderer-comparison',
    [ValidateRange(1,10)][int]$Repetitions = 3,
    [ValidateRange(0,10000)][int]$WarmupFrames = 60,
    [ValidateRange(1,10000)][int]$Frames = 240,
    [ValidateSet('gauntlet','wall')][string[]]$Scenarios = @('gauntlet','wall')
)
$ErrorActionPreference = 'Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
$variables = @('PERF_TIER','PERF_SCENARIO','PERF_FONTS','PERF_FREEZE','PERF_REPORT',
    'PERF_WARMUP_FRAMES','PERF_MEASURE_FRAMES','PERF_RESULTS_JSON','PERF_CAPTURE_PATH',
    'FERROUI_D3D_PROFILE','FERROUI_D3D_DEBUG','FERROUI_RENDERER','FERROUI_SMOKE_RENDERING')
$previous = @{}
try {
    $binaries = @{skia=(Resolve-Path -LiteralPath $SkiaExecutable).Path;native=(Resolve-Path -LiteralPath $NativeExecutable).Path}
    $hashes = @{}
    foreach($backend in @('skia','native')) {$hashes[$backend]=(Get-FileHash -LiteralPath $binaries[$backend] -Algorithm SHA256).Hash}
    $destination = [IO.Path]::GetFullPath($OutputDirectory)
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    foreach($key in $variables) {
        $previous[$key]=[Environment]::GetEnvironmentVariable($key,'Process')
        Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue
    }
    $env:PERF_FONTS=(Resolve-Path src/FerroUI.Themes.Material/Assets).Path
    $env:PERF_FREEZE='0'
    $env:PERF_REPORT='1'
    $env:PERF_WARMUP_FRAMES="$WarmupFrames"
    $env:PERF_MEASURE_FRAMES="$Frames"
    @{
        source_revision=(& git rev-parse HEAD)
        source_changes=@(& git status --porcelain --untracked-files=no)
        started_utc=[DateTime]::UtcNow.ToString('o')
        order=@('skia','native');scenarios=$Scenarios;repetitions=$Repetitions;warmup_frames=$WarmupFrames;measured_frames=$Frames
        per_second_callback_reporting=$true
        executable_sha256=$hashes
        font_sha256=@(Get-FileHash -LiteralPath "$env:PERF_FONTS/Roboto-Regular.ttf","$env:PERF_FONTS/Roboto-Bold.ttf" -Algorithm SHA256|Select-Object Path,Hash)
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destination 'run.json')
    foreach($repeat in 1..$Repetitions) {
        foreach($backend in @('skia','native')) {
            foreach($scenario in $Scenarios) {
                $env:PERF_SCENARIO=$scenario
                $env:PERF_TIER=if($scenario -eq 'gauntlet') {'16'} else {'20'}
                $env:PERF_RESULTS_JSON=Join-Path $destination "pair-$repeat-$backend-$scenario.json"
                if($repeat -eq $Repetitions) {$env:PERF_CAPTURE_PATH=Join-Path $destination "$backend-$scenario.png"}
                else {Remove-Item Env:PERF_CAPTURE_PATH -ErrorAction SilentlyContinue}
                & $binaries[$backend] *> (Join-Path $destination "pair-$repeat-$backend-$scenario.log")
                if($LASTEXITCODE -ne 0) {throw "Benchmark failed: $repeat $backend $scenario"}
                $result=Get-Content -LiteralPath $env:PERF_RESULTS_JSON -Raw|ConvertFrom-Json
                if($result.measured_frames -ne $Frames -or $result.samples_ms.Count -ne $Frames -or $result.last_scene_frame -ne ($WarmupFrames+$Frames)) {throw 'Incomplete fixed-frame measurement'}
                if($result.viewport_changed_during_measurement) {throw "The viewport or DPI changed during $backend $scenario"}
                if($backend -eq 'native') {
                    if($result.native_measured_presents -ne $Frames) {throw 'Native renderer did not present once per measured scene frame'}
                    if($result.native_statistics_including_startup_and_warmup[4] -ne 0) {throw 'Native renderer read back pixels during measurement'}
                    if($result.native_statistics_including_startup_and_warmup[8] -ne 0) {throw 'Native renderer reported errors'}
                }
                if((Get-FileHash -LiteralPath $binaries[$backend] -Algorithm SHA256).Hash -ne $hashes[$backend]) {throw 'Executable changed during comparison'}
                '{0} {1} {2}: {3:F2} callback fps, {4:F2} ms/frame, pointer-over frames {5}' -f $repeat,$backend,$scenario,$result.callback_fps,$result.mean_ms,$result.pointer_over_frames
                if($backend -eq 'native') {
                    $skia=Get-Content -LiteralPath (Join-Path $destination "pair-$repeat-skia-$scenario.json") -Raw|ConvertFrom-Json
                    if($result.client_width -ne $skia.client_width -or $result.client_height -ne $skia.client_height -or $result.render_scaling -ne $skia.render_scaling -or $result.pointer_over_frames -ne $skia.pointer_over_frames) {throw 'Unmatched viewport, scaling or pointer conditions'}
                }
            }
        }
    }
} finally {
    foreach($key in $previous.Keys) {
        if($null -eq $previous[$key]) {Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue}
        else {Set-Item -LiteralPath "Env:$key" -Value $previous[$key]}
    }
    Pop-Location
}
