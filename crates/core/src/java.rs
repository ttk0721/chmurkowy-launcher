use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::progress::{Progress, Stage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    Osx,
}

impl Os {
    /// Nazwa systemu w regułach profili wersji Mojanga.
    pub fn nazwa_mojang(&self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::Linux => "linux",
            Os::Osx => "osx",
        }
    }
}

pub fn biezacy_os() -> Os {
    if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::Osx
    } else {
        Os::Linux
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JavaError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("nie udało się rozpakować Javy: {0}")]
    Rozpakowanie(String),
    #[error("po rozpakowaniu nie znaleziono pliku wykonywalnego Javy w {0}")]
    BrakBinarki(String),
    #[error("rozpakowana Java jest niepełna — brakuje lib/jvm.cfg w {0}")]
    Niepelna(String),
    #[error("błąd operacji na pliku: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct JavaInstall {
    pub java_bin: PathBuf,
}

pub fn adoptium_url(major: u32, os: Os, arch: &str) -> String {
    let os_seg = match os {
        Os::Windows => "windows",
        Os::Linux => "linux",
        Os::Osx => "mac",
    };
    format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/{os_seg}/{arch}/jre/hotspot/normal/eclipse")
}

pub fn nazwa_javy(os: Os) -> &'static str {
    // javaw.exe nie otwiera okna konsoli obok gry.
    if os == Os::Windows {
        "javaw.exe"
    } else {
        "java"
    }
}

/// Zapewnia obecność JRE w `<root>/java/<major>` i zwraca ścieżkę do binarki.
pub async fn ensure(
    root: &Path,
    major: u32,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<JavaInstall, JavaError> {
    let os = biezacy_os();
    let katalog = root.join("java").join(major.to_string());

    if let Some(b) = znajdz_binarke(&katalog, os) {
        if instalacja_kompletna(&b) {
            return Ok(JavaInstall { java_bin: b });
        }
        // Niepelne JRE wyglada jak zainstalowane, wiec bez tego kasowania launcher
        // przyjmowalby je przy kazdym kolejnym starcie i nigdy sam z tego nie wyszedl.
        on(Progress::trwa(
            Stage::Java,
            "Java jest niepełna — pobieram od nowa…",
        ));
        std::fs::remove_dir_all(&katalog)?;
    }

    let rozszerzenie = if os == Os::Windows { "zip" } else { "tar.gz" };
    let archiwum = root.join(format!("java-{major}.{rozszerzenie}"));
    dl.fetch_one_obserwowane(
        &DownloadSpec {
            urls: vec![adoptium_url(major, os, "x64")],
            dest: archiwum.clone(),
            // Adoptium nie podaje hasha w samym przekierowaniu, wiec ufamy TLS-owi,
            // a wynik weryfikujemy przez znalezienie dzialajacej binarki nizej.
            expect: Expect::Any,
        },
        Some((Stage::Java, format!("Java {major}"), on.clone())),
    )
    .await?;

    // Rozpakowanie ~180 MB potrafi zajac kilka sekund i nie da sie go zmierzyc
    // z zewnatrz — lepiej powiedziec, co sie dzieje, niz zostawic pasek w miejscu.
    on(Progress::trwa(Stage::Java, "Rozpakowuję Javę…"));
    // Rozpakowujemy obok, a na miejsce docelowe przenosimy dopiero kompletny wynik.
    // Przy rozpakowywaniu wprost do celu przerwanie w polowie zostawia pol-JRE
    // dokladnie tam, gdzie launcher szuka gotowej Javy.
    let tymczasowy = root.join("java").join(format!("{major}.czesciowy"));
    let _ = std::fs::remove_dir_all(&tymczasowy);
    std::fs::create_dir_all(&tymczasowy)?;

    let wynik = rozpakuj_i_sprawdz(&archiwum, &tymczasowy, os);
    let _ = std::fs::remove_file(&archiwum);
    if let Err(e) = wynik {
        let _ = std::fs::remove_dir_all(&tymczasowy);
        return Err(e);
    }

    let _ = std::fs::remove_dir_all(&katalog);
    std::fs::rename(&tymczasowy, &katalog)?;

    let bin = znajdz_binarke(&katalog, os)
        .ok_or_else(|| JavaError::BrakBinarki(katalog.display().to_string()))?;

    on(Progress::pliki(Stage::Java, 1, 1, "Java gotowa"));
    Ok(JavaInstall { java_bin: bin })
}

/// Archiwa Adoptium mają jeden katalog na wierzchu (np. `jdk-21.0.12+7-jre`),
/// więc binarki szukamy o poziom głębiej, nie na sztywno.
///
/// Publiczne, bo Ustawienia pokazują wprost, której Javy launcher używa —
/// bez tego zakładka Java mówiłaby „pobranej przez launcher" i tyle.
pub fn znajdz_binarke(katalog: &Path, os: Os) -> Option<PathBuf> {
    let nazwa = nazwa_javy(os);
    let bezposrednio = katalog.join("bin").join(nazwa);
    if bezposrednio.is_file() {
        return Some(bezposrednio);
    }
    for w in std::fs::read_dir(katalog).ok()?.flatten() {
        let kandydat = w.path().join("bin").join(nazwa);
        if kandydat.is_file() {
            return Some(kandydat);
        }
    }
    None
}

/// Czy to pełne JRE, a nie resztka po przerwanym rozpakowywaniu.
///
/// Archiwum Adoptium rozpakowuje się alfabetycznie: `bin`, `conf`, `legal`,
/// `lib`, `release`. `bin/javaw.exe` wypada więc jako jeden z pierwszych plików
/// i sama jego obecność niczego nie dowodzi — JRE urwane w połowie ma binarkę,
/// ale nie ma `lib/jvm.cfg`, przez co JVM nie wstaje w ogóle:
/// „Error: could not open … \lib\jvm.cfg". Sprawdzamy więc dwie ostatnie
/// pozycje z tej listy, bo one padają pierwsze przy urwanym rozpakowaniu.
pub fn instalacja_kompletna(java_bin: &Path) -> bool {
    let Some(dom) = java_bin.parent().and_then(|bin| bin.parent()) else {
        return false;
    };
    dom.join("lib").join("jvm.cfg").is_file() && dom.join("release").is_file()
}

/// Rozpakowuje archiwum i potwierdza, że wyszło z tego pełne JRE.
fn rozpakuj_i_sprawdz(archiwum: &Path, cel: &Path, os: Os) -> Result<(), JavaError> {
    if os == Os::Windows {
        rozpakuj_zip(archiwum, cel)?;
    } else {
        rozpakuj_targz(archiwum, cel)?;
    }
    let bin =
        znajdz_binarke(cel, os).ok_or_else(|| JavaError::BrakBinarki(cel.display().to_string()))?;
    if !instalacja_kompletna(&bin) {
        return Err(JavaError::Niepelna(cel.display().to_string()));
    }
    Ok(())
}

fn rozpakuj_zip(archiwum: &Path, cel: &Path) -> Result<(), JavaError> {
    let plik = std::fs::File::open(archiwum)?;
    let mut zip = zip::ZipArchive::new(plik).map_err(|e| JavaError::Rozpakowanie(e.to_string()))?;
    zip.extract(cel)
        .map_err(|e| JavaError::Rozpakowanie(e.to_string()))?;
    Ok(())
}

fn rozpakuj_targz(archiwum: &Path, cel: &Path) -> Result<(), JavaError> {
    let plik = std::fs::File::open(archiwum)?;
    let dekompresor = flate2::read::GzDecoder::new(plik);
    let mut archiwum = tar::Archive::new(dekompresor);
    archiwum
        .unpack(cel)
        .map_err(|e| JavaError::Rozpakowanie(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buduje_adres_adoptium() {
        // Sprawdzone empirycznie: oba adresy zwracaja HTTP 307 z przekierowaniem
        // na wydanie Temurina na GitHubie.
        assert_eq!(
            adoptium_url(21, Os::Windows, "x64"),
            "https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jre/hotspot/normal/eclipse"
        );
        assert_eq!(
            adoptium_url(21, Os::Linux, "x64"),
            "https://api.adoptium.net/v3/binary/latest/21/ga/linux/x64/jre/hotspot/normal/eclipse"
        );
    }

    /// Odtwarza usterke zgloszona przez testera: rozpakowywanie stanelo miedzy
    /// `legal` a `lib`, wiec binarka byla, a JVM nie mial `lib/jvm.cfg`.
    /// Sama obecnosc binarki nie moze wystarczac za dowod gotowosci.
    #[test]
    fn urwane_jre_nie_uchodzi_za_kompletne() {
        let tmp = std::env::temp_dir().join("chmurka-test-urwane-jre");
        let _ = std::fs::remove_dir_all(&tmp);
        let dom = tmp.join("jdk-21.0.12.1+1-jre");
        for katalog in ["bin", "conf", "legal"] {
            std::fs::create_dir_all(dom.join(katalog)).unwrap();
        }
        let bin = dom.join("bin").join("java");
        std::fs::write(&bin, b"").unwrap();

        assert!(
            !instalacja_kompletna(&bin),
            "JRE bez lib/jvm.cfg zostalo uznane za gotowe"
        );

        std::fs::create_dir_all(dom.join("lib")).unwrap();
        std::fs::write(dom.join("lib").join("jvm.cfg"), b"-server KNOWN\n").unwrap();
        std::fs::write(dom.join("release"), b"JAVA_VERSION=\"21\"\n").unwrap();
        assert!(instalacja_kompletna(&bin), "pelne JRE zostalo odrzucone");

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn nazwa_binarki_zalezy_od_systemu() {
        assert_eq!(nazwa_javy(Os::Windows), "javaw.exe");
        assert_eq!(nazwa_javy(Os::Linux), "java");
    }
}
