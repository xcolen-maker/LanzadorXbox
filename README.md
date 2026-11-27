# Lanzador Xbox

A Windows launcher that installs, updates and runs Xbox Series emulators on PC:
[xcl](https://github.com/xcl-dev/xcl), [VIctory](https://github.com/Marvisak/VIctory),
[VIctory Fork](https://github.com/fxceless/victory-fork) and
[VIctory NVIDIA](https://github.com/trustno1-coder/VIctory-NVIDIA). You can also add
other forks of VIctory or xcl yourself.

No game is included. You need your own copy.

The launcher's interface is in Spanish.

## Download

Get the latest version from [Releases](../../releases/latest):

- **`LanzadorXbox-Setup-<version>.exe`** (recommended): installer. Creates Start menu and
  (optionally) desktop shortcuts, and can be uninstalled from Windows. No administrator
  rights needed.
- **`LanzadorXbox-v<version>.zip`**: portable. Unzip it and run `Lanzador-Xbox.exe`
  (keep the `imagenes` folder next to the `.exe`).

The program is not signed, so Windows may show **"Windows protected your PC"**. Click
**More info → Run anyway**.

## How to use it

1. Open `Lanzador-Xbox.exe`. The first time, the **initial setup** screen appears
   (*Configuración inicial*):
   - **Game folder**: if the launcher found it, it is already filled in; otherwise click
     the field and pick it. **CONTINUAR** (continue) is enabled once the folder contains
     the game.
   - **Emulators**: if you already have one downloaded, click its field and pick its
     folder (or **Usar esa** if the launcher found a copy). Otherwise leave it as is: the
     launcher downloads and builds it for you.
2. In **INSTALAR** (install), install whatever is missing to build (Rust, Visual Studio
   Build Tools, Windows SDK). The launcher checks this for you.
3. Pick an emulator on the left and click **INSTALAR**: it downloads it from GitHub and
   builds it.
4. Click **JUGAR** (play).

To go back to the initial setup: **Ajustes → Configuración inicial** (Settings → Initial
setup).

## Adding your own emulator

Click **+ Agregar emulador** (add emulator) in the left sidebar and fill in:

- **Name**: how it will appear in the launcher.
- **GitHub address**: for example `https://github.com/someone/victory-amd`.
- **Type**: whether it is based on **VIctory** or on **xcl**. The launcher builds and
  runs it the same way as the original it is based on, so it only works with forks of
  those two projects.
- **Branch** (optional): leave it empty to use the repository's default branch.

The launcher checks that the address exists before adding it. Then the emulator appears
in the sidebar like the others, and you can install, update and play it. To remove it:
**Ajustes → Dónde está cada emulador → Quitar** (this does not delete its files).

## What it finds by itself

- **The game folder**: it looks for the game executable (`*_master.exe`) in your personal
  folders (Desktop, Documents, Downloads…) and in `Games`, `Juegos` and `XboxGames` on
  every drive. If it cannot find it, pick it in **Ajustes → Cambiar carpeta**.
- **Emulators you already cloned**: it looks at the top-level folders of those same
  places (plus `source\repos`, `code`, `dev`, `projects`…) and recognizes each copy by
  the address of its `origin` remote. If it finds one, it offers **USAR ESA COPIA** (use
  that copy) instead of downloading it again. If your copy is somewhere else, pick it in
  **Ajustes → Dónde está cada emulador**.

When you don't use your own copy, the launcher keeps its own copies in
`%LOCALAPPDATA%\XboxLanzador\repos`. When updating one of your own copies, it merges the
new changes with yours; if they conflict, it tells you and leaves everything as it was.

To test without opening the window: `Lanzador-Xbox.exe --detect` shows what it finds
(also `--check`, `--prereq`, `--env` and `--install <id>`).

## Images

The `imagenes` folder (next to the `.exe`) holds the logo and the backgrounds. Without
images the launcher draws a sunset. To change them, replace those files or use
**Ajustes → Imágenes** (`%LOCALAPPDATA%\XboxLanzador\imagenes`, which takes priority):

- `logo.png`: the logo shown above the main button.
- The backgrounds, as `.jpg` or `.png`, with any name. Each emulator uses one, in order.

Changes show up the next time you open the launcher.

## VIctory NVIDIA settings

They are tuned for a 6 GB graphics card with little RAM (less memory for copies and
textures than the original `play-nvidia.ps1`). With a bigger card you can raise them by
editing `%LOCALAPPDATA%\XboxLanzador\config.json` (**Ajustes → Configuración**).

## Building

```
cargo build --release
```

The executable ends up in `target\release\xbox-lanzador.exe`. The icon (`icon\app.ico`)
is embedded in the `.exe` at build time; this needs the Windows SDK (without it the build
still works, just without the icon).

## Installer

The installer (`LanzadorXbox-Setup-<version>.exe`) is built with
[Inno Setup 6](https://jrsoftware.org/isinfo.php):

```
ISCC.exe installer\LanzadorXbox.iss
```

It ends up in `dist\`. It installs per user (no administrator rights), creates Start menu
shortcuts and, if chosen, a desktop shortcut, and can be uninstalled from Windows.

## License

[MIT](LICENSE). The emulators have their own licenses, in their repositories.
