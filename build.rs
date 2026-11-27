//! Mete el icono en el .exe, para que Windows lo muestre en el escritorio y en el explorador.

fn main() {
    println!("cargo:rerun-if-changed=icon/app.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("icon/app.ico");
        res.set("FileDescription", "Lanzador Xbox");
        res.set("ProductName", "Lanzador Xbox");
        if let Err(e) = res.compile() {
            // Sin el SDK de Windows (rc.exe) se compila igual, solo que sin icono.
            println!("cargo:warning=No se pudo poner el icono en el .exe: {e}");
        }
    }
}
