param([switch]$WithVoicePack, [switch]$SkipTests, [switch]$Store,
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_.-]*$')][string]$OutputName = 'pre-alpha')
$ErrorActionPreference = 'Stop'
$appWorkspace = Split-Path -Parent $PSScriptRoot
$cacheRoot = Join-Path $appWorkspace '.cache'
foreach ($folder in @('tmp','npm','cargo-home','cargo-target')) { New-Item -ItemType Directory -Path (Join-Path $cacheRoot $folder) -Force | Out-Null }
$env:TEMP = Join-Path $cacheRoot 'tmp'
$env:TMP = $env:TEMP
$env:npm_config_cache = Join-Path $cacheRoot 'npm'
$env:CARGO_HOME = Join-Path $cacheRoot 'cargo-home'
$env:CARGO_TARGET_DIR = Join-Path $cacheRoot 'cargo-target'
Push-Location (Join-Path $appWorkspace 'app')
try {
    if (!(Test-Path -LiteralPath 'node_modules')) {
        npm.cmd ci --no-audit --no-fund
        if ($LASTEXITCODE -ne 0) { throw 'Launcher dependency installation failed.' }
    }
    if (!$SkipTests) {
        cargo test --manifest-path src-tauri/Cargo.toml --locked $(if ($Store) { "--features store" })
        if ($LASTEXITCODE -ne 0) { throw 'Native launcher checks failed.' }
    }
    $desktopBuild = @('run', 'desktop:build')
    if ($Store) { $desktopBuild += @('--', '--features', 'store') }
    & npm.cmd @desktopBuild
    if ($LASTEXITCODE -ne 0) { throw 'Standalone launcher compilation failed.' }
} finally { Pop-Location }
$outputDirectory = Join-Path $appWorkspace ('builds/' + $OutputName)
New-Item -ItemType Directory -Path $outputDirectory -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $env:CARGO_TARGET_DIR 'release/deadlock-dubbing.exe') -Destination (Join-Path $outputDirectory 'DeadVox.exe') -Force
Copy-Item -LiteralPath (Join-Path $appWorkspace 'docs/PORTABLE-README.txt') -Destination (Join-Path $outputDirectory 'README.txt') -Force
Copy-Item -LiteralPath (Join-Path $appWorkspace 'docs/THIRD-PARTY-NOTICES.txt') -Destination (Join-Path $outputDirectory 'THIRD-PARTY-NOTICES.txt') -Force
$receipt = [ordered]@{
    version = (Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'app/package.json') | ConvertFrom-Json).version
    store = [bool]$Store
    executable_sha256 = (Get-FileHash -LiteralPath (Join-Path $outputDirectory 'DeadVox.exe')).Hash.ToLowerInvariant()
    update_source_sha256 = (Get-FileHash -LiteralPath (Join-Path $appWorkspace 'app/src-tauri/resources/update-source.json')).Hash.ToLowerInvariant()
    third_party_notices_sha256 = (Get-FileHash -LiteralPath (Join-Path $outputDirectory 'THIRD-PARTY-NOTICES.txt')).Hash.ToLowerInvariant()
}
$receipt | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $appWorkspace 'builds/build-receipt.json') -Encoding utf8
if ($WithVoicePack) {
    $catalog = Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'app/src-tauri/resources/voice-packs.json') | ConvertFrom-Json
    foreach ($pack in $catalog.packs) {
        $source = Join-Path $appWorkspace $pack.file
        if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant() -ne $pack.sha256) { throw 'Voice-pack checksum failed.' }
        $target = Join-Path $outputDirectory $pack.file
        New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
        Copy-Item -LiteralPath $source -Destination $target -Force
    }
}
$legacyClient = Join-Path $outputDirectory 'Deadlock Dubbing.exe'
if (Test-Path -LiteralPath $legacyClient) {
    $previousClients = Join-Path $appWorkspace 'builds/previous-clients'
    New-Item -ItemType Directory -Path $previousClients -Force | Out-Null
    $previousClient = Join-Path $previousClients ('Deadlock-Dubbing-' + (Get-Date -Format 'yyyyMMdd-HHmmssfff') + '.exe')
    Move-Item -LiteralPath $legacyClient -Destination $previousClient
}
& (Join-Path $PSScriptRoot 'audit-publication.ps1') -PortableDirectory $outputDirectory
Write-Host "Standalone launcher: $outputDirectory\DeadVox.exe"
Write-Host "Portable folder: $outputDirectory"
