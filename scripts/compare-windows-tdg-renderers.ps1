#requires -Version 7.2
param(
    [Parameter(Mandatory)][string]$Executable,
    [string]$OutputDirectory = 'target/tdg-renderer-comparison',
    [ValidateRange(1,100)][int]$Repetitions = 5
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
$settings = @{
    FERROUI_BENCH_ITERATIONS = '15'
    FERROUI_BENCH_WARMUP = '3'
    FERROUI_TDG_FRAMES_PER_BATCH = '20'
    FERROUI_TDG_DRAW_PATH = 'recorded'
    FERROUI_TDG_BACKEND = $null
    FERROUI_TDG_RESULTS_JSON = $null
    FERROUI_D3D_PROFILE = $null
    FERROUI_TDG_CAPTURE_DIR = $null
}
$previous = @{}
try {
    $binary = (Resolve-Path -LiteralPath $Executable).Path
    $destination = [IO.Path]::GetFullPath($OutputDirectory)
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    $hash = (Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash
    foreach ($key in $settings.Keys) {
        $previous[$key] = [Environment]::GetEnvironmentVariable($key, 'Process')
        if ($null -eq $settings[$key]) {
            Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue
        } else {
            Set-Item -LiteralPath "Env:$key" -Value $settings[$key]
        }
    }
    @{
        revision = (& git rev-parse HEAD)
        source_changes = @(& git status --porcelain --untracked-files=no)
        executable_sha256 = $hash
        started_utc = [DateTime]::UtcNow.ToString('o')
        repetitions = $Repetitions
        order = @('skia','direct2d')
        settings = $settings
    } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $destination 'run.json')
    foreach ($repeat in 1..$Repetitions) {
        foreach ($backend in @('skia','direct2d')) {
            $env:FERROUI_TDG_BACKEND = $backend
            $env:FERROUI_TDG_RESULTS_JSON = Join-Path $destination "pair-$repeat-$backend.json"
            & $binary run_renderer_benchmarks --ignored --nocapture --test-threads=1 *> (Join-Path $destination "pair-$repeat-$backend.log")
            if ($LASTEXITCODE -ne 0) { throw "Benchmark failed: pair $repeat, $backend" }
            if ((Get-FileHash -LiteralPath $binary -Algorithm SHA256).Hash -ne $hash) { throw 'Executable changed during comparison' }
            Write-Host "Finished pair $repeat, $backend"
        }
    }
} finally {
    foreach ($key in $previous.Keys) {
        if ($null -eq $previous[$key]) {
            Remove-Item -LiteralPath "Env:$key" -ErrorAction SilentlyContinue
        } else {
            Set-Item -LiteralPath "Env:$key" -Value $previous[$key]
        }
    }
    Pop-Location
}
