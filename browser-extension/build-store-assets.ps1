$ErrorActionPreference = 'Stop'

Add-Type -AssemblyName System.Drawing
$extensionDirectory = $PSScriptRoot
$outputDirectory = Join-Path $extensionDirectory 'store-assets'
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null

$width = 440
$height = 280
$bitmap = New-Object System.Drawing.Bitmap($width, $height)
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$graphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit

try {
    $bounds = New-Object System.Drawing.Rectangle(0, 0, $width, $height)
    $background = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        $bounds,
        [System.Drawing.Color]::FromArgb(12, 28, 58),
        [System.Drawing.Color]::FromArgb(16, 105, 125),
        25
    )
    $graphics.FillRectangle($background, $bounds)
    $background.Dispose()

    $glow = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(35, 100, 220, 210))
    $graphics.FillEllipse($glow, 250, -100, 280, 280)
    $glow.Dispose()

    $icon = [System.Drawing.Image]::FromFile((Join-Path $extensionDirectory 'icons/128.png'))
    try { $graphics.DrawImage($icon, 34, 67, 112, 112) } finally { $icon.Dispose() }

    $titleFont = [System.Drawing.Font]::new('Segoe UI', [single]29, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $subtitleFont = [System.Drawing.Font]::new('Segoe UI', [single]15, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
    $smallFont = [System.Drawing.Font]::new('Segoe UI', [single]11, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $white = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
    $muted = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(215, 235, 245, 250))
    $accent = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(130, 230, 215))
    $graphics.DrawString('Irani IDM', $titleFont, $white, 164, 83)
    $graphics.DrawString('Browser Integration', $subtitleFont, $muted, 167, 128)
    $graphics.DrawString('CHROME  /  EDGE  /  FIREFOX', $smallFont, $accent, 167, 166)

    $linePen = New-Object System.Drawing.Pen([System.Drawing.Color]::FromArgb(155, 175, 240, 230), 3)
    $graphics.DrawLine($linePen, 62, 218, 375, 218)
    $linePen.Dispose()
    $graphics.DrawString('Send downloads to your desktop manager', $subtitleFont, $muted, 34, 235)

    $bitmap.Save((Join-Path $outputDirectory 'store-promo-440x280.png'), [System.Drawing.Imaging.ImageFormat]::Png)
}
finally {
    $graphics.Dispose()
    $bitmap.Dispose()
    if ($titleFont) { $titleFont.Dispose() }
    if ($subtitleFont) { $subtitleFont.Dispose() }
    if ($smallFont) { $smallFont.Dispose() }
    if ($white) { $white.Dispose() }
    if ($muted) { $muted.Dispose() }
    if ($accent) { $accent.Dispose() }
}

# Microsoft Edge Add-ons recommends a square 300x300 extension logo.
$logoBitmap = New-Object System.Drawing.Bitmap(300, 300)
$logoGraphics = [System.Drawing.Graphics]::FromImage($logoBitmap)
$logoGraphics.Clear([System.Drawing.Color]::Transparent)
$logoGraphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
$logoGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
$sourceIcon = [System.Drawing.Image]::FromFile((Join-Path $extensionDirectory 'icons/128.png'))
try {
    $logoGraphics.DrawImage($sourceIcon, 0, 0, 300, 300)
    $logoBitmap.Save((Join-Path $outputDirectory 'edge-logo-300x300.png'), [System.Drawing.Imaging.ImageFormat]::Png)
}
finally {
    $sourceIcon.Dispose()
    $logoGraphics.Dispose()
    $logoBitmap.Dispose()
}

# Optional 1400x560 marquee asset for the Chrome Web Store and Edge Add-ons.
$marqueeBitmap = New-Object System.Drawing.Bitmap(1400, 560)
$marqueeGraphics = [System.Drawing.Graphics]::FromImage($marqueeBitmap)
$marqueeGraphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$marqueeGraphics.TextRenderingHint = [System.Drawing.Text.TextRenderingHint]::AntiAliasGridFit
$marqueeBounds = New-Object System.Drawing.Rectangle(0, 0, 1400, 560)
$marqueeBrush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
    $marqueeBounds,
    [System.Drawing.Color]::FromArgb(10, 24, 50),
    [System.Drawing.Color]::FromArgb(18, 112, 128),
    20
)
$marqueeTitle = [System.Drawing.Font]::new('Segoe UI', [single]68, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
$marqueeSubtitle = [System.Drawing.Font]::new('Segoe UI', [single]30, [System.Drawing.FontStyle]::Regular, [System.Drawing.GraphicsUnit]::Pixel)
$marqueeWhite = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::White)
$marqueeMuted = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(220, 235, 245, 250))
$marqueeIcon = [System.Drawing.Image]::FromFile((Join-Path $extensionDirectory 'icons/128.png'))
try {
    $marqueeGraphics.FillRectangle($marqueeBrush, $marqueeBounds)
    $marqueeGlow = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(28, 120, 230, 210))
    $marqueeGraphics.FillEllipse($marqueeGlow, 940, -180, 650, 650)
    $marqueeGlow.Dispose()
    $marqueeGraphics.DrawImage($marqueeIcon, 150, 155, 250, 250)
    $marqueeGraphics.DrawString('Irani IDM', $marqueeTitle, $marqueeWhite, 470, 190)
    $marqueeGraphics.DrawString('Browser download integration', $marqueeSubtitle, $marqueeMuted, 478, 285)
    $marqueeBitmap.Save((Join-Path $outputDirectory 'store-marquee-1400x560.png'), [System.Drawing.Imaging.ImageFormat]::Png)
}
finally {
    $marqueeIcon.Dispose()
    $marqueeBrush.Dispose()
    $marqueeTitle.Dispose()
    $marqueeSubtitle.Dispose()
    $marqueeWhite.Dispose()
    $marqueeMuted.Dispose()
    $marqueeGraphics.Dispose()
    $marqueeBitmap.Dispose()
}

Write-Output "Created: $(Join-Path $outputDirectory 'store-promo-440x280.png')"
Write-Output "Created: $(Join-Path $outputDirectory 'edge-logo-300x300.png')"
Write-Output "Created: $(Join-Path $outputDirectory 'store-marquee-1400x560.png')"
