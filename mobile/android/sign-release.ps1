param(
    [Parameter(Mandatory=$true)][string]$AndroidSdk,
    [Parameter(Mandatory=$true)][string]$SigningDirectory
)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$tools = Join-Path $AndroidSdk 'build-tools\35.0.0'
$key = Join-Path $SigningDirectory 'musiccraft-release.p12'
$passwordFile = Join-Path $SigningDirectory 'password.txt'
$inputApk = Join-Path $root 'build-local\aligned.apk'
$outputApk = Join-Path $root 'MusicCraft-Android-firmado.apk'
if (!(Test-Path -LiteralPath $key) -or !(Test-Path -LiteralPath $passwordFile)) { throw 'No se encontró la clave o su contraseña.' }
if (!(Test-Path -LiteralPath $inputApk)) { throw 'Primero hay que compilar la aplicación.' }
$env:MUSICCRAFT_SIGNING_SECRET = Get-Content -LiteralPath $passwordFile -Raw
try {
    & (Join-Path $tools 'apksigner.bat') sign --ks $key --ks-key-alias musiccraft --ks-pass env:MUSICCRAFT_SIGNING_SECRET --out $outputApk $inputApk
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo firmar la aplicación.' }
    & (Join-Path $tools 'apksigner.bat') verify --verbose --print-certs $outputApk
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo verificar la firma.' }
} finally { Remove-Item Env:MUSICCRAFT_SIGNING_SECRET -ErrorAction SilentlyContinue }
Write-Host "Aplicación firmada: $outputApk"
