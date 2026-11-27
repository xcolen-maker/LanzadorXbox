//! El estado que comparten la ventana y las tareas en segundo plano.

use crate::config::Config;
use crate::detect;
use crate::prereq::{self, Prereq};
use crate::repo::{self, Status};
use eframe::egui;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Child;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

pub struct Job {
    pub title: String,
    pub cancel: Arc<AtomicBool>,
}

pub struct Running {
    pub child: Child,
    pub since: Instant,
}

pub struct Shared {
    pub cfg: Mutex<Config>,
    pub status: Mutex<HashMap<String, Status>>,
    pub prereqs: Mutex<Vec<Prereq>>,
    pub log: Mutex<Vec<String>>,
    pub job: Mutex<Option<Job>>,
    pub checking: Mutex<HashSet<String>>,
    pub running: Mutex<HashMap<String, Running>>,
    pub exits: Mutex<HashMap<String, String>>,
    /// Copias de cada emulador que ya hay en esta PC, fuera de donde las busca el lanzador.
    pub found: Mutex<HashMap<String, Vec<PathBuf>>>,
    /// El último aviso y si es un error.
    pub message: Mutex<(String, bool)>,
    pub ctx: egui::Context,
}

pub fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Shared {
    pub fn new(ctx: egui::Context) -> Arc<Shared> {
        Arc::new(Shared {
            cfg: Mutex::new(Config::load()),
            status: Mutex::default(),
            prereqs: Mutex::default(),
            log: Mutex::default(),
            job: Mutex::default(),
            checking: Mutex::default(),
            running: Mutex::default(),
            exits: Mutex::default(),
            found: Mutex::default(),
            message: Mutex::default(),
            ctx,
        })
    }

    pub fn cfg(&self) -> Config {
        lock(&self.cfg).clone()
    }

    pub fn log(&self, line: impl Into<String>) {
        let mut log = lock(&self.log);
        log.push(line.into());
        if log.len() > 3000 {
            log.drain(..1000);
        }
        drop(log);
        self.ctx.request_repaint();
    }

    pub fn say(&self, text: impl Into<String>, error: bool) {
        *lock(&self.message) = (text.into(), error);
        self.ctx.request_repaint();
    }

    /// Vuelve a leer el estado de los repositorios y los requisitos (en segundo plano).
    pub fn refresh(self: &Arc<Self>) {
        let me = self.clone();
        std::thread::spawn(move || {
            let cfg = me.cfg();
            let mut map = HashMap::new();
            for r in &cfg.repos {
                map.insert(r.id.clone(), repo::status(&cfg, r));
            }
            *lock(&me.status) = map;
            me.ctx.request_repaint();
            let p = prereq::check_all();
            *lock(&me.prereqs) = p;
            me.ctx.request_repaint();
            // Lo que la persona ya tenga en la PC.
            *lock(&me.found) = detect::find_clones(&cfg.repos);
            if (cfg.game_dir.is_empty() || repo::find_game_exe(std::path::Path::new(&cfg.game_dir)).is_none())
                && let Some(dir) = detect::find_game_dir()
            {
                let text = dir.to_string_lossy().into_owned();
                me.log(format!("Encontré el juego en {text}"));
                me.say(format!("Encontré el juego en {text}"), false);
                let mut c = lock(&me.cfg);
                c.game_dir = text;
                c.save();
            }
            me.ctx.request_repaint();
        });
    }

    /// Cambia la carpeta de un emulador (una copia que ya tenía la persona, u otra que eligió).
    /// `None` vuelve a la copia propia del lanzador.
    pub fn set_repo_path(self: &Arc<Self>, id: &str, path: Option<PathBuf>) {
        if lock(&self.running).contains_key(id) {
            self.say("Cerrá el juego antes de cambiar la carpeta del emulador.", true);
            return;
        }
        let Some(default) = Config::defaults_repo(id) else { return };
        {
            let mut c = lock(&self.cfg);
            let Some(r) = c.repos.iter_mut().find(|r| r.id == id) else { return };
            r.path = path.map(|p| p.to_string_lossy().into_owned()).unwrap_or(default.path);
            let (name, shown) = (r.name.clone(), r.path.clone());
            c.built.remove(id);
            c.save();
            drop(c);
            self.say(format!("{name} ahora usa {shown}"), false);
        }
        self.refresh();
    }

    /// Empieza una tarea larga (una sola a la vez).
    pub fn start_job(
        self: &Arc<Self>,
        title: String,
        f: impl FnOnce(&Arc<Shared>, &AtomicBool) -> Result<String, String> + Send + 'static,
    ) {
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut j = lock(&self.job);
            if j.is_some() {
                drop(j);
                self.say("Ya hay una tarea en curso.", true);
                return;
            }
            *j = Some(Job { title: title.clone(), cancel: cancel.clone() });
        }
        let me = self.clone();
        std::thread::spawn(move || {
            me.log(format!("== {title} =="));
            match f(&me, &cancel) {
                Ok(m) => {
                    me.log(m.clone());
                    me.say(m, false);
                }
                Err(e) => {
                    me.log(format!("ERROR: {e}"));
                    me.say(e, true);
                }
            }
            *lock(&me.job) = None;
            me.refresh();
        });
    }

    /// Instala o actualiza un repositorio.
    pub fn install_repo(self: &Arc<Self>, id: &str) {
        let id = id.to_string();
        let Some(name) = self.cfg().repo(&id).map(|r| r.name.clone()) else { return };
        // Windows no deja reemplazar un ejecutable en uso: la compilación fallaría al final.
        if lock(&self.running).contains_key(&id) {
            self.say(format!("Cerrá el juego antes de actualizar {name}: no se puede reemplazar mientras está abierto."), true);
            return;
        }
        self.start_job(format!("Instalando {name}"), move |sh, cancel| {
            let cfg = sh.cfg();
            let repo = cfg.repo(&id).ok_or("repositorio desconocido")?.clone();
            let head = repo::install(&repo, cancel, &|l| sh.log(l))?;
            let mut c = lock(&sh.cfg);
            c.built.insert(id.clone(), head);
            c.save();
            Ok(format!("{} quedó instalado y compilado.", repo.name))
        });
    }

    /// Instala o arregla un requisito.
    pub fn install_prereq(self: &Arc<Self>, id: &'static str) {
        self.start_job(format!("Instalando {id}"), move |sh, cancel| prereq::install(id, cancel, &|l| sh.log(l)));
    }

    /// Busca actualizaciones de un repositorio (rápido, en segundo plano).
    pub fn check_updates(self: &Arc<Self>, id: &str) {
        if !lock(&self.checking).insert(id.to_string()) {
            return;
        }
        let (me, id) = (self.clone(), id.to_string());
        std::thread::spawn(move || {
            let cfg = me.cfg();
            if let Some(repo) = cfg.repo(&id)
                && repo::is_cloned(repo)
            {
                match repo::fetch(repo, &AtomicBool::new(false), &|l| me.log(l)) {
                    Ok(()) => {
                        let st = repo::status(&cfg, repo);
                        lock(&me.status).insert(id.clone(), st.clone());
                        if st.behind > 0 {
                            me.say(format!("{}: hay {}.", repo.name, repo::cambios(st.behind)), false);
                        } else {
                            me.say(format!("{}: ya tenés lo último.", repo.name), false);
                        }
                    }
                    Err(e) => me.say(format!("{}: no se pudo buscar ({e}). ¿Hay internet?", repo.name), true),
                }
            }
            lock(&me.checking).remove(&id);
            me.ctx.request_repaint();
        });
    }

    /// Inicia el juego con este emulador.
    pub fn launch(self: &Arc<Self>, id: &str) {
        if !lock(&self.running).is_empty() {
            self.say("Ya hay un juego en ejecución.", true);
            return;
        }
        let cfg = self.cfg();
        let Some(repo) = cfg.repo(id) else { return };
        match repo::launch(&cfg, repo) {
            Ok(child) => {
                lock(&self.exits).remove(id);
                lock(&self.running).insert(id.to_string(), Running { child, since: Instant::now() });
                self.say(format!("{} iniciado. Su registro: {}", repo.name, repo::log_file(repo).display()), false);
            }
            Err(e) => self.say(e, true),
        }
    }

    pub fn stop(&self, id: &str) {
        if let Some(r) = lock(&self.running).get_mut(id) {
            crate::sys::kill_tree(r.child.id());
        }
    }

    /// Mira si el juego sigue en marcha.
    pub fn poll_running(&self) {
        let mut done = Vec::new();
        {
            let mut running = lock(&self.running);
            for (id, r) in running.iter_mut() {
                if let Ok(Some(status)) = r.child.try_wait() {
                    let code = status.code().unwrap_or(-1);
                    let secs = r.since.elapsed().as_secs();
                    let text = if code == 0 {
                        format!("Terminó bien ({secs} s).")
                    } else {
                        format!("Terminó con código {:#x} después de {secs} s. Mirá el registro.", code as u32)
                    };
                    done.push((id.clone(), text));
                }
            }
            for (id, _) in &done {
                running.remove(id);
            }
        }
        if !done.is_empty() {
            let mut ex = lock(&self.exits);
            for (id, text) in done {
                ex.insert(id, text);
            }
        }
    }
}
