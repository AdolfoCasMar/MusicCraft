# Compatibilidad con Linux y macOS

El prototipo actual está preparado para Windows. El código de ventanas, el motor integrado y el filtrado de solicitudes dependen en varios puntos de interfaces específicas de Windows y WebView2. Cambiar solo el sistema de compilación no produce una versión funcional en Linux o macOS.

Para cada plataforma quedan pendientes:

1. Adaptar las ventanas, el acoplamiento, la escala y el comportamiento en la barra de tareas a las interfaces de esa plataforma.
2. Integrar el navegador nativo y comprobar la reproducción, los avisos de cookies y el ecualizador.
3. Investigar e implementar el filtrado de solicitudes y recursos con las capacidades disponibles; no se debe presentar como equivalente a los escudos actuales sin pruebas.
4. Empaquetar y probar la aplicación en equipos reales, incluyendo dependencias, audio y persistencia. En macOS será necesario decidir cómo distribuir y firmar la aplicación.

No hay todavía ejecutables de MusicCraft para estos sistemas.
