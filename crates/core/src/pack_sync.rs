use crate::manifest::{Manifest, Policy};
use crate::state::State;
use std::collections::BTreeSet;

/// Źródło informacji o tym, co leży na dysku.
/// Abstrakcja istnieje po to, żeby `plan` dało się testować bez plików.
pub trait FileProbe {
    /// SHA-512 pliku albo `None`, gdy pliku nie ma.
    fn hash_of(&self, rel: &str) -> Option<String>;
    /// Ścieżki względne wszystkich plików w katalogu, rekurencyjnie.
    fn list(&self, dir: &str) -> Vec<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Pobierz plik o tym indeksie w `manifest.files`.
    Download { index: usize },
    Delete { path: String },
    /// Plik jest już właściwy — tylko zapamiętaj to w state.json.
    Record { path: String, sha512: String },
    /// Gracz zmienił plik, zostawiamy jego wersję.
    SkipModified { path: String },
    /// Plik istnieje, ale nie wiemy, kto go wgrał. Nie ruszamy.
    SkipUnknown { path: String },
}

pub fn plan(m: &Manifest, st: &State, probe: &dyn FileProbe) -> Vec<Action> {
    let mut akcje = Vec::new();
    let mut znane: BTreeSet<String> = BTreeSet::new();

    for (index, wpis) in m.files.iter().enumerate() {
        znane.insert(wpis.path.clone());
        let na_dysku = probe.hash_of(&wpis.path);
        let zapisany = st.written.get(&wpis.path);

        match wpis.policy {
            Policy::Mirror => match na_dysku {
                Some(h) if h.eq_ignore_ascii_case(&wpis.sha512) => {}
                _ => akcje.push(Action::Download { index }),
            },

            Policy::Seed => {
                if na_dysku.is_none() {
                    akcje.push(Action::Download { index });
                }
            }

            Policy::Smart => match (na_dysku, zapisany) {
                (None, _) => akcje.push(Action::Download { index }),

                (Some(h), Some(z)) if h.eq_ignore_ascii_case(z) => {
                    // Gracz nie ruszał pliku, więc wolno go zaktualizować.
                    if !h.eq_ignore_ascii_case(&wpis.sha512) {
                        akcje.push(Action::Download { index });
                    }
                }

                (Some(_), Some(_)) => akcje.push(Action::SkipModified {
                    path: wpis.path.clone(),
                }),

                (Some(h), None) if h.eq_ignore_ascii_case(&wpis.sha512) => {
                    akcje.push(Action::Record {
                        path: wpis.path.clone(),
                        sha512: h,
                    })
                }

                (Some(_), None) => akcje.push(Action::SkipUnknown {
                    path: wpis.path.clone(),
                }),
            },
        }
    }

    // Kasowanie obcych plików tylko w katalogach oznaczonych jako lustrzane.
    for dir in &m.mirror_dirs {
        for sciezka in probe.list(dir) {
            if !znane.contains(&sciezka) {
                akcje.push(Action::Delete { path: sciezka });
            }
        }
    }

    akcje
}

use crate::hash::sha512_file;
use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::paths::{join_within, PathError};
use crate::progress::{Progress, Stage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("nieprawidłowa ścieżka w manifeście: {0}")]
    Path(#[from] PathError),
    #[error("błąd operacji na pliku {0}: {1}")]
    Io(String, std::io::Error),
}

pub struct DiskProbe {
    root: PathBuf,
}

impl DiskProbe {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }
}

impl FileProbe for DiskProbe {
    fn hash_of(&self, rel: &str) -> Option<String> {
        let p = join_within(&self.root, rel).ok()?;
        sha512_file(&p).ok()
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let baza = match join_within(&self.root, dir) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        zbierz(&baza, dir, &mut out);
        out
    }
}

fn zbierz(katalog: &Path, prefiks: &str, out: &mut Vec<String>) {
    let Ok(wpisy) = std::fs::read_dir(katalog) else {
        return;
    };
    for wpis in wpisy.flatten() {
        let nazwa = wpis.file_name().to_string_lossy().to_string();
        // Katalog .index to metadane PrismLaunchera, nie nasza sprawa.
        if nazwa.starts_with('.') {
            continue;
        }
        let rel = format!("{prefiks}/{nazwa}");
        if wpis.path().is_dir() {
            zbierz(&wpis.path(), &rel, out);
        } else {
            out.push(rel);
        }
    }
}

pub async fn apply(
    m: &Manifest,
    root: &Path,
    st: &mut State,
    actions: &[Action],
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<Vec<String>, SyncError> {
    let mut notatki = Vec::new();
    let mut do_pobrania = Vec::new();

    for akcja in actions {
        match akcja {
            Action::Download { index } => {
                let wpis = &m.files[*index];
                do_pobrania.push((
                    wpis.path.clone(),
                    wpis.sha512.clone(),
                    DownloadSpec {
                        urls: wpis.urls.clone(),
                        dest: join_within(root, &wpis.path)?,
                        expect: Expect::Sha512(wpis.sha512.clone()),
                    },
                ));
            }
            Action::Delete { path } => {
                let p = join_within(root, path)?;
                if p.exists() {
                    std::fs::remove_file(&p).map_err(|e| SyncError::Io(path.clone(), e))?;
                }
                st.written.remove(path);
                notatki.push(format!("usunięto obcy plik: {path}"));
            }
            Action::Record { path, sha512 } => {
                st.written.insert(path.clone(), sha512.clone());
            }
            Action::SkipModified { path } => {
                notatki.push(format!("pomijam {path} — plik został zmieniony ręcznie"));
            }
            Action::SkipUnknown { path } => {
                notatki.push(format!("pomijam {path} — nie wiadomo, skąd pochodzi"));
            }
        }
    }

    let specyfikacje: Vec<DownloadSpec> = do_pobrania.iter().map(|(_, _, s)| s.clone()).collect();
    dl.fetch_many(specyfikacje, Stage::Pack, on).await?;

    // Stan zapisujemy dopiero po udanym pobraniu wszystkiego.
    for (sciezka, hash, _) in do_pobrania {
        st.written.insert(sciezka, hash);
    }
    Ok(notatki)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::*;
    use std::collections::BTreeMap;

    /// Sonda pamięciowa: udaje dysk bez dotykania dysku.
    struct Mapa {
        pliki: BTreeMap<String, String>,
    }

    impl Mapa {
        fn nowa(pary: &[(&str, &str)]) -> Self {
            Self {
                pliki: pary
                    .iter()
                    .map(|(a, b)| (a.to_string(), b.to_string()))
                    .collect(),
            }
        }
    }

    impl FileProbe for Mapa {
        fn hash_of(&self, rel: &str) -> Option<String> {
            self.pliki.get(rel).cloned()
        }
        fn list(&self, dir: &str) -> Vec<String> {
            let prefiks = format!("{dir}/");
            self.pliki
                .keys()
                .filter(|k| k.starts_with(&prefiks))
                .cloned()
                .collect()
        }
    }

    fn manifest(wpisy: Vec<(&str, &str, Policy)>) -> Manifest {
        Manifest {
            schema: 1,
            pack: Pack {
                name: "T".into(),
                edition: "".into(),
                version: "1".into(),
                minecraft: "1.21.1".into(),
                loader: Loader {
                    kind: "neoforge".into(),
                    version: "21.1.249".into(),
                },
            },
            java: JavaReq {
                major: 21,
                distribution: "temurin".into(),
            },
            memory: Memory {
                min_mb: 512,
                max_mb: 4096,
            },
            auth: AuthCfg {
                msa_client_id: "x".into(),
            },
            launcher: LauncherInfo {
                latest_version: "1".into(),
                urls: BTreeMap::new(),
            },
            mirror_dirs: vec!["mods".into()],
            files: wpisy
                .into_iter()
                .map(|(p, h, pol)| FileEntry {
                    path: p.into(),
                    size: 1,
                    sha512: h.into(),
                    policy: pol,
                    urls: vec!["https://example.test/x".into()],
                })
                .collect(),
        }
    }

    fn stan(pary: &[(&str, &str)]) -> State {
        State {
            written: pary
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        }
    }

    // --- tryb mirror ---

    #[test]
    fn mirror_pobiera_brakujacy_mod() {
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[]));
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn mirror_nie_rusza_zgodnego_moda() {
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("mods/a.jar", "AAA")]));
        assert_eq!(akcje, vec![]);
    }

    #[test]
    fn mirror_pobiera_ponownie_przy_zlym_hashu() {
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("mods/a.jar", "ZLE")]));
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn mirror_kasuje_obcy_mod() {
        // Obcy mod w mods/ wywala cala paczke przy starcie gry, wiec musi zniknac.
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(
            &m,
            &State::new(),
            &Mapa::nowa(&[("mods/a.jar", "AAA"), ("mods/obcy.jar", "XXX")]),
        );
        assert_eq!(
            akcje,
            vec![Action::Delete {
                path: "mods/obcy.jar".into()
            }]
        );
    }

    // --- tryb smart ---

    #[test]
    fn smart_pobiera_brakujacy_config() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[]));
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn smart_nadpisuje_nietkniety_config() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(
            &m,
            &stan(&[("config/a.toml", "STARY")]),
            &Mapa::nowa(&[("config/a.toml", "STARY")]),
        );
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn smart_nic_nie_robi_gdy_juz_aktualny() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(
            &m,
            &stan(&[("config/a.toml", "NOWY")]),
            &Mapa::nowa(&[("config/a.toml", "NOWY")]),
        );
        assert_eq!(akcje, vec![]);
    }

    #[test]
    fn smart_zostawia_config_zmieniony_przez_gracza() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(
            &m,
            &stan(&[("config/a.toml", "STARY")]),
            &Mapa::nowa(&[("config/a.toml", "RECZNIE")]),
        );
        assert_eq!(
            akcje,
            vec![Action::SkipModified {
                path: "config/a.toml".into()
            }]
        );
    }

    #[test]
    fn smart_dopisuje_wpis_gdy_plik_juz_jest_wlasciwy() {
        // Plik zgodny z manifestem, ale bez wpisu w state.json — np. po recznej
        // instalacji. Nie pobieramy, tylko zapamietujemy, ze taki jest.
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("config/a.toml", "NOWY")]));
        assert_eq!(
            akcje,
            vec![Action::Record {
                path: "config/a.toml".into(),
                sha512: "NOWY".into()
            }]
        );
    }

    #[test]
    fn smart_zostawia_nieznany_plik_bez_wpisu() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("config/a.toml", "CUDZE")]));
        assert_eq!(
            akcje,
            vec![Action::SkipUnknown {
                path: "config/a.toml".into()
            }]
        );
    }

    // --- tryb seed ---

    #[test]
    fn seed_wgrywa_tylko_raz() {
        let m = manifest(vec![("options.txt", "NOWY", Policy::Seed)]);
        assert_eq!(
            plan(&m, &State::new(), &Mapa::nowa(&[])),
            vec![Action::Download { index: 0 }]
        );
        // Istniejacy plik zostaje nietkniety, nawet jesli manifest ma inna tresc.
        assert_eq!(
            plan(&m, &State::new(), &Mapa::nowa(&[("options.txt", "COKOLWIEK")])),
            vec![]
        );
    }
}
