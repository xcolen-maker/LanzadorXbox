//! La ventana, con el estilo de un lanzador de juegos de PC: barra lateral negra con los
//! emuladores, pestañas arriba a la derecha, un gran banner con el botón principal y
//! tarjetas de novedades, y la barra de tareas abajo.

use crate::config::{Config, Kind, Repo, Settings, data_dir};
use crate::i18n::{self, Lang, tr};
use crate::prereq::PState;
use crate::repo::{self, Status};
use crate::state::{Shared, lock};
use crate::sys;
use eframe::egui::{
    self, Align, Align2, Button, Color32, CursorIcon, FontId, Frame, Layout, Margin, Painter, Pos2, Rect, Response, RichText, Sense, Shape,
    Stroke, StrokeKind, TextureFilter, TextureHandle, TextureOptions, Ui, UiBuilder, Vec2, pos2, vec2,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;

mod setup;

const SIDE: Color32 = Color32::from_rgb(6, 6, 7);
const SIDE_HOVER: Color32 = Color32::from_rgb(30, 30, 34);
const BAR: Color32 = Color32::from_rgb(74, 74, 78);
const BAR_DARK: Color32 = Color32::from_rgb(30, 30, 32);
const CARD: Color32 = Color32::from_rgb(22, 22, 26);
const CARD_LINE: Color32 = Color32::from_rgb(50, 50, 58);
const BUTTON: Color32 = Color32::from_rgb(40, 40, 46);
const TEXT: Color32 = Color32::from_rgb(240, 240, 242);
const MUTED: Color32 = Color32::from_rgb(160, 160, 168);
const DIM: Color32 = Color32::from_rgb(100, 100, 108);
const GREEN: Color32 = Color32::from_rgb(40, 205, 135);
const ORANGE: Color32 = Color32::from_rgb(245, 165, 50);
const RED: Color32 = Color32::from_rgb(235, 85, 95);
const YELLOW: Color32 = Color32::from_rgb(252, 176, 28);

/// Dos colores por emulador, para su icono, su tarjeta y la sombra del título.
const ACCENTS: [(Color32, Color32); 4] = [
    (Color32::from_rgb(255, 72, 150), Color32::from_rgb(120, 70, 255)),
    (Color32::from_rgb(40, 200, 220), Color32::from_rgb(40, 90, 230)),
    (Color32::from_rgb(255, 150, 40), Color32::from_rgb(230, 50, 90)),
    (Color32::from_rgb(110, 220, 90), Color32::from_rgb(20, 140, 150)),
];

fn accent(i: usize) -> (Color32, Color32) {
    ACCENTS[i % ACCENTS.len()]
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Play,
    Install,
    Settings,
    /// Agregar un emulador propio (un fork de VIctory o de xcl).
    Add,
}

/// Las imágenes del juego, metidas dentro del ejecutable.
struct Art {
    /// El logo del lanzador (el mismo icono del .exe).
    app_logo: Option<TextureHandle>,
    logo: Option<TextureHandle>,
    /// Un fondo por emulador (se repiten si hay más emuladores que fondos), con el encuadre
    /// de su miniatura: centro de los personajes (x, y) y qué parte del alto mostrar, en 0..1.
    bgs: Vec<(TextureHandle, [f32; 3])>,
}

fn load_tex(ctx: &egui::Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    // Con mipmaps las miniaturas no se ven granuladas al achicar 2000 px a 200.
    let mut opt = TextureOptions::LINEAR;
    opt.mipmap_mode = Some(TextureFilter::Linear);
    Some(ctx.load_texture(name, ci, opt))
}

/// La carpeta de las imágenes del lanzador. No vienen con el programa: cada uno pone
/// las suyas (`logo.png` y los fondos, en .jpg o .png). Sin imágenes se dibuja un atardecer.
pub fn images_dir() -> PathBuf {
    data_dir().join("imagenes")
}

/// Dónde se buscan las imágenes: la carpeta de datos y una carpeta `imagenes` junto al .exe.
fn image_dirs() -> Vec<PathBuf> {
    let mut v = vec![images_dir()];
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("imagenes"))) {
        v.push(dir);
    }
    v
}

/// El encuadre de la miniatura de las imágenes conocidas: centro de los personajes (x, y)
/// y qué parte del alto mostrar. Para cualquier otra imagen, el centro completo.
fn framing(stem: &str) -> [f32; 3] {
    match stem {
        // En el muelle: los dos en el centro, la cabeza de él tapa el final del logo.
        "fondo1" => [0.48, 0.57, 0.6],
        // Sobre el auto: los dos en el centro, debajo del logo.
        "fondo2" => [0.51, 0.645, 0.6],
        // En la gasolinera: los dos a la derecha, el logo a la izquierda.
        "fondo3" => [0.63, 0.45, 0.7],
        // El collage: el cuadro de la pareja, arriba a la izquierda del logo.
        "fondo4" => [0.335, 0.19, 0.32],
        _ => [0.5, 0.5, 1.0],
    }
}

impl Art {
    fn load(ctx: &egui::Context) -> Art {
        let is_image = |p: &PathBuf| p.extension().and_then(|e| e.to_str()).is_some_and(|e| ["png", "jpg", "jpeg"].contains(&e.to_lowercase().as_str()));
        // La primera carpeta que tenga imágenes.
        let files: Vec<PathBuf> = image_dirs()
            .into_iter()
            .map(|d| {
                let mut f: Vec<PathBuf> = std::fs::read_dir(&d).map(|rd| rd.flatten().map(|e| e.path()).filter(is_image).collect()).unwrap_or_default();
                f.sort();
                f
            })
            .find(|f| !f.is_empty())
            .unwrap_or_default();
        let stem = |p: &PathBuf| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
        let load = |p: &PathBuf| std::fs::read(p).ok().and_then(|b| load_tex(ctx, &p.to_string_lossy(), &b));
        let logo = files.iter().find(|p| stem(p) == "logo").and_then(load);
        let bgs = files.iter().filter(|p| stem(p) != "logo").filter_map(|p| load(p).map(|t| (t, framing(&stem(p))))).collect();
        let app_logo = load_tex(ctx, "logo-lanzador", include_bytes!("../icon/logo.png"));
        Art { app_logo, logo, bgs }
    }

    /// Dibuja el logo del lanzador; sin la imagen, un cuadrado amarillo con una X.
    fn paint_app_logo(&self, p: &Painter, r: Rect) {
        match &self.app_logo {
            Some(t) => {
                p.image(t.id(), r, FULL, Color32::WHITE);
            }
            None => {
                p.rect_filled(r, 6.0, YELLOW);
                bold(p, r.center() + vec2(0.0, 1.0), Align2::CENTER_CENTER, "X", r.height() * 0.65, Color32::from_gray(12));
            }
        }
    }

    fn bg(&self, i: usize) -> Option<&TextureHandle> {
        if self.bgs.is_empty() { None } else { Some(&self.bgs[i % self.bgs.len()].0) }
    }

    /// La parte de la imagen que va en una miniatura de proporción `aspect` (ancho / alto),
    /// centrada en los personajes.
    fn thumb_uv(&self, i: usize, aspect: f32) -> Option<(&TextureHandle, Rect)> {
        if self.bgs.is_empty() {
            return None;
        }
        let (tex, [cx, cy, hf]) = &self.bgs[i % self.bgs.len()];
        let h = hf.min(1.0);
        let w = (h * aspect / tex.aspect_ratio()).min(1.0);
        let x0 = (cx - w / 2.0).clamp(0.0, 1.0 - w);
        let y0 = (cy - h / 2.0).clamp(0.0, 1.0 - h);
        Some((tex, Rect::from_min_size(pos2(x0, y0), vec2(w, h))))
    }
}

/// Dibuja una imagen llenando el rectángulo sin deformarla (recorta lo que sobra).
/// `focus` dice qué parte conservar: (0,0) arriba a la izquierda, (0.5,0.5) el centro.
/// `src` limita qué parte de la imagen se usa (en 0..1), por ejemplo para dejar fuera el logo de arriba.
fn paint_cover(p: &Painter, tex: &TextureHandle, r: Rect, focus: Vec2, src: Rect) {
    let ta = tex.aspect_ratio() * src.width() / src.height();
    let ra = r.width() / r.height();
    let uv = if ra > ta {
        let h = ta / ra;
        let y0 = (1.0 - h) * focus.y;
        Rect::from_min_max(pos2(0.0, y0), pos2(1.0, y0 + h))
    } else {
        let w = ra / ta;
        let x0 = (1.0 - w) * focus.x;
        Rect::from_min_max(pos2(x0, 0.0), pos2(x0 + w, 1.0))
    };
    let uv = Rect::from_min_max(src.lerp_inside(uv.min.to_vec2()), src.lerp_inside(uv.max.to_vec2()));
    p.image(tex.id(), r, uv, Color32::WHITE);
}

const FULL: Rect = Rect { min: Pos2 { x: 0.0, y: 0.0 }, max: Pos2 { x: 1.0, y: 1.0 } };

pub struct App {
    sh: Arc<Shared>,
    art: Art,
    tab: Tab,
    /// El emulador elegido en la barra lateral.
    sel: usize,
    /// La primera tarjeta visible del carrusel.
    card_start: usize,
    show_log: bool,
    show_path: bool,
    /// La ayuda de tr("¿Dónde está el juego?") en la configuración inicial.
    setup_help: bool,
    /// El formulario de tr("Agregar emulador").
    add_name: String,
    add_url: String,
    add_branch: String,
    add_kind: Kind,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> App {
        let ctx = cc.egui_ctx.clone();
        style(&ctx);
        let art = Art::load(&ctx);
        let sh = Shared::new(ctx);
        i18n::set(i18n::resolve(&sh.cfg().settings.lang));
        sh.refresh();
        // Al abrir, busca actualizaciones de lo que ya está descargado.
        for r in sh.cfg().repos {
            sh.check_updates(&r.id);
        }
        let tab = match std::env::var("LANZADOR_TAB").as_deref() {
            Ok("instalar") => Tab::Install,
            Ok("ajustes") => Tab::Settings,
            Ok("agregar") => Tab::Add,
            _ => Tab::Play,
        };
        App { sh, art, tab, sel: 0, card_start: 0, show_log: false, show_path: false, setup_help: false,
            add_name: String::new(),
            add_url: String::new(),
            add_branch: String::new(),
            add_kind: Kind::Victory,
        }
    }
}

fn style(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = SIDE;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(10, 10, 12);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, CARD_LINE);
    v.widgets.inactive.weak_bg_fill = BUTTON;
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(56, 56, 64);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(70, 70, 80);
    v.selection.bg_fill = Color32::from_rgb(90, 90, 100);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = vec2(10.0, 10.0);
        s.spacing.button_padding = vec2(14.0, 8.0);
    });
}

// ---------- Dibujo ----------

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { format!("{}…", s.chars().take(n - 1).collect::<String>()) }
}

/// Texto "en negrita": la fuente de egui no tiene negrita, así que se dibuja dos veces.
fn bold(p: &Painter, pos: Pos2, align: Align2, text: &str, size: f32, color: Color32) -> Rect {
    p.text(pos + vec2(0.7, 0.0), align, text, FontId::proportional(size), color);
    p.text(pos, align, text, FontId::proportional(size), color)
}

fn vgrad(p: &Painter, r: Rect, top: Color32, bottom: Color32) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(r.left_top(), top);
    m.colored_vertex(r.right_top(), top);
    m.colored_vertex(r.right_bottom(), bottom);
    m.colored_vertex(r.left_bottom(), bottom);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

fn hgrad(p: &Painter, r: Rect, left: Color32, right: Color32) {
    let mut m = egui::Mesh::default();
    m.colored_vertex(r.left_top(), left);
    m.colored_vertex(r.right_top(), right);
    m.colored_vertex(r.right_bottom(), right);
    m.colored_vertex(r.left_bottom(), left);
    m.add_triangle(0, 1, 2);
    m.add_triangle(0, 2, 3);
    p.add(Shape::mesh(m));
}

/// Un degradado vertical con varias paradas (posición 0..1, color).
fn vstops(p: &Painter, r: Rect, stops: &[(f32, Color32)]) {
    for w in stops.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg = Rect::from_min_max(pos2(r.left(), r.top() + r.height() * a.0), pos2(r.right(), r.top() + r.height() * b.0));
        vgrad(p, seg, a.1, b.1);
    }
}

/// Generador fijo de números (para que los edificios no cambien de un cuadro a otro).
fn noise(i: u32) -> f32 {
    let mut x = i.wrapping_mul(747_796_405).wrapping_add(2_891_336_453);
    x = (x ^ (x >> 16)).wrapping_mul(0x45d9_f3b);
    x ^= x >> 16;
    (x % 1000) as f32 / 1000.0
}

fn palm(p: &Painter, base: Pos2, height: f32, lean: f32, color: Color32) {
    let top = base + vec2(lean, -height);
    let trunk: Vec<Pos2> = (0..=12)
        .map(|k| {
            let t = k as f32 / 12.0;
            // Una curva suave: se inclina más arriba.
            pos2(base.x + lean * t * t, base.y - height * t)
        })
        .collect();
    p.add(Shape::line(trunk, Stroke::new(height * 0.035 + 2.0, color)));
    for (dx, dy) in [(-1.0, 0.15), (-0.8, -0.35), (-0.3, -0.6), (0.35, -0.55), (0.85, -0.3), (1.0, 0.2), (0.55, 0.45), (-0.5, 0.45)] {
        let len = height * 0.38;
        let end = top + vec2(dx * len, dy * len * 0.7 + len * 0.25);
        let ctrl = top + vec2(dx * len * 0.55, dy * len * 0.55 - len * 0.18);
        let pts: Vec<Pos2> = (0..=10)
            .map(|k| {
                let t = k as f32 / 10.0;
                let u = 1.0 - t;
                pos2(u * u * top.x + 2.0 * u * t * ctrl.x + t * t * end.x, u * u * top.y + 2.0 * u * t * ctrl.y + t * t * end.y)
            })
            .collect();
        p.add(Shape::line(pts, Stroke::new(height * 0.018 + 1.5, color)));
    }
}

/// El fondo del banner: un atardecer en la costa, dibujado (sin imágenes).
fn scene(p: &Painter, r: Rect) {
    let horizon = r.top() + r.height() * 0.64;
    let sky = Rect::from_min_max(r.min, pos2(r.right(), horizon));
    vstops(
        p,
        sky,
        &[
            (0.0, Color32::from_rgb(34, 26, 78)),
            (0.35, Color32::from_rgb(92, 58, 140)),
            (0.7, Color32::from_rgb(210, 104, 150)),
            (1.0, Color32::from_rgb(252, 168, 120)),
        ],
    );
    // Sol con franjas.
    let sun = pos2(r.left() + r.width() * 0.6, horizon - r.height() * 0.06);
    let sun_r = r.height() * 0.2;
    p.circle_filled(sun, sun_r, Color32::from_rgb(255, 206, 128));
    for k in 0..6 {
        let y = sun.y + sun_r * 0.05 + k as f32 * sun_r * 0.17;
        let h = 2.0 + k as f32 * 1.6;
        let band = Rect::from_min_max(pos2(sun.x - sun_r, y), pos2(sun.x + sun_r, y + h));
        p.rect_filled(band, 0.0, Color32::from_rgb(246, 150, 130));
    }
    // Ciudad en el horizonte.
    let city = Color32::from_rgb(58, 32, 82);
    let mut x = r.left();
    let mut i = 0;
    while x < r.right() {
        let w = 14.0 + noise(i) * 34.0;
        let tall = if (r.left() + r.width() * 0.08..r.left() + r.width() * 0.4).contains(&x) { 90.0 } else { 34.0 };
        let h = 10.0 + noise(i + 99) * tall;
        p.rect_filled(Rect::from_min_max(pos2(x, horizon - h), pos2(x + w, horizon)), 0.0, city);
        x += w + 2.0;
        i += 1;
    }
    // Mar con el reflejo del sol.
    let sea = Rect::from_min_max(pos2(r.left(), horizon), r.max);
    vgrad(p, sea, Color32::from_rgb(86, 48, 112), Color32::from_rgb(16, 12, 34));
    for k in 0..9 {
        let y = horizon + 6.0 + k as f32 * 9.0;
        let w = sun_r * (1.1 - k as f32 * 0.1);
        let a = (170 - k * 16) as u8;
        p.rect_filled(
            Rect::from_center_size(pos2(sun.x, y), vec2(w * 2.0, 2.0)),
            1.0,
            Color32::from_rgba_unmultiplied(255, 190, 140, a),
        );
    }
    // Palmeras.
    let dark = Color32::from_rgb(18, 10, 28);
    palm(p, pos2(r.right() - r.width() * 0.06, r.bottom() + 10.0), r.height() * 0.75, -60.0, dark);
    palm(p, pos2(r.right() - r.width() * 0.17, r.bottom() + 10.0), r.height() * 0.55, 40.0, dark);
    palm(p, pos2(r.left() + r.width() * 0.33, horizon + 4.0), r.height() * 0.22, 14.0, city);
}

/// Sombras sobre el fondo para que se lea el texto: a la izquierda, arriba y abajo.
fn shade(p: &Painter, r: Rect) {
    hgrad(p, Rect::from_min_max(r.min, pos2(r.left() + r.width() * 0.6, r.bottom())), Color32::from_black_alpha(215), Color32::TRANSPARENT);
    vgrad(p, Rect::from_min_max(r.min, pos2(r.right(), r.top() + 80.0)), Color32::from_black_alpha(140), Color32::TRANSPARENT);
    vgrad(p, Rect::from_min_max(pos2(r.left(), r.bottom() - 60.0), r.max), Color32::TRANSPARENT, Color32::from_black_alpha(120));
}

fn initials(name: &str) -> String {
    name.chars().filter(|c| c.is_alphanumeric()).take(2).collect::<String>().to_uppercase()
}

/// El icono cuadrado de cada emulador.
fn repo_icon(p: &Painter, r: Rect, name: &str, i: usize) {
    let (a0, a1) = accent(i);
    p.rect_filled(r, 4.0, a1);
    vgrad(p, r.shrink(2.0), a0, a1);
    bold(p, r.center(), Align2::CENTER_CENTER, &initials(name), r.height() * 0.45, Color32::WHITE);
}

/// El gran botón blanco del banner.
fn big_button(ui: &mut Ui, r: Rect, text: &str, enabled: bool) -> Response {
    let resp = ui.interact(r, ui.scope_id().with(("big", text)), if enabled { Sense::click() } else { Sense::hover() });
    let fill = if !enabled {
        Color32::from_gray(120)
    } else if resp.is_pointer_button_down_on() {
        Color32::from_gray(200)
    } else if resp.hovered() {
        Color32::from_gray(228)
    } else {
        Color32::WHITE
    };
    let p = ui.painter();
    p.rect_filled(r, 6.0, fill);
    p.rect_stroke(r, 6.0, Stroke::new(2.0, Color32::from_gray(185)), StrokeKind::Inside);
    bold(p, r.center(), Align2::CENTER_CENTER, text, 18.0, Color32::from_gray(14));
    if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp }
}

/// Un botón redondeado: relleno blanco o solo el borde.
fn pill(ui: &mut Ui, r: Rect, text: &str, filled: bool, enabled: bool) -> Response {
    let resp = ui.interact(r, ui.scope_id().with(("pill", text, r.min.x as i32)), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    let radius = r.height() / 2.0;
    let hot = enabled && resp.hovered();
    if filled {
        let fill = if !enabled { Color32::from_gray(110) } else if hot { Color32::from_gray(225) } else { Color32::WHITE };
        p.rect_filled(r, radius, fill);
        bold(p, r.center(), Align2::CENTER_CENTER, text, 12.0, Color32::from_gray(14));
    } else {
        if hot {
            p.rect_filled(r, radius, Color32::from_white_alpha(24));
        }
        let c = if enabled { Color32::WHITE } else { DIM };
        p.rect_stroke(r, radius, Stroke::new(1.5, c), StrokeKind::Inside);
        bold(p, r.center(), Align2::CENTER_CENTER, text, 12.0, c);
    }
    if enabled { resp.on_hover_cursor(CursorIcon::PointingHand) } else { resp }
}

/// Un texto que se puede pulsar, como un enlace.
fn link(ui: &mut Ui, pos: Pos2, text: &str, size: f32) -> bool {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(size), MUTED);
    let r = Rect::from_min_size(pos, galley.size());
    let resp = ui.interact(r, ui.scope_id().with(("link", text)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    let color = if resp.hovered() { Color32::WHITE } else { Color32::from_gray(215) };
    ui.painter().text(pos, Align2::LEFT_TOP, text, FontId::proportional(size), color);
    if resp.hovered() {
        ui.painter().line_segment([pos2(r.left(), r.bottom() + 1.0), pos2(r.right(), r.bottom() + 1.0)], Stroke::new(1.0, color));
    }
    resp.clicked()
}

/// La nota de un emulador ("versión mejorada (fxceless)"…), en el idioma elegido.
fn repo_note(r: &Repo) -> String {
    // Las configuraciones guardadas traen la marca en español: se la quita y se traduce.
    let base = r.note.trim_end_matches(" · agregado por vos");
    if r.custom { trf!("{} · agregado por vos", base) } else { tr(base).to_string() }
}

/// Qué decir del estado de un emulador, y con qué color.
fn status_line(st: &Status) -> (String, Color32) {
    let ver = st.head_ts.map(repo::when).unwrap_or_default();
    if !st.cloned {
        (tr("Todavía no está instalado").into(), MUTED)
    } else if !st.built {
        (tr("Descargado, falta compilarlo").into(), ORANGE)
    } else if st.behind > 0 {
        (trf!("Hay {} · tenés la versión de {}", repo::cambios(st.behind), ver), ORANGE)
    } else if st.needs_build {
        (tr("Hay cambios descargados sin compilar").into(), ORANGE)
    } else {
        (trf!("Versión de {}", ver), MUTED)
    }
}

// ---------- Piezas de las pestañas Instalar y Ajustes ----------

fn muted(t: impl Into<String>) -> RichText {
    RichText::new(t.into()).small().color(MUTED)
}

fn card(ui: &mut Ui, add: impl FnOnce(&mut Ui)) {
    Frame::new().fill(CARD).stroke(Stroke::new(1.0, CARD_LINE)).corner_radius(8).inner_margin(Margin::same(14)).show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui);
    });
}

fn dot(ui: &mut Ui, color: Color32) {
    let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(r.center(), 4.5, color);
}

fn button(ui: &mut Ui, text: &str, fill: Color32, color: Color32, size: Vec2, enabled: bool) -> egui::Response {
    let b = Button::new(RichText::new(text).color(color).strong()).fill(fill).corner_radius(18).min_size(size);
    ui.add_enabled(enabled, b)
}

fn small_button(ui: &mut Ui, text: &str) -> bool {
    ui.add(Button::new(RichText::new(text).small().color(TEXT)).fill(CARD).stroke(Stroke::new(1.0, CARD_LINE)).corner_radius(14)).clicked()
}

/// Una fila de la pestaña Instalar. Devuelve si se pulsó el botón.
fn row(ui: &mut Ui, state: PState, title: &str, desc: &str, label: &str, enabled: bool) -> bool {
    let mut clicked = false;
    let button_w = 118.0;
    let left_w = (ui.available_width() - button_w - 26.0 - ui.spacing().item_spacing.x * 2.0).max(80.0);
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::hover());
        let c = r.center();
        let col = match state {
            PState::Ok => GREEN,
            PState::Warn => ORANGE,
            PState::Missing => RED,
        };
        let p = ui.painter();
        p.circle_filled(c, 12.0, col);
        let w = Stroke::new(2.2, Color32::WHITE);
        match state {
            PState::Ok => {
                p.line_segment([pos2(c.x - 5.0, c.y), pos2(c.x - 1.5, c.y + 4.0)], w);
                p.line_segment([pos2(c.x - 1.5, c.y + 4.0), pos2(c.x + 5.5, c.y - 4.0)], w);
            }
            PState::Warn => {
                p.line_segment([pos2(c.x, c.y - 6.0), pos2(c.x, c.y + 1.5)], w);
                p.circle_filled(pos2(c.x, c.y + 5.0), 1.4, Color32::WHITE);
            }
            PState::Missing => {
                p.line_segment([pos2(c.x - 4.5, c.y - 4.5), pos2(c.x + 4.5, c.y + 4.5)], w);
                p.line_segment([pos2(c.x - 4.5, c.y + 4.5), pos2(c.x + 4.5, c.y - 4.5)], w);
            }
        }
        ui.allocate_ui_with_layout(vec2(left_w, 34.0), Layout::top_down(Align::LEFT), |ui| {
            ui.set_width(left_w);
            ui.label(RichText::new(title).strong().color(TEXT));
            ui.label(muted(desc));
        });
        let (fill, color) = if state == PState::Ok { (BUTTON, GREEN) } else { (Color32::WHITE, Color32::from_gray(14)) };
        clicked = button(ui, label, fill, color, vec2(button_w, 34.0), enabled).clicked();
    });
    clicked
}

impl App {
    fn set_lang(&self, lang: Lang) {
        self.change(|s| s.lang = lang.code().to_string());
        i18n::set(lang);
        self.sh.refresh();
    }

    /// Los idiomas, como botones; el elegido queda marcado.
    fn lang_picker(&self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            for l in Lang::ALL {
                if ui.selectable_label(i18n::current() == l, l.name()).clicked() && i18n::current() != l {
                    self.set_lang(l);
                }
            }
        });
    }

    fn change(&self, f: impl FnOnce(&mut Settings)) {
        let mut c = lock(&self.sh.cfg);
        f(&mut c.settings);
        c.save();
    }

    fn pick_folder(&self) {
        let sh = self.sh.clone();
        std::thread::spawn(move || {
            if let Some(dir) = rfd::FileDialog::new().set_title(tr("Elegí la carpeta del juego")).pick_folder() {
                let mut c = lock(&sh.cfg);
                c.game_dir = dir.to_string_lossy().into_owned();
                c.save();
            }
            sh.ctx.request_repaint();
        });
    }

    fn game_found(cfg: &Config) -> bool {
        !cfg.game_dir.is_empty() && repo::find_game_exe(Path::new(&cfg.game_dir)).is_some()
    }

    // ---------- Barra lateral ----------

    fn side_item(&self, ui: &mut Ui, i: usize, name: &str, key: &str) -> bool {
        let selected = self.tab == Tab::Play && self.sel == i;
        let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 32.0), Sense::click());
        let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
        let p = ui.painter();
        if selected || resp.hovered() {
            p.rect_filled(r, 4.0, SIDE_HOVER);
        }
        if selected {
            p.rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 1.0, YELLOW);
        }
        let ic = Rect::from_min_size(pos2(r.left() + 8.0, r.center().y - 11.0), vec2(22.0, 22.0));
        repo_icon(p, ic, name, i);
        let color = if selected { Color32::WHITE } else { Color32::from_gray(200) };
        p.text(pos2(ic.right() + 10.0, r.center().y), Align2::LEFT_CENTER, clip(name, 18), FontId::proportional(13.0), color);
        let _ = key;
        resp.clicked()
    }

    fn sidebar(&mut self, root: &mut Ui, cfg: &Config, status: &HashMap<String, Status>) {
        let prereqs = lock(&self.sh.prereqs).clone();
        egui::Panel::left("side")
            .exact_size(212.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::new().fill(SIDE).inner_margin(Margin::same(16)))
            .show(root, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                // Logo propio del lanzador.
                let (r, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                self.art.paint_app_logo(ui.painter(), r);
                ui.add_space(30.0);

                ui.label(RichText::new(tr("Disponibles")).size(14.0).color(Color32::WHITE).strong());
                ui.add_space(4.0);
                for (i, repo) in cfg.repos.iter().enumerate() {
                    if self.side_item(ui, i, &repo.name, "disp") {
                        self.sel = i;
                        self.tab = Tab::Play;
                    }
                }
                // Agregar un emulador propio.
                let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
                let on = self.tab == Tab::Add;
                let p = ui.painter();
                if on || resp.hovered() {
                    p.rect_filled(r, 4.0, SIDE_HOVER);
                }
                if on {
                    p.rect_filled(Rect::from_min_size(r.min, vec2(3.0, r.height())), 1.0, YELLOW);
                }
                let ic = Rect::from_min_size(pos2(r.left() + 8.0, r.center().y - 11.0), vec2(22.0, 22.0));
                p.rect_stroke(ic, 4.0, Stroke::new(1.3, Color32::from_gray(140)), StrokeKind::Inside);
                p.text(ic.center(), Align2::CENTER_CENTER, "+", FontId::proportional(17.0), Color32::from_gray(220));
                let color = if on { Color32::WHITE } else { Color32::from_gray(170) };
                p.text(pos2(ic.right() + 10.0, r.center().y), Align2::LEFT_CENTER, tr("Agregar emulador"), FontId::proportional(13.0), color);
                if resp.clicked() {
                    self.tab = Tab::Add;
                }
                ui.add_space(18.0);
                ui.label(RichText::new(tr("Mi biblioteca")).size(14.0).color(Color32::WHITE).strong());
                ui.add_space(4.0);
                let mut any = false;
                for (i, repo) in cfg.repos.iter().enumerate() {
                    let st = status.get(&repo.id).cloned().unwrap_or_default();
                    if st.cloned && st.built {
                        any = true;
                        if self.side_item(ui, i, &repo.name, "lib") {
                            self.sel = i;
                            self.tab = Tab::Play;
                        }
                    }
                }
                if !any {
                    ui.label(RichText::new(tr("Nada instalado todavía")).small().color(DIM));
                }

                // Abajo: el estado de los requisitos.
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    let (text, color) = if prereqs.is_empty() {
                        (tr("Revisando requisitos…"), MUTED)
                    } else if prereqs.iter().all(|p| p.state == PState::Ok) && App::game_found(cfg) {
                        (tr("Todo listo para jugar"), GREEN)
                    } else {
                        (tr("Faltan requisitos"), ORANGE)
                    };
                    let (r, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 30.0), Sense::click());
                    let resp = resp.on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tr("Abrir la pestaña Instalar"));
                    let p = ui.painter();
                    if resp.hovered() {
                        p.rect_filled(r, 4.0, SIDE_HOVER);
                    }
                    p.circle_filled(pos2(r.left() + 12.0, r.center().y), 4.5, color);
                    p.text(pos2(r.left() + 24.0, r.center().y), Align2::LEFT_CENTER, text, FontId::proportional(12.0), Color32::from_gray(210));
                    if resp.clicked() {
                        self.tab = Tab::Install;
                    }
                });
            });
    }

    // ---------- Pestañas de arriba ----------

    fn nav(&mut self, ui: &mut Ui, area: Rect) {
        let y = area.top() + 30.0;
        // El círculo de la derecha abre la carpeta de datos.
        let c = pos2(area.right() - 38.0, y);
        let cr = Rect::from_center_size(c, vec2(32.0, 32.0));
        let resp = ui.interact(cr, ui.scope_id().with("perfil"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tr("Carpeta de datos"));
        let p = ui.painter();
        p.circle_filled(c, 16.0, if resp.hovered() { Color32::from_gray(235) } else { Color32::from_gray(205) });
        p.circle_filled(c + vec2(0.0, -4.0), 5.0, Color32::from_gray(120));
        p.circle_filled(c + vec2(0.0, 9.0), 9.0, Color32::from_gray(120));
        if resp.clicked() {
            let _ = std::fs::create_dir_all(data_dir());
            sys::open_folder(&data_dir());
        }
        let mut x = cr.left() - 30.0;
        for (tab, label) in [(Tab::Settings, tr("AJUSTES")), (Tab::Install, tr("INSTALAR")), (Tab::Play, tr("JUEGO"))] {
            let galley = ui.painter().layout_no_wrap(label.to_string(), FontId::proportional(15.0), TEXT);
            let r = Rect::from_min_size(pos2(x - galley.size().x, y - galley.size().y / 2.0), galley.size()).expand(6.0);
            let resp = ui.interact(r, ui.scope_id().with(("nav", label)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let color = if self.tab == tab {
                Color32::WHITE
            } else if resp.hovered() {
                Color32::from_gray(205)
            } else {
                Color32::from_gray(140)
            };
            bold(ui.painter(), pos2(x, y), Align2::RIGHT_CENTER, label, 15.0, color);
            if resp.clicked() {
                self.tab = tab;
            }
            x -= galley.size().x + 44.0;
        }
    }

    // ---------- Banner (pestaña Juego) ----------

    fn hero(&mut self, ui: &mut Ui, body: Rect, cfg: &Config, status: &HashMap<String, Status>) {
        let Some(repo) = cfg.repos.get(self.sel).cloned() else {
            ui.painter().text(body.center(), Align2::CENTER_CENTER, tr("No hay emuladores configurados"), FontId::proportional(18.0), TEXT);
            return;
        };
        let st = status.get(&repo.id).cloned().unwrap_or_default();
        let sh = self.sh.clone();
        let busy = lock(&sh.job).is_some();
        let running = lock(&sh.running).contains_key(&repo.id);
        let other_running = !running && !lock(&sh.running).is_empty();
        let checking = lock(&sh.checking).contains(&repo.id);
        let exit = lock(&sh.exits).get(&repo.id).cloned();
        let installed = st.cloned && st.built;
        let updates = st.behind > 0 || st.needs_build;
        let (a0, a1) = accent(self.sel);
        let p = ui.painter().clone();

        let x = body.left() + 46.0;
        let mut y;
        if let Some(logo) = &self.art.logo {
            // El logo del juego arriba del botón, y debajo con qué emulador se juega.
            y = body.top() + body.height() * 0.04;
            let h = (body.height() * 0.36).clamp(120.0, 250.0);
            let r = Rect::from_min_size(pos2(x - 4.0, y), vec2(h * logo.aspect_ratio(), h));
            p.image(logo.id(), r, FULL, Color32::WHITE);
            y = r.bottom() + 12.0;
            let t = bold(&p, pos2(x, y), Align2::LEFT_TOP, &trf!("con {}", repo.name), 20.0, Color32::WHITE);
            p.rect_filled(Rect::from_min_size(pos2(x, t.bottom() + 4.0), vec2(t.width(), 3.0)), 1.5, a0);
            y = t.bottom() + 12.0;
        } else {
            // Sin imagen: el nombre del emulador con sombras de color, como un logo.
            y = body.top() + body.height() * 0.15;
            let size = if repo.name.chars().count() > 10 { 42.0 } else { 56.0 };
            p.text(pos2(x + 5.0, y + 5.0), Align2::LEFT_TOP, &repo.name, FontId::proportional(size), a1);
            p.text(pos2(x + 2.5, y + 2.5), Align2::LEFT_TOP, &repo.name, FontId::proportional(size), a0);
            y = bold(&p, pos2(x, y), Align2::LEFT_TOP, &repo.name, size, Color32::WHITE).bottom() + 8.0;
        }
        p.text(pos2(x, y), Align2::LEFT_TOP, clip(&repo_note(&repo), 52), FontId::proportional(14.0), Color32::from_white_alpha(215));
        y += 30.0;

        // Botón principal.
        let main = Rect::from_min_size(pos2(x, y), vec2(230.0, 52.0));
        let (label, enabled) = if !installed {
            (if st.cloned { tr("COMPILAR E INSTALAR") } else { tr("INSTALAR") }, !busy)
        } else if running {
            (tr("DETENER"), true)
        } else {
            (tr("JUGAR"), !busy && !other_running)
        };
        if big_button(ui, main, label, enabled).clicked() {
            if !installed {
                sh.install_repo(&repo.id);
            } else if running {
                sh.stop(&repo.id);
            } else {
                sh.launch(&repo.id);
            }
        }
        // Botón secundario a la derecha.
        let side = Rect::from_min_size(pos2(main.right() + 14.0, main.center().y - 19.0), vec2(200.0, 38.0));
        // El botón crece con el texto: en otros idiomas puede ser más largo.
        let fit = |text: &str| {
            let w = p.layout_no_wrap(text.to_string(), FontId::proportional(12.0), Color32::WHITE).size().x;
            Rect::from_min_size(side.min, vec2((w + 48.0).max(side.width()), side.height()))
        };
        if running {
            bold(&p, pos2(side.left(), side.center().y), Align2::LEFT_CENTER, tr("EN EJECUCIÓN"), 13.0, GREEN);
        } else if installed {
            if updates {
                if pill(ui, fit(tr("ACTUALIZAR")), tr("ACTUALIZAR"), true, !busy).clicked() {
                    sh.install_repo(&repo.id);
                }
            } else {
                let label = if checking { tr("BUSCANDO…") } else { tr("BUSCAR ACTUALIZACIÓN") };
                if pill(ui, fit(label), label, false, !busy && !checking).clicked() {
                    sh.check_updates(&repo.id);
                }
            }
        }
        y = main.bottom() + 14.0;
        if link(ui, pos2(x + 26.0, y), tr("¿Problemas al instalar?  ›"), 13.0) {
            self.tab = Tab::Install;
        }
        y += 34.0;
        // Si ya tenía este emulador en otra carpeta, se puede usar esa en vez de descargarlo.
        let found = lock(&sh.found).get(&repo.id).and_then(|d| d.first().cloned());
        if !installed && let Some(dir) = found {
            p.text(pos2(x, y), Align2::LEFT_TOP, trf!("Ya tenés una copia en {}", clip(&dir.to_string_lossy(), 44)), FontId::proportional(13.0), Color32::WHITE);
            y += 24.0;
            if pill(ui, Rect::from_min_size(pos2(x, y), vec2((p.layout_no_wrap(tr("USAR ESA COPIA").to_string(), FontId::proportional(12.0), Color32::WHITE).size().x + 48.0).max(170.0), 32.0)), tr("USAR ESA COPIA"), true, !busy).clicked() {
                sh.set_repo_path(&repo.id, Some(dir));
            }
            y += 46.0;
        }

        // Estado y novedades.
        let (line, color) = status_line(&st);
        p.text(pos2(x, y), Align2::LEFT_TOP, clip(&line, 56), FontId::proportional(13.0), color);
        y += 20.0;
        for c in st.new_commits.iter().take(3) {
            p.text(pos2(x, y), Align2::LEFT_TOP, format!("•  {}", clip(c, 52)), FontId::proportional(12.0), MUTED);
            y += 17.0;
        }
        if st.new_commits.len() > 3 {
            p.text(pos2(x, y), Align2::LEFT_TOP, trf!("… y {} más", st.new_commits.len() - 3), FontId::proportional(12.0), MUTED);
            y += 17.0;
        }
        if let Some(e) = exit {
            p.text(pos2(x, y), Align2::LEFT_TOP, clip(&e, 60), FontId::proportional(12.0), Color32::from_gray(200));
            y += 17.0;
        }
        if repo.kind == Kind::Victory {
            let text = if repo.own_env {
                let console = if repo.console.as_deref() == Some("series-x") { "Series X" } else { "Series S" };
                trf!("Ajustes propios: {} a media resolución, para NVIDIA", console)
            } else if cfg.settings.safe_mode {
                tr("Modo seguro activado (se cambia en Ajustes)").to_string()
            } else {
                let console = if cfg.settings.console == "series-x" { "Series X" } else { "Series S" };
                trf!("Consola: {} (se cambia en Ajustes)", console)
            };
            p.text(pos2(x, y), Align2::LEFT_TOP, text, FontId::proportional(12.0), Color32::from_gray(185));
        }

        self.cards(ui, body, cfg, status, x + 480.0);
    }

    /// El carrusel de la derecha: una tarjeta por emulador.
    fn cards(&mut self, ui: &mut Ui, body: Rect, cfg: &Config, status: &HashMap<String, Status>, x0: f32) {
        let n = cfg.repos.len();
        if n == 0 {
            return;
        }
        let (w, img_h, cap_h, gap) = (196.0, 150.0, 70.0, 12.0);
        let top = body.top() + body.height() * 0.15 + 6.0;
        let mut x = x0.max(body.left() + body.width() * 0.44);
        let mut overflow = false;
        for k in 0..n {
            let i = (self.card_start + k) % n;
            if x > body.right() {
                overflow = true;
                break;
            }
            let repo = &cfg.repos[i];
            let st = status.get(&repo.id).cloned().unwrap_or_default();
            let img = Rect::from_min_size(pos2(x, top), vec2(w, img_h));
            let cap = Rect::from_min_size(pos2(x, img.bottom()), vec2(w, cap_h));
            let all = img.union(cap);
            let resp = ui.interact(all, ui.scope_id().with(("card", i)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let p = ui.painter();
            let (a0, a1) = accent(i);
            if let Some((tex, uv)) = self.art.thumb_uv(i, img.aspect_ratio()) {
                // La miniatura, encuadrada en los personajes de cada imagen.
                paint_cover(p, tex, img, vec2(0.5, 0.5), uv);
            } else {
                vgrad(p, img, a0, a1);
                bold(p, pos2(img.left() + 14.0, img.top() + 12.0), Align2::LEFT_TOP, &initials(&repo.name), 34.0, Color32::from_white_alpha(225));
            }
            vgrad(p, Rect::from_min_max(pos2(img.left(), img.bottom() - 44.0), img.max), Color32::TRANSPARENT, Color32::from_black_alpha(170));
            p.rect_filled(Rect::from_min_size(img.min, vec2(img.width(), 3.0)), 0.0, a0);
            let installed = st.cloned && st.built;
            let (badge, bcol) = if !installed {
                (tr("NO INSTALADO"), Color32::from_gray(60))
            } else if st.behind > 0 || st.needs_build {
                (tr("NUEVA VERSIÓN"), ORANGE)
            } else {
                (tr("INSTALADO"), GREEN)
            };
            let g = p.layout_no_wrap(badge.to_string(), FontId::proportional(10.0), Color32::WHITE);
            let br = Rect::from_min_size(pos2(img.left() + 12.0, img.bottom() - 28.0), g.size() + vec2(14.0, 8.0));
            p.rect_filled(br, 3.0, bcol);
            bold(p, br.center(), Align2::CENTER_CENTER, badge, 10.0, Color32::WHITE);
            p.rect_filled(cap, 0.0, Color32::from_rgb(8, 8, 10));
            bold(p, pos2(cap.left() + 12.0, cap.top() + 12.0), Align2::LEFT_TOP, &clip(&repo.name, 22), 13.0, Color32::WHITE);
            let (line, _) = status_line(&st);
            p.text(pos2(cap.left() + 12.0, cap.top() + 34.0), Align2::LEFT_TOP, clip(&line, 28), FontId::proportional(11.5), MUTED);
            if self.sel == i || resp.hovered() {
                let c = if self.sel == i { Color32::WHITE } else { Color32::from_gray(150) };
                p.rect_stroke(all, 0.0, Stroke::new(2.0, c), StrokeKind::Outside);
            }
            if resp.clicked() {
                self.sel = i;
            }
            x += w + gap;
        }
        if overflow || x - gap > body.right() {
            let c = pos2(body.right() - 24.0, top + img_h / 2.0);
            let r = Rect::from_center_size(c, vec2(32.0, 32.0));
            let resp = ui.interact(r, ui.scope_id().with("siguiente"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            let p = ui.painter();
            p.circle_filled(c, 16.0, Color32::from_black_alpha(if resp.hovered() { 220 } else { 150 }));
            bold(p, c + vec2(1.0, -1.0), Align2::CENTER_CENTER, "›", 24.0, Color32::WHITE);
            if resp.clicked() {
                self.card_start = (self.card_start + 1) % n;
            }
        }
    }

    // ---------- Pestañas Instalar y Ajustes, sobre el banner ----------

    fn overlay(&mut self, ui: &mut Ui, body: Rect, title: &str, f: impl FnOnce(&mut Self, &mut Ui)) {
        let r = Rect::from_min_max(pos2(body.left() + 36.0, body.top() + 4.0), pos2(body.right() - 36.0, body.bottom() - 22.0));
        ui.painter().rect_filled(r, 10.0, Color32::from_black_alpha(215));
        let inner = r.shrink(22.0);
        ui.scope_builder(UiBuilder::new().max_rect(inner), |ui| {
            bold(ui.painter(), inner.min, Align2::LEFT_TOP, title, 24.0, Color32::WHITE);
            ui.add_space(44.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                ui.set_width(ui.available_width() - 8.0);
                f(self, ui);
            });
        });
    }

    fn folder_row(&mut self, ui: &mut Ui, cfg: &Config) {
        let found = App::game_found(cfg);
        card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(muted(tr("CARPETA DEL JUEGO")));
                dot(ui, if found { GREEN } else { RED });
                let (text, color) = if found { (tr("Juego encontrado"), GREEN) } else { (tr("No se encontró el juego"), RED) };
                ui.label(RichText::new(text).small().color(color));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if small_button(ui, tr("Cambiar carpeta…")) {
                        self.pick_folder();
                    }
                    if small_button(ui, if self.show_path { tr("Ocultar") } else { tr("Mostrar") }) {
                        self.show_path = !self.show_path;
                    }
                });
            });
            if self.show_path {
                ui.label(muted(if cfg.game_dir.is_empty() { tr("(sin elegir)").to_string() } else { cfg.game_dir.clone() }));
            }
        });
    }

    fn settings(&mut self, ui: &mut Ui) {
        let cfg = self.sh.cfg();
        card(ui, |ui| {
            ui.label(RichText::new(tr("Idioma")).strong().color(TEXT));
            self.lang_picker(ui);
        });
        self.folder_row(ui, &cfg);
        card(ui, |ui| {
            ui.label(RichText::new(tr("Opciones de juego")).strong().color(TEXT));
            ui.horizontal(|ui| {
                ui.label(muted(tr("Consola que ve el juego:")));
                for (value, label) in [("series-s", tr("Series S (menos memoria)")), ("series-x", "Series X")] {
                    if ui.selectable_label(cfg.settings.console == value, label).clicked() {
                        self.change(|s| s.console = value.to_string());
                    }
                }
            });
            let mut enter = cfg.settings.auto_enter;
            if ui.checkbox(&mut enter, RichText::new(tr("Pulsar Enter solo para pasar las pantallas iniciales")).small().color(TEXT)).changed() {
                self.change(|s| s.auto_enter = enter);
            }
            let mut safe = cfg.settings.safe_mode;
            if ui.checkbox(&mut safe, RichText::new(tr("Modo seguro · más lento, gasta menos memoria")).small().color(ORANGE)).changed() {
                self.change(|s| s.safe_mode = safe);
            }
            ui.label(muted(
                tr("La consola y el modo seguro valen para VIctory y su fork. VIctory NVIDIA usa siempre sus ajustes propios; xcl no los usa."),
            ));
        });
        card(ui, |ui| {
            ui.label(RichText::new(tr("Dónde está cada emulador")).strong().color(TEXT));
            ui.label(muted(
                tr("Por defecto el lanzador descarga su propia copia. Si ya tenés uno, elegí su carpeta: al actualizar se unen los cambios nuevos con los tuyos."),
            ));
            for repo in &cfg.repos {
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&repo.name).color(TEXT));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if repo.custom && small_button(ui, tr("Quitar")) {
                            self.sh.remove_custom(&repo.id);
                        }
                        if !repo.owned() && small_button(ui, tr("Usar copia del lanzador")) {
                            self.sh.set_repo_path(&repo.id, None);
                        }
                        if small_button(ui, tr("Elegir carpeta…")) {
                            let (sh, id) = (self.sh.clone(), repo.id.clone());
                            std::thread::spawn(move || {
                                if let Some(dir) = rfd::FileDialog::new().set_title(tr("Elegí la carpeta del emulador")).pick_folder() {
                                    sh.set_repo_path(&id, Some(dir));
                                }
                            });
                        }
                    });
                });
                ui.label(muted(&repo.path));
            }
        });
        card(ui, |ui| {
            ui.label(RichText::new(tr("Registros y archivos")).strong().color(TEXT));
            self.shortcuts(ui, &cfg);
        });
    }

    /// El formulario para agregar un emulador propio, y la lista de los agregados.
    fn add_form(&mut self, ui: &mut Ui) {
        let busy = lock(&self.sh.job).is_some();
        card(ui, |ui| {
            ui.label(RichText::new(tr("Un fork de VIctory o de xcl")).strong().color(TEXT));
            ui.label(muted(
                tr("El lanzador lo descarga, lo compila y lo ejecuta igual que el original en el que se basa, así que solo sirve para forks de esos dos proyectos. Antes de agregarlo se comprueba que la dirección exista."),
            ));
            ui.add_space(6.0);
            let w = ui.available_width();
            ui.label(muted(tr("Nombre")));
            ui.add(egui::TextEdit::singleline(&mut self.add_name).hint_text(tr("Por ejemplo: VIctory AMD")).desired_width(w));
            ui.label(muted(tr("Dirección de GitHub")));
            ui.add(egui::TextEdit::singleline(&mut self.add_url).hint_text("https://github.com/dueño/repositorio").desired_width(w));
            ui.label(muted(tr("Basado en")));
            ui.horizontal(|ui| {
                for (k, label) in [(Kind::Victory, "VIctory"), (Kind::Xcl, "xcl")] {
                    if ui.selectable_label(self.add_kind == k, label).clicked() {
                        self.add_kind = k;
                    }
                }
            });
            ui.label(muted(tr("Rama (opcional)")));
            ui.add(egui::TextEdit::singleline(&mut self.add_branch).hint_text(tr("Vacía: la principal del repositorio")).desired_width(w));
            ui.add_space(8.0);
            let ready = !self.add_name.trim().is_empty() && !self.add_url.trim().is_empty();
            let label = if busy { tr("Comprobando…") } else { tr("Agregar") };
            if button(ui, label, Color32::WHITE, Color32::from_gray(14), vec2(160.0, 36.0), ready && !busy).clicked() {
                self.sh.add_custom(self.add_name.clone(), self.add_url.clone(), self.add_kind, self.add_branch.clone());
            }
        });
        let cfg = self.sh.cfg();
        let custom: Vec<_> = cfg.repos.iter().filter(|r| r.custom).cloned().collect();
        if !custom.is_empty() {
            card(ui, |ui| {
                ui.label(RichText::new(tr("Agregados por vos")).strong().color(TEXT));
                for r in &custom {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&r.name).color(TEXT));
                        ui.label(muted(trf!("{} · rama {}", r.url, r.branch)));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if small_button(ui, tr("Quitar")) {
                                self.sh.remove_custom(&r.id);
                            }
                        });
                    });
                }
                ui.label(muted(tr("Quitar no borra sus archivos: siguen en la carpeta del lanzador (repos).")));
            });
        }
    }

    fn shortcuts(&mut self, ui: &mut Ui, cfg: &Config) {
        ui.horizontal_wrapped(|ui| {
            for repo in &cfg.repos {
                if small_button(ui, &trf!("Registro {}", repo.name)) {
                    let f = repo::log_file(repo);
                    if f.exists() {
                        sys::open_text(&f);
                    } else {
                        self.sh.say(trf!("{} todavía no se ejecutó: no hay registro.", repo.name), true);
                    }
                }
            }
            if small_button(ui, tr("Errores VIctory")) {
                match repo::newest_victory_crash_log() {
                    Some(f) => sys::open_text(&f),
                    None => self.sh.say(tr("VIctory todavía no registró errores."), false),
                }
            }
            if small_button(ui, tr("Carpeta")) {
                let _ = std::fs::create_dir_all(data_dir());
                sys::open_folder(&data_dir());
            }
            if small_button(ui, tr("Configuración")) {
                sys::open_text(&Config::file());
            }
            if small_button(ui, tr("Configuración inicial")) {
                let mut c = lock(&self.sh.cfg);
                c.setup_done = false;
                c.save();
            }
            if small_button(ui, tr("Imágenes")) {
                let _ = std::fs::create_dir_all(images_dir());
                sys::open_folder(&images_dir());
            }
        });
    }

    fn install(&mut self, ui: &mut Ui) {
        let cfg = self.sh.cfg();
        let status = lock(&self.sh.status).clone();
        let prereqs = lock(&self.sh.prereqs).clone();
        let busy = lock(&self.sh.job).is_some();
        let mut all_ok = true;
        let sh = self.sh.clone();
        card(ui, |ui| {
            for p in &prereqs {
                all_ok &= p.state == PState::Ok;
                let label = match p.state {
                    PState::Ok => tr("Listo"),
                    PState::Warn => tr("Arreglar"),
                    PState::Missing => tr("Instalar"),
                };
                if row(ui, p.state, p.name, &p.desc, label, !busy && p.state != PState::Ok) {
                    sh.install_prereq(p.id);
                }
                ui.separator();
            }
            for repo in &cfg.repos {
                let st = status.get(&repo.id).cloned().unwrap_or_default();
                let installed = st.cloned && st.built;
                let (state, label, desc) = if !installed {
                    (PState::Missing, tr("Instalar"), tr("Todavía no está instalado").to_string())
                } else if st.behind > 0 || st.needs_build {
                    (PState::Warn, tr("Actualizar"), trf!("Hay cambios nuevos · versión de {}", st.head_ts.map(repo::when).unwrap_or_default()))
                } else {
                    (PState::Ok, tr("Listo"), trf!("Versión de {}", st.head_ts.map(repo::when).unwrap_or_default()))
                };
                all_ok &= state == PState::Ok;
                if row(ui, state, &repo.name, &desc, label, !busy && state != PState::Ok) {
                    sh.install_repo(&repo.id);
                }
                for dir in lock(&sh.found).get(&repo.id).cloned().unwrap_or_default() {
                    ui.horizontal(|ui| {
                        ui.add_space(36.0);
                        ui.label(muted(trf!("Encontré otra copia en {}", dir.display())));
                        if small_button(ui, tr("Usar esta")) {
                            sh.set_repo_path(&repo.id, Some(dir.clone()));
                        }
                    });
                }
                ui.separator();
            }
            let found = App::game_found(&cfg);
            all_ok &= found;
            let desc = if cfg.game_dir.is_empty() { tr("Sin elegir").to_string() } else { cfg.game_dir.clone() };
            if row(ui, if found { PState::Ok } else { PState::Missing }, tr("Carpeta del juego"), &desc, tr("Cambiar"), true) {
                self.pick_folder();
            }
        });
        let (text, color) = if all_ok { (tr("Todo instalado"), GREEN) } else { (tr("Faltan cosas por instalar o actualizar"), ORANGE) };
        Frame::new().stroke(Stroke::new(1.0, color)).corner_radius(8).inner_margin(Margin::same(12)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| ui.label(RichText::new(text).strong().color(color)));
        });
    }

    // ---------- Barra de tareas de abajo ----------

    fn bottom_bar(&mut self, root: &mut Ui) {
        let job = lock(&self.sh.job).as_ref().map(|j| (j.title.clone(), j.cancel.clone()));
        let (msg, is_err) = lock(&self.sh.message).clone();
        let last = lock(&self.sh.log).last().cloned().unwrap_or_default();
        egui::Panel::bottom("bar").resizable(false).show_separator_line(false).frame(Frame::new().fill(BAR_DARK)).show(root, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 64.0), Sense::hover());
            let right_w = 290.0_f32.min(rect.width() * 0.4);
            let left = Rect::from_min_max(rect.min, pos2(rect.right() - right_w, rect.bottom()));
            let p = ui.painter().clone();
            p.rect_filled(left, 0.0, BAR);
            let (l1, l2, c2) = match &job {
                Some((title, _)) => (title.to_uppercase(), clip(&last, 90), Color32::from_gray(225)),
                None if !msg.is_empty() => {
                    ((if is_err { tr("AVISO") } else { tr("LISTO") }).to_string(), clip(&msg, 110), if is_err { Color32::from_rgb(255, 170, 170) } else { Color32::from_gray(225) })
                }
                None => (tr("SIN TAREAS").to_string(), tr("Elegí un emulador a la izquierda y pulsá Jugar").to_string(), Color32::from_gray(200)),
            };
            bold(&p, pos2(left.left() + 22.0, left.top() + 22.0), Align2::LEFT_CENTER, &l1, 13.0, Color32::WHITE);
            p.text(pos2(left.left() + 22.0, left.top() + 43.0), Align2::LEFT_CENTER, l2, FontId::proportional(12.0), c2);
            if job.is_some() {
                ui.put(Rect::from_center_size(pos2(left.right() - 32.0, left.center().y), vec2(22.0, 22.0)), egui::Spinner::new().color(Color32::WHITE));
            }
            let (bw, bh, gap) = (112.0, 38.0, 12.0);
            let b2 = Rect::from_min_size(pos2(rect.right() - 22.0 - bw, rect.center().y - bh / 2.0), vec2(bw, bh));
            let b1 = b2.translate(vec2(-(bw + gap), 0.0));
            if pill(ui, b1, if self.show_log { tr("OCULTAR") } else { tr("REGISTRO") }, true, true).clicked() {
                self.show_log = !self.show_log;
            }
            match &job {
                Some((_, cancel)) => {
                    if pill(ui, b2, tr("CANCELAR"), false, true).clicked() {
                        cancel.store(true, Ordering::Relaxed);
                    }
                }
                None => {
                    if pill(ui, b2, tr("REVISAR"), false, true).on_hover_text(tr("Volver a revisar requisitos y actualizaciones")).clicked() {
                        self.sh.refresh();
                        for r in &self.sh.cfg().repos {
                            self.sh.check_updates(&r.id);
                        }
                    }
                }
            }
            if self.show_log {
                Frame::new().fill(Color32::from_rgb(12, 12, 14)).inner_margin(Margin::symmetric(16, 8)).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let lines = lock(&self.sh.log).clone();
                    egui::ScrollArea::vertical().max_height(180.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                        for l in lines.iter().rev().take(300).rev() {
                            ui.label(RichText::new(l).monospace().small().color(MUTED));
                        }
                    });
                });
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, root: &mut Ui, _frame: &mut eframe::Frame) {
        self.sh.poll_running();
        let busy = lock(&self.sh.job).is_some() || !lock(&self.sh.running).is_empty() || !lock(&self.sh.checking).is_empty();
        if busy {
            root.ctx().request_repaint_after(Duration::from_millis(300));
        }
        let cfg = self.sh.cfg();
        let status = lock(&self.sh.status).clone();
        // Recién agregado: se lo muestra en el banner, listo para instalar.
        if let Some(id) = lock(&self.sh.added).take()
            && let Some(i) = cfg.repos.iter().position(|r| r.id == id)
        {
            self.sel = i;
            self.tab = Tab::Play;
            self.add_name.clear();
            self.add_url.clear();
            self.add_branch.clear();
        }
        if self.sel >= cfg.repos.len() {
            self.sel = 0;
        }
        if !cfg.setup_done {
            self.setup_screen(root, &cfg);
            return;
        }
        self.bottom_bar(root);
        self.sidebar(root, &cfg, &status);
        egui::CentralPanel::default().frame(Frame::new().fill(Color32::BLACK)).show(root, |ui| {
            let area = ui.max_rect();
            match self.art.bg(self.sel) {
                // Sin el borde derecho ni el de abajo, donde está el sello de la esquina.
                Some(tex) => paint_cover(ui.painter(), tex, area, vec2(0.5, 0.0), Rect::from_min_max(pos2(0.0, 0.0), pos2(0.92, 0.9))),
                None => scene(ui.painter(), area),
            }
            shade(ui.painter(), area);
            self.nav(ui, area);
            let body = Rect::from_min_max(pos2(area.left(), area.top() + 64.0), area.max);
            match self.tab {
                Tab::Play => self.hero(ui, body, &cfg, &status),
                Tab::Install => self.overlay(ui, body, tr("Instalar"), |me, ui| me.install(ui)),
                Tab::Settings => self.overlay(ui, body, tr("Ajustes"), |me, ui| me.settings(ui)),
                Tab::Add => self.overlay(ui, body, tr("Agregar emulador"), |me, ui| me.add_form(ui)),
            }
        });
    }
}
