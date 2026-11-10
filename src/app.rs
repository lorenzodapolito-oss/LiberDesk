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
    local: Option<String>,
    status: Status,
}

pub struct LiberApp {
    base: PathBuf,
    cfg_path: PathBuf,
    cfg: Config,
    rows: Vec<Row>,
    filter: String,
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
        Color32::from_rgb(0xA7, 0xF3, 0xD0),
        Color32::from_rgb(0xFF, 0xD6, 0xA5),
        Color32::from_rgb(0xC4, 0xB5, 0xFD),
        Color32::from_rgb(0xFF, 0xB7, 0xC5),
        Color32::from_rgb(0xA5, 0xD8, 0xFF),
        Color32::from_rgb(0xFD, 0xF0, 0xA0),
    ];
    let h = name.bytes().fold(7usize, |a, b| {
        a.wrapping_mul(31).wrapping_add(usize::from(b))
    });
    COLORS[h % COLORS.len()]
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
                Row { app, local, status }
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

    fn list(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let filter = self.filter.trim().to_lowercase();
        let mut launch = None;
        let mut upd = None;
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
            for (i, r) in self.rows.iter().enumerate() {
                if !filter.is_empty() && !r.app.name.to_lowercase().contains(&filter) {
                    continue;
                }
                egui::Frame::new()
                    .fill(theme::SURFACE)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .corner_radius(10.0)
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal(|ui| {
                            let (rect, _) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 10.0, avatar_color(&r.app.name));
                            let initial = r.app.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
                            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, initial, egui::FontId::proportional(22.0), theme::TEXT);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&r.app.name).strong().size(16.0));
                                let ver = r.local.as_deref().map_or("versione non nota".to_string(), |v| format!("versione {v}"));
                                let cat = r.app.entry.map_or("", |e| e.category);
                                ui.label(RichText::new(format!("{ver}   {cat}")).color(theme::MUTED));
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let start = egui::Button::new(RichText::new("Avvia").strong().color(theme::ON_COLOR)).fill(theme::DOWNLOAD);
                                if ui.add(start).clicked() {
                                    launch = Some(i);
                                }
                                if Self::can_update(r)
                                    && ui.add_enabled(!self.busy, egui::Button::new("Aggiorna")).clicked()
                                {
                                    upd = Some(i);
                                }
                                let (t, c) = Self::status_text(r);
                                let resp = ui.label(RichText::new(t).color(c).strong());
                                if let Status::Result(Check::Error(e)) = &r.status {
                                    resp.on_hover_text(e);
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        });
        if let Some(i) = launch {
            self.launch(i);
        }
        if let Some(i) = upd {
            self.check(ctx, Some(i), true);
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
    fn avatar_color_is_stable() {
        assert_eq!(avatar_color("Notepad++"), avatar_color("Notepad++"));
    }
}
