$ErrorActionPreference = 'Stop'

$extensionDirectory = $PSScriptRoot
$outputDirectory = Join-Path $extensionDirectory 'firefox-build'

if (Test-Path -LiteralPath $outputDirectory) {
    Remove-Item -LiteralPath $outputDirectory -Recurse -Force
}

New-Item -ItemType Directory -Path $outputDirectory | Out-Null
Copy-Item -LiteralPath (Join-Path $extensionDirectory 'background.js') -Destination $outputDirectory
Copy-Item -LiteralPath (Join-Path $extensionDirectory 'options.html') -Destination $outputDirectory
Copy-Item -LiteralPath (Join-Path $extensionDirectory 'options.js') -Destination $outputDirectory
Copy-Item -LiteralPath (Join-Path $extensionDirectory 'icons') -Destination $outputDirectory -Recurse
Copy-Item -LiteralPath (Join-Path $extensionDirectory 'manifest.firefox.json') -Destination (Join-Path $outputDirectory 'manifest.json')

Write-Output "Firefox extension package created at: $outputDirectory"
