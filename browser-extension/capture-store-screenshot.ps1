$ErrorActionPreference = 'Stop'

$extensionDirectory = $PSScriptRoot
$edgeCandidates = @(
    (Join-Path ([Environment]::GetEnvironmentVariable('ProgramFiles(x86)')) 'Microsoft\Edge\Application\msedge.exe'),
    (Join-Path $env:ProgramFiles 'Microsoft\Edge\Application\msedge.exe')
)
$edge = $edgeCandidates | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1
if (-not $edge) {
    throw 'Microsoft Edge was not found. Install Edge or capture the Options page at 1280x800 in another browser.'
}

$screenshotDirectory = Join-Path $extensionDirectory 'store-assets/screenshots'
New-Item -ItemType Directory -Path $screenshotDirectory -Force | Out-Null
$screenshot = Join-Path $screenshotDirectory 'consent-first-run-1280x800.png'
$optionsPage = (Join-Path $extensionDirectory 'options.html').Replace('\', '/')
$pageUrl = "file:///$optionsPage"

& $edge --headless=new --disable-gpu --no-first-run --hide-scrollbars --window-size=1280,800 "--screenshot=$screenshot" $pageUrl
if (-not (Test-Path -LiteralPath $screenshot -PathType Leaf)) {
    throw 'Edge did not create the store screenshot.'
}

Add-Type -AssemblyName System.Drawing
$image = [System.Drawing.Image]::FromFile($screenshot)
try {
    if ($image.Width -ne 1280 -or $image.Height -ne 800) {
        throw "Store screenshot must be 1280x800; got $($image.Width)x$($image.Height)."
    }
}
finally {
    $image.Dispose()
}

Write-Output "Created: $screenshot"
