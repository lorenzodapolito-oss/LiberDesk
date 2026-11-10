//! Trova le app nella cartella: ogni sottocartella con un `.exe` è un'app.

use crate::catalog::{self, normalize, Entry};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct App {
    pub name: String,
    pub dir: PathBuf,
    pub exe: Option<PathBuf>,
    pub entry: Option<&'static Entry>,
}

const SKIP_WORDS: [&str; 6] = ["unins", "uninst", "setup", "update", "crash", "helper"];

fn is_exe(p: &Path) -> bool {
    p.extension()
        .is_some_and(|e| e.to_string_lossy().eq_ignore_ascii_case("exe"))
}

/// File `.exe` della cartella e delle sottocartelle (al massimo 2 livelli).
fn exes_in(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        let Ok(ft) = e.file_type() else {
            continue;
        };
        if ft.is_symlink() {
            continue;
        }
        if ft.is_dir() && depth < 2 {
            exes_in(&p, depth + 1, out);
        } else if ft.is_file() && is_exe(&p) {
            out.push(p);
        }
    }
}

/// Eseguibile principale: quello del catalogo, poi quello col nome della cartella,
/// poi il più grande che non sia un disinstallatore o un programma di servizio.
pub fn find_exe(dir: &Path, entry: Option<&Entry>) -> Option<PathBuf> {
    if let Some(e) = entry {
        if !e.exe.is_empty() {
            let p = e
                .exe
                .split(['\\', '/'])
                .fold(dir.to_path_buf(), |a, s| a.join(s));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    let mut all = Vec::new();
    exes_in(dir, 0, &mut all);
    let folder = normalize(
        &dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default(),
    );
    let stem = |p: &Path| {
        normalize(
            &p.file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
        )
    };
    if let Some(p) = all.iter().find(|p| stem(p) == folder) {
        return Some(p.clone());
    }
    let good = |p: &&PathBuf| {
        let s = stem(p);
        !SKIP_WORDS.iter().any(|w| s.contains(w))
    };
    let size = |p: &PathBuf| fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    all.iter().filter(good).max_by_key(|p| size(p)).cloned()
}

/// Elenco delle app, in ordine alfabetico. Le cartelle che cominciano con `_` o `.` si ignorano.
pub fn scan(apps_dir: &Path) -> Vec<App> {
    let Ok(rd) = fs::read_dir(apps_dir) else {
        return Vec::new();
    };
    let mut apps: Vec<App> = rd
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with(['_', '.']) {
                return None;
            }
            let dir = e.path();
            let entry = catalog::find_for_folder(&name);
            let exe = find_exe(&dir, entry);
            exe.as_ref()?;
            let shown = entry.map_or(name, |en| en.name.to_string());
            Some(App {
                name: shown,
                dir,
                exe,
                entry,
            })
        })
        .collect();
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ld_scan_{}_{tag}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn file(p: &Path, size: usize) {
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, vec![b'x'; size]).unwrap();
    }

    #[test]
    fn finds_apps_and_picks_the_right_exe() {
        let d = tmp("a");
        file(&d.join("Notepad++/notepad++.exe"), 10);
        file(&d.join("Notepad++/updater/gup.exe"), 500);
        file(&d.join("Zeta/zeta.exe"), 5);
        file(&d.join("Zeta/unins000.exe"), 900);
        file(&d.join("Alfa/strumento.exe"), 50);
        file(&d.join("Alfa/setup.exe"), 9000);
        file(&d.join("SoloDati/leggimi.txt"), 5);
        file(&d.join("_backup/x/x.exe"), 5);
        file(&d.join(".nascosta/y.exe"), 5);
        let apps = scan(&d);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, vec!["Alfa", "Notepad++", "Zeta"]);
        let exe = |n: &str| {
            apps.iter()
                .find(|a| a.name == n)
                .and_then(|a| a.exe.as_ref())
                .and_then(|p| p.file_name())
                .map(|s| s.to_string_lossy().to_string())
        };
        assert_eq!(exe("Alfa").as_deref(), Some("strumento.exe"));
        assert_eq!(exe("Zeta").as_deref(), Some("zeta.exe"));
        assert_eq!(exe("Notepad++").as_deref(), Some("notepad++.exe"));
        assert!(apps
            .iter()
            .find(|a| a.name == "Notepad++")
            .unwrap()
            .entry
            .is_some());
        assert!(apps
            .iter()
            .find(|a| a.name == "Zeta")
            .unwrap()
            .entry
            .is_none());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn catalog_exe_in_a_subfolder_is_used() {
        let d = tmp("b");
        file(&d.join("OBS/bin/64bit/obs64.exe"), 10);
        file(&d.join("OBS/bin/64bit/obs-ffmpeg-mux.exe"), 900);
        let apps = scan(&d);
        assert_eq!(apps.len(), 1);
        assert!(apps[0].exe.as_ref().unwrap().ends_with("obs64.exe"));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn missing_folder_is_empty() {
        assert!(scan(Path::new("/non/esiste/mai")).is_empty());
    }
}
