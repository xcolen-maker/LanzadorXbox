//! La configuración: qué repositorios se manejan, dónde están, y los ajustes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// VIctory o una versión suya: `victory run --console ... <juego.exe> --execute`.
    Victory,
    /// xcl: `xcl run <carpeta del juego>`.
    Xcl,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Repo {
    pub id: String,
    pub name: String,
    pub note: String,
    pub kind: Kind,
    /// De dónde se descarga.
    pub url: String,
    /// Dónde está en esta PC.
    pub path: String,
    /// Nombre del remoto de git y rama de la que se toman las actualizaciones.
    pub remote: String,
    pub branch: String,
    /// El ejecutable que deja la compilación, relativo a `path`.
    pub exe: String,
    /// Carpeta de compilación (CARGO_TARGET_DIR), relativa a `path`; vacía = la de siempre.
    #[serde(default)]
    pub target_dir: String,
    /// Argumentos de `cargo` para compilar.
    pub build_args: Vec<String>,
    /// Variables de entorno propias de este repositorio, al lanzarlo.
    #[serde(default)]
    pub env: Vec<(String, String)>,
    /// Usa solo `env` (no los ajustes comunes de VIctory: modo seguro, etc.).
    #[serde(default)]
    pub own_env: bool,
    /// La consola que ve el juego con este repositorio, por encima del ajuste común.
    #[serde(default)]
    pub console: Option<String>,
    /// Lo agregó la persona desde el lanzador (se puede quitar).
    #[serde(default)]
    pub custom: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Settings {
    /// "series-s" o "series-x".
    pub console: String,
    /// Modo seguro: más lento, pero gasta menos memoria.
    pub safe_mode: bool,
    /// Pulsar Enter solo cada pocos segundos para pasar las pantallas iniciales.
    pub auto_enter: bool,
    /// Idioma de la interfaz ("es", "en", "pt", "fr", "ru"); vacío = el de Windows.
    #[serde(default)]
    pub lang: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub game_dir: String,
    pub repos: Vec<Repo>,
    pub settings: Settings,
    /// El commit con el que se compiló cada repositorio.
    #[serde(default)]
    pub built: HashMap<String, String>,
    /// Ya pasó por la pantalla de configuración inicial (carpeta del juego y emuladores).
    #[serde(default)]
    pub setup_done: bool,
}

pub fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    base.join("XboxLanzador")
}

pub fn log_dir() -> PathBuf {
    data_dir().join("logs")
}

/// Donde el lanzador guarda sus propias copias de los repositorios.
pub fn repos_dir() -> PathBuf {
    data_dir().join("repos")
}

/// trustno1-coder/VIctory-NVIDIA: hecho para placas NVIDIA, con el perfil
/// Series S a media resolución. Sus ajustes son los de su
/// `scripts\play-nvidia.ps1` (medidos en una RTX 5070 de 12 GB), adaptados a
/// una placa de 6 GB y poca RAM: menos memoria de la placa para copias, menos
/// lugar para texturas, sin Smooth Motion (no hay en la serie 30) y sin el
/// vigilante del juego (que corta los fotogramas lentos).
pub fn nvidia_repo() -> Repo {
    let cache = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("victory")
        .join("cache-nvidia");
    let mut env: Vec<(String, String)> = [
        // Lo mismo que play-nvidia.ps1.
        ("VICTORY_TEXTURE_NO_WAIT", "1"),
        ("VICTORY_RELAY", "2"),
        ("VICTORY_WIDE_BACK", "1"),
        ("VICTORY_CARD_REHASH_RUNS", "1"),
        ("VICTORY_JOIN_COPIES", "1"),
        ("VICTORY_INDIRECT_WINDOW", "1"),
        ("VICTORY_RELAY_COPY", "1"),
        ("VICTORY_CARD_STEPS", "2"),
        ("VICTORY_INDIRECT_BATCH", "2"),
        ("VICTORY_MAP_THREADS", "1"),
        ("VICTORY_TITLE_PRIORITIES", "0"),
        ("VICTORY_CARD_AWAKE", "1"),
        ("VICTORY_SHOW_AFTER", "1"),
        ("VICTORY_AMBIENT", "0.8"),
        ("VICTORY_D3D12_DRED", "light"),
        ("VICTORY_SKIP_PIPELINES", "none"),
        ("VICTORY_THIN", "1"),
        ("VICTORY_JOIN_PARTS", "8"),
        ("VICTORY_UPLOAD_AHEAD", "1"),
        ("VICTORY_PIPELINE_WAIT_MS", "10000"),
        // El arreglo de NVIDIA: sin esto se descartan miles de dibujos por fotograma.
        ("VICTORY_DYNAMIC_SLOTS", "524288"),
        ("VICTORY_RENDER_SCALE", "0.5"),
        ("VICTORY_OWN_UPSCALER", "1"),
        ("VICTORY_BVH_MISS", "1"),
        ("VICTORY_COMPOSE", "1"),
        ("VICTORY_CARD_BUFFERS_MIN_MB", "1"),
        // Adaptado a 6 GB de placa (el original: 1536).
        ("VICTORY_CARD_BUFFERS_MB", "768"),
        // Adaptado: más margen en la placa (por defecto 90).
        ("VICTORY_TEXTURE_ROOM_PERCENT", "80"),
        // Agregado: que el juego no corte un fotograma lento.
        ("VICTORY_NO_WATCHDOG", "1"),
        ("VICTORY_LOG", "info"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect();
    // Su propia caché: shaders y pipelines de otra versión del traductor no se mezclan.
    env.push(("VICTORY_CACHE_DIR".into(), cache.to_string_lossy().into_owned()));
    Repo {
        id: "nvidia".into(),
        name: "VIctory NVIDIA".into(),
        note: "para NVIDIA (trustno1-coder)".into(),
        kind: Kind::Victory,
        url: "https://github.com/trustno1-coder/VIctory-NVIDIA".into(),
        path: repos_dir().join("victory-nvidia").to_string_lossy().into_owned(),
        remote: "origin".into(),
        branch: "main".into(),
        exe: "target/diag/victory.exe".into(),
        target_dir: String::new(),
        build_args: ["build", "--locked", "--profile", "diag", "-p", "victory", "--features", "diagnostics"]
            .map(String::from)
            .to_vec(),
        env,
        own_env: true,
        console: Some("series-s".into()),
        custom: false,
    }
}

impl Repo {
    /// La copia es del lanzador (está en su carpeta): nadie trabaja en ella,
    /// así que al actualizar se la deja igual al remoto, sin unir nada.
    pub fn owned(&self) -> bool {
        // (Windows no distingue mayúsculas en las rutas.)
        let repos = repos_dir().to_string_lossy().to_lowercase();
        self.path.to_lowercase().starts_with(&repos)
    }
}

impl Config {
    pub fn file() -> PathBuf {
        data_dir().join("config.json")
    }

    pub fn load() -> Config {
        if let Ok(text) = std::fs::read_to_string(Self::file())
            && let Ok(mut cfg) = serde_json::from_str::<Config>(&text)
        {
            if cfg.migrate() {
                cfg.save();
            }
            return cfg;
        }
        let cfg = Self::defaults();
        cfg.save();
        cfg
    }

    pub fn save(&self) {
        let p = Self::file();
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, text);
        }
    }

    pub fn repo(&self, id: &str) -> Option<&Repo> {
        self.repos.iter().find(|r| r.id == id)
    }

    /// Arregla configuraciones viejas. El fork apuntaba a la carpeta de trabajo
    /// del usuario (`%USERPROFILE%\VIctory`), con sus propios cambios: cada
    /// actualización intentaba unir el remoto encima y fallaba en cuanto los dos
    /// tocaban el mismo código. Ahora el fork tiene su propia copia. Devuelve si
    /// cambió algo.
    fn migrate(&mut self) -> bool {
        let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
        let dev = home.join("VIctory").to_string_lossy().to_lowercase();
        let mut changed = false;
        for r in &mut self.repos {
            if r.id == "fork" && r.path.trim_end_matches(['\\', '/']).to_lowercase() == dev {
                r.path = repos_dir().join("victory-fork").to_string_lossy().into_owned();
                self.built.remove("fork");
                changed = true;
            }
        }
        // VIctory NVIDIA se agregó después: las configuraciones guardadas no lo tienen.
        if self.repo("nvidia").is_none() {
            self.repos.push(nvidia_repo());
            changed = true;
        }
        changed
    }

    /// La carpeta propia del lanzador para un emulador: la de fábrica, o `repos\<id>` si lo agregó la persona.
    pub fn default_path(&self, id: &str) -> Option<String> {
        match Self::defaults_repo(id) {
            Some(r) => Some(r.path),
            None => self.repo(id).filter(|r| r.custom).map(|_| repos_dir().join(id).to_string_lossy().into_owned()),
        }
    }

    /// Cómo viene configurado un emulador de fábrica (su carpeta propia del lanzador, etc.).
    pub fn defaults_repo(id: &str) -> Option<Repo> {
        Self::defaults().repos.into_iter().find(|r| r.id == id)
    }

    pub fn defaults() -> Config {
        let repos_dir = repos_dir();
        // Una copia propia: la carpeta de trabajo del usuario tiene cambios suyos.
        let fork_path = repos_dir.join("victory-fork");
        let victory_build: Vec<String> =
            ["build", "--profile", "diag", "-p", "victory", "--features", "diagnostics"].map(String::from).to_vec();
        let s = |p: PathBuf| p.to_string_lossy().into_owned();
        Config {
            // Vacía: el lanzador la busca solo al abrir (ver `detect::find_game_dir`).
            game_dir: String::new(),
            repos: vec![
                Repo {
                    id: "xcl".into(),
                    name: "xcl".into(),
                    note: "xcl-dev/xcl".into(),
                    kind: Kind::Xcl,
                    url: "https://github.com/xcl-dev/xcl".into(),
                    path: s(repos_dir.join("xcl")),
                    remote: "origin".into(),
                    branch: "main".into(),
                    exe: "target/release/xcl.exe".into(),
                    target_dir: String::new(),
                    build_args: ["build", "--release", "--workspace"].map(String::from).to_vec(),
                    env: Vec::new(),
                    own_env: false,
                    console: None,
                    custom: false,
                },
                Repo {
                    id: "victory".into(),
                    name: "VIctory".into(),
                    note: "original (Marvisak)".into(),
                    kind: Kind::Victory,
                    url: "https://github.com/Marvisak/VIctory".into(),
                    path: s(repos_dir.join("VIctory")),
                    remote: "origin".into(),
                    branch: "main".into(),
                    exe: "target/diag/victory.exe".into(),
                    target_dir: String::new(),
                    build_args: victory_build.clone(),
                    env: Vec::new(),
                    own_env: false,
                    console: None,
                    custom: false,
                },
                Repo {
                    id: "fork".into(),
                    name: "VIctory Fork".into(),
                    note: "versión mejorada (fxceless)".into(),
                    kind: Kind::Victory,
                    url: "https://github.com/fxceless/victory-fork".into(),
                    path: s(fork_path),
                    remote: "fxceless".into(),
                    branch: "main".into(),
                    exe: "target/fork/diag/victory.exe".into(),
                    target_dir: "target/fork".into(),
                    build_args: victory_build,
                    env: Vec::new(),
                    own_env: false,
                    console: None,
                    custom: false,
                },
                nvidia_repo(),
            ],
            settings: Settings { console: "series-s".into(), safe_mode: true, auto_enter: true, lang: String::new() },
            built: HashMap::new(),
            setup_done: false,
        }
    }
}

/// Un identificador corto y sin espacios a partir del nombre: "VIctory AMD" → "victory-amd".
fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() { "emulador".into() } else { out }
}

/// Un emulador que agrega la persona: un fork de VIctory o de xcl, que se compila y se
/// ejecuta igual que el original en el que se basa.
pub fn custom_repo(cfg: &Config, name: &str, url: &str, kind: Kind, branch: &str) -> Repo {
    let base = slug(name);
    let mut id = base.clone();
    let mut n = 2;
    while cfg.repo(&id).is_some() || Config::defaults_repo(&id).is_some() {
        id = format!("{base}-{n}");
        n += 1;
    }
    let url = url.trim().trim_end_matches('/').to_string();
    // "dueño/repo" como nota, para reconocerlo.
    let parts: Vec<&str> = url.trim_end_matches(".git").rsplit('/').take(2).collect();
    let note = format!("{}/{}", parts.get(1).unwrap_or(&""), parts.first().unwrap_or(&""));
    let (exe, build_args): (&str, &[&str]) = match kind {
        Kind::Victory => ("target/diag/victory.exe", &["build", "--profile", "diag", "-p", "victory", "--features", "diagnostics"]),
        Kind::Xcl => ("target/release/xcl.exe", &["build", "--release", "--workspace"]),
    };
    Repo {
        path: repos_dir().join(&id).to_string_lossy().into_owned(),
        id,
        name: name.trim().to_string(),
        note: format!("{note} · agregado por vos"),
        kind,
        url,
        remote: "origin".into(),
        branch: branch.to_string(),
        exe: exe.into(),
        target_dir: String::new(),
        build_args: build_args.iter().map(|s| s.to_string()).collect(),
        env: Vec::new(),
        own_env: false,
        console: None,
        custom: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slug("VIctory AMD"), "victory-amd");
        assert_eq!(slug("  Mi  Emulador!! "), "mi-emulador");
        assert_eq!(slug("¿?"), "emulador");
    }

    #[test]
    fn repo_propio() {
        let cfg = Config::defaults();
        let r = custom_repo(&cfg, "VIctory", "https://github.com/JoshuuDev/victory-amd/", Kind::Victory, "main");
        // "victory" ya existe: no se pisa.
        assert_eq!(r.id, "victory-2");
        assert_eq!(r.url, "https://github.com/JoshuuDev/victory-amd");
        assert!(r.note.starts_with("JoshuuDev/victory-amd"));
        assert_eq!(r.exe, "target/diag/victory.exe");
        assert!(r.custom && r.path.ends_with("victory-2"));
    }
}
