# Lanzador Xbox

Un lanzador para Windows que instala, actualiza y ejecuta emuladores de Xbox Series
en PC: [xcl](https://github.com/xcl-dev/xcl), [VIctory](https://github.com/Marvisak/VIctory),
[VIctory Fork](https://github.com/fxceless/victory-fork) y
[VIctory NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA).

No incluye ningún juego. Necesitás tu propia copia.

## Cómo se usa

1. Abrí `Lanzador-Xbox.exe`. La primera vez aparece la **Configuración inicial**:
   - **Carpeta del juego**: si el lanzador la encontró ya está puesta; si no, pulsá el
     campo y elegila. **CONTINUAR** se activa cuando la carpeta tiene el juego.
   - **Emuladores**: si ya tenés alguno descargado, pulsá su campo y elegí su carpeta
     (o **Usar esa** si el lanzador encontró una copia). Si no, dejalo como está: el
     lanzador lo descarga y lo compila solo.
2. En **Instalar**, instalá lo que falte para compilar (Rust, Visual Studio Build Tools,
   Windows SDK). El lanzador lo revisa solo.
3. Elegí un emulador a la izquierda y pulsá **INSTALAR**: lo descarga de GitHub y lo compila.
4. Pulsá **JUGAR**.

Para volver a la configuración inicial: **Ajustes → Configuración inicial**.

## Qué encuentra solo

- **La carpeta del juego**: busca el ejecutable (`*_master.exe`) en tus carpetas
  personales (Escritorio, Documentos, Descargas…) y en `Games`, `Juegos` y `XboxGames`
  de cada disco. Si no lo encuentra, elegila en **Ajustes → Cambiar carpeta**.
- **Emuladores que ya tengas clonados**: mira las carpetas de primer nivel de esos mismos
  lugares (y `source\repos`, `code`, `dev`, `projects`…) y reconoce cada copia por la
  dirección de su remoto `origin`. Si encuentra una, te ofrece **USAR ESA COPIA** en vez de
  descargarla de nuevo. Si tu copia está en otro lado, elegila en
  **Ajustes → Dónde está cada emulador**.

Si no usás una copia tuya, el lanzador guarda las suyas en
`%LOCALAPPDATA%\XboxLanzador\repos`. Al actualizar una copia tuya, une los cambios
nuevos con los tuyos; si chocan, te avisa y no toca nada.

Para probar sin abrir la ventana: `Lanzador-Xbox.exe --detect` muestra lo que encuentra
(también `--check`, `--prereq`, `--env` e `--install <xcl|victory|fork|nvidia>`).

## Imágenes

El lanzador no trae imágenes: sin ellas dibuja un atardecer. Para ponerle las tuyas,
abrí **Ajustes → Imágenes** (es `%LOCALAPPDATA%\XboxLanzador\imagenes`) y copiá:

- `logo.png`: el logo que va arriba del botón.
- Los fondos, en `.jpg` o `.png`, con cualquier nombre. Cada emulador usa uno, en orden.

También vale una carpeta `imagenes` junto al `.exe`. Se ven al volver a abrir el lanzador.

## Ajustes de VIctory NVIDIA

Están adaptados a una placa de 6 GB con poca RAM (menos memoria para copias y texturas
que el `play-nvidia.ps1` original). Con una placa más grande podés subirlos editando
`%LOCALAPPDATA%\XboxLanzador\config.json` (**Ajustes → Configuración**).

## Compilar

```
cargo build --release
```

El ejecutable queda en `target\release\xbox-lanzador.exe`. El icono (`icon\app.ico`) se
mete en el `.exe` al compilar; hace falta el SDK de Windows (sin él compila igual, pero
sin icono).

## Licencia

[MIT](LICENSE). Los emuladores tienen sus propias licencias, en sus repositorios.
