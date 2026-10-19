$ErrorActionPreference = 'Stop'
$appWorkspace = Split-Path -Parent $PSScriptRoot
$sourceDirectory = Join-Path $appWorkspace 'builds/pre-alpha'
$testDirectory = Join-Path $appWorkspace ('.cache/portable-smoke-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $testDirectory -Force | Out-Null
Copy-Item -LiteralPath (Join-Path $sourceDirectory 'DeadVox.exe') -Destination $testDirectory
Copy-Item -LiteralPath (Join-Path $sourceDirectory 'voice-packs') -Destination $testDirectory -Recurse
$executable = Join-Path $testDirectory 'DeadVox.exe'
$reportPath = Join-Path $testDirectory 'user-data/native-check.json'
function Invoke-NativeCheck([string]$Action) {
    $process = Start-Process -FilePath $executable -ArgumentList "--check=$Action" -WorkingDirectory $testDirectory -WindowStyle Hidden -Wait -PassThru
    $snapshot = Get-Content -Raw -LiteralPath $reportPath | ConvertFrom-Json
    if ($process.ExitCode -ne 0 -or $snapshot.error -or $snapshot.game_error) { throw "Native operation $Action failed: $($snapshot | ConvertTo-Json -Compress -Depth 5)" }
    return $snapshot
}
$initial = Invoke-NativeCheck 'status'
if ($initial.game_running) { throw 'Close Deadlock before verifying the portable build.' }
$gameRoot = $initial.game_root
$gameinfo = Join-Path $gameRoot 'game/citadel/gameinfo.gi'
$before = [IO.File]::ReadAllText($gameinfo)
# Strip only the owned current/legacy block to determine the exact vanilla baseline.
$begin = @('// Deadlock Dubbing launcher: begin','// Deadlock AI Dubbing launcher: begin')
$end = @('// Deadlock Dubbing launcher: end','// Deadlock AI Dubbing launcher: end')
$segments = [regex]::Matches($before, '[^\n]*\n|[^\n]+$')
$skip = $false
$baseline = ''
foreach ($segment in $segments) {
    $trimmed = $segment.Value.Trim()
    if ($begin -contains $trimmed) { $skip = $true }
    elseif ($end -contains $trimmed) { $skip = $false }
    elseif (!$skip) { $baseline += $segment.Value }
}
$checks = @()
foreach ($action in @('disable','enable','status')) {
    $snapshot = Invoke-NativeCheck $action
    $expected = if ($action -eq 'disable') { 'en' } else { 'pt-BR' }
    $hero = $snapshot.heroes | Where-Object { $_.asset_id -eq 'ghost' }
    if ($snapshot.language -ne $expected -or $hero.available -ne 1835 -or $hero.coverage -ne 100) { throw "Incorrect portable state after $action." }
    if ($action -eq 'disable' -and [IO.File]::ReadAllText($gameinfo) -cne $baseline) { throw 'The original game configuration was not restored exactly.' }
    $checks += [ordered]@{ action = $action; language = $snapshot.language; available_pt_br = $hero.available; coverage = $hero.coverage }
    Write-Host "Portable executable: $action -> $($snapshot.language), $($hero.available) PT-BR lines."
}
$result = [ordered]@{ portable_copy = $true; native_only = $true; checks = $checks; exact_original_configuration_restored = $true; final_language = 'pt-BR'; game_launched = $false }
$result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $appWorkspace 'builds/portable-verification.json') -Encoding utf8
