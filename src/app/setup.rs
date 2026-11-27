//! La pantalla de la primera vez: elegir la carpeta del juego y dónde está cada emulador.
//! A la izquierda un panel de tubos de neón dibujado; a la derecha el formulario.

use super::*;
use crate::i18n::tr;

const PANEL: Color32 = Color32::from_rgb(22, 22, 25);
const FIELD: Color32 = Color32::from_rgb(27, 27, 31);
const FIELD_LINE: Color32 = Color32::from_rgb(76, 76, 84);
const LINE: Color32 = Color32::from_rgb(58, 58, 64);
const ORANGE_LINK: Color32 = Color32::from_rgb(255, 170, 40);

fn rgba(c: (u8, u8, u8), a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.0, c.1, c.2, a)
}

/// Un tubo de neón: tres trazos anchos y transparentes (el brillo) y un centro casi blanco.
fn glow_line(p: &Painter, pts: Vec<Pos2>, c: (u8, u8, u8)) {
    for (w, a) in [(14.0, 16), (8.0, 40), (4.0, 210)] {
        p.add(Shape::line(pts.clone(), Stroke::new(w, rgba(c, a))));
    }
    p.add(Shape::line(pts, Stroke::new(1.2, Color32::from_rgba_unmultiplied(255, 246, 225, 210))));
}

fn glow_rect(p: &Painter, r: Rect, radius: f32, c: (u8, u8, u8)) {
    for (w, a) in [(14.0, 16), (8.0, 40), (4.0, 210)] {
        p.rect_stroke(r, radius, Stroke::new(w, rgba(c, a)), StrokeKind::Middle);
    }
    p.rect_stroke(r, radius, Stroke::new(1.2, Color32::from_rgba_unmultiplied(255, 246, 225, 210)), StrokeKind::Middle);
}

/// El arte de la izquierda: un laberinto de tubos de neón que se doblan en ángulo recto.
fn neon(p: &Painter, r: Rect) {
    let p = p.with_clip_rect(r);
    vgrad(&p, r, Color32::from_rgb(74, 12, 56), Color32::from_rgb(16, 6, 22));
    hgrad(&p, r, Color32::from_rgba_unmultiplied(255, 40, 120, 50), Color32::TRANSPARENT);
    let colors = [(255, 168, 36), (255, 214, 92), (255, 64, 150), (255, 98, 58)];
    // Anillos redondeados uno dentro de otro.
    for (ci, (cx, cy, rings)) in [(0.38, 0.28, 7), (0.8, 0.7, 6), (0.08, 0.86, 5)].into_iter().enumerate() {
        let c = pos2(r.left() + r.width() * cx, r.top() + r.height() * cy);
        for k in 0..rings {
            let s = 26.0 + k as f32 * 22.0;
            glow_rect(&p, Rect::from_center_size(c, vec2(s * 1.5, s * 2.0)), 10.0 + k as f32 * 3.0, colors[(ci + k) % colors.len()]);
        }
    }
    // Tubos que van y vienen, alternando horizontal y vertical.
    for i in 0..16u32 {
        let mut pt = pos2(r.left() + noise(i * 7) * r.width(), r.top() + noise(i * 7 + 1) * r.height());
        let mut pts = vec![pt];
        for s in 0..10u32 {
            let len = (22.0 + noise(i * 31 + s) * 95.0) * if noise(i * 13 + s * 3) < 0.5 { -1.0 } else { 1.0 };
            pt += if s % 2 == 0 { vec2(len, 0.0) } else { vec2(0.0, len) };
            pts.push(pt);
        }
        glow_line(&p, pts, colors[i as usize % colors.len()]);
    }
    // Un poco de sombra hacia el formulario.
    hgrad(&p, Rect::from_min_max(pos2(r.right() - 40.0, r.top()), r.max), Color32::TRANSPARENT, Color32::from_black_alpha(120));
}

/// Recorta un camino por la izquierda, que es lo que menos importa: `…\VIctory`.
fn clip_left(s: &str, n: usize) -> String {
    let count = s.chars().count();
    if count <= n { s.to_string() } else { format!("…{}", s.chars().skip(count - (n - 1)).collect::<String>()) }
}

fn folder_icon(p: &Painter, c: Pos2, col: Color32) {
    let body = Rect::from_center_size(c + vec2(0.0, 1.5), vec2(20.0, 14.0));
    p.rect_filled(Rect::from_min_size(body.min - vec2(0.0, 3.0), vec2(9.0, 4.0)), 1.5, col);
    p.rect_stroke(body, 2.0, Stroke::new(1.6, col), StrokeKind::Inside);
}

/// Un campo como los de un formulario de inicio de sesión, pero para una carpeta:
/// al pulsarlo se elige la carpeta. Devuelve (elegir, volver a la de fábrica).
fn field(ui: &mut Ui, label: &str, value: Option<&str>, placeholder: &str, can_reset: bool) -> (bool, bool) {
    let w = ui.available_width();
    let (r, resp) = ui.allocate_exact_size(vec2(w, 46.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let mut reset = false;
    if can_reset {
        let rr = Rect::from_center_size(pos2(r.right() - 56.0, r.center().y), vec2(24.0, 24.0));
        let rresp = ui.interact(rr, ui.scope_id().with(("reset", label)), Sense::click()).on_hover_text(tr("Volver a la copia del lanzador"));
        reset = rresp.clicked();
        let col = if rresp.hovered() { Color32::WHITE } else { Color32::from_gray(150) };
        ui.painter().text(rr.center(), Align2::CENTER_CENTER, "×", FontId::proportional(20.0), col);
    }
    let p = ui.painter();
    p.rect_filled(r, 4.0, FIELD);
    let line = if resp.hovered() { Color32::from_gray(150) } else { FIELD_LINE };
    p.rect_stroke(r, 4.0, Stroke::new(1.0, line), StrokeKind::Inside);
    let mut tx = r.left() + 16.0;
    if !label.is_empty() {
        p.text(pos2(tx, r.center().y), Align2::LEFT_CENTER, clip(label, 15), FontId::proportional(15.0), Color32::from_gray(200));
        tx += 138.0;
    }
    let room = ((r.right() - 80.0 - tx) / 7.6).max(10.0) as usize;
    match value {
        Some(v) => p.text(pos2(tx, r.center().y), Align2::LEFT_CENTER, clip_left(v, room), FontId::proportional(15.0), Color32::WHITE),
        None => p.text(pos2(tx, r.center().y), Align2::LEFT_CENTER, placeholder, FontId::proportional(15.0), Color32::from_gray(115)),
    };
    folder_icon(p, pos2(r.right() - 24.0, r.center().y), if resp.hovered() { Color32::WHITE } else { Color32::from_gray(200) });
    if can_reset {
        // Se dibujó antes del fondo del campo: se vuelve a dibujar encima.
        let rr = Rect::from_center_size(pos2(r.right() - 56.0, r.center().y), vec2(24.0, 24.0));
        p.text(rr.center(), Align2::CENTER_CENTER, "×", FontId::proportional(20.0), Color32::from_gray(170));
    }
    (resp.clicked() && !reset, reset)
}

/// Una casilla cuadrada con su texto, como las de "recordarme".
fn check(ui: &mut Ui, on: bool, text: &str) -> bool {
    let galley = ui.painter().layout_no_wrap(text.to_string(), FontId::proportional(14.0), MUTED);
    let (r, resp) = ui.allocate_exact_size(vec2(26.0 + galley.size().x, 22.0), Sense::click());
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);
    let p = ui.painter();
    let b = Rect::from_min_size(pos2(r.left(), r.center().y - 8.0), vec2(16.0, 16.0));
    p.rect_stroke(b, 2.0, Stroke::new(1.3, if resp.hovered() { Color32::WHITE } else { Color32::from_gray(150) }), StrokeKind::Inside);
    if on {
        p.rect_filled(b.shrink(3.5), 1.0, ORANGE_LINK);
    }
    p.text(pos2(b.right() + 10.0, r.center().y), Align2::LEFT_CENTER, text, FontId::proportional(14.0), Color32::from_gray(if on { 220 } else { 160 }));
    resp.clicked()
}

impl App {
    fn pick_repo_folder(&self, id: &str) {
        let (sh, id) = (self.sh.clone(), id.to_string());
        std::thread::spawn(move || {
            if let Some(dir) = rfd::FileDialog::new().set_title(tr("Elegí la carpeta del emulador")).pick_folder() {
                sh.set_repo_path(&id, Some(dir));
            }
            sh.ctx.request_repaint();
        });
    }

    fn finish_setup(&self) {
        let mut c = lock(&self.sh.cfg);
        c.setup_done = true;
        c.save();
        drop(c);
        self.sh.refresh();
    }

    pub(super) fn setup_screen(&mut self, root: &mut Ui, cfg: &Config) {
        let found = lock(&self.sh.found).clone();
        let busy = lock(&self.sh.job).is_some();
        egui::CentralPanel::default().frame(Frame::new().fill(PANEL)).show(root, |ui| {
            let area = ui.max_rect();
            let art = Rect::from_min_size(area.min, vec2((area.width() * 0.26).clamp(220.0, 330.0), area.height()));
            neon(ui.painter(), art);
            let x0 = art.right() + 36.0;
            let x1 = area.right() - 36.0;
            let p = ui.painter().clone();

            // Encabezado: logo, nombre y título grande.
            let logo = Rect::from_min_size(pos2(x0, area.top() + 42.0), vec2(44.0, 44.0));
            self.art.paint_app_logo(&p, logo);
            bold(&p, pos2(logo.right() + 20.0, logo.top() - 2.0), Align2::LEFT_TOP, "Lanzador Xbox", 13.0, Color32::WHITE);
            p.text(pos2(logo.right() + 18.0, logo.top() + 16.0), Align2::LEFT_TOP, tr("Configuración inicial"), FontId::proportional(32.0), Color32::WHITE);

            // A la derecha: el engranaje (carpeta de datos) y un atajo para saltear.
            let gear = pos2(x1 - 160.0, area.top() + 66.0);
            let gr = Rect::from_center_size(gear, vec2(42.0, 42.0));
            let gresp = ui.interact(gr, ui.scope_id().with("engranaje"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand).on_hover_text(tr("Carpeta de datos"));
            p.circle_stroke(gear, 21.0, Stroke::new(1.2, if gresp.hovered() { Color32::WHITE } else { Color32::from_gray(120) }));
            p.text(gear, Align2::CENTER_CENTER, "⚙", FontId::proportional(20.0), Color32::from_gray(200));
            if gresp.clicked() {
                let _ = std::fs::create_dir_all(data_dir());
                sys::open_folder(&data_dir());
            }
            let tag = Rect::from_min_max(pos2(gr.right() + 12.0, gr.top() + 4.0), pos2(x1, gr.bottom() - 4.0));
            p.rect_stroke(tag, 18.0, Stroke::new(1.6, YELLOW), StrokeKind::Inside);
            bold(&p, tag.center(), Align2::CENTER_CENTER, tr("Primera vez"), 15.0, YELLOW);
            let q = tr("¿Ya lo tenés todo?");
            let qg = p.layout_no_wrap(q.to_string(), FontId::proportional(13.0), Color32::WHITE);
            let ly = gr.bottom() + 12.0;
            let skip_w = p.layout_no_wrap(tr("Saltear este paso").into(), FontId::proportional(13.0), ORANGE_LINK).size().x;
            p.text(pos2(x1 - skip_w - 6.0 - qg.size().x, ly), Align2::LEFT_TOP, q, FontId::proportional(13.0), Color32::WHITE);
            let skip = Rect::from_min_size(pos2(x1 - skip_w, ly), vec2(skip_w, 16.0));
            let sresp = ui.interact(skip, ui.scope_id().with("saltear"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
            p.text(skip.min, Align2::LEFT_TOP, tr("Saltear este paso"), FontId::proportional(13.0), if sresp.hovered() { Color32::WHITE } else { ORANGE_LINK });
            if sresp.clicked() {
                self.finish_setup();
            }
            let sep = area.top() + 134.0;
            p.line_segment([pos2(x0, sep), pos2(x1, sep)], Stroke::new(1.0, LINE));

            // El formulario.
            let form = Rect::from_min_max(pos2(x0, sep + 26.0), pos2((x0 + 660.0).min(x1), area.bottom() - 112.0));
            let game_ok = App::game_found(cfg);
            ui.scope_builder(UiBuilder::new().max_rect(form), |ui| {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 10.0;
                    ui.set_width(ui.available_width() - 10.0);
                    let game = (!cfg.game_dir.is_empty()).then_some(cfg.game_dir.as_str());
                    if field(ui, "", game, tr("Carpeta del juego"), false).0 {
                        self.pick_folder();
                    }
                    let (text, color) = if game_ok {
                        (tr("Juego encontrado en esta carpeta"), GREEN)
                    } else if cfg.game_dir.is_empty() {
                        (tr("Elegí la carpeta donde está game_scarlett_master.exe"), Color32::from_gray(170))
                    } else {
                        (tr("En esta carpeta no está el juego (falta el .exe que termina en _master.exe)"), ORANGE)
                    };
                    ui.label(RichText::new(text).size(13.0).color(color));
                    ui.add_space(8.0);
                    ui.label(RichText::new(tr("Emuladores")).size(15.0).color(Color32::WHITE).strong());
                    ui.label(RichText::new(tr("Si ya tenés alguno descargado, elegí su carpeta. Si no, dejalo así: el lanzador lo descarga y lo compila solo.")).size(12.5).color(Color32::from_gray(150)));
                    for repo in &cfg.repos {
                        let custom = !repo.owned();
                        let (pick, reset) = field(ui, &repo.name, custom.then_some(repo.path.as_str()), tr("Lo descarga el lanzador"), custom);
                        if pick && !busy {
                            self.pick_repo_folder(&repo.id);
                        }
                        if reset {
                            self.sh.set_repo_path(&repo.id, None);
                        }
                        if !custom && let Some(dir) = found.get(&repo.id).and_then(|d| d.first()) {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(trf!("Encontré una copia en {}", dir.display())).size(12.5).color(Color32::from_gray(170)));
                                let l = ui.add(egui::Label::new(RichText::new(tr("Usar esa")).size(12.5).color(ORANGE_LINK)).sense(Sense::click()));
                                if l.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                    self.sh.set_repo_path(&repo.id, Some(dir.clone()));
                                }
                            });
                        }
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if check(ui, cfg.settings.safe_mode, tr("Modo seguro")) {
                            self.change(|s| s.safe_mode = !s.safe_mode);
                        }
                        ui.add_space(14.0);
                        if check(ui, cfg.settings.auto_enter, tr("Pasar solo las pantallas iniciales")) {
                            self.change(|s| s.auto_enter = !s.auto_enter);
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let l = ui.add(egui::Label::new(RichText::new(tr("¿Dónde está el juego?")).size(14.0).color(ORANGE_LINK)).sense(Sense::click()));
                            if l.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                                self.setup_help = !self.setup_help;
                            }
                        });
                    });
                    if self.setup_help {
                        ui.label(
                            RichText::new(
                                tr("Es la carpeta con game_scarlett_master.exe. Si lo instalaste con la app de Xbox suele ser C:\\XboxGames\\<juego>\\Content. El lanzador busca en Escritorio, Documentos, Descargas y las carpetas Games, Juegos y XboxGames de cada disco."),
                            )
                            .size(12.5)
                            .color(Color32::from_gray(170)),
                        );
                    }
                });
            });

            // Botones abajo a la derecha.
            let (bw, bh) = (140.0, 46.0);
            // Alineados con el borde derecho del formulario.
            let go = Rect::from_min_size(pos2(form.right() - 10.0 - bw, area.bottom() - 100.0), vec2(bw, bh));
            let back = go.translate(vec2(-(bw + 12.0), 0.0));
            if pill(ui, back, tr("BUSCAR"), false, true).on_hover_text(tr("Volver a buscar el juego y los emuladores")).clicked() {
                self.sh.refresh();
            }
            let resp = ui.interact(go, ui.scope_id().with("continuar"), if game_ok { Sense::click() } else { Sense::hover() });
            let fill = if !game_ok { Color32::from_gray(235) } else if resp.hovered() { Color32::from_gray(222) } else { Color32::WHITE };
            p.rect_filled(go, bh / 2.0, fill);
            bold(&p, go.center(), Align2::CENTER_CENTER, tr("CONTINUAR"), 13.0, if game_ok { Color32::from_gray(14) } else { Color32::from_gray(160) });
            if game_ok {
                resp.on_hover_cursor(CursorIcon::PointingHand).clicked().then(|| self.finish_setup());
            } else {
                resp.on_hover_text(tr("Primero elegí la carpeta del juego"));
            }

            // Pie de página, con el idioma a la derecha.
            let fy = area.bottom() - 26.0;
            ui.scope_builder(UiBuilder::new().max_rect(Rect::from_min_max(pos2(x1 - 420.0, fy - 14.0), pos2(x1, fy + 14.0))), |ui| {
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    egui::ComboBox::from_id_salt("idioma").selected_text(i18n::current().name()).show_ui(ui, |ui| {
                        for l in Lang::ALL {
                            if ui.selectable_label(i18n::current() == l, l.name()).clicked() && i18n::current() != l {
                                self.set_lang(l);
                            }
                        }
                    });
                    ui.label(RichText::new(tr("Idioma")).size(12.0).color(Color32::from_gray(150)));
                });
            });
            let mut fx = x0;
            let small = FontId::proportional(10.5);
            let year = chrono::Local::now().format("%Y").to_string();
            let mut items: Vec<(String, Option<PathBuf>)> = vec![(format!("© {year} LANZADOR XBOX"), None)];
            items.push((tr("CARPETA DE DATOS").into(), Some(data_dir())));
            items.push((tr("IMÁGENES").into(), Some(images_dir())));
            items.push((tr("REGISTROS").into(), Some(crate::config::log_dir())));
            for (k, (text, dir)) in items.into_iter().enumerate() {
                if k > 0 {
                    p.text(pos2(fx, fy), Align2::LEFT_CENTER, "|", small.clone(), Color32::from_gray(80));
                    fx += 14.0;
                }
                let g = p.layout_no_wrap(text.clone(), small.clone(), Color32::GRAY);
                let r = Rect::from_min_size(pos2(fx, fy - g.size().y / 2.0), g.size());
                let col = match &dir {
                    Some(d) => {
                        let resp = ui.interact(r, ui.scope_id().with(("pie", k)), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
                        if resp.clicked() {
                            let _ = std::fs::create_dir_all(d);
                            sys::open_folder(d);
                        }
                        if resp.hovered() { Color32::WHITE } else { Color32::from_gray(150) }
                    }
                    None => Color32::from_gray(150),
                };
                bold(&p, pos2(fx, fy), Align2::LEFT_CENTER, &text, 10.5, col);
                fx += g.size().x + 14.0;
            }
        });
    }
}
