$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.IO.Compression.FileSystem
Add-Type -AssemblyName System.IO.Compression

$extensionDirectory = $PSScriptRoot
$outputDirectory = Join-Path $extensionDirectory 'store-packages'
$stagingDirectory = Join-Path $outputDirectory 'staging'
$firefoxDirectory = Join-Path $extensionDirectory 'firefox-build'

if (Test-Path -LiteralPath $outputDirectory) {
    Remove-Item -LiteralPath $outputDirectory -Recurse -Force
}
New-Item -ItemType Directory -Path $outputDirectory | Out-Null

function New-ExtensionArchive {
    param(
        [Parameter(Mandatory = $true)][string]$SourceDirectory,
        [Parameter(Mandatory = $true)][string]$ArchivePath
    )

    if (Test-Path -LiteralPath $ArchivePath) {
        Remove-Item -LiteralPath $ArchivePath -Force
    }
    $fileStream = [System.IO.File]::Open($ArchivePath, [System.IO.FileMode]::CreateNew)
    $zip = [System.IO.Compression.ZipArchive]::new(
        $fileStream,
        [System.IO.Compression.ZipArchiveMode]::Create,
        $false
    )
    try {
        foreach ($file in Get-ChildItem -LiteralPath $SourceDirectory -File -Recurse) {
            $relativePath = $file.FullName.Substring($SourceDirectory.Length).TrimStart([char[]]@('\', '/')).Replace('\', '/')
            $entry = $zip.CreateEntry($relativePath, [System.IO.Compression.CompressionLevel]::Optimal)
            $entryStream = $entry.Open()
            $inputStream = [System.IO.File]::OpenRead($file.FullName)
            try { $inputStream.CopyTo($entryStream) }
            finally {
                $inputStream.Dispose()
                $entryStream.Dispose()
            }
        }
    }
    finally {
        $zip.Dispose()
        $fileStream.Dispose()
    }
}

function New-StagedExtension {
    param(
        [Parameter(Mandatory = $true)][string]$SourceDirectory,
        [Parameter(Mandatory = $true)][string]$StageDirectory
    )
    New-Item -ItemType Directory -Path $StageDirectory -Force | Out-Null
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'manifest.json') -Destination $StageDirectory
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'background.js') -Destination $StageDirectory
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'options.html') -Destination $StageDirectory
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'options.js') -Destination $StageDirectory
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'popup.html') -Destination $StageDirectory
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'popup.js') -Destination $StageDirectory
    Copy-Item -LiteralPath (Join-Path $SourceDirectory 'icons') -Destination $StageDirectory -Recurse
}

$requiredFiles = @(
    'manifest.json',
    'background.js',
    'options.html',
    'options.js',
    'popup.html',
    'popup.js',
    'icons/16.png',
    'icons/32.png',
    'icons/48.png',
    'icons/128.png'
)
foreach ($file in $requiredFiles) {
    if (-not (Test-Path -LiteralPath (Join-Path $extensionDirectory $file) -PathType Leaf)) {
        throw "Missing extension file: $file"
    }
}

New-StagedExtension -SourceDirectory $extensionDirectory -StageDirectory (Join-Path $stagingDirectory 'chrome')
New-StagedExtension -SourceDirectory $extensionDirectory -StageDirectory (Join-Path $stagingDirectory 'edge')
New-ExtensionArchive -SourceDirectory (Join-Path $stagingDirectory 'chrome') -ArchivePath (Join-Path $outputDirectory 'Irani-IDM-Chrome.zip')
New-ExtensionArchive -SourceDirectory (Join-Path $stagingDirectory 'edge') -ArchivePath (Join-Path $outputDirectory 'Irani-IDM-Edge.zip')

& (Join-Path $extensionDirectory 'build-firefox.ps1')
New-StagedExtension -SourceDirectory $firefoxDirectory -StageDirectory (Join-Path $stagingDirectory 'firefox')
New-ExtensionArchive -SourceDirectory (Join-Path $stagingDirectory 'firefox') -ArchivePath (Join-Path $outputDirectory 'Irani-IDM-Firefox.zip')

Remove-Item -LiteralPath $stagingDirectory -Recurse -Force
if (Test-Path -LiteralPath $firefoxDirectory) {
    Remove-Item -LiteralPath $firefoxDirectory -Recurse -Force
}

foreach ($archive in Get-ChildItem -LiteralPath $outputDirectory -Filter '*.zip') {
    $entries = [System.IO.Compression.ZipFile]::OpenRead($archive.FullName)
    try {
        $names = @($entries.Entries | ForEach-Object { $_.FullName.Replace('\', '/') })
        $expected = @(
            'manifest.json', 'background.js', 'options.html', 'options.js', 'popup.html', 'popup.js',
            'icons/16.png', 'icons/32.png', 'icons/48.png', 'icons/128.png'
        )
        if ($names.Count -ne $expected.Count -or @($expected | Where-Object { $_ -notin $names }).Count -gt 0) {
            throw "Package file list is incomplete or contains unexpected files: $($archive.Name) [$($names -join ', ')]"
        }

        $manifestEntry = $entries.GetEntry('manifest.json')
        $reader = [System.IO.StreamReader]::new($manifestEntry.Open())
        try { $manifest = $reader.ReadToEnd() | ConvertFrom-Json } finally { $reader.Dispose() }
        if ($manifest.manifest_version -ne 3 -or $manifest.version -ne '1.0.1') {
            throw "Unexpected manifest version in $($archive.Name)"
        }
        if ($manifest.permissions -notcontains 'downloads' -or $manifest.permissions -notcontains 'storage') {
            throw "Required permissions are missing in $($archive.Name)"
        }
        if ($manifest.options_ui.page -ne 'options.html' -or $manifest.content_scripts -or $manifest.host_permissions) {
            throw "Unexpected page access or missing options page in $($archive.Name)"
        }
        if ($manifest.action.default_popup -ne 'popup.html') {
            throw "Download toggle popup is missing in $($archive.Name)"
        }
        if ($archive.Name -like '*Firefox*') {
            if ($manifest.background.scripts -notcontains 'background.js' -or $manifest.background.service_worker) {
                throw 'Firefox background configuration is invalid.'
            }
            $gecko = $manifest.browser_specific_settings.gecko
            if ($gecko.strict_min_version -ne '142.0' -or $gecko.data_collection_permissions.required -notcontains 'none') {
                throw 'Firefox minimum version or required data declaration is invalid.'
            }
            if ($gecko.data_collection_permissions.optional -notcontains 'browsingActivity' -or $gecko.data_collection_permissions.optional -notcontains 'websiteActivity') {
                throw 'Firefox optional data categories are missing.'
            }
        } else {
            if ($manifest.permissions -notcontains 'nativeMessaging') {
                throw "Chromium Native Messaging permission is missing in $($archive.Name)"
            }
            if (-not $manifest.background.service_worker -or $manifest.background.service_worker -ne 'background.js' -or $manifest.background.scripts) {
                throw "Chromium background configuration is invalid in $($archive.Name)"
            }
            if ($manifest.browser_specific_settings) {
                throw "Firefox-specific settings leaked into $($archive.Name)"
            }
        }
    }
    finally {
        $entries.Dispose()
    }
    Write-Output ("Created {0} ({1:N0} bytes)" -f $archive.Name, $archive.Length)
}
