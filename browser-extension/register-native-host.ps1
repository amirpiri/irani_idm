$ErrorActionPreference = 'Stop'

$extensionDirectory = (Resolve-Path $PSScriptRoot).Path.TrimEnd('\')
$repoRoot = Split-Path $PSScriptRoot -Parent
$tauriDirectory = Join-Path $repoRoot 'src-tauri'
$hostName = 'com.iranidm.browser'
$nativeDirectory = Join-Path $env:LOCALAPPDATA 'com.iranidm.desktop\native-messaging'
$hostExecutable = Join-Path $nativeDirectory 'irani-idm-native-host.exe'
$hostManifest = Join-Path $nativeDirectory "$hostName.json"

$manifest = Get-Content -LiteralPath (Join-Path $extensionDirectory 'manifest.json') -Raw | ConvertFrom-Json
if ($manifest.permissions -notcontains 'nativeMessaging') {
    throw 'The unpacked extension does not request the nativeMessaging permission. Reload the updated extension first.'
}

$browserRoots = @(
    @{ Name = 'Chrome'; Path = (Join-Path $env:LOCALAPPDATA 'Google\Chrome\User Data'); Registry = 'HKCU:\Software\Google\Chrome\NativeMessagingHosts' },
    @{ Name = 'Edge'; Path = (Join-Path $env:LOCALAPPDATA 'Microsoft\Edge\User Data'); Registry = 'HKCU:\Software\Microsoft\Edge\NativeMessagingHosts' }
)
$allowedOrigins = [System.Collections.Generic.List[string]]::new()
$registrations = [System.Collections.Generic.List[object]]::new()

foreach ($browser in $browserRoots) {
    if (-not (Test-Path -LiteralPath $browser.Path)) { continue }
    foreach ($profile in Get-ChildItem -LiteralPath $browser.Path -Directory) {
        $securePreferences = Join-Path $profile.FullName 'Secure Preferences'
        if (-not (Test-Path -LiteralPath $securePreferences)) { continue }
        try { $settings = (Get-Content -LiteralPath $securePreferences -Raw | ConvertFrom-Json).extensions.settings }
        catch { continue }
        foreach ($extension in $settings.PSObject.Properties) {
            $installedPath = [string]$extension.Value.path
            if ([string]::IsNullOrWhiteSpace($installedPath)) { continue }
            $matchesSourceFolder = $installedPath.TrimEnd('\') -ieq $extensionDirectory
            $installedManifestPath = Join-Path $installedPath 'manifest.json'
            $matchesPublishedExtension = $false
            if (-not $matchesSourceFolder -and (Test-Path -LiteralPath $installedManifestPath)) {
                try {
                    $installedManifest = Get-Content -LiteralPath $installedManifestPath -Raw | ConvertFrom-Json
                    $matchesPublishedExtension = $installedManifest.name -eq 'Irani IDM Browser Integration' -and $installedManifest.homepage_url -eq 'https://github.com/amirpiri/irani_idm'
                }
                catch { $matchesPublishedExtension = $false }
            }
            if ($matchesSourceFolder -or $matchesPublishedExtension) {
                $origin = "chrome-extension://$($extension.Name)/"
                if (-not $allowedOrigins.Contains($origin)) { $allowedOrigins.Add($origin) }
                $registrations.Add([pscustomobject]@{ Browser = $browser.Name; Registry = $browser.Registry; Id = $extension.Name; Profile = $profile.Name })
            }
        }
    }
}

if ($allowedOrigins.Count -eq 0) {
    throw "Could not find this unpacked extension in Chrome or Edge profiles. Load it from '$extensionDirectory' first, then run this script again."
}

Push-Location $tauriDirectory
try {
    & cargo build --release --bin irani-idm-native-host
    if ($LASTEXITCODE -ne 0) { throw 'Building the Native Messaging host failed.' }
}
finally { Pop-Location }

$builtHost = Join-Path $tauriDirectory 'target\release\irani-idm-native-host.exe'
if (-not (Test-Path -LiteralPath $builtHost)) { throw "Native Messaging host executable was not created: $builtHost" }
New-Item -ItemType Directory -Path $nativeDirectory -Force | Out-Null
Copy-Item -LiteralPath $builtHost -Destination $hostExecutable -Force

$hostConfiguration = [ordered]@{
    name = $hostName
    description = 'Passes explicitly enabled browser downloads to Irani IDM on this computer.'
    path = $hostExecutable
    type = 'stdio'
    allowed_origins = @($allowedOrigins)
}
$json = $hostConfiguration | ConvertTo-Json -Depth 5
[System.IO.File]::WriteAllText($hostManifest, $json, [System.Text.UTF8Encoding]::new($false))

foreach ($registryPath in ($registrations | Select-Object -ExpandProperty Registry -Unique)) {
    $key = Join-Path $registryPath $hostName
    New-Item -Path $key -Force | Out-Null
    Set-Item -Path $key -Value $hostManifest
}

Write-Output "Registered Native Messaging for: $(($registrations | ForEach-Object { "$($_.Browser) ($($_.Profile))" } | Sort-Object -Unique) -join ', ')"
Write-Output "Native host: $hostExecutable"
Write-Output "Manifest: $hostManifest"
Write-Output 'Restart Irani IDM after this script completes. Keep the unpacked extension loaded from its current folder.'
