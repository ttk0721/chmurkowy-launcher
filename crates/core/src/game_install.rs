use crate::java::JavaInstall;
use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::progress::{Progress, Stage};
use crate::version::{rules_allow, VersionJson};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("instalator {loader} zakończył się kodem {kod}; koniec wyjścia:\n{wyjscie}")]
    Instalator {
        loader: String,
        kod: i32,
        wyjscie: String,
    },
    #[error("nie udało się uruchomić instalatora: {0}")]
    Uruchomienie(std::io::Error),
    #[error("błąd operacji na pliku: {0}")]
    Io(#[from] std::io::Error),
    #[error("nieprawidłowy indeks zasobów: {0}")]
    IndeksZasobow(String),
}

fn neoforge_installer_url(wersja: &str) -> String {
    format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{wersja}/neoforge-{wersja}-installer.jar")
}

/// Ostatnie `n` znaków tekstu — instalator potrafi wypisać megabajty,
/// a w komunikacie błędu liczy się tylko końcówka.
fn ogon(s: &str, n: usize) -> String {
    let znaki: Vec<char> = s.chars().collect();
    let od = znaki.len().saturating_sub(n);
    znaki[od..].iter().collect()
}

/// Uruchamia oficjalny instalator loadera w trybie bezobsługowym.
///
/// To on patchuje `client.jar` (procesor `binarypatcher`), więc nie musimy
/// odtwarzać tego procesu u siebie ani redystrybuować plików Mojanga.
pub async fn ensure_loader(
    mc_dir: &Path,
    java: &JavaInstall,
    loader_kind: &str,
    loader_version: &str,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<String, InstallError> {
    let profil = format!("{loader_kind}-{loader_version}");
    let json_profilu = mc_dir
        .join("versions")
        .join(&profil)
        .join(format!("{profil}.json"));
    if json_profilu.is_file() {
        return Ok(profil);
    }

    on(Progress {
        stage: Stage::Loader,
        done: 0,
        total: 1,
        bytes: 0,
        label: profil.clone(),
    });

    let instalator = mc_dir.join(format!("{profil}-installer.jar"));
    dl.fetch_one(&DownloadSpec {
        urls: vec![neoforge_installer_url(loader_version)],
        dest: instalator.clone(),
        expect: Expect::Any,
    })
    .await?;

    // Instalator odmawia pracy bez tego pliku — sprawdza go, zanim cokolwiek zrobi.
    std::fs::create_dir_all(mc_dir)?;
    let profile = mc_dir.join("launcher_profiles.json");
    if !profile.exists() {
        std::fs::write(
            &profile,
            br#"{"profiles":{},"selectedProfile":"","clientToken":"","authenticationDatabase":{},"launcherVersion":{"name":"","format":21},"settings":{}}"#,
        )?;
    }

    let wyjscie = std::process::Command::new(&java.java_bin)
        .arg("-jar")
        .arg(&instalator)
        .arg("--install-client")
        .arg(mc_dir)
        .current_dir(mc_dir)
        .output()
        .map_err(InstallError::Uruchomienie)?;

    if !wyjscie.status.success() {
        let mut tekst = String::from_utf8_lossy(&wyjscie.stdout).into_owned();
        tekst.push_str(&String::from_utf8_lossy(&wyjscie.stderr));
        return Err(InstallError::Instalator {
            loader: profil,
            kod: wyjscie.status.code().unwrap_or(-1),
            wyjscie: ogon(&tekst, 2000),
        });
    }

    let _ = std::fs::remove_file(&instalator);
    let _ = std::fs::remove_file(mc_dir.join(format!("{profil}-installer.jar.log")));

    if !json_profilu.is_file() {
        return Err(InstallError::Instalator {
            loader: profil,
            kod: 0,
            wyjscie: "instalator zakończył się sukcesem, ale nie powstał profil wersji".into(),
        });
    }
    on(Progress {
        stage: Stage::Loader,
        done: 1,
        total: 1,
        bytes: 0,
        label: "NeoForge gotowy".into(),
    });
    Ok(profil)
}

pub async fn ensure_libraries(
    mc_dir: &Path,
    v: &VersionJson,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<(), InstallError> {
    let os = crate::java::biezacy_os();
    let libs = mc_dir.join("libraries");
    let mut specyfikacje = Vec::new();

    for l in &v.libraries {
        if !rules_allow(&l.rules, os) {
            continue;
        }
        let Some(art) = l.downloads.as_ref().and_then(|d| d.artifact.as_ref()) else {
            // Instalator NeoForge kładzie część bibliotek sam, bez wpisu downloads.
            continue;
        };
        specyfikacje.push(DownloadSpec {
            urls: vec![art.url.clone()],
            dest: libs.join(&art.path),
            expect: Expect::Sha1(art.sha1.clone()),
        });
    }

    // Klient vanilla — potrzebny, bo NeoForge patchuje go, a nie zastepuje.
    if let Some(klient) = v.downloads.as_ref().and_then(|d| d.client.as_ref()) {
        specyfikacje.push(DownloadSpec {
            urls: vec![klient.url.clone()],
            dest: mc_dir
                .join("versions")
                .join(&v.id)
                .join(format!("{}.jar", v.id)),
            expect: Expect::Sha1(klient.sha1.clone()),
        });
    }

    dl.fetch_many(specyfikacje, Stage::Libraries, on).await?;
    Ok(())
}

pub async fn ensure_assets(
    mc_dir: &Path,
    v: &VersionJson,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<(), InstallError> {
    let Some(idx) = v.asset_index.as_ref() else {
        return Ok(());
    };
    let assets = mc_dir.join("assets");
    let plik_indeksu = assets.join("indexes").join(format!("{}.json", idx.id));

    dl.fetch_one(&DownloadSpec {
        urls: vec![idx.url.clone()],
        dest: plik_indeksu.clone(),
        expect: Expect::Sha1(idx.sha1.clone()),
    })
    .await?;

    #[derive(serde::Deserialize)]
    struct Indeks {
        objects: BTreeMap<String, Obiekt>,
    }
    #[derive(serde::Deserialize)]
    struct Obiekt {
        hash: String,
    }

    let tekst = std::fs::read_to_string(&plik_indeksu)?;
    let indeks: Indeks =
        serde_json::from_str(&tekst).map_err(|e| InstallError::IndeksZasobow(e.to_string()))?;

    let specyfikacje: Vec<DownloadSpec> = indeks
        .objects
        .values()
        .map(|o| DownloadSpec {
            urls: vec![format!(
                "https://resources.download.minecraft.net/{}/{}",
                &o.hash[0..2],
                o.hash
            )],
            dest: assets.join("objects").join(&o.hash[0..2]).join(&o.hash),
            expect: Expect::Sha1(o.hash.clone()),
        })
        .collect();

    dl.fetch_many(specyfikacje, Stage::Assets, on).await?;
    Ok(())
}
