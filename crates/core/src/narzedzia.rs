//! Zewnętrzne programy, których wymagają niektóre mody.
//!
//! Create: Harmonics potrzebuje `yt-dlp` i `ffmpeg`, żeby odtwarzać dźwięk.
//! Sam potrafi je pobrać, ale pyta gracza o zgodę w trakcie gry. Launcher
//! przygotowuje je wcześniej, biorąc pliki z tych samych oficjalnych źródeł,
//! z których korzysta mod — niczego nie hostujemy u siebie.

use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::paths::{join_within, PathError};
use crate::progress::{Progress, Stage};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum NarzedzieError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("nieprawidłowa ścieżka narzędzia: {0}")]
    Sciezka(#[from] PathError),
    #[error("nie udało się rozpakować {0}: {1}")]
    Rozpakowanie(String, String),
    #[error("błąd operacji na pliku {0}: {1}")]
    Io(String, std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct Narzedzie {
    /// Nazwa pokazywana graczowi w pasku postępu.
    pub nazwa: String,
    /// Katalog w instancji, do którego trafia zawartość.
    pub katalog: String,
    /// Docelowa nazwa pliku dla każdego systemu, gdy to pojedynczy program.
    /// Puste oznacza archiwum. Nazwy się różnią: `yt-dlp` kontra `yt-dlp.exe`.
    #[serde(default)]
    pub pliki: BTreeMap<String, String>,
    /// Adresy pobierania dla poszczególnych systemów, np. `linux-x64`.
    pub zrodla: BTreeMap<String, String>,
}

impl Narzedzie {
    /// Klucz systemu, po którym wybieramy adres.
    pub fn klucz_systemu() -> &'static str {
        if cfg!(target_os = "windows") {
            "windows-x64"
        } else if cfg!(target_os = "macos") {
            "macos-x64"
        } else {
            "linux-x64"
        }
    }

    pub fn zrodlo(&self) -> Option<&str> {
        self.zrodla.get(Self::klucz_systemu()).map(|s| s.as_str())
    }

    pub fn plik_docelowy(&self) -> Option<&str> {
        self.pliki.get(Self::klucz_systemu()).map(|s| s.as_str())
    }

    /// Format archiwum wyprowadzony z adresu. Trzymanie go osobno groziło
    /// rozjazdem: Linux dostaje `.tar.xz`, a Windows `.zip`.
    pub fn format_archiwum(url: &str) -> Option<&'static str> {
        let bez_zapytania = url.split('?').next().unwrap_or(url);
        if bez_zapytania.ends_with(".tar.xz") {
            Some("tar.xz")
        } else if bez_zapytania.ends_with(".zip") {
            Some("zip")
        } else {
            None
        }
    }
}

/// Czy narzędzie jest już na miejscu.
///
/// Mod zostawia w katalogu plik z instrukcją, więc sama jego obecność nic nie
/// znaczy — szukamy czegokolwiek poza plikami `.md`.
fn juz_jest(katalog: &Path) -> bool {
    let Ok(wpisy) = std::fs::read_dir(katalog) else {
        return false;
    };
    wpisy.flatten().any(|w| {
        w.path()
            .extension()
            .map(|e| !e.eq_ignore_ascii_case("md"))
            .unwrap_or(true)
    })
}

/// Pobiera i rozpakowuje brakujące narzędzia.
pub async fn zapewnij(
    instancja: &Path,
    narzedzia: &[Narzedzie],
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<(), NarzedzieError> {
    for n in narzedzia {
        let katalog = join_within(instancja, &n.katalog)?;
        if juz_jest(&katalog) {
            continue;
        }
        let Some(url) = n.zrodlo() else {
            // Brak adresu dla tego systemu — mod poradzi sobie sam.
            continue;
        };

        std::fs::create_dir_all(&katalog)
            .map_err(|e| NarzedzieError::Io(katalog.display().to_string(), e))?;

        match (n.plik_docelowy(), Narzedzie::format_archiwum(url)) {
            (Some(nazwa_pliku), _) => {
                let cel = katalog.join(nazwa_pliku);
                dl.fetch_one_obserwowane(
                    &DownloadSpec {
                        urls: vec![url.to_string()],
                        dest: cel.clone(),
                        // Wydania „latest" zmieniaja sie w czasie, wiec nie da sie
                        // przypiac hasha. Ufamy HTTPS i oficjalnemu zrodlu — tak samo
                        // jak przy pobieraniu Javy z Adoptium.
                        expect: Expect::Any,
                    },
                    Some((Stage::Narzedzia, n.nazwa.clone(), on.clone())),
                )
                .await?;
                nadaj_prawa_wykonywania(&cel);
            }
            (None, Some(format)) => {
                let archiwum = katalog.join(format!("pobrane.{}", format.replace('.', "-")));
                dl.fetch_one_obserwowane(
                    &DownloadSpec {
                        urls: vec![url.to_string()],
                        dest: archiwum.clone(),
                        expect: Expect::Any,
                    },
                    Some((Stage::Narzedzia, n.nazwa.clone(), on.clone())),
                )
                .await?;

                on(Progress::trwa(
                    Stage::Narzedzia,
                    format!("Rozpakowuję {}…", n.nazwa),
                ));
                rozpakuj(&archiwum, &katalog, format, &n.nazwa)?;
                let _ = std::fs::remove_file(&archiwum);
                nadaj_prawa_rekurencyjnie(&katalog);
            }
            (None, None) => continue,
        }
    }
    Ok(())
}

fn rozpakuj(
    archiwum: &Path,
    cel: &Path,
    format: &str,
    nazwa: &str,
) -> Result<(), NarzedzieError> {
    let blad = |e: String| NarzedzieError::Rozpakowanie(nazwa.to_string(), e);

    match format {
        "zip" => {
            let plik = std::fs::File::open(archiwum)
                .map_err(|e| NarzedzieError::Io(archiwum.display().to_string(), e))?;
            let mut zip = zip::ZipArchive::new(plik).map_err(|e| blad(e.to_string()))?;
            zip.extract(cel).map_err(|e| blad(e.to_string()))?;
        }
        "tar.xz" => {
            // Rozpakowujemy przez plik posredni, a nie w pamieci — archiwum
            // ffmpeg rozwija sie do ponad 350 MB.
            let rozpakowany = archiwum.with_extension("tar");
            {
                let wejscie = std::fs::File::open(archiwum)
                    .map_err(|e| NarzedzieError::Io(archiwum.display().to_string(), e))?;
                let mut wejscie = std::io::BufReader::new(wejscie);
                let wyjscie = std::fs::File::create(&rozpakowany)
                    .map_err(|e| NarzedzieError::Io(rozpakowany.display().to_string(), e))?;
                let mut wyjscie = std::io::BufWriter::new(wyjscie);
                lzma_rs::xz_decompress(&mut wejscie, &mut wyjscie)
                    .map_err(|e| blad(format!("{e:?}")))?;
            }
            let plik = std::fs::File::open(&rozpakowany)
                .map_err(|e| NarzedzieError::Io(rozpakowany.display().to_string(), e))?;
            tar::Archive::new(plik)
                .unpack(cel)
                .map_err(|e| blad(e.to_string()))?;
            let _ = std::fs::remove_file(&rozpakowany);
        }
        inny => return Err(blad(format!("nieznany format archiwum: {inny}"))),
    }
    Ok(())
}

#[cfg(unix)]
fn nadaj_prawa_wykonywania(sciezka: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(dane) = std::fs::metadata(sciezka) {
        let mut prawa = dane.permissions();
        prawa.set_mode(prawa.mode() | 0o755);
        let _ = std::fs::set_permissions(sciezka, prawa);
    }
}

#[cfg(not(unix))]
fn nadaj_prawa_wykonywania(_sciezka: &Path) {}

/// Nadaje prawa wykonywania wszystkiemu, co wylądowało w katalogu `bin`.
fn nadaj_prawa_rekurencyjnie(katalog: &Path) {
    let mut kolejka: Vec<PathBuf> = vec![katalog.to_path_buf()];
    while let Some(k) = kolejka.pop() {
        let Ok(wpisy) = std::fs::read_dir(&k) else {
            continue;
        };
        for w in wpisy.flatten() {
            let s = w.path();
            // `file_type` z `read_dir` NIE podąża za dowiązaniami, w odróżnieniu
            // od `is_dir()`. Archiwum z dowiązaniem wskazującym na własnego
            // przodka zapętliłoby chodzenie po drzewie: kolejka rosłaby bez
            // końca, a gracz stałby na „Rozpakowuję…" na zawsze.
            let katalog = w.file_type().map(|t| t.is_dir()).unwrap_or(false);
            if katalog {
                kolejka.push(s);
            } else if k.file_name().map(|n| n == "bin").unwrap_or(false) {
                nadaj_prawa_wykonywania(&s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn narzedzie(plik: Option<&str>, _archiwum: Option<&str>) -> Narzedzie {
        let mut zrodla = BTreeMap::new();
        zrodla.insert("linux-x64".to_string(), "https://example.test/l".to_string());
        zrodla.insert("windows-x64".to_string(), "https://example.test/w".to_string());
        let mut pliki = BTreeMap::new();
        if let Some(p) = plik {
            pliki.insert("linux-x64".to_string(), p.to_string());
            pliki.insert("windows-x64".to_string(), format!("{p}.exe"));
        }
        Narzedzie {
            nazwa: "test".into(),
            katalog: "audio_providers/test".into(),
            pliki,
            zrodla,
        }
    }

    #[test]
    fn nazwa_pliku_zalezy_od_systemu() {
        let n = narzedzie(Some("yt-dlp"), None);
        let oczekiwana = if cfg!(target_os = "windows") {
            "yt-dlp.exe"
        } else {
            "yt-dlp"
        };
        assert_eq!(n.plik_docelowy(), Some(oczekiwana));
    }

    #[test]
    fn wybiera_zrodlo_dla_biezacego_systemu() {
        let n = narzedzie(Some("x"), None);
        let oczekiwane = if cfg!(target_os = "windows") {
            "https://example.test/w"
        } else {
            "https://example.test/l"
        };
        assert_eq!(n.zrodlo(), Some(oczekiwane));
    }

    #[test]
    fn brak_zrodla_dla_systemu_nie_jest_bledem() {
        let mut n = narzedzie(Some("x"), None);
        n.zrodla.clear();
        n.zrodla
            .insert("solaris-sparc".into(), "https://example.test/s".into());
        assert_eq!(n.zrodlo(), None);
    }

    #[test]
    fn sam_plik_instrukcji_nie_znaczy_ze_narzedzie_jest() {
        // Mod zostawia w katalogu plik .md — gdybysmy uznali go za dowod
        // instalacji, narzedzie nigdy by sie nie pobralo.
        let d = std::env::temp_dir().join("chmurka-narz-instrukcja");
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("yt-dlp-instructions.md"), b"x").unwrap();
        assert!(!juz_jest(&d));

        std::fs::write(d.join("yt-dlp"), b"binarka").unwrap();
        assert!(juz_jest(&d));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn pusty_katalog_znaczy_brak_narzedzia() {
        let d = std::env::temp_dir().join("chmurka-narz-pusty");
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        assert!(!juz_jest(&d));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn format_archiwum_z_adresu() {
        assert_eq!(
            Narzedzie::format_archiwum(
                "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-lgpl.tar.xz"
            ),
            Some("tar.xz")
        );
        assert_eq!(
            Narzedzie::format_archiwum("https://example.test/ffmpeg-win64.zip"),
            Some("zip")
        );
        // Pojedyncza binarka nie jest archiwum.
        assert_eq!(
            Narzedzie::format_archiwum("https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux"),
            None
        );
    }

    #[test]
    fn nieznany_format_archiwum_konczy_sie_bledem() {
        let d = std::env::temp_dir().join("chmurka-narz-format");
        std::fs::create_dir_all(&d).unwrap();
        let a = d.join("cos.rar");
        std::fs::write(&a, b"x").unwrap();
        let wynik = rozpakuj(&a, &d, "rar", "test");
        assert!(wynik.is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
