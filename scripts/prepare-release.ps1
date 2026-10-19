param(
    [Parameter(Mandatory=$true)][ValidateSet('client','voices')][string]$Kind,
    [Parameter(Mandatory=$true)][string]$Repository,
    [switch]$BuildClient,
    [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_.-]*$')][string]$ClientBuildName = 'pre-alpha',
    [string]$MinimumClientVersion = '0.1.0-alpha.5'
)
$ErrorActionPreference = 'Stop'
$appWorkspace = Split-Path -Parent $PSScriptRoot
if ($Repository -notmatch '^[A-Za-z0-9_.-]{1,100}/[A-Za-z0-9_.-]{1,100}$' -or ($Repository.Split('/') | Where-Object { $_ -in @('.','..') })) { throw 'Use owner/repository for the GitHub repository.' }
$config = Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'app/src-tauri/resources/update-source.json') | ConvertFrom-Json
if ($config.repository -and $config.repository -cne $Repository) { throw 'Repository differs from the official build source.' }
$versionPattern = '^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$'
if ($MinimumClientVersion -notmatch $versionPattern) { throw 'Invalid minimum client version.' }
function Assert-Fields($Value, [string[]]$Expected) {
    if (@(Compare-Object @($Value.PSObject.Properties.Name | Sort-Object) @($Expected | Sort-Object)).Count) { throw 'Unexpected public metadata fields.' }
}
function Copy-Verified([string]$Source, [string]$Target, [long]$Bytes, [string]$Hash) {
    if ((Get-Item -LiteralPath $Source).Length -ne $Bytes -or (Get-FileHash -LiteralPath $Source -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Hash) { throw 'Release source integrity failed.' }
    Copy-Item -LiteralPath $Source -Destination $Target
    if ((Get-FileHash -LiteralPath $Target -Algorithm SHA256).Hash.ToLowerInvariant() -ne $Hash) { throw 'Release copy integrity failed.' }
}
$deployRoot = Join-Path $appWorkspace 'builds/deploy'
New-Item -ItemType Directory -Path $deployRoot -Force | Out-Null
$stage = Join-Path $deployRoot ('.staging-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stage | Out-Null
try {
    if ($Kind -eq 'client') {
        if (!$config.repository) { throw 'Configure the official repository and rebuild before preparing a distribution client.' }
        if ($BuildClient) { & (Join-Path $PSScriptRoot 'build.ps1') -OutputName $ClientBuildName }
        $version = (Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'app/package.json') | ConvertFrom-Json).version
        if ($version -notmatch $versionPattern) { throw 'Invalid client version.' }
        $tag = 'client-v' + $version
        $output = Join-Path $deployRoot ('client/' + $version)
        $source = Join-Path $appWorkspace ('builds/' + $ClientBuildName + '/DeadVox.exe')
        $compiled = Join-Path $appWorkspace '.cache/cargo-target/release/deadlock-dubbing.exe'
        if (!(Test-Path -LiteralPath $source -PathType Leaf) -or !(Test-Path -LiteralPath $compiled -PathType Leaf)) { throw 'Build the launcher first.' }
        $hash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash.ToLowerInvariant()
        $receipt = Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'builds/build-receipt.json') | ConvertFrom-Json
        if ($receipt.executable_sha256 -cne $hash -or $receipt.version -cne $version -or $receipt.update_source_sha256 -cne (Get-FileHash -LiteralPath (Join-Path $appWorkspace 'app/src-tauri/resources/update-source.json')).Hash.ToLowerInvariant()) { throw 'Update source changed after the build. Rebuild the client.' }
        if ((Get-FileHash -LiteralPath $compiled).Hash.ToLowerInvariant() -ne $hash -or (Get-Item -LiteralPath $source).VersionInfo.ProductVersion -ne $version) { throw 'Rebuild the portable client before preparing its release.' }
        $length = (Get-Item -LiteralPath $source).Length
        if ($length -gt 250MB) { throw 'Client exceeds the update size limit.' }
        Copy-Verified $source (Join-Path $stage 'DeadVox.exe') $length $hash
        $manifest = [ordered]@{ schema_version = 1; version = $version; bytes = $length; sha256 = $hash; download_url = "https://github.com/$Repository/releases/download/$tag/DeadVox.exe" }
        $manifestName = 'client-update.json'
    } else {
        $catalog = Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'app/src-tauri/resources/voice-packs.json') | ConvertFrom-Json
        Assert-Fields $catalog @('schema_version','packs')
        if ($catalog.schema_version -ne 1 -or @($catalog.packs).Count -lt 1 -or @($catalog.packs).Count -gt 512) { throw 'Unsupported voice catalog.' }
        $version = 'catalog-' + (Get-Date -AsUTC -Format 'yyyyMMddTHHmmssfffffffZ')
        $tag = 'voices-pt-br-' + $version
        $output = Join-Path $deployRoot ('voices/pt-BR/' + $version)
        $ids = @(); $heroIds = @(); $names = @()
        $publicPacks = @($catalog.packs | ForEach-Object {
            $pack = $_
            Assert-Fields $pack @('id','language','version','file','sha256','bytes','entry_count','heroes','download_url')
            if ($pack.id -cnotmatch '^[a-z0-9-]{1,100}$' -or $ids -contains $pack.id -or $pack.language -cne 'pt-BR' -or $pack.version -notmatch $versionPattern -or $pack.file -cnotmatch '^voice-packs/pt-BR/[A-Za-z0-9][A-Za-z0-9_.+-]*_dir\.vpk$' -or $pack.sha256 -cnotmatch '^[a-f0-9]{64}$' -or $pack.bytes -le 0 -or $pack.bytes -ge 2GB -or $pack.entry_count -le 0) { throw 'Invalid public voice pack metadata.' }
            $ids += $pack.id
            if (@($pack.heroes).Count -eq 0) { throw 'Missing hero coverage.' }
            $speech = 0
            $publicHeroes = @($pack.heroes | ForEach-Object {
                Assert-Fields $_ @('asset_id','total','available')
                if ($_.asset_id -cnotmatch '^[a-z0-9_]+$' -or $heroIds -contains $_.asset_id -or $_.total -lt 0 -or $_.available -lt 0 -or $_.available -gt $_.total) { throw 'Invalid or overlapping hero coverage.' }
                $heroIds += $_.asset_id; $speech += $_.available
                [ordered]@{ asset_id = $_.asset_id; total = $_.total; available = $_.available }
            })
            if ($speech -gt $pack.entry_count) { throw 'Coverage exceeds packaged assets.' }
            $name = $pack.id + '-' + $pack.version + '_dir.vpk'
            if ($names -contains $name) { throw 'Duplicate release filename.' }; $names += $name
            Copy-Verified (Join-Path $appWorkspace $pack.file) (Join-Path $stage $name) $pack.bytes $pack.sha256
            # Existing hosted packs retain their immutable URL; only new packs point to this release.
            $url = "https://github.com/$Repository/releases/download/$tag/$name"
            if ($pack.download_url) {
                if ($pack.download_url -cnotmatch ('^https://github\.com/' + [regex]::Escape($Repository) + '/releases/download/[A-Za-z0-9_.+-]+/[A-Za-z0-9_.+-]+_dir\.vpk$')) { throw 'Untrusted existing pack URL.' }
                $url = $pack.download_url
            }
            [ordered]@{ id = $pack.id; language = $pack.language; version = $pack.version; file = "voice-packs/pt-BR/$name"; sha256 = $pack.sha256; bytes = $pack.bytes; entry_count = $pack.entry_count; heroes = $publicHeroes; download_url = $url }
        })
        $manifest = [ordered]@{ schema_version = 1; minimum_client_version = $MinimumClientVersion; packs = $publicPacks }
        $manifestName = 'voice-update.json'
    }
    $manifest | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $stage $manifestName) -Encoding utf8NoBOM
    if (Test-Path -LiteralPath $output) {
        $old = @(Get-ChildItem -LiteralPath $output -File -Force); $new = @(Get-ChildItem -LiteralPath $stage -File -Force)
        if ($old.Count -ne $new.Count -or @(Get-ChildItem -LiteralPath $output -Directory -Force).Count) { throw 'Existing release contains different files. Use a new version.' }
        foreach ($file in $new) {
            $previous = Join-Path $output $file.Name
            if (!(Test-Path -LiteralPath $previous -PathType Leaf) -or (Get-FileHash -LiteralPath $previous).Hash -ne (Get-FileHash -LiteralPath $file.FullName).Hash) { throw 'An immutable release already exists with different content. Use a new version.' }
        }
    } else {
        New-Item -ItemType Directory -Path (Split-Path -Parent $output) -Force | Out-Null
        Move-Item -LiteralPath $stage -Destination $output
    }
    Write-Host "Prepared $tag locally: $output"
    Write-Host 'No upload or ZIP. Only final assets and public metadata are included.'
} finally {
    $resolvedStage = [IO.Path]::GetFullPath($stage)
    if ($resolvedStage.StartsWith([IO.Path]::GetFullPath($deployRoot) + [IO.Path]::DirectorySeparatorChar) -and (Test-Path -LiteralPath $resolvedStage)) { Remove-Item -LiteralPath $resolvedStage -Recurse -Force }
}
