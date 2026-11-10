# MusicCraft para Android — prototipo local

Esta prueba es independiente de la aplicación de Windows. Usa una interfaz local dentro de una actividad Android para reproducir archivos elegidos por el usuario; no sube sus archivos. Incluye cola mixta de canciones locales y de YouTube, reproducción aleatoria y repetición, ecualizador de nueve bandas con preamplificador y ajustes, y biblioteca para guardar playlists de YouTube. La cola no se conserva al cerrar.

La búsqueda utiliza YouTube normal (m.youtube.com) en una vista integrada detrás del reproductor; los resultados aparecen dentro de MusicCraft y al elegir una canción se intenta iniciar sin cambiar de pantalla. Se pueden buscar canciones o playlists, añadir pistas a la cola e importar playlists públicas. Las playlists largas pueden importarse parcialmente, dependiendo de lo que YouTube cargue. La biblioteca guarda los vídeos de YouTube; los archivos locales deben seleccionarse de nuevo al volver a abrir la aplicación.

Si YouTube solicita una elección de cookies, aparecen botones dentro de MusicCraft; en avisos no reconocidos puede ser necesario pulsar «AVISO YOUTUBE». Los controles de reproducción y el ecualizador se han probado con YouTube en un Android virtual; la ecualización depende de la compatibilidad de la fuente de audio y la aplicación mostrará un aviso si no puede conectarse.

Incluye un adaptador experimental del motor Rust `adblock-rust` y de una instantánea de reglas de Brave. La interceptación de peticiones y la inyección de filtros del WebView tienen limitaciones y no equivalen a integrar el navegador Brave completo; no se garantiza bloquear los anuncios.

La aplicación utiliza un servicio de reproducción y una sesión multimedia de Android para continuar en segundo plano y mostrar controles en las notificaciones. Se ha probado al salir de la aplicación y al apagar la pantalla en un Android virtual, tanto con música local como con reproducción online; es necesario comprobar el comportamiento y los controles en cada teléfono. Cerrar MusicCraft explícitamente o detener su proceso termina la reproducción. Algunos sistemas pueden restringir la ejecución en segundo plano.

El prototipo usa Java para la ventana y JavaScript/Web Audio para la interfaz y audio; el filtrado usa Rust. No es una versión publicada.

Requiere Android 8.0 o posterior con procesador ARM de 64 bits o x86-64. La selección de archivos utiliza el selector del sistema; no pide acceso general a los archivos.
