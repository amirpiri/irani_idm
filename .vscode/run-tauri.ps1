param(
    [Parameter(Mandatory = $true)]
    [ValidateSet("dev", "build")]
    [string] $Action
)

$ErrorActionPreference = "Stop"

$vswherePaths = @(
    (Join-Path ${env:ProgramFiles(x86)} "Microsoft Visual Studio\Installer\vswhere.exe"),
    (Join-Path $env:ProgramFiles "Microsoft Visual Studio\Installer\vswhere.exe")
)
$vswhere = $vswherePaths | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $vswhere) {
    throw "Visual Studio Installer's vswhere.exe was not found. Install the MSVC C++ Build Tools."
}

$vsInstallation = & $vswhere -latest -products "*" -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
if ($LASTEXITCODE -ne 0 -or -not $vsInstallation) {
    throw "An MSVC C++ Build Tools installation was not found."
}

$vsDevCmd = Join-Path $vsInstallation "Common7\Tools\VsDevCmd.bat"
$environment = & $env:ComSpec /d /s /c "`"$vsDevCmd`" -arch=x64 -host_arch=x64 >nul && set"
if ($LASTEXITCODE -ne 0) {
    throw "Failed to initialize the MSVC build environment."
}

foreach ($entry in $environment) {
    $separator = $entry.IndexOf("=")
    if ($separator -gt 0) {
        [Environment]::SetEnvironmentVariable(
            $entry.Substring(0, $separator),
            $entry.Substring($separator + 1),
            "Process"
        )
    }
}

$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo was not found. Install the stable Rust toolchain and restart VS Code."
}
if (-not (Get-Command cargo-tauri -ErrorAction SilentlyContinue)) {
    throw 'Tauri CLI was not found. Install it with: cargo install tauri-cli --version "^2"'
}

$tauriDirectory = Join-Path (Split-Path -Parent $PSScriptRoot) "src-tauri"
Push-Location $tauriDirectory
try {
    & cargo tauri $Action
    exit $LASTEXITCODE
}
finally {
    Pop-Location
}
