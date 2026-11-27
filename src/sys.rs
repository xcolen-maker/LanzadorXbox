//! Cosas de Windows: ejecutar programas sin ventana, leer su salida,
//! abrir carpetas y archivos, y consultar la memoria.

use std::io::{BufRead, BufReader, Read};
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Que el programa no abra una ventana de consola.
pub fn quiet(cmd: &mut Command) -> &mut Command {
    cmd.creation_flags(CREATE_NO_WINDOW)
}

/// Ejecuta un programa y devuelve su salida si terminó bien.
pub fn capture(program: &str, args: &[&str], cwd: Option<&Path>) -> Option<String> {
    let mut c = Command::new(program);
    c.args(args).stdin(Stdio::null());
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    quiet(&mut c);
    let out = c.output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn spawn_reader<R: Read + Send + 'static>(r: R, tx: mpsc::Sender<String>) {
    std::thread::spawn(move || {
        let mut br = BufReader::new(r);
        let mut buf = Vec::new();
        loop {
            buf.clear();
            match br.read_until(b'\n', &mut buf) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let text = String::from_utf8_lossy(&buf);
                    let text = text.trim_end_matches(['\r', '\n']);
                    // Las barras de progreso reescriben la línea con \r.
                    let line = text.rsplit('\r').next().unwrap_or("").trim_end();
                    if !line.is_empty() && tx.send(line.to_string()).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

/// Ejecuta un programa mostrando su salida línea a línea. Si `cancel` se
/// activa, lo mata (con todo lo que haya lanzado). Devuelve el código de salida.
pub fn run_stream(
    mut cmd: Command,
    cancel: &AtomicBool,
    on_line: &mut dyn FnMut(String),
) -> std::io::Result<i32> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).stdin(Stdio::null());
    quiet(&mut cmd);
    let mut child = cmd.spawn()?;
    let (tx, rx) = mpsc::channel::<String>();
    if let Some(o) = child.stdout.take() {
        spawn_reader(o, tx.clone());
    }
    if let Some(e) = child.stderr.take() {
        spawn_reader(e, tx.clone());
    }
    drop(tx);
    let mut killed = false;
    loop {
        match rx.recv_timeout(Duration::from_millis(150)) {
            Ok(line) => on_line(line),
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
        if cancel.load(Ordering::Relaxed) && !killed {
            killed = true;
            kill_tree(child.id());
            on_line("Cancelado.".into());
        }
    }
    let status = child.wait()?;
    Ok(status.code().unwrap_or(-1))
}

/// Mata un proceso y todos los que lanzó.
pub fn kill_tree(pid: u32) {
    let mut c = Command::new("taskkill");
    c.args(["/PID", &pid.to_string(), "/T", "/F"]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    quiet(&mut c);
    let _ = c.status();
}

/// La ruta de cargo (o "cargo" si está en el PATH).
pub fn cargo_path() -> String {
    static CARGO: OnceLock<String> = OnceLock::new();
    CARGO
        .get_or_init(|| {
            if capture("cargo", &["--version"], None).is_some() {
                return "cargo".into();
            }
            let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
            let p = home.join(".cargo").join("bin").join("cargo.exe");
            if p.exists() { p.to_string_lossy().into_owned() } else { "cargo".into() }
        })
        .clone()
}

/// Abre una carpeta en el Explorador.
pub fn open_folder(p: &Path) {
    let _ = Command::new("explorer").arg(p).spawn();
}

/// Abre un archivo de texto en el Bloc de notas.
pub fn open_text(p: &Path) {
    let mut c = Command::new("notepad");
    c.arg(p);
    let _ = c.spawn();
}

/// Abre una dirección o un programa con la aplicación predeterminada.
pub fn open_uri(uri: &str) {
    let mut c = Command::new("cmd");
    c.args(["/C", "start", "", uri]);
    quiet(&mut c);
    let _ = c.spawn();
}

/// (memoria física en MB, límite de memoria comprometida en MB).
pub fn memory_mb() -> (u64, u64) {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    // SAFETY: una estructura nuestra, con su tamaño puesto.
    unsafe {
        let mut s: MEMORYSTATUSEX = std::mem::zeroed();
        s.dwLength = size_of::<MEMORYSTATUSEX>() as u32;
        if GlobalMemoryStatusEx(&mut s) != 0 {
            (s.ullTotalPhys >> 20, s.ullTotalPageFile >> 20)
        } else {
            (0, 0)
        }
    }
}

/// Conecta con la consola desde la que se lanzó (para los modos de línea de comandos).
pub fn attach_console() {
    // SAFETY: una llamada simple.
    unsafe {
        windows_sys::Win32::System::Console::AttachConsole(u32::MAX);
    }
}
