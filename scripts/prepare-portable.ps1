param(
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_.-]*$')][string]$BuildName = 'pre-alpha',
    [Parameter(Mandatory=$true)][ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_.-]*$')][string]$ReleaseName,
    [switch]$WithVoicePack
)
$ErrorActionPreference = 'Stop'
$appWorkspace = Split-Path -Parent $PSScriptRoot
$buildsRoot = [IO.Path]::GetFullPath((Join-Path $appWorkspace 'builds'))
$source = Join-Path $buildsRoot $BuildName
$destination = [IO.Path]::GetFullPath((Join-Path $buildsRoot ('distribution/' + $ReleaseName)))
if (!$destination.StartsWith($buildsRoot + [IO.Path]::DirectorySeparatorChar)) { throw 'Distribution must stay inside builds.' }
if (Test-Path -LiteralPath $destination) { throw 'Choose a new distribution name; existing folders are preserved.' }
$receipt = Get-Content -Raw -LiteralPath (Join-Path $buildsRoot 'build-receipt.json') | ConvertFrom-Json
$clientPath = Join-Path $source 'DeadVox.exe'
$noticesPath = Join-Path $source 'THIRD-PARTY-NOTICES.txt'
if ((Get-FileHash -LiteralPath $clientPath).Hash.ToLowerInvariant() -cne $receipt.executable_sha256 -or
    (Get-FileHash -LiteralPath $noticesPath).Hash.ToLowerInvariant() -cne $receipt.third_party_notices_sha256) { throw 'Rebuild this client and its notices before preparing distribution.' }
$files = @('DeadVox.exe','README.txt','THIRD-PARTY-NOTICES.txt')
if ($WithVoicePack) {
    $catalog = Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'app/src-tauri/resources/voice-packs.json') | ConvertFrom-Json
    $files += @($catalog.packs | ForEach-Object {
        if ($_.file -cnotmatch '^voice-packs/pt-BR/[A-Za-z0-9][A-Za-z0-9_.+-]*_dir\.vpk$') { throw 'Invalid voice-pack path.' }
        $packSource = Join-Path $source $_.file
        if ((Get-Item -LiteralPath $packSource).Length -ne $_.bytes -or
            (Get-FileHash -LiteralPath $packSource).Hash.ToLowerInvariant() -cne $_.sha256) { throw 'Voice-pack integrity failed.' }
        $_.file
    })
}
# Copy only explicitly selected public files. Never copy the development folder or user-data.
foreach ($file in $files) {
    $original = Join-Path $source $file
    $target = Join-Path $destination $file
    New-Item -ItemType Directory -Path (Split-Path -Parent $target) -Force | Out-Null
    Copy-Item -LiteralPath $original -Destination $target
    if ((Get-FileHash -LiteralPath $original).Hash -cne (Get-FileHash -LiteralPath $target).Hash) { throw 'Distribution copy integrity failed.' }
}
& (Join-Path $PSScriptRoot 'audit-publication.ps1') -PortableDirectory $destination -ForDistribution
Write-Host "Prepared clean portable folder: $destination"
Write-Host 'No ZIP, upload, game modification or publication was performed.'
