#requires -Version 7.2
param(
    [ValidateSet('dist','dist-unwind')][string]$CargoProfile = 'dist',
    [string]$OutputDirectory = 'target/windows-native-size',
    [switch]$IncludeRuntimeShaders,
    [ValidateRange(0,2)][int]$BuildRetries = 1
)
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
Push-Location $repoRoot
try {
    $destination = [IO.Path]::GetFullPath($OutputDirectory, $repoRoot)
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    # The import audit needs dumpbin.exe: from the path (a developer shell), else from the newest
    # Visual Studio installation. Found before anything is built.
    $dumpbinPath = (Get-Command dumpbin.exe -ErrorAction SilentlyContinue)?.Source
    if (-not $dumpbinPath) {
        $programFiles = if (${env:ProgramFiles(x86)}) { ${env:ProgramFiles(x86)} } else { $env:ProgramFiles }
        $vswhere = Join-Path $programFiles 'Microsoft Visual Studio/Installer/vswhere.exe'
        if (-not (Test-Path -LiteralPath $vswhere -PathType Leaf)) {
            throw ('dumpbin.exe is not on the path and vswhere.exe is not installed: ' +
                'run from a Visual Studio developer shell or install the MSVC build tools')
        }
        $installation = & $vswhere -latest -products '*' -property installationPath | Select-Object -First 1
        $tools = if ($installation) { Join-Path $installation 'VC/Tools/MSVC' } else { $null }
        if (-not $tools -or -not (Test-Path -LiteralPath $tools -PathType Container)) {
            throw 'dumpbin.exe is not on the path and Visual Studio has no MSVC tools: install the C++ build tools'
        }
        $dumpbinPath = (Get-ChildItem -LiteralPath $tools -Filter dumpbin.exe -Recurse |
            Where-Object FullName -Match 'Hostx64\\x64\\dumpbin\.exe$' |
            Sort-Object FullName | Select-Object -Last 1)?.FullName
        if (-not $dumpbinPath) {
            throw 'The MSVC tools of Visual Studio have no dumpbin.exe: install the C++ build tools'
        }
    }
    # Where cargo puts the executable: CARGO_TARGET_DIR (relative to the repository root, where cargo
    # runs) or `target`, then the target triple when CARGO_BUILD_TARGET names one, then the profile.
    $targetDirectory = if ($env:CARGO_TARGET_DIR) {
        [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR, $repoRoot)
    } else { Join-Path $repoRoot 'target' }
    if ($env:CARGO_BUILD_TARGET) { $targetDirectory = Join-Path $targetDirectory $env:CARGO_BUILD_TARGET }
    $builtExecutable = Join-Path (Join-Path $targetDirectory $CargoProfile) 'hello-world.exe'
    $toolchain = (& rustc -vV) -join "`n"
    if ($LASTEXITCODE -ne 0) { throw 'Rust toolchain inspection failed' }
    $revision = (& git rev-parse HEAD) -join ''
    $sourceChanges = & git status --porcelain --untracked-files=no
    if ($LASTEXITCODE -ne 0 -or $sourceChanges) { throw 'Measure a clean, committed source revision' }
    $lockHash = (Get-FileHash -Algorithm SHA256 -LiteralPath Cargo.lock).Hash.ToLowerInvariant()
    $buildTarget = if ($env:CARGO_BUILD_TARGET) { $env:CARGO_BUILD_TARGET } else {
        ([regex]::Match($toolchain, '(?m)^host: (.+)$')).Groups[1].Value.Trim()
    }
    $results = @()
    $backends = @('skia','direct2d')
    if ($IncludeRuntimeShaders) { $backends += 'direct2d-runtime-shaders' }
    foreach ($backend in $backends) {
        $timer = [Diagnostics.Stopwatch]::StartNew()
        $buildAttempts = 0
        do {
            $buildAttempts++
            & cargo build --locked --profile $CargoProfile -p hello-world --no-default-features --features $backend
            $buildExit = $LASTEXITCODE
            if ($buildExit -ne 0 -and $buildAttempts -le $BuildRetries) {
                Write-Warning "$backend build failed (exit $buildExit); retrying the same configuration"
            }
        } while ($buildExit -ne 0 -and $buildAttempts -le $BuildRetries)
        if ($buildExit -ne 0) { throw "$backend build failed after $buildAttempts attempt(s)" }
        $timer.Stop()
        $deploy = Join-Path $destination $backend
        New-Item -ItemType Directory -Force -Path $deploy | Out-Null
        $executable = Join-Path $deploy 'hello-world.exe'
        Copy-Item -LiteralPath $builtExecutable -Destination $executable
        # Compressed distribution size is evidence, not an additional shipped file.
        $compressedPath = Join-Path $destination "hello-world-$backend.exe.gz"
        $inputStream = [IO.File]::OpenRead($executable)
        $compressedStream = [IO.File]::Create($compressedPath)
        $compressor = [IO.Compression.GZipStream]::new($compressedStream, [IO.Compression.CompressionLevel]::SmallestSize)
        try { $inputStream.CopyTo($compressor) }
        finally { $compressor.Dispose(); $compressedStream.Dispose(); $inputStream.Dispose() }
        $tree = & cargo tree --locked -p hello-world --no-default-features --features $backend -e normal
        if ($LASTEXITCODE -ne 0) { throw "$backend dependency audit failed" }
        # cargo names the directory of each workspace package: relative to the repository root here.
        $tree = @($tree | ForEach-Object { $_.Replace($repoRoot, '.', [StringComparison]::OrdinalIgnoreCase) })
        $tree | Set-Content -Encoding utf8 -LiteralPath (Join-Path $deploy 'dependencies.txt')
        if ($backend -ne 'skia' -and ($tree | Select-String 'ferroui-skia|harfbuzz|mozangle|ferroui-vello|wgpu')) {
            throw 'The native deployment includes a bundled portable backend'
        }
        # From the deployment directory, so that the audit names the file and not where it is.
        Push-Location $deploy
        try { $imports = & $dumpbinPath /DEPENDENTS 'hello-world.exe' } finally { Pop-Location }
        if ($LASTEXITCODE -ne 0) { throw "$backend DLL import audit failed" }
        $imports | Set-Content -Encoding utf8 -LiteralPath (Join-Path $deploy 'imports.txt')
        $importNames = @($imports | ForEach-Object {
            if ($_ -match '^\s+([A-Za-z0-9_.-]+\.dll)\s*$') { $Matches[1].ToLowerInvariant() }
        } | Sort-Object -Unique)
        $prerequisites = @($importNames | ForEach-Object {
            $dllName = $_
            $systemPath = Join-Path (Join-Path $env:WINDIR 'System32') $dllName
            $apiSet = $dllName -match '^(api|ext)-ms-'
            if (-not $apiSet -and -not (Test-Path -LiteralPath $systemPath -PathType Leaf)) {
                throw "Unaccounted application DLL: $dllName. Include it in the deployment before measuring."
            }
            $kind = if ($dllName -match '^(msvcp|vcruntime|concrt)\d') { 'VC runtime' } else { 'Windows system/API set' }
            $version = if (Test-Path -LiteralPath $systemPath -PathType Leaf) {
                (Get-Item -LiteralPath $systemPath).VersionInfo.FileVersion
            } else { $null }
            @{name=$dllName;kind=$kind;installedVersion=$version}
        })
        # Normal native and Skia HelloWorld ship one EXE. Framework-system DLLs
        # are prerequisites, not copied into either deployment. Extra application
        # DLLs/assets, if introduced, must be put in these deployment directories.
        $files = @(Get-ChildItem -LiteralPath $deploy -File -Recurse | Where-Object Name -NotIn 'dependencies.txt','imports.txt')
        $manifest = @($files | ForEach-Object {
            @{name=[IO.Path]::GetRelativePath($deploy,$_.FullName);bytes=$_.Length;sha256=(Get-FileHash -Algorithm SHA256 -LiteralPath $_.FullName).Hash.ToLowerInvariant()}
        })
        $shaderCompiler = switch ($backend) {
            'skia' { 'Skia built-in SkSL' }
            'direct2d-runtime-shaders' { 'Naga WGSL-to-HLSL and Windows d3dcompiler_47.dll' }
            default { 'Build-time compiled kernels only' }
        }
        $results += @{backend=$backend;command="cargo build --locked --profile $CargoProfile -p hello-world --no-default-features --features $backend";runtimeShaders=($backend -ne 'direct2d');shaderCompiler=$shaderCompiler;buildAttempts=$buildAttempts;seconds=$timer.Elapsed.TotalSeconds;executableBytes=(Get-Item -LiteralPath $executable).Length;gzipBytes=(Get-Item -LiteralPath $compressedPath).Length;deployedBytes=($files | Measure-Object Length -Sum).Sum;files=$manifest;prerequisites=$prerequisites}
    }
    $endingRevision = (& git rev-parse HEAD) -join ''
    $sourceChanges = & git status --porcelain --untracked-files=no
    if ($LASTEXITCODE -ne 0 -or $endingRevision -ne $revision -or $sourceChanges) { throw 'Source changed during the paired measurement; repeat from one clean revision' }
    $report = @{revision=$revision;cargoLockSha256=$lockHash;target=$buildTarget;profile=$CargoProfile;toolchain=$toolchain;os=(Get-CimInstance Win32_OperatingSystem | Select-Object Caption,Version,BuildNumber);gpu=@(Get-CimInstance Win32_VideoController | Select-Object Name,DriverVersion);results=$results}
    $report | ConvertTo-Json -Depth 8 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $destination 'results.json')
    $report.results | Format-Table backend,executableBytes,gzipBytes,deployedBytes,seconds
} finally { Pop-Location }
