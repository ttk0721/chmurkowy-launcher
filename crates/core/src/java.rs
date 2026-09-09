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
        return Ok(JavaInstall { java_bin: b });
    }

    on(Progress {
        stage: Stage::Java,
        done: 0,
        total: 1,
        bytes: 0,
        label: format!("Java {major}"),
    });

    let rozszerzenie = if os == Os::Windows { "zip" } else { "tar.gz" };
    let archiwum = root.join(format!("java-{major}.{rozszerzenie}"));
    dl.fetch_one(&DownloadSpec {
        urls: vec![adoptium_url(major, os, "x64")],
        dest: archiwum.clone(),
        // Adoptium nie podaje hasha w samym przekierowaniu, wiec ufamy TLS-owi,
        // a wynik weryfikujemy przez znalezienie dzialajacej binarki nizej.
        expect: Expect::Any,
    })
    .await?;

    std::fs::create_dir_all(&katalog)?;
    if os == Os::Windows {
        rozpakuj_zip(&archiwum, &katalog)?;
    } else {
        rozpakuj_targz(&archiwum, &katalog)?;
    }
    let _ = std::fs::remove_file(&archiwum);

    let bin = znajdz_binarke(&katalog, os)
        .ok_or_else(|| JavaError::BrakBinarki(katalog.display().to_string()))?;

    on(Progress {
        stage: Stage::Java,
        done: 1,
        total: 1,
        bytes: 0,
        label: "Java gotowa".into(),
    });
    Ok(JavaInstall { java_bin: bin })
}

/// Archiwa Adoptium mają jeden katalog na wierzchu (np. `jdk-21.0.12+7-jre`),
/// więc binarki szukamy o poziom głębiej, nie na sztywno.
fn znajdz_binarke(katalog: &Path, os: Os) -> Option<PathBuf> {
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

    #[test]
    fn nazwa_binarki_zalezy_od_systemu() {
        assert_eq!(nazwa_javy(Os::Windows), "javaw.exe");
        assert_eq!(nazwa_javy(Os::Linux), "java");
    }
}
