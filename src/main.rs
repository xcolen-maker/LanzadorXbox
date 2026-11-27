// En la versión final no se abre una ventana de consola.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod config;
mod detect;
mod prereq;
mod repo;
mod state;
mod sys;

use config::Config;
use eframe::egui;
use std::io::Write;
use std::sync::atomic::AtomicBool;

fn main() -> eframe::Result {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        cli(&args);
        return Ok(());
    }
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1240.0, 700.0])
            .with_min_inner_size([960.0, 580.0])
            .with_title("Lanzador Xbox")
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../icon/logo.png")).unwrap_or_default()),
        ..Default::default()
    };
    eframe::run_native("Lanzador Xbox", options, Box::new(|cc| Ok(Box::new(app::App::new(cc)))))
}

/// Imprime en la consola y también en un archivo (para poder revisarlo).
fn out(line: &str) {
    println!("{line}");
    let _ = std::fs::create_dir_all(config::log_dir());
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(config::log_dir().join("cli.txt")) {
        let _ = writeln!(f, "{line}");
    }
}

/// Modos de línea de comandos, para probar sin abrir la ventana.
fn cli(args: &[String]) {
    sys::attach_console();
    let cfg = Config::load();
    let cancel = AtomicBool::new(false);
    match args[0].as_str() {
        "--check" => {
            for r in &cfg.repos {
                if repo::is_cloned(r)
                    && let Err(e) = repo::fetch(r, &cancel, &|l| out(&format!("  {l}")))
                {
                    out(&format!("  no se pudo buscar: {e}"));
                }
                let st = repo::status(&cfg, r);
                out(&format!(
                    "{:<8} descargado={} compilado={} commits_nuevos={} compilar={} versión={}",
                    r.id,
                    st.cloned,
                    st.built,
                    st.behind,
                    st.needs_build,
                    st.head_ts.map(repo::when).unwrap_or_else(|| "-".into())
                ));
            }
        }
        "--prereq" => {
            for p in prereq::check_all() {
                out(&format!("{:<14} {:?}  {} · {}", p.id, p.state, p.name, p.desc));
            }
        }
        "--install" => {
            let Some(r) = args.get(1).and_then(|id| cfg.repo(id)) else {
                out("Uso: --install <xcl|victory|fork|nvidia>");
                return;
            };
            match repo::install(r, &cancel, &|l| out(&format!("  {l}"))) {
                Ok(head) => out(&format!("Listo: {head}")),
                Err(e) => out(&format!("ERROR: {e}")),
            }
        }
        "--env" => {
            // Las variables con las que se lanzaría cada VIctory (para revisarlas).
            for r in cfg.repos.iter().filter(|r| r.kind == config::Kind::Victory) {
                let console = r.console.as_deref().unwrap_or(&cfg.settings.console);
                out(&format!("{} (consola {console}):", r.id));
                for (k, v) in repo::launch_env(r, &cfg.settings) {
                    out(&format!("  {k}={v}"));
                }
            }
        }
        "--detect" => {
            // Lo que el lanzador encuentra en esta PC.
            for (id, dirs) in detect::find_clones(&cfg.repos) {
                for d in dirs {
                    out(&format!("{id:<8} copia en {}", d.display()));
                }
            }
            match detect::find_game_dir() {
                Some(d) => out(&format!("juego    {}", d.display())),
                None => out("juego    no encontrado"),
            }
        }
        _ => out("Uso: --check | --prereq | --env | --detect | --install <xcl|victory|fork|nvidia>"),
    }
}
