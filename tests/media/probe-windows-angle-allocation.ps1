#requires -Version 7.0
# Run in an x64 MSVC developer shell after building the Windows sample. This
# diagnostic links the same static ANGLE artifacts; it does not patch ANGLE.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$AngleSourceDirectory,
    [Parameter(Mandatory)][string]$AngleBuildDirectory,
    [Parameter(Mandatory)][string]$ZlibLibrary,
    [Parameter(Mandatory)][string]$OutputDirectory,
    [ValidateRange(25,10000)][int]$Cycles=300
)
$ErrorActionPreference='Stop'
$angle=(Resolve-Path -LiteralPath $AngleSourceDirectory).Path
$build=(Resolve-Path -LiteralPath $AngleBuildDirectory).Path
$zlib=(Resolve-Path -LiteralPath $ZlibLibrary).Path
$output=[IO.Path]::GetFullPath($OutputDirectory)
if (Test-Path -LiteralPath $output) { throw "Use a fresh output directory: $output" }
[IO.Directory]::CreateDirectory($output) | Out-Null
$source=Join-Path $PSScriptRoot 'windows-angle-allocation-probe.cpp'
$exe=Join-Path $output 'angle-allocation-probe.exe'
$arguments=@('/nologo','/EHsc','/std:c++17','/MD','/O2','/DNOMINMAX','/DEGLAPI=','/DANGLE_ENABLE_D3D11',
    "/I$angle/include","/I$angle/src","/I$angle/src/common/third_party/base","/I$build",$source,
    "/Fo$output/probe.obj","/Fe$exe",'/link',"/LIBPATH:$build",
    'libGLESv2.lib','angle_common.lib','angle_gpu_info_util.lib','angle_image_util.lib',
    'compression_utils_portable.lib','glslang_glue.lib','preprocessor.lib','translator.lib',
    'd3d11.lib','d3d9.lib','dxgi.lib','dxguid.lib','d3dcompiler.lib','gdi32.lib','user32.lib',
    'setupapi.lib','advapi32.lib','version.lib','uuid.lib','psapi.lib',$zlib)
& cl @arguments > (Join-Path $output 'build.log') 2>&1
if ($LASTEXITCODE -ne 0) { throw "Probe build failed: $output/build.log" }
foreach ($mode in @('d3d','display','angle','thread','reuse')) {
    $log=Join-Path $output "$mode.log"
    & $exe $mode $Cycles > $log 2>&1
    if ($LASTEXITCODE -ne 0 -or !([IO.File]::ReadAllText($log).Contains("ALLOCATION_PROBE_PASS mode=$mode loops=$Cycles"))) {
        throw "Probe failed: $log"
    }
}
Write-Output "ANGLE allocation evidence: $output"
