# In the Windows virtual machine: builds the desktop catalog with the Vello backend in release from
# sources synced by scripts/windows/vm-sync.sh. Writes the log and the exit code to $Out.
#
#   build.ps1 -Src <synced source directory> -Out <directory on the shared volume>
param([Parameter(Mandatory)][string]$Src, [Parameter(Mandatory)][string]$Out,
      [string]$Target = 'C:\ferroui-target')
$env:CARGO_TARGET_DIR = $Target
Set-Location $Src
cmd /c "cargo build --release -p control-catalog-desktop --features vello > `"$Out\build.log`" 2>&1"
"exit $LASTEXITCODE" | Out-File -Encoding ascii (Join-Path $Out 'build.exit')
