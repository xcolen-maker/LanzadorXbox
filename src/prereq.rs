//! Lo que piden los emuladores: herramientas, componentes de Windows y memoria.

use crate::sys;
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PState {
    Ok,
    /// Está, pero no del todo bien.
    Warn,
    Missing,
}

#[derive(Clone, Debug)]
pub struct Prereq {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: String,
    pub state: PState,
}

const VSWHERE: &str = r"C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe";

fn git() -> Prereq {
    let v = sys::capture("git", &["--version"], None);
    let desc = match &v {
        Some(v) => format!("Para bajar el código de los proyectos · {}", v.trim_start_matches("git version ")),
        None => "Para bajar el código de los proyectos".into(),
    };
    Prereq { id: "git", name: "Git", desc, state: if v.is_some() { PState::Ok } else { PState::Missing } }
}

fn rust() -> Prereq {
    let v = sys::capture(&sys::cargo_path(), &["--version"], None);
    let desc = match &v {
        Some(v) => format!("El lenguaje con el que están hechos · {}", v.trim_start_matches("cargo ")),
        None => "El lenguaje con el que están hechos".into(),
    };
    Prereq { id: "rust", name: "Rust", desc, state: if v.is_some() { PState::Ok } else { PState::Missing } }
}

fn windows_sdk_has_dxil() -> bool {
    let bin = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    std::fs::read_dir(bin)
        .map(|d| d.flatten().any(|e| e.path().join("x64").join("dxil.dll").exists()))
        .unwrap_or(false)
}

fn msvc() -> Prereq {
    let cpp = Path::new(VSWHERE).exists()
        && sys::capture(
            VSWHERE,
            &["-products", "*", "-requires", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64", "-property", "installationPath"],
            None,
        )
        .is_some_and(|s| !s.is_empty());
    let sdk = windows_sdk_has_dxil();
    let (state, extra) = match (cpp, sdk) {
        (true, true) => (PState::Ok, ""),
        (true, false) => (PState::Warn, " · falta el Windows SDK"),
        (false, true) => (PState::Warn, " · falta el compilador de C++"),
        (false, false) => (PState::Missing, ""),
    };
    Prereq {
        id: "msvc",
        name: "Compilador de C++ y Windows SDK",
        desc: format!("Visual Studio Build Tools · tarda unos minutos{extra}"),
        state,
    }
}

fn gaming_services() -> Prereq {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let found = Path::new(&root).join("System32").join("xgameruntime.dll").exists();
    Prereq {
        id: "xgameruntime",
        name: "Gaming Services (XGameRuntime)",
        desc: "Lo piden xcl y los juegos de Xbox · viene de la Microsoft Store".into(),
        state: if found { PState::Ok } else { PState::Missing },
    }
}

fn memory() -> Prereq {
    let (phys, commit) = sys::memory_mb();
    let gb = |mb: u64| mb as f64 / 1024.0;
    Prereq {
        id: "memory",
        name: "Memoria virtual de Windows",
        desc: format!(
            "Tenés {:.0} GB de RAM y un límite de {:.0} GB · se recomienda 32 GB o más",
            gb(phys),
            gb(commit)
        ),
        state: if commit >= 32 * 1024 { PState::Ok } else { PState::Warn },
    }
}

pub fn check_all() -> Vec<Prereq> {
    vec![git(), rust(), msvc(), gaming_services(), memory()]
}

fn winget(extra: &[&str], cancel: &AtomicBool, log: &dyn Fn(String)) -> Result<String, String> {
    let mut c = Command::new("winget");
    c.arg("install").arg("-e").args(extra).args(["--accept-package-agreements", "--accept-source-agreements"]);
    log("Se va a abrir una ventana de Windows pidiendo permiso para instalar.".into());
    let code = sys::run_stream(c, cancel, &mut |l| log(l)).map_err(|e| format!("No se pudo ejecutar winget: {e}"))?;
    if code == 0 {
        Ok("Instalado. Cerrá y volvé a abrir el lanzador para que lo detecte.".into())
    } else {
        Err(format!("winget terminó con error (código {code}). Si Windows Update o la tienda están bloqueados, instalalo a mano."))
    }
}

/// Instala o arregla uno de los requisitos.
pub fn install(id: &str, cancel: &AtomicBool, log: &dyn Fn(String)) -> Result<String, String> {
    match id {
        "git" => winget(&["--id", "Git.Git"], cancel, log),
        "rust" => winget(&["--id", "Rustlang.Rustup"], cancel, log),
        "msvc" => winget(
            &[
                "--id",
                "Microsoft.VisualStudio.2022.BuildTools",
                "--override",
                "--passive --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --add Microsoft.VisualStudio.Component.Windows11SDK.26100 --includeRecommended",
            ],
            cancel,
            log,
        ),
        "xgameruntime" => {
            sys::open_uri("ms-windows-store://pdp/?ProductId=9MWPM2CQNLHN");
            Ok("Se abrió la Microsoft Store en Gaming Services: instalalo desde ahí.".into())
        }
        "memory" => {
            let _ = Command::new("SystemPropertiesAdvanced.exe").spawn();
            Ok("Rendimiento → Configuración → Opciones avanzadas → Memoria virtual → Cambiar. Poné un tamaño fijo de 32768 MB o más y reiniciá.".into())
        }
        _ => Err("Requisito desconocido.".into()),
    }
}
