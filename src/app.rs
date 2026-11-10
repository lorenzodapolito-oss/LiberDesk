//! Interfaccia: elenco delle app, controllo e aggiornamento all'avvio, registro.

use crate::config::{self, Config};
use crate::net::{Http, RealHttp};
use crate::scan::{self, App};
use crate::state;
use crate::theme;
use crate::update::{self, Check, Kind, Probe, RealProbe};
use eframe::egui::{self, Color32, RichText};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;

/// Categorie disponibili (il catalogo usa le stesse), nell'ordine in cui si mostrano.
pub const CATEGORIES: [&str; 10] = [
    "Internet",
    "Ufficio",
    "Multimedia",
    "Immagini",
    "Sistema",
    "Sicurezza",
    "Sviluppo",
    "Utilità",
    "Giochi",
    "Altro",
];

pub const APP_TITLE: &str = "LiberDesk di Lorenzo D'Apolito";

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    Unchecked,
    Checking,
    Updating,
    Done(String),
    Result(Check),
}

enum Event {
    Log(Kind, String),
    Status(PathBuf, Status),
    Rescan,
    Finished,
}

struct Row {
    app: App,
    category: String,
    local: Option<String>,
    status: Status,
}

pub struct LiberApp {
    base: PathBuf,
    cfg_path: PathBuf,
    cfg: Config,
    rows: Vec<Row>,
    filter: String,
    cat: Option<String>,
    log: Vec<(Color32, String)>,
    busy: bool,
    started: bool,
    tx: Sender<Event>,
    rx: Receiver<Event>,
    http: Arc<dyn Http>,
    probe: Arc<dyn Probe>,
}

fn kind_color(k: Kind) -> Color32 {
    match k {
        Kind::Info => theme::LOG_INFO,
        Kind::Ok => theme::LOG_OK,
        Kind::Warn => theme::LOG_WARN,
        Kind::Err => theme::LOG_ERR,
    }
}

/// Colore pastello dell'iniziale, scelto dal nome.
fn avatar_color(name: &str) -> Color32 {
    const COLORS: [Color32; 6] = [
        Color32::from_rgb(0xFF, 0xB3, 0xDE),
        Color32::from_rgb(0xB7, 0xC4, 0xFF),
        Color32::from_rgb(0xC4, 0xB5, 0xFD),
        Color32::from_rgb(0xFF, 0xB7, 0xC5),
        Color32::from_rgb(0xA5, 0xD8, 0xFF),
        Color32::from_rgb(0xD9, 0xC2, 0xFF),
    ];
    let h = name.bytes().fold(7usize, |a, b| {
        a.wrapping_mul(31).wrapping_add(usize::from(b))
    });
    COLORS[h % COLORS.len()]
}

/// Categoria scelta dall'utente, altrimenti quella del catalogo, altrimenti «Altro».
pub fn category_of(app: &App, st: &state::State) -> String {
    st.category
        .clone()
        .filter(|c| CATEGORIES.contains(&c.as_str()))
        .or_else(|| app.entry.map(|e| e.category.to_string()))
        .unwrap_or_else(|| "Altro".to_string())
}

/// Indici delle righe per categoria, nell'ordine di `CATEGORIES`; salta quelle vuote.
fn group(rows: &[(String, String)], filter: &str) -> Vec<(String, Vec<usize>)> {
    let f = filter.trim().to_lowercase();
    CATEGORIES
        .iter()
        .filter_map(|c| {
            let idx: Vec<usize> = rows
                .iter()
                .enumerate()
                .filter(|(_, (cat, name))| {
                    cat == c && (f.is_empty() || name.to_lowercase().contains(&f))
                })
                .map(|(i, _)| i)
                .collect();
            (!idx.is_empty()).then(|| ((*c).to_string(), idx))
        })
        .collect()
}

impl LiberApp {
    pub fn new() -> Self {
        let base = config::base_dir();
        let cfg_path = base.join("liberdesk.json");
        let cfg = config::load(&cfg_path);
        let (tx, rx) = channel();
        let mut s = Self {
            base,
            cfg_path,
            cfg,
            rows: Vec::new(),
            filter: String::new(),
            cat: None,
            log: Vec::new(),
            busy: false,
            started: false,
            tx,
            rx,
            http: Arc::new(RealHttp::new()),
            probe: Arc::new(RealProbe),
        };
        let dir = s.apps_dir();
        let _ = std::fs::create_dir_all(&dir);
        s.rescan();
        s
    }

    fn apps_dir(&self) -> PathBuf {
        self.cfg.effective_dir(&self.base)
    }

    fn say(&mut self, k: Kind, m: impl Into<String>) {
        self.log.push((kind_color(k), m.into()));
        if self.log.len() > 500 {
            self.log.remove(0);
        }
    }

    fn save_cfg(&mut self) {
        if let Err(e) = config::save(&self.cfg_path, &self.cfg) {
            self.say(Kind::Err, format!("Impostazioni non salvate: {e}"));
        }
    }

    /// Rilegge la cartella delle app, tenendo gli stati già noti.
    fn rescan(&mut self) {
        let old: Vec<(PathBuf, Status)> = self
            .rows
            .iter()
            .map(|r| (r.app.dir.clone(), r.status.clone()))
            .collect();
        let apps = scan::scan(&self.apps_dir());
        self.rows = apps
            .into_iter()
            .map(|app| {
                let st = state::load(&app.dir);
                let local = update::local_version(&app, &st, &*self.probe);
                let status = old
                    .iter()
                    .find(|(d, _)| *d == app.dir)
                    .map_or(Status::Unchecked, |(_, s)| s.clone());
                let category = category_of(&app, &st);
                Row {
                    app,
                    category,
                    local,
                    status,
                }
            })
            .collect();
    }

    fn pump(&mut self) {
        let mut rescan = false;
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Event::Log(k, m) => self.say(k, m),
                Event::Status(dir, st) => {
                    if let Some(r) = self.rows.iter_mut().find(|r| r.app.dir == dir) {
                        r.status = st;
                    }
                }
                Event::Rescan => rescan = true,
                Event::Finished => self.busy = false,
            }
        }
        if rescan {
            self.rescan();
        }
    }

    fn spawn(&mut self, ctx: &egui::Context, job: impl FnOnce(&Sender<Event>) + Send + 'static) {
        self.busy = true;
        let tx = self.tx.clone();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            job(&tx);
            let _ = tx.send(Event::Finished);
            ctx.request_repaint();
        });
    }

    /// Controlla le app indicate; se `install`, scarica e installa quelle con una versione nuova.
    fn check(&mut self, ctx: &egui::Context, only: Option<usize>, install: bool) {
        let targets: Vec<(App, Option<String>)> = self
            .rows
            .iter()
            .enumerate()
            .filter(|(i, r)| only.is_none_or(|o| o == *i) && r.app.entry.is_some())
            .map(|(_, r)| (r.app.clone(), r.local.clone()))
            .collect();
        if targets.is_empty() {
            self.say(Kind::Info, "Nessuna app con aggiornamenti automatici.");
            return;
        }
        let total = self.rows.len();
        self.say(
            Kind::Info,
            format!(
                "{} app trovate, {} con aggiornamenti automatici",
                total,
                targets.len()
            ),
        );
        for r in &mut self.rows {
            if targets.iter().any(|(a, _)| a.dir == r.app.dir) {
                r.status = Status::Checking;
            }
        }
        let http = Arc::clone(&self.http);
        let explicit = only.is_some();
        self.spawn(ctx, move |tx| {
            let log = |k: Kind, m: String| {
                let _ = tx.send(Event::Log(k, m));
            };
            let mut changed = false;
            for (app, local) in targets {
                let res = update::check(&*http, &app, local.as_deref());
                match &res {
                    Check::Current(v) => log(Kind::Info, format!("{}: già aggiornata ({v})", app.name)),
                    Check::Available { latest, .. } => log(
                        Kind::Info,
                        format!(
                            "{}: nuova versione {latest} (hai {})",
                            app.name,
                            local.as_deref().unwrap_or("?")
                        ),
                    ),
                    Check::Unknown { latest, .. } => log(
                        Kind::Warn,
                        format!(
                            "{}: versione installata non nota, ultima {latest}: usa «Aggiorna» se vuoi sostituirla",
                            app.name
                        ),
                    ),
                    _ => {}
                }
                let st = Status::Result(res.clone());
                let _ = tx.send(Event::Status(app.dir.clone(), st));
                let (latest, asset, ok) = match res {
                    Check::Available { latest, asset } => (latest, asset, true),
                    // Con la versione sconosciuta si installa solo se l'utente lo chiede.
                    Check::Unknown { latest, asset } => (latest, asset, explicit),
                    Check::Error(e) => {
                        log(Kind::Warn, format!("{}: {e}", app.name));
                        continue;
                    }
                    _ => continue,
                };
                if !(install && ok) {
                    continue;
                }
                let _ = tx.send(Event::Status(app.dir.clone(), Status::Updating));
                let mut l = |k: Kind, m: &str| {
                    let _ = tx.send(Event::Log(k, m.to_string()));
                };
                match update::apply(&*http, &app, local.as_deref(), &latest, &asset, &mut l) {
                    Ok(()) => {
                        changed = true;
                        let _ = tx.send(Event::Status(app.dir.clone(), Status::Done(latest)));
                    }
                    Err(e) => {
                        log(Kind::Err, format!("{}: {e}", app.name));
                        let _ = tx.send(Event::Status(
                            app.dir.clone(),
                            Status::Result(Check::Error(e)),
                        ));
                    }
                }
            }
            if changed {
                let _ = tx.send(Event::Rescan);
            }
            log(Kind::Info, "Controllo finito.".into());
        });
    }

    fn launch(&mut self, idx: usize) {
        let Some(exe) = self.rows.get(idx).and_then(|r| r.app.exe.clone()) else {
            return;
        };
        let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        match std::process::Command::new(&exe).current_dir(dir).spawn() {
            Ok(_) => {}
            Err(e) => self.say(Kind::Err, format!("{}: {e}", exe.display())),
        }
    }

    fn open_folder(&mut self) {
        let dir = self.apps_dir();
        if let Err(e) = std::process::Command::new("explorer").arg(&dir).spawn() {
            self.say(Kind::Err, format!("{}: {e}", dir.display()));
        }
    }

    fn status_text(r: &Row) -> (String, Color32) {
        match &r.status {
            Status::Unchecked => match r.app.entry {
                Some(_) => ("Da controllare".into(), theme::MUTED),
                None => ("Senza aggiornamenti automatici".into(), theme::MUTED),
            },
            Status::Checking => ("Controllo…".into(), theme::SCAN),
            Status::Updating => ("Aggiorno…".into(), theme::INSTALL),
            Status::Done(v) => (format!("Aggiornata alla {v}"), theme::OK),
            Status::Result(c) => match c {
                Check::NoSource => ("Senza aggiornamenti automatici".into(), theme::MUTED),
                Check::Current(_) => ("Aggiornata".into(), theme::OK),
                Check::Available { latest, .. } => {
                    (format!("Nuova versione {latest}"), theme::WARN)
                }
                Check::Unknown { latest, .. } => (
                    format!("Versione {latest} disponibile (la tua non è nota)"),
                    theme::WARN,
                ),
                Check::Error(_) => ("Controllo non riuscito".into(), theme::ERR),
            },
        }
    }

    fn can_update(r: &Row) -> bool {
        matches!(
            r.status,
            Status::Result(Check::Available { .. } | Check::Unknown { .. })
        )
    }

    fn set_category(&mut self, idx: usize, cat: &str) {
        let Some(r) = self.rows.get_mut(idx) else {
            return;
        };
        let mut st = state::load(&r.app.dir);
        st.category = Some(cat.to_string());
        let dir = r.app.dir.clone();
        r.category = cat.to_string();
        if let Err(e) = state::save(&dir, &st) {
            self.say(Kind::Err, format!("Categoria non salvata: {e}"));
        }
    }

    fn side(&mut self, ui: &mut egui::Ui) {
        let pairs: Vec<(String, String)> = self
            .rows
            .iter()
            .map(|r| (r.category.clone(), r.app.name.clone()))
            .collect();
        let groups = group(&pairs, "");
        ui.add_space(6.0);
        ui.label(RichText::new("Categorie").strong().color(theme::MUTED));
        if ui
            .selectable_label(self.cat.is_none(), format!("Tutte ({})", self.rows.len()))
            .clicked()
        {
            self.cat = None;
        }
        for (c, idx) in &groups {
            let sel = self.cat.as_deref() == Some(c.as_str());
            if ui
                .selectable_label(sel, format!("{c} ({})", idx.len()))
                .clicked()
            {
                self.cat = Some(c.clone());
            }
        }
    }

    fn list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pairs: Vec<(String, String)> = self
            .rows
            .iter()
            .map(|r| (r.category.clone(), r.app.name.clone()))
            .collect();
        let groups = group(&pairs, &self.filter);
        let mut launch = None;
        let mut upd = None;
        let mut moved: Option<(usize, String)> = None;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            if self.rows.is_empty() {
                ui.add_space(20.0);
                ui.label(
                    RichText::new(format!(
                        "Nessuna app trovata in {}.\nMetti qui le app portabili, una cartella ciascuna: compariranno da sole.",
                        self.apps_dir().display()
                    ))
                    .size(16.0)
                    .color(theme::MUTED),
                );
            }
            for (cat, idx) in &groups {
                if self.cat.as_deref().is_some_and(|c| c != cat) {
                    continue;
                }
                ui.add_space(4.0);
                ui.label(RichText::new(cat).size(18.0).strong().color(theme::PRIMARY));
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(10.0, 10.0);
                    for &i in idx {
                        let Some(r) = self.rows.get(i) else {
                            continue;
                        };
                        let tile = egui::Frame::new()
                            .fill(theme::SURFACE)
                            .stroke(egui::Stroke::new(1.0, theme::BORDER))
                            .corner_radius(12.0)
                            .inner_margin(egui::Margin::same(10))
                            .show(ui, |ui| {
                              ui.vertical(|ui| {
                                ui.set_width(210.0);
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::hover());
                                    ui.painter().rect_filled(rect, 12.0, avatar_color(&r.app.name));
                                    let initial = r.app.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, initial, egui::FontId::proportional(24.0), theme::TEXT);
                                    ui.vertical(|ui| {
                                        ui.add(egui::Label::new(RichText::new(&r.app.name).strong().size(15.0)).truncate());
                                        let ver = r.local.as_deref().map_or("versione non nota".to_string(), |v| format!("versione {v}"));
                                        ui.label(RichText::new(ver).color(theme::MUTED).small());
                                    });
                                });
                                let (t, c) = Self::status_text(r);
                                let resp = ui.add(egui::Label::new(RichText::new(t).color(c).small().strong()).truncate());
                                if let Status::Result(Check::Error(e)) = &r.status {
                                    resp.on_hover_text(e);
                                }
                                ui.horizontal(|ui| {
                                    let start = egui::Button::new(RichText::new("Avvia").strong().color(theme::ON_COLOR)).fill(theme::DOWNLOAD);
                                    if ui.add(start).clicked() {
                                        launch = Some(i);
                                    }
                                    if Self::can_update(r)
                                        && ui.add_enabled(!self.busy, egui::Button::new("Aggiorna")).clicked()
                                    {
                                        upd = Some(i);
                                    }
                                });
                            });
                            });
                        tile.response.context_menu(|ui| {
                            ui.menu_button("Sposta in…", |ui| {
                                for c in CATEGORIES {
                                    if ui.button(c).clicked() {
                                        moved = Some((i, c.to_string()));
                                        ui.close();
                                    }
                                }
                            });
                        });
                    }
                });
                ui.add_space(8.0);
            }
        });
        if let Some(i) = launch {
            self.launch(i);
        }
        if let Some(i) = upd {
            self.check(ctx, Some(i), true);
        }
        if let Some((i, c)) = moved {
            self.set_category(i, &c);
        }
    }
}

impl eframe::App for LiberApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.pump();
        let ctx = ui.ctx().clone();
        if !self.started {
            self.started = true;
            if self.cfg.auto_check {
                self.say(Kind::Info, "Controllo gli aggiornamenti…");
                let install = self.cfg.auto_install;
                self.check(&ctx, None, install);
            }
        }
        egui::Panel::top("top").show(ui, |ui| {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("LiberDesk")
                        .size(22.0)
                        .strong()
                        .color(theme::PRIMARY),
                );
                ui.label(
                    RichText::new("di Lorenzo D'Apolito")
                        .size(15.0)
                        .color(theme::MUTED),
                );
            });
            ui.horizontal_wrapped(|ui| {
                let busy = self.busy;
                if ui
                    .add_enabled(
                        !busy,
                        egui::Button::new(
                            RichText::new("Controlla e aggiorna tutto")
                                .strong()
                                .color(theme::ON_COLOR),
                        )
                        .fill(theme::SCAN),
                    )
                    .clicked()
                {
                    self.check(&ctx, None, true);
                }
                if ui
                    .add_enabled(!busy, egui::Button::new("Solo controlla"))
                    .clicked()
                {
                    self.check(&ctx, None, false);
                }
                if ui.button("Rileggi la cartella").clicked() {
                    self.rescan();
                }
                if ui.button("Apri la cartella delle app").clicked() {
                    self.open_folder();
                }
                if ui.button("Cambia cartella…").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        self.cfg.apps_dir = p.to_string_lossy().to_string();
                        self.save_cfg();
                        self.rows.clear();
                        self.rescan();
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                let mut changed = false;
                changed |= ui
                    .checkbox(&mut self.cfg.auto_check, "Controlla all'avvio")
                    .changed();
                changed |= ui
                    .checkbox(
                        &mut self.cfg.auto_install,
                        "Installa subito gli aggiornamenti trovati",
                    )
                    .changed();
                if changed {
                    self.save_cfg();
                }
                ui.label("Cerca:");
                ui.add(egui::TextEdit::singleline(&mut self.filter).desired_width(160.0));
                ui.label(RichText::new(self.apps_dir().display().to_string()).color(theme::MUTED));
            });
            ui.add_space(4.0);
        });
        egui::Panel::bottom("log").show(ui, |ui| {
            egui::Frame::new()
                .fill(theme::LOG_BG)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(110.0)
                        .auto_shrink([false, true])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            for (c, m) in &self.log {
                                ui.label(RichText::new(m).color(*c).monospace());
                            }
                        });
                });
        });
        egui::Panel::left("cats")
            .default_size(170.0)
            .show(ui, |ui| self.side(ui));
        egui::CentralPanel::default().show(ui, |ui| self.list(ui, &ctx));
        if self.busy {
            ctx.request_repaint_after(std::time::Duration::from_millis(150));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog;

    fn row(status: Status) -> Row {
        Row {
            app: App {
                name: "X".into(),
                dir: PathBuf::from("X"),
                exe: None,
                entry: catalog::find_for_folder("Rufus"),
            },
            category: "Sistema".into(),
            local: None,
            status,
        }
    }

    #[test]
    fn status_texts_and_update_button() {
        assert_eq!(
            LiberApp::status_text(&row(Status::Unchecked)).0,
            "Da controllare"
        );
        let mut r = row(Status::Unchecked);
        r.app.entry = None;
        assert!(LiberApp::status_text(&r).0.contains("Senza aggiornamenti"));
        assert_eq!(
            LiberApp::status_text(&row(Status::Result(Check::Current("1".into())))).0,
            "Aggiornata"
        );
        assert!(!LiberApp::can_update(&row(Status::Unchecked)));
        assert!(!LiberApp::can_update(&row(Status::Result(Check::Current(
            "1".into()
        )))));
        let a = crate::github::Asset {
            name: "a.zip".into(),
            browser_download_url: "https://x".into(),
            size: 1,
        };
        assert!(LiberApp::can_update(&row(Status::Result(
            Check::Available {
                latest: "2".into(),
                asset: a.clone()
            }
        ))));
        assert!(LiberApp::can_update(&row(Status::Result(Check::Unknown {
            latest: "2".into(),
            asset: a
        }))));
    }

    #[test]
    fn groups_follow_the_fixed_order_and_filter() {
        let rows = vec![
            ("Sistema".to_string(), "Rufus".to_string()),
            ("Internet".to_string(), "Firefox".to_string()),
            ("Sistema".to_string(), "BCU".to_string()),
            ("Altro".to_string(), "Mio".to_string()),
        ];
        let g = group(&rows, "");
        let names: Vec<_> = g.iter().map(|(c, _)| c.as_str()).collect();
        assert_eq!(names, vec!["Internet", "Sistema", "Altro"]);
        assert_eq!(g[1].1, vec![0, 2]);
        let f = group(&rows, "ruf");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].1, vec![0]);
        assert!(group(&[("Strana".into(), "X".into())], "").is_empty());
    }

    #[test]
    fn category_user_choice_wins() {
        let mut a = row(Status::Unchecked).app;
        let mut st = state::State::default();
        assert_eq!(category_of(&a, &st), "Sistema");
        st.category = Some("Giochi".into());
        assert_eq!(category_of(&a, &st), "Giochi");
        st.category = Some("Inventata".into());
        assert_eq!(category_of(&a, &st), "Sistema");
        a.entry = None;
        assert_eq!(category_of(&a, &state::State::default()), "Altro");
    }

    #[test]
    fn catalog_categories_are_all_known() {
        for e in crate::catalog::CATALOG {
            assert!(CATEGORIES.contains(&e.category), "{}", e.category);
        }
    }

    #[test]
    fn avatar_color_is_stable() {
        assert_eq!(avatar_color("Notepad++"), avatar_color("Notepad++"));
    }
}
