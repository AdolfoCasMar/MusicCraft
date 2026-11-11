# MusicCraft

<p align="center"><img src="assets/musiccraft-icon-propuesta.png" width="170" alt="MusicCraft: zorro con auriculares"></p>

<p align="center"><strong>Tu música, con el aspecto clásico de los reproductores de siempre.</strong></p>

MusicCraft es un reproductor gratuito y experimental inspirado en Winamp. Reúne música local, búsqueda de canciones y listas online, ecualizador y cola de reproducción en una interfaz clásica. El proyecto no incorpora publicidad propia.

> MusicCraft es independiente; no está afiliado a Winamp, ArtCraft, Brave ni YouTube. La reproducción online utiliza YouTube. El filtrado está basado en componentes y reglas de Brave, pero no integra el navegador Brave completo ni garantiza bloquear todos los anuncios.

## Descargas

| Plataforma | Estado | Descarga |
| --- | --- | --- |
| Android | Versión experimental firmada; Android 8 o superior, ARM64 o x86-64. | [Descargar APK 0.13](https://github.com/AdolfoCasMar/MusicCraft/releases/download/android-v0.13/MusicCraft-Android-0.13.apk) |
| Windows | Versión experimental para Windows de 64 bits; necesita Microsoft Edge WebView2 Runtime. | [Descargar versión 0.8.2](https://github.com/AdolfoCasMar/MusicCraft/releases/download/windows-v0.8.2/MusicCraft-Windows-0.8.2.exe) |
| iPhone | Prototipo inicial, sin búsqueda online ni filtrado; requiere firma para instalarlo. | [Consultar el proyecto iOS](mobile/ios/README.md) |
| Linux y macOS | Adaptación pendiente; todavía no existen versiones funcionales. | — |

[Ver todas las versiones](https://github.com/AdolfoCasMar/MusicCraft/releases).

**Actualización de Android:** el APK 0.13 tiene una firma distinta de las versiones de prueba anteriores. Puede ser necesario desinstalar la anterior, lo que puede borrar sus ajustes y playlists. Android o el fabricante pueden mostrar avisos al instalar aplicaciones de fuera de una tienda.

## Funciones

- Interfaz clásica con reproductor, ecualizador, cola de reproducción y biblioteca.
- Música local, búsqueda online y listas guardadas; la disponibilidad de contenido depende del servicio de origen.
- En Windows, módulos que pueden moverse, acoplarse y plegarse. En Android, módulos plegables y reordenables.
- En Android, controles multimedia y reproducción en segundo plano; el comportamiento puede variar según el teléfono.
- Visualizador de frecuencias en Android para archivos locales y fuentes online compatibles.
- Pausa durante llamadas telefónicas en Android: se ha probado con una llamada simulada y música local; falta probarla en teléfonos reales y con fuentes online.

## Imágenes

Estas capturas son de una versión anterior de escritorio y pueden mostrar el nombre anterior y controles diferentes.

| Reproductor | Ecualizador |
| --- | --- |
| ![Reproductor clásico](docs/images/reproductor.png) | ![Ecualizador](docs/images/ecualizador.png) |

| Lista de reproducción | Biblioteca |
| --- | --- |
| ![Cola y búsqueda](docs/images/playlist.png) | ![Biblioteca local](docs/images/biblioteca.png) |

## Privacidad y permisos en Android

Para abrir música local, MusicCraft usa el selector de archivos de Android y accede a los archivos elegidos, sin pedir acceso general al almacenamiento. La pausa en llamadas solicita permiso para conocer el estado del teléfono; no solicita el historial ni los números. También utiliza acceso a Internet y un servicio de reproducción en segundo plano. Algunas funciones y controles pueden variar entre dispositivos.

## Desarrollo

El escritorio actual usa Rust, Tao y WebView2 en Windows. La versión Android utiliza Java, JavaScript/Web Audio y el motor de filtrado en Rust. Consulta [Android](mobile/android/README.md) y [iOS](mobile/ios/README.md) para instrucciones y limitaciones. Para compilar Windows se necesitan Rust, herramientas C++ de Visual Studio y Microsoft Edge WebView2 Runtime:

```powershell
cargo build --release
cargo test
```

La clave privada de firma de Android no se incluye en el repositorio. Los vídeos online pueden solicitar cookies o interacción; las playlists largas pueden importarse parcialmente y el ecualizador depende de la compatibilidad de la fuente.

## Licencias

El código propio se distribuye bajo **MIT o Apache 2.0**, a tu elección: [LICENSE-MIT](LICENSE-MIT) y [LICENSE-APACHE](LICENSE-APACHE). Los componentes y listas de terceros tienen sus propias condiciones; consulta [LICENCIA-ADBLOCK.txt](LICENCIA-ADBLOCK.txt) y los encabezados de las listas incluidas.

