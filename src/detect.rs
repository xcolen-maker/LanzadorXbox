//! Busca en esta PC lo que la persona ya tenga: copias de los emuladores (por la
//! dirección de GitHub de sus remotos) y la carpeta del juego. Solo mira las carpetas
//! de primer nivel de los lugares habituales, para que sea rápido.

use crate::config::{Repo, repos_dir};
use crate::repo::find_game_exe;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Deja una dirección de git en una forma comparable:
/// `https://github.com/A/B.git`, `git@github.com:a/b` → `github.com/a/b`.
pub fn norm_url(u: &str) -> String {
    let mut u = u.trim().to_lowercase();
    for p in ["https://", "http://", "ssh://", "git://"] {
        if let Some(r) = u.strip_prefix(p) {
            u = r.to_string();
        }
    }
    if let Some(r) = u.strip_prefix("git@") {
        u = r.replacen(':', "/", 1);
    }
    let u = u.trim_start_matches("www.").trim_end_matches('/');
    u.strip_suffix(".git").unwrap_or(u).trim_end_matches('/').to_string()
}

/// La dirección que dice de qué es una copia de git (leyendo su `.git\config`): la del
/// remoto `origin`, de donde se clonó. Si no tiene `origin`, valen todos sus remotos.
/// (Una copia de VIctory con el fork agregado como otro remoto sigue siendo VIctory.)
fn remote_urls(dir: &Path) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(dir.join(".git").join("config")) else { return Vec::new() };
    let mut remotes: Vec<(String, String)> = Vec::new();
    let mut current: Option<String> = None;
    for l in text.lines().map(str::trim) {
        if l.starts_with('[') {
            current = l.strip_prefix("[remote \"").and_then(|r| r.strip_suffix("\"]")).map(str::to_string);
        } else if let Some(name) = &current
            && let Some(url) = l.strip_prefix("url").map(str::trim_start).and_then(|r| r.strip_prefix('='))
        {
            remotes.push((name.clone(), norm_url(url)));
        }
    }
    match remotes.iter().find(|(n, _)| n == "origin") {
        Some((_, u)) => vec![u.clone()],
        None => remotes.into_iter().map(|(_, u)| u).collect(),
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(PathBuf::from)
}

/// Las carpetas personales donde la gente suele dejar las cosas.
fn user_folders() -> Vec<PathBuf> {
    let Some(home) = home() else { return Vec::new() };
    [
        "",
        "Desktop",
        "Escritorio",
        "Documents",
        "Documentos",
        "Downloads",
        "Descargas",
        "OneDrive\\Desktop",
        "OneDrive\\Escritorio",
        "OneDrive\\Documents",
        "OneDrive\\Documentos",
        "source\\repos",
        "Documents\\GitHub",
        "code",
        "dev",
        "projects",
        "repos",
        "git",
        "GitHub",
        "src",
        "Games",
        "Juegos",
    ]
    .iter()
    .map(|s| if s.is_empty() { home.clone() } else { home.join(s) })
    .collect()
}

/// Las raíces de los discos (C:\, D:\, …) y algunas carpetas habituales en ellas.
fn drive_folders() -> Vec<PathBuf> {
    let mut v = Vec::new();
    for d in b'C'..=b'Z' {
        let root = PathBuf::from(format!("{}:\\", d as char));
        if !root.is_dir() {
            continue;
        }
        for sub in ["", "dev", "code", "repos", "projects", "src", "Games", "Juegos", "XboxGames"] {
            v.push(if sub.is_empty() { root.clone() } else { root.join(sub) });
        }
    }
    v
}

/// Las subcarpetas directas de cada lugar, sin repetir.
fn children(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for root in roots {
        let Ok(rd) = std::fs::read_dir(root) else { continue };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() && seen.insert(p.to_string_lossy().to_lowercase()) {
                out.push(p);
            }
        }
    }
    out
}

fn same(a: &Path, b: &str) -> bool {
    a.to_string_lossy().trim_end_matches(['\\', '/']).to_lowercase() == b.trim_end_matches(['\\', '/']).to_lowercase()
}

/// Para cada emulador, las copias que ya hay en esta PC fuera de donde lo busca el lanzador.
pub fn find_clones(repos: &[Repo]) -> HashMap<String, Vec<PathBuf>> {
    let own = repos_dir().to_string_lossy().to_lowercase();
    let mut roots = user_folders();
    roots.extend(drive_folders());
    let mut found: HashMap<String, Vec<PathBuf>> = HashMap::new();
    for dir in children(&roots) {
        if dir.to_string_lossy().to_lowercase().starts_with(&own) {
            continue;
        }
        let urls = remote_urls(&dir);
        if urls.is_empty() {
            continue;
        }
        for r in repos {
            if !same(&dir, &r.path) && urls.contains(&norm_url(&r.url)) {
                found.entry(r.id.clone()).or_default().push(dir.clone());
            }
        }
    }
    found
}

/// Busca la carpeta del juego: en las carpetas personales, un nivel más abajo, y en
/// las carpetas de juegos de cada disco (también dentro de `Content`, como deja la app de Xbox).
pub fn find_game_dir() -> Option<PathBuf> {
    let mut roots = user_folders();
    roots.extend(drive_folders());
    for dir in children(&roots) {
        for d in [dir.clone(), dir.join("Content")] {
            if find_game_exe(&d).is_some() {
                return Some(d);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::norm_url;

    #[test]
    fn urls_iguales() {
        let a = norm_url("https://github.com/Marvisak/VIctory");
        assert_eq!(a, "github.com/marvisak/victory");
        assert_eq!(norm_url("https://github.com/Marvisak/VIctory.git"), a);
        assert_eq!(norm_url("git@github.com:Marvisak/VIctory.git"), a);
        assert_eq!(norm_url("https://www.github.com/marvisak/victory/"), a);
        assert_ne!(norm_url("https://github.com/fxceless/victory-fork"), a);
    }
}
