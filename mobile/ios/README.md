# MusicCraft para iPhone — base experimental

Esta carpeta contiene una primera base local para iOS, sin publicar. Reutiliza la interfaz de Android y prepara reproducción de archivos locales, ecualizador Web Audio, sesión de audio y controles de pantalla bloqueada. **No se ha compilado ni probado**: este equipo usa Windows y no dispone de Xcode ni del SDK de iOS. No hay archivo instalable IPA.

La búsqueda y reproducción online todavía no están implementadas en iOS. Tampoco se ha integrado el filtrado Rust de Android ni el navegador Brave: la versión Android usa componentes propios de esa plataforma. La reproducción en segundo plano, el ecualizador, el selector de archivos y los controles deben validarse en un iPhone real; configurar la sesión no garantiza que WKWebView siga reproduciendo bajo todas las condiciones de iOS.

## Abrir en un Mac

1. Instalar Xcode y XcodeGen en el Mac.
2. En esta carpeta, ejecutar `xcodegen generate` y abrir `MusicCraft.xcodeproj`.
3. En Signing & Capabilities seleccionar un equipo de desarrollo y, si es necesario, un identificador de aplicación único.
4. Conectar un iPhone, seleccionar el dispositivo y ejecutar desde Xcode.

El proyecto necesita trabajo adicional antes de considerarse equivalente a la edición de Android. La distribución, si se desea en el futuro, requiere revisar sus condiciones y la firma correspondientes.
