#requires -Version 7.2

<#
.SYNOPSIS
Runs the gauntlet and the still wall of the gauntlet benchmark on the Skia/ANGLE renderer and on the
Direct2D renderer, in pairs, and checks that the runs are comparable.

.DESCRIPTION
Both executables are `perf-compare-ferroui.exe` of the package `gauntlet-benchmark`, built with the
feature `renderer-measurement`. The fixed-frame measurement this script drives with
`PERF_MEASURE_FRAMES` is compiled only with that feature: a build without it would never end a run,
so the script refuses it. Build once per backend, into distinct target directories:

    cargo build --locked --release -p gauntlet-benchmark --features renderer-measurement `
        --target-dir target/gauntlet-skia
    cargo build --locked --release -p gauntlet-benchmark --no-default-features `
        --features direct2d-runtime-shaders,renderer-measurement --target-dir target/gauntlet-native

Relative paths (the executables and the output directory) are relative to the repository root, not
to the current directory. The output directory receives `run.json` (what was run), one result and
one log per run, and a capture per backend and scenario from the last repetition. These files name
no directory of the machine: the fonts are recorded by file name and hash, the executables by hash.

.PARAMETER SkiaExecutable
The executable of the default build (Skia on ANGLE) with `--features renderer-measurement`.

.PARAMETER NativeExecutable
The executable of the Direct2D build with `renderer-measurement` among its features.

.PARAMETER OutputDirectory
Where the results go. Created when missing.

.PARAMETER GauntletTier
The load tier of the gauntlet, 1 to 16.

.PARAMETER WallTier
The load tier of the still wall, 1 to 20.

.EXAMPLE
./scripts/compare-windows-gauntlet-renderers.ps1 `
    -SkiaExecutable target/gauntlet-skia/release/perf-compare-ferroui.exe `
    -NativeExecutable target/gauntlet-native/release/perf-compare-ferroui.exe
#>
param(
    [Parameter(Mandatory)][string]$SkiaExecutable,
    [Parameter(Mandatory)][string]$NativeExecutable,
    [string]$OutputDirectory = 'target/gauntlet-renderer-comparison',
    [ValidateRange(1,10)][int]$Repetitions = 3,
    [ValidateRange(0,10000)][int]$WarmupFrames = 60,
    [ValidateRange(1,10000)][int]$Frames = 240,
    [ValidateSet('gauntlet','wall')][string[]]$Scenarios = @('gauntlet','wall'),
    [ValidateRange(1,16)][int]$GauntletTier = 16,
    [ValidateRange(1,20)][int]$WallTier = 20
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
$variables = @('PERF_TIER','PERF_SCENARIO','PERF_FONTS','PERF_FREEZE','PERF_REPORT',
    'PERF_WARMUP_FRAMES','PERF_MEASURE_FRAMES','PERF_RESULTS_JSON','PERF_CAPTURE_PATH',
    'FERROUI_D3D_PROFILE','FERROUI_D3D_DEBUG','FERROUI_RENDERER','FERROUI_SMOKE_RENDERING')
$previous = @{}
try {
    $binaries = @{skia=(Resolve-Path -LiteralPath $SkiaExecutable).Path;native=(Resolve-Path -LiteralPath $NativeExecutable).Path}
    $hashes = @{}
    foreach($backend in @('skia','native')) {
        $hashes[$backend]=(Get-FileHash -LiteralPath $binaries[$backend] -Algorithm SHA256).Hash
        # The name of the variable is in the program only when the measurement is compiled in.
        $image=[Text.Encoding]::Latin1.GetString([IO.File]::ReadAllBytes($binaries[$backend]))
        if(-not $image.Contains('PERF_MEASURE_FRAMES')) {
            throw "The $backend executable was built without the feature renderer-measurement"
        }
    }
    $destination = [IO.Path]::GetFullPath($OutputDirectory, $repoRoot)
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    foreach($key in $variables) {
        $previous[$key]=[Environment]::GetEnvironmentVariable($key,'Process')
        Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue
    }
    $fonts = Join-Path $repoRoot 'src/FerroUI.Themes.Material/Assets'
    $fontFiles = @('Roboto-Regular.ttf','Roboto-Bold.ttf' | ForEach-Object {Join-Path $fonts $_})
    $env:PERF_FONTS=$fonts
    $env:PERF_FREEZE='0'
    $env:PERF_REPORT='1'
    $env:PERF_WARMUP_FRAMES="$WarmupFrames"
    $env:PERF_MEASURE_FRAMES="$Frames"
    @{
        source_revision=(& git rev-parse HEAD)
        source_changes=@(& git status --porcelain --untracked-files=no)
        started_utc=[DateTime]::UtcNow.ToString('o')
        order=@('skia','native');scenarios=$Scenarios;repetitions=$Repetitions;warmup_frames=$WarmupFrames;measured_frames=$Frames
        tiers=@{gauntlet=$GauntletTier;wall=$WallTier}
        per_second_callback_reporting=$true
        executable_sha256=$hashes
        # The file names alone: a full path names a directory of the machine.
        font_sha256=@(Get-FileHash -LiteralPath $fontFiles -Algorithm SHA256|
            Select-Object @{Name='Path';Expression={Split-Path -Leaf $_.Path}},Hash)
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destination 'run.json')
    foreach($repeat in 1..$Repetitions) {
        foreach($backend in @('skia','native')) {
            foreach($scenario in $Scenarios) {
                $env:PERF_SCENARIO=$scenario
                $env:PERF_TIER=if($scenario -eq 'gauntlet') {"$GauntletTier"} else {"$WallTier"}
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
