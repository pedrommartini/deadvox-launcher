param(
 [string]$EnginePath,
 [ValidatePattern('^[a-fA-F0-9]{64}$')][string]$ExpectedEngineSha256 = '80caaa3639d4530c77c1eea4512307bbdd812a93f6f9ab54bae5f28668810927',
 [ValidatePattern('^[A-Za-z0-9][A-Za-z0-9_.-]*$')][string]$OutputName = 'installer-0.1.0-beta.1-r2'
)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
if (!$EnginePath) { $EnginePath=Join-Path $root 'builds/installer-0.1.0-beta.1/DeadVox-Setup-0.1.0-beta.1.exe' }
$expected=$ExpectedEngineSha256.ToLowerInvariant()
if ((Get-FileHash -LiteralPath $EnginePath).Hash.ToLowerInvariant() -ne $expected) { throw 'Unexpected installation engine. Validate the engine before updating the pinned hash.' }
$framework=Join-Path $env:WINDIR 'Microsoft.NET/Framework64/v4.0.30319'
$out=Join-Path $root ('builds/'+$OutputName)
New-Item -ItemType Directory -Force -Path $out | Out-Null
$exe=Join-Path $out ('DeadVox-Setup-'+$OutputName+'.exe')
$refs=@('System.dll','System.Core.dll','System.Xaml.dll','System.Windows.Forms.dll','WindowsBase.dll','PresentationCore.dll','PresentationFramework.dll') | ForEach-Object {
 $file=Join-Path $framework $_
 if (!(Test-Path $file)) { $file=Join-Path $framework ('WPF/'+$_) }
 '/reference:'+$file
}
$argsFile=Join-Path $out 'compile.rsp'
$arguments=@('/nologo','/target:winexe','/platform:anycpu','/optimize+',('/out:"'+$exe+'"'),('/win32icon:"'+$root+'\app\src-tauri\icons\icon.ico"'),('/win32manifest:"'+$root+'\installer\Setup.manifest"'))
$arguments += $refs | ForEach-Object { '/reference:"'+$_.Substring(11)+'"' }
$arguments += @(('/resource:"'+$root+'\installer\Setup.xaml",Setup.xaml'),('/resource:"'+$root+'\app\public\assets\brand\deadvox-wordmark.png",Wordmark.png'),('/resource:"'+$root+'\app\src-tauri\icons\icon.png",Icon.png'),('/resource:"'+$EnginePath+'",Engine.exe'),('"'+$root+'\installer\Setup.cs"'))
$arguments | Set-Content -LiteralPath $argsFile -Encoding utf8
& (Join-Path $framework 'csc.exe') ('@'+$argsFile)
if ($LASTEXITCODE -ne 0) { throw 'Installer frontend compilation failed.' }
$hash=(Get-FileHash -LiteralPath $exe).Hash.ToLowerInvariant()
"$hash  $([IO.Path]::GetFileName($exe))" | Set-Content -LiteralPath (Join-Path $out 'SHA256SUMS.txt') -Encoding ascii
Write-Output $exe


