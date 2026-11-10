param([Parameter(Mandatory=$true)][string]$AndroidSdk)
$ErrorActionPreference = 'Stop'
$root = $PSScriptRoot
$build = Join-Path $root 'build-local'
$platform = Join-Path $AndroidSdk 'platforms\android-35\android.jar'
$tools = Join-Path $AndroidSdk 'build-tools\35.0.0'
New-Item -ItemType Directory -Force (Join-Path $build 'classes'),(Join-Path $build 'dex') | Out-Null
$javaFiles = Get-ChildItem (Join-Path $root 'app\src\main\java') -Filter '*.java' -Recurse | ForEach-Object { $_.FullName }
& javac -encoding UTF-8 -source 8 -target 8 -classpath $platform -d (Join-Path $build 'classes') $javaFiles
if ($LASTEXITCODE -ne 0) { throw 'No se pudo compilar la actividad Android.' }
$classes = Get-ChildItem (Join-Path $build 'classes') -Filter '*.class' -Recurse | ForEach-Object { $_.FullName }
& (Join-Path $tools 'd8.bat') --min-api 26 --lib $platform --output (Join-Path $build 'dex') $classes
if ($LASTEXITCODE -ne 0) { throw 'No se pudo generar el código Android.' }
$unaligned = Join-Path $build 'unsigned.apk'
& (Join-Path $tools 'aapt2.exe') compile --dir (Join-Path $root 'app\src\main\res') -o (Join-Path $build 'resources.zip')
if ($LASTEXITCODE -ne 0) { throw 'No se pudieron compilar los recursos Android.' }
& (Join-Path $tools 'aapt2.exe') link -I $platform --manifest (Join-Path $root 'app\src\main\AndroidManifest.xml') --min-sdk-version 26 --target-sdk-version 35 -A (Join-Path $root 'app\src\main\assets') -R (Join-Path $build 'resources.zip') -o $unaligned
if ($LASTEXITCODE -ne 0) { throw 'No se pudo crear el paquete Android.' }
Push-Location (Join-Path $build 'dex')
try { & jar uf $unaligned 'classes.dex'; if ($LASTEXITCODE -ne 0) { throw 'No se pudo añadir el código.' } } finally { Pop-Location }
$native = Join-Path $root 'app\src\main\jniLibs\arm64-v8a\libmusiccraft_android_filter.so'
if (-not (Test-Path $native)) { throw 'Falta el motor Rust para Android arm64.' }
$nativeDir = Join-Path $build 'dex\lib\arm64-v8a'
New-Item -ItemType Directory -Force $nativeDir | Out-Null
Copy-Item -LiteralPath $native -Destination $nativeDir -Force
$x86Native = Join-Path $root 'app\src\main\jniLibs\x86_64\libmusiccraft_android_filter.so'
if (Test-Path $x86Native) {
    $x86Dir = Join-Path $build 'dex\lib\x86_64'
    New-Item -ItemType Directory -Force $x86Dir | Out-Null
    Copy-Item -LiteralPath $x86Native -Destination $x86Dir -Force
}
Push-Location (Join-Path $build 'dex')
try { & jar uf $unaligned 'lib'; if ($LASTEXITCODE -ne 0) { throw 'No se pudo incluir el motor Rust.' } } finally { Pop-Location }
$aligned = Join-Path $build 'aligned.apk'
& (Join-Path $tools 'zipalign.exe') -f 4 $unaligned $aligned
if ($LASTEXITCODE -ne 0) { throw 'No se pudo alinear el APK.' }
$key = Join-Path $build 'debug.keystore'
if (-not (Test-Path $key)) {
    & keytool -genkeypair -keystore $key -storepass android -alias androiddebugkey -keypass android -dname 'CN=Android Debug,O=MusicCraft Local,C=ES' -keyalg RSA -keysize 2048 -validity 10000
    if ($LASTEXITCODE -ne 0) { throw 'No se pudo crear la clave local de prueba.' }
}
$apk = Join-Path $root 'MusicCraft-Android-prueba.apk'
& (Join-Path $tools 'apksigner.bat') sign --ks $key --ks-key-alias androiddebugkey --ks-pass pass:android --key-pass pass:android --out $apk $aligned
if ($LASTEXITCODE -ne 0) { throw 'No se pudo firmar el APK de prueba.' }
& (Join-Path $tools 'apksigner.bat') verify --verbose $apk
if ($LASTEXITCODE -ne 0) { throw 'La firma del APK no es válida.' }
Write-Host "APK de prueba: $apk"
