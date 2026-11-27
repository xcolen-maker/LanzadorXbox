//! Los repositorios: estado, búsqueda de actualizaciones, instalación y ejecución.

use crate::config::{Config, Kind, Repo, Settings, log_dir};
use crate::i18n::tr;
use crate::sys;
use chrono::{Local, TimeZone};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::AtomicBool;

#[derive(Clone, Default, Debug)]
pub struct Status {
    /// Descargado en la carpeta.
    pub cloned: bool,
    /// Ya existe el ejecutable.
    pub built: bool,
    pub head_full: String,
    /// Fecha (segundos) del commit en el que está.
    pub head_ts: Option<i64>,
    /// Cuántos commits nuevos hay en el remoto.
    pub behind: u32,
    pub new_commits: Vec<String>,
    /// Se descargaron cambios que todavía no se compilaron.
    pub needs_build: bool,
}

type Log<'a> = &'a dyn Fn(String);

fn git(path: &Path, args: &[&str]) -> Option<String> {
    sys::capture("git", args, Some(path))
}

pub fn is_cloned(repo: &Repo) -> bool {
    Path::new(&repo.path).join(".git").exists()
}

/// El estado, leyendo solo lo que ya hay en la PC (sin usar la red).
pub fn status(cfg: &Config, repo: &Repo) -> Status {
    let path = Path::new(&repo.path);
    let mut st = Status { cloned: is_cloned(repo), built: path.join(&repo.exe).exists(), ..Default::default() };
    if !st.cloned {
        return st;
    }
    st.head_full = git(path, &["rev-parse", "HEAD"]).unwrap_or_default();
    st.head_ts = git(path, &["log", "-1", "--format=%ct"]).and_then(|s| s.parse().ok());
    let upstream = format!("{}/{}", repo.remote, repo.branch);
    let range = format!("HEAD..{upstream}");
    if let Some(n) = git(path, &["rev-list", "--count", &range]) {
        st.behind = n.parse().unwrap_or(0);
    }
    if st.behind > 0
        && let Some(l) = git(path, &["log", "--format=%s", "-n", "8", &range])
    {
        st.new_commits = l.lines().map(str::to_owned).collect();
    }
    st.needs_build = cfg.built.get(&repo.id).is_some_and(|b| *b != st.head_full);
    st
}

/// tr("1 cambio nuevo") / "3 cambios nuevos".
pub fn cambios(n: u32) -> String {
    if n == 1 { tr("1 cambio nuevo").into() } else { trf!("{} cambios nuevos", n) }
}

/// "hoy, 02:58", "ayer, 21:57" o la fecha.
pub fn when(ts: i64) -> String {
    let Some(dt) = Local.timestamp_opt(ts, 0).single() else {
        return tr("fecha desconocida").into();
    };
    let today = Local::now().date_naive();
    let day = dt.date_naive();
    if day == today {
        trf!("hoy, {}", dt.format("%H:%M"))
    } else if day == today.pred_opt().unwrap_or(today) {
        trf!("ayer, {}", dt.format("%H:%M"))
    } else {
        dt.format("%d/%m/%Y, %H:%M").to_string()
    }
}

fn run(mut cmd: Command, cancel: &AtomicBool, log: Log) -> Result<(), String> {
    let code = sys::run_stream(cmd_clone(&mut cmd), cancel, &mut |l| log(l)).map_err(|e| trf!("No se pudo ejecutar: {}", e))?;
    if code == 0 { Ok(()) } else { Err(trf!("terminó con error (código {})", code)) }
}

fn cmd_clone(c: &mut Command) -> Command {
    // `Command` no se puede clonar: se reconstruye con lo mismo.
    let mut n = Command::new(c.get_program());
    n.args(c.get_args());
    if let Some(d) = c.get_current_dir() {
        n.current_dir(d);
    }
    for (k, v) in c.get_envs() {
        match v {
            Some(v) => n.env(k, v),
            None => n.env_remove(k),
        };
    }
    n
}

fn git_cmd(repo: &Repo, args: &[&str]) -> Command {
    let mut c = Command::new("git");
    c.args(args).current_dir(&repo.path);
    c
}

/// `git ls-remote` sin que git pida usuario y contraseña (ni en la consola ni en una
/// ventana): un repositorio que no existe o es privado falla enseguida.
fn ls_remote(url: &str, args: &[&str]) -> Result<String, String> {
    let mut c = Command::new("git");
    c.args(["-c", "credential.helper=", "ls-remote"]).args(args).arg(url);
    c.env("GIT_TERMINAL_PROMPT", "0").env("GCM_INTERACTIVE", "never");
    let out = sys::quiet(&mut c).output().map_err(|e| trf!("No se pudo ejecutar git: {}", e))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(tr("No encontré ese repositorio. Revisá la dirección (tiene que ser público).").into())
    }
}

/// La rama principal de un repositorio remoto (la que se descarga por defecto).
pub fn default_branch(url: &str) -> Result<String, String> {
    let out = ls_remote(url, &["--symref"])?;
    out.lines()
        .find_map(|l| l.strip_prefix("ref: refs/heads/").and_then(|r| r.split_whitespace().next()).map(str::to_string))
        .ok_or_else(|| tr("El repositorio está vacío o no tiene rama principal.").into())
}

/// Que la rama exista en el repositorio remoto.
pub fn check_branch(url: &str, branch: &str) -> Result<(), String> {
    let out = ls_remote(url, &["--heads"])?;
    let want = format!("refs/heads/{branch}");
    if out.lines().any(|l| l.split_whitespace().nth(1) == Some(want.as_str())) {
        Ok(())
    } else {
        Err(trf!("El repositorio no tiene la rama {}.", branch))
    }
}

/// Que exista el remoto con el nombre que se espera.
fn ensure_remote(repo: &Repo) {
    let path = Path::new(&repo.path);
    if git(path, &["remote", "get-url", &repo.remote]).is_none() {
        let _ = git(path, &["remote", "add", &repo.remote, &repo.url]);
    }
}

/// Busca actualizaciones (descarga lo nuevo del remoto, sin tocar nada tuyo).
pub fn fetch(repo: &Repo, cancel: &AtomicBool, log: Log) -> Result<(), String> {
    ensure_remote(repo);
    run(git_cmd(repo, &["fetch", &repo.remote]), cancel, log)
}

fn build(repo: &Repo, cancel: &AtomicBool, log: Log) -> Result<(), String> {
    if std::env::var_os("LANZADOR_SIN_COMPILAR").is_some() {
        log("(prueba: se omite la compilación)".into());
        return Ok(());
    }
    let path = Path::new(&repo.path);
    let mut c = Command::new(sys::cargo_path());
    c.args(&repo.build_args).current_dir(path);
    if !repo.target_dir.is_empty() {
        c.env("CARGO_TARGET_DIR", path.join(&repo.target_dir));
    }
    log(tr("Compilando… la primera vez tarda varios minutos.").into());
    run(c, cancel, log).map_err(|e| trf!("La compilación {}", e))?;
    if path.join(&repo.exe).exists() {
        Ok(())
    } else if repo.custom {
        Err(trf!(
            "La compilación terminó pero no dejó {}. ¿Es un fork de {}? Si el ejecutable quedó en otro lado, cambiá \"exe\" en la configuración.",
            repo.exe,
            if repo.kind == Kind::Xcl { "xcl" } else { "VIctory" }
        ))
    } else {
        Err(tr("La compilación terminó pero no dejó el ejecutable.").into())
    }
}

/// Instala o actualiza: descarga (o une los cambios nuevos) y compila.
/// Devuelve el commit con el que quedó compilado.
pub fn install(repo: &Repo, cancel: &AtomicBool, log: Log) -> Result<String, String> {
    let path = Path::new(&repo.path);
    if !is_cloned(repo) {
        log(trf!("Descargando {} desde {} …", repo.name, repo.url));
        let parent = path.parent().ok_or(tr("carpeta inválida"))?;
        std::fs::create_dir_all(parent).map_err(|e| trf!("No se pudo crear la carpeta: {}", e))?;
        let mut c = Command::new("git");
        c.args(["clone", "--origin", &repo.remote, &repo.url]).arg(path).current_dir(parent);
        run(c, cancel, log).map_err(|e| trf!("La descarga {}", e))?;
    } else if repo.owned() {
        // Una copia del lanzador: nadie trabaja en ella, así que se la deja
        // exactamente como el remoto. Sin unir nada, no hay con qué chocar
        // (ni con cambios a medias de una actualización cortada).
        log(tr("Buscando cambios nuevos…").into());
        fetch(repo, cancel, log)?;
        let upstream = format!("{}/{}", repo.remote, repo.branch);
        log(trf!("Pasando a la última versión de {} …", upstream));
        // `--force -B`: la rama queda en el remoto y se descarta lo que hubiera.
        run(git_cmd(repo, &["checkout", "--force", "-B", &repo.branch, &upstream]), cancel, log)
            .map_err(|e| trf!("No se pudo pasar a la versión nueva: {}", e))?;
    } else {
        // Una carpeta de trabajo del usuario: se unen los cambios, sin pisar los suyos.
        log(tr("Buscando cambios nuevos…").into());
        fetch(repo, cancel, log)?;
        let upstream = format!("{}/{}", repo.remote, repo.branch);
        log(trf!("Uniendo los cambios de {} …", upstream));
        if run(git_cmd(repo, &["merge", "--no-edit", &upstream]), cancel, log).is_err() {
            let files = git(path, &["diff", "--name-only", "--diff-filter=U"]).unwrap_or_default();
            let _ = git(path, &["merge", "--abort"]);
            let list: Vec<&str> = files.lines().take(8).collect();
            return Err(trf!(
                "Los cambios nuevos chocan con los tuyos en {} archivo(s): {}. No se tocó nada. Esta carpeta ({}) tiene cambios propios; para que el lanzador la actualice solo, usá una copia en su carpeta (repos).",
                files.lines().count(),
                list.join(", "),
                repo.path
            ));
        }
    }
    build(repo, cancel, log)?;
    Ok(git(path, &["rev-parse", "HEAD"]).unwrap_or_default())
}

/// El ejecutable del juego dentro de la carpeta.
pub fn find_game_exe(dir: &Path) -> Option<PathBuf> {
    let exact = dir.join("game_scarlett_master.exe");
    if exact.exists() {
        return Some(exact);
    }
    std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.to_lowercase().ends_with("_master.exe")))
}

/// Las variables de entorno con las que se lanza VIctory.
pub fn victory_env(s: &Settings) -> Vec<(String, String)> {
    let mut env: Vec<(&str, &str)> = vec![("VICTORY_LOG", "info"), ("VICTORY_D3D12_DRED", "1"), ("VICTORY_SKIP_WAVE64", "mesh")];
    if s.safe_mode {
        // Lo más lento y austero que se probó: espera a la tarjeta en cada paso,
        // no sube nada por adelantado, reduce las texturas al máximo y libera RAM.
        env.extend([
            ("VICTORY_TRIM_MS", "250"),
            ("VICTORY_TRIM_BELOW_MB", "1536"),
            ("VICTORY_LATE_WAIT", "0"),
            ("VICTORY_RELAY", "0"),
            ("VICTORY_WIDE_BACK", "0"),
            ("VICTORY_EARLY_TEXTURES_MS", "0"),
            ("VICTORY_BAKE_TEXTURES_MS", "0"),
            ("VICTORY_TEXTURE_ROOM_PERCENT", "70"),
            ("VICTORY_CARD_BUFFERS_MIN_KB", "64"),
            ("VICTORY_CARD_BUFFERS_MB", "1800"),
            ("VICTORY_TEXTURE_SKIP_MIPS", "4"),
        ]);
    } else {
        env.push(("VICTORY_TEXTURE_SKIP_MIPS", "3"));
    }
    let mut out: Vec<(String, String)> = env.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    if s.auto_enter {
        let keys: Vec<String> = (20..=420).step_by(6).map(|t| format!("{t}:Enter")).collect();
        out.push(("VICTORY_KEYS".into(), keys.join(",")));
    }
    out
}

/// Las variables con las que se lanza este repositorio: las suyas propias, o
/// las comunes de VIctory más las suyas. "Pasar las pantallas iniciales" vale
/// para todos.
pub fn launch_env(repo: &Repo, s: &Settings) -> Vec<(String, String)> {
    let out = if repo.own_env {
        let mut v = repo.env.clone();
        if s.auto_enter {
            let keys: Vec<String> = (20..=420).step_by(6).map(|t| format!("{t}:Enter")).collect();
            v.push(("VICTORY_KEYS".into(), keys.join(",")));
        }
        v
    } else {
        let mut v = victory_env(s);
        v.extend(repo.env.iter().cloned());
        v
    };
    // La caché propia tiene que existir.
    if let Some((_, dir)) = out.iter().find(|(k, _)| k == "VICTORY_CACHE_DIR") {
        let _ = std::fs::create_dir_all(dir);
    }
    // (Si una variable se repite, al lanzar gana la última.)
    out
}

pub fn log_file(repo: &Repo) -> PathBuf {
    log_dir().join(format!("{}.log", repo.id))
}

/// Lanza el juego con este emulador. La salida va a su archivo de registro.
pub fn launch(cfg: &Config, repo: &Repo) -> Result<Child, String> {
    let path = Path::new(&repo.path);
    let exe = path.join(&repo.exe);
    if !exe.exists() {
        return Err(trf!("{} todavía no está compilado: usá Instalar.", repo.name));
    }
    let game_dir = Path::new(&cfg.game_dir);
    let mut c = Command::new(&exe);
    match repo.kind {
        Kind::Victory => {
            let game = find_game_exe(game_dir).ok_or(tr("No encuentro el juego en la carpeta elegida (falta game_scarlett_master.exe)."))?;
            let console = repo.console.as_deref().unwrap_or(&cfg.settings.console);
            c.arg("run").arg("--console").arg(console).arg(game).arg("--execute");
            // Ningún ajuste que quede en el entorno del lanzador se cuela.
            for (k, _) in std::env::vars() {
                if k.starts_with("VICTORY_") {
                    c.env_remove(k);
                }
            }
            for (k, v) in launch_env(repo, &cfg.settings) {
                c.env(k, v);
            }
        }
        Kind::Xcl => {
            if !game_dir.join("MicrosoftGame.Config").exists() && !game_dir.join("MicrosoftGame.config").exists() {
                return Err(tr("No encuentro el juego en la carpeta elegida.").into());
            }
            c.arg("run").arg(game_dir);
        }
    }
    c.current_dir(path);
    let log = log_file(repo);
    let _ = std::fs::create_dir_all(log_dir());
    let f = File::create(&log).map_err(|e| trf!("No se pudo crear el registro: {}", e))?;
    c.stdout(f.try_clone().map_err(|e| e.to_string())?).stderr(f).stdin(Stdio::null());
    sys::quiet(&mut c);
    c.spawn().map_err(|e| trf!("No se pudo iniciar {}: {}", repo.name, e))
}

/// El registro de errores más reciente de VIctory.
pub fn newest_victory_crash_log() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from)?.join("victory").join("data");
    std::fs::read_dir(base)
        .ok()?
        .flatten()
        .map(|e| e.path().join("crashes.log"))
        .filter(|p| p.exists())
        .max_by_key(|p| std::fs::metadata(p).and_then(|m| m.modified()).ok())
}
