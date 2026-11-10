param(
    [Parameter(Mandatory=$true)][ValidatePattern('^[A-Za-z0-9.-]{3,50}$')][string]$IdentityName,
    [Parameter(Mandatory=$true)][ValidatePattern('^CN=.+')][string]$Publisher,
    [Parameter(Mandatory=$true)][ValidateLength(1,256)][string]$PublisherDisplayName,
    [Parameter(Mandatory=$true)][ValidatePattern('^\\d{1,5}\\.\\d{1,5}\\.\\d{1,5}\\.\\d{1,5}$')][string]$PackageVersion,
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_.-]*$')][string]$OutputName = 'store',
    [switch]$SkipTests
)
$ErrorActionPreference = 'Stop'
$workspace = Split-Path -Parent $PSScriptRoot
$makeAppx = Join-Path ([Environment]::GetFolderPath('ProgramFilesX86')) 'Windows Kits\\10\\bin\\x64\\makeappx.exe'
if (!(Test-Path -LiteralPath $makeAppx -PathType Leaf)) { throw 'Install the Windows SDK with MakeAppx before building a Store package.' }
$versionParts = $PackageVersion.Split('.') | ForEach-Object { [int]$_ }
if ($versionParts | Where-Object { $_ -gt 65535 }) { throw 'Each MSIX version component must be at most 65535.' }

$buildParameters = @{ Store = $true; OutputName = $OutputName }
if ($SkipTests) { $buildParameters.SkipTests = $true }
& (Join-Path $PSScriptRoot 'build.ps1') @buildParameters
if ($LASTEXITCODE -ne 0) { throw 'Store launcher build failed.' }

$output = Join-Path $workspace ('builds/' + $OutputName)
$stage = Join-Path $output '.msix-stage'
if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
$programFiles = Join-Path $stage 'VFS\\ProgramFilesX64\\DeadVox'
$assets = Join-Path $stage 'Assets'
New-Item -ItemType Directory -Path $programFiles, $assets -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $output 'DeadVox.exe') -Destination (Join-Path $programFiles 'DeadVox.exe')
Add-Type -AssemblyName System.Drawing
function Save-IconAsset([string]$Path, [int]$Width, [int]$Height) {
    $source = [System.Drawing.Image]::FromFile((Join-Path $workspace 'app\\src-tauri\\icons\\icon.png'))
    try {
        $bitmap = [System.Drawing.Bitmap]::new($Width, $Height)
        $graphics = [System.Drawing.Graphics]::FromImage($bitmap)
        try {
            $graphics.Clear([System.Drawing.Color]::FromArgb(13, 13, 15))
            $size = [Math]::Min($Width, $Height)
            $x = [int](($Width - $size) / 2)
            $y = [int](($Height - $size) / 2)
            $graphics.DrawImage($source, $x, $y, $size, $size)
            $bitmap.Save($Path, [System.Drawing.Imaging.ImageFormat]::Png)
        } finally {
            $graphics.Dispose()
            $bitmap.Dispose()
        }
    } finally {
        $source.Dispose()
    }
}
@(
    @{ Name = 'StoreLogo.png'; Width = 50; Height = 50 },
    @{ Name = 'Square44x44Logo.png'; Width = 44; Height = 44 },
    @{ Name = 'Square150x150Logo.png'; Width = 150; Height = 150 },
    @{ Name = 'Square71x71Logo.png'; Width = 71; Height = 71 },
    @{ Name = 'Square310x310Logo.png'; Width = 310; Height = 310 },
    @{ Name = 'Wide310x150Logo.png'; Width = 310; Height = 150 }
) | ForEach-Object {
    Save-IconAsset (Join-Path $assets $_.Name) $_.Width $_.Height
}
$xml = @"
<?xml version="1.0" encoding="utf-8"?>
<Package xmlns="http://schemas.microsoft.com/appx/manifest/foundation/windows10"
         xmlns:uap="http://schemas.microsoft.com/appx/manifest/uap/windows10"
         xmlns:rescap="http://schemas.microsoft.com/appx/manifest/foundation/windows10/restrictedcapabilities"
         IgnorableNamespaces="uap rescap">
  <Identity Name="$IdentityName" Publisher="$Publisher" Version="$PackageVersion" ProcessorArchitecture="x64" />
  <Properties>
    <DisplayName>DeadVox</DisplayName>
    <PublisherDisplayName>$PublisherDisplayName</PublisherDisplayName>
    <Logo>Assets\\StoreLogo.png</Logo>
  </Properties>
  <Resources><Resource Language="pt-br" /></Resources>
  <Applications>
    <Application Id="App" Executable="VFS\\ProgramFilesX64\\DeadVox\\DeadVox.exe" EntryPoint="Windows.FullTrustApplication">
      <uap:VisualElements DisplayName="DeadVox" Description="Brazilian Portuguese voice-pack launcher for Deadlock." BackgroundColor="#0D0D0F" Square150x150Logo="Assets\\Square150x150Logo.png" Square44x44Logo="Assets\\Square44x44Logo.png">
        <uap:DefaultTile Wide310x150Logo="Assets\\Wide310x150Logo.png" Square71x71Logo="Assets\\Square71x71Logo.png" Square310x310Logo="Assets\\Square310x310Logo.png" />
      </uap:VisualElements>
    </Application>
  </Applications>
  <Capabilities><rescap:Capability Name="runFullTrust" /></Capabilities>
</Package>
"@
Set-Content -LiteralPath (Join-Path $stage 'AppxManifest.xml') -Value $xml -Encoding utf8NoBOM
$package = Join-Path $output 'DeadVox.msix'
if (Test-Path -LiteralPath $package) { Remove-Item -LiteralPath $package -Force }
& $makeAppx pack /d $stage /p $package /o
if ($LASTEXITCODE -ne 0) { throw 'MSIX packaging failed.' }
$verification = Join-Path $output '.msix-verify'
& $makeAppx unpack /p $package /d $verification /o
if ($LASTEXITCODE -ne 0) { throw 'MSIX verification failed.' }
Remove-Item -LiteralPath $stage, $verification -Recurse -Force
Write-Host "Unsigned Store package: $package"
