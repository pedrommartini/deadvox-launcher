param([string]$PortableDirectory, [switch]$ForDistribution)
$ErrorActionPreference = 'Stop'
$appWorkspace = Split-Path -Parent $PSScriptRoot
$sourceFiles = & git -C $appWorkspace ls-files --cached --others --exclude-standard
if ($LASTEXITCODE -ne 0) { throw 'Initialize the local application repository before auditing publication.' }
$allowed = @('^\.gitignore$','^README\.md$','^RELEASE-NOTES\.md$','^app/src/[^/]+\.(ts|tsx|css)$','^app/public/assets/heroes\.json$','^app/public/assets/brand/(deadvox-wordmark|deadvox-icon)\.png$','^app/public/assets/support/(curiosity-shop-support|support-characters-v2)\.png$','^app/public/assets/heroes/\d+\.(webp|png)$','^app/(index\.html|package(-lock)?\.json|tsconfig\.json|vite\.config\.ts)$','^app/src-tauri/src/[^/]+\.rs$','^app/src-tauri/(Cargo\.(toml|lock)|build\.rs|tauri\.conf\.json)$','^app/src-tauri/resources/(voice-packs|update-source)\.json$','^app/src-tauri/capabilities/default\.json$','^app/src-tauri/icons/[^/]+\.(ico|png|icns)$','^scripts/(build|audit-publication|verify-portable|prepare-release|prepare-portable)\.ps1$','^scripts/third-party-notices\.mjs$','^docs/THIRD-PARTY-NOTICES\.txt$','^docs/third-party-license-overrides\.json$','^docs/(public-pack-contract\.md|TESTING\.md|PORTABLE-README\.txt|UPDATES\.md)$','^\.github/workflows/windows\.yml$')
foreach ($file in $sourceFiles) {
    if (!($allowed | Where-Object { $file -match $_ })) { throw "Unexpected repository file: $file" }
}
$portableRoot = if ($PortableDirectory) { [IO.Path]::GetFullPath($PortableDirectory) } else { Join-Path $appWorkspace 'builds/pre-alpha' }
if (!$portableRoot.StartsWith([IO.Path]::GetFullPath((Join-Path $appWorkspace 'builds')) + [IO.Path]::DirectorySeparatorChar)) { throw 'Audit only build folders inside this workspace.' }
foreach ($required in @('DeadVox.exe','README.txt')) {
    if (!(Test-Path -LiteralPath (Join-Path $portableRoot $required) -PathType Leaf)) { throw "Missing portable file: $required" }
}
if ($ForDistribution -and !(Test-Path -LiteralPath (Join-Path $portableRoot 'THIRD-PARTY-NOTICES.txt') -PathType Leaf)) { throw 'Distribution requires third-party notices.' }
$localRuntimeFiles = @()
$entries = @(Get-ChildItem -LiteralPath $portableRoot -Recurse -File -Force | ForEach-Object {
    $relative = $_.FullName.Substring($portableRoot.Length + 1).Replace('\','/')
    if ($relative.StartsWith('user-data/')) {
        if ($ForDistribution) { throw 'Distribution folder contains user settings or backups.' }
        $localRuntimeFiles += $relative
    } else {
        if ($relative -notin @('DeadVox.exe','README.txt','THIRD-PARTY-NOTICES.txt') -and $relative -notmatch '^voice-packs/pt-BR/[A-Za-z0-9][A-Za-z0-9_.+-]*_dir\.vpk$') { throw "Unexpected portable file: $relative" }
        $relative
    }
})
$catalog = Get-Content -Raw -LiteralPath (Join-Path $appWorkspace 'app/src-tauri/resources/voice-packs.json') | ConvertFrom-Json
foreach ($pack in $catalog.packs) {
    $packPath = Join-Path $portableRoot $pack.file
    if (Test-Path -LiteralPath $packPath -PathType Leaf) {
        if ((Get-Item -LiteralPath $packPath).Length -ne $pack.bytes -or (Get-FileHash -LiteralPath $packPath -Algorithm SHA256).Hash.ToLowerInvariant() -ne $pack.sha256) { throw 'Portable voice-pack integrity failed.' }
    }
}
$result = [ordered]@{ source_file_count = @($sourceFiles).Count; source_allowlist_passed = $true; portable_folder = $portableRoot; release_files = $entries; release_allowlist_passed = $true; local_runtime_files_excluded = $localRuntimeFiles; remove_user_data_before_external_distribution = ($localRuntimeFiles.Count -gt 0) }
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $appWorkspace 'builds/publication-audit.json') -Encoding utf8
Write-Host "Allowlist passed: $(@($sourceFiles).Count) private source files, $($entries.Count) distribution files. Never publish the source tree. No ZIP or publication."
