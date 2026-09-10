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
    #[error("instalator {loader} nie zdołał połączyć się z serwerami; koniec wyjścia:\n{wyjscie}")]
    InstalatorSiec { loader: String, wyjscie: String },
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

/// Spis wszystkich wersji Minecrafta. Dwa adresy z tą samą treścią —
/// gdy pierwszy host nie odpowiada, `Downloader` sięga po drugi.
const SPIS_WERSJI: [&str; 2] = [
    "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
    "https://launchermeta.mojang.com/mc/game/version_manifest_v2.json",
];

#[derive(serde::Deserialize)]
struct SpisWersji {
    versions: Vec<WpisSpisu>,
}

#[derive(serde::Deserialize)]
struct WpisSpisu {
    id: String,
    url: String,
    sha1: String,
}

/// Czy wyjście instalatora mówi o problemie z połączeniem?
///
/// Klasyfikujemy pełne wyjście, razem z ramkami stosu — nazwa wyjątku bywa
/// ucięta, ale `timedFinishConnect` czy `DownloadUtils` zostają w końcówce.
fn wyglada_na_siec(wyjscie: &str) -> bool {
    const TROPY: [&str; 8] = [
        "SocketTimeoutException",
        "UnknownHostException",
        "ConnectException",
        "SSLHandshakeException",
        "timedFinishConnect",
        "Failed to establish connection to",
        "DownloadUtils",
        "downloadManifest",
    ];
    TROPY.iter().any(|t| wyjscie.contains(t))
}

/// Usuwa z wyjścia instalatora ramki stosu Javy.
///
/// Rodzic dziesięciolatka nie zrobi nic z linią `at java.base/sun.nio...`,
/// a to właśnie one zapełniają całe okno błędu i spychają poza ekran jedyne
/// zdanie, które cokolwiek znaczy.
fn bez_ramek_stosu(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("at "))
        .filter(|l| !l.trim_start().starts_with("... "))
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Kładzie na dysku profil i plik czystego Minecrafta, zanim ruszy instalator.
///
/// Instalator umie pobrać je sam, ale robi to własnym połączeniem z limitem
/// pięciu sekund wpisanym na sztywno (`DownloadUtils.getConnection`). Na wolnej
/// maszynie albo przy niesprawnym IPv6 ten limit mija, zanim cokolwiek
/// przyjdzie, i instalacja pada timeoutem — mimo że przeglądarka na tym samym
/// komputerze otwiera strony normalnie. Gdy oba pliki już leżą na miejscu,
/// instalator sprawdza `versions/<wersja>/<wersja>.jar`, widzi że jest,
/// i pomija całą tę ścieżkę. Problem znika u źródła, a pobieraniem zajmuje się
/// nasz `Downloader`, który ma powtórki i zapasowe adresy.
async fn zasiej_czysta_wersje(
    mc_dir: &Path,
    minecraft: &str,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<(), InstallError> {
    let katalog = mc_dir.join("versions").join(minecraft);
    let json = katalog.join(format!("{minecraft}.json"));
    let jar = katalog.join(format!("{minecraft}.jar"));
    if json.is_file() && jar.is_file() {
        return Ok(());
    }

    on(Progress::trwa(Stage::Loader, "Pobieram pliki Minecrafta"));

    // Spis nie ma znanego z góry hasha, więc `Expect::Any` przepuściłby stary
    // plik z dysku. Kasujemy go, żeby zawsze czytać świeżą listę wersji.
    let spis_plik = mc_dir.join("version_manifest_v2.json");
    let _ = std::fs::remove_file(&spis_plik);
    dl.fetch_one(&DownloadSpec {
        urls: SPIS_WERSJI.iter().map(|s| s.to_string()).collect(),
        dest: spis_plik.clone(),
        expect: Expect::Any,
    })
    .await?;

    let tekst = std::fs::read_to_string(&spis_plik)?;
    let spis: SpisWersji =
        serde_json::from_str(&tekst).map_err(|e| InstallError::IndeksZasobow(e.to_string()))?;
    let Some(wpis) = spis.versions.into_iter().find(|w| w.id == minecraft) else {
        return Err(InstallError::IndeksZasobow(format!(
            "w spisie wersji Mojanga nie ma wersji {minecraft}"
        )));
    };

    dl.fetch_one(&DownloadSpec {
        urls: vec![wpis.url],
        dest: json.clone(),
        expect: Expect::Sha1(wpis.sha1),
    })
    .await?;

    let tekst = std::fs::read_to_string(&json)?;
    let wersja: VersionJson =
        serde_json::from_str(&tekst).map_err(|e| InstallError::IndeksZasobow(e.to_string()))?;
    let Some(klient) = wersja.downloads.as_ref().and_then(|d| d.client.as_ref()) else {
        return Err(InstallError::IndeksZasobow(format!(
            "profil wersji {minecraft} nie podaje pliku klienta"
        )));
    };

    dl.fetch_one_obserwowane(
        &DownloadSpec {
            urls: vec![klient.url.clone()],
            dest: jar,
            expect: Expect::Sha1(klient.sha1.clone()),
        },
        Some((Stage::Loader, format!("Minecraft {minecraft}"), on)),
    )
    .await?;

    let _ = std::fs::remove_file(&spis_plik);
    Ok(())
}

/// Uruchamia oficjalny instalator loadera w trybie bezobsługowym.
///
/// To on patchuje `client.jar` (procesor `binarypatcher`), więc nie musimy
/// odtwarzać tego procesu u siebie ani redystrybuować plików Mojanga.
pub async fn ensure_loader(
    mc_dir: &Path,
    java: &JavaInstall,
    minecraft: &str,
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

    let instalator = mc_dir.join(format!("{profil}-installer.jar"));
    dl.fetch_one_obserwowane(
        &DownloadSpec {
            urls: vec![neoforge_installer_url(loader_version)],
            dest: instalator.clone(),
            expect: Expect::Any,
        },
        Some((Stage::Loader, "Instalator NeoForge".to_string(), on.clone())),
    )
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

    // Pliki czystej wersji bierzemy na siebie — inaczej instalator poszedłby
    // po nie sam, swoim połączeniem z pięciosekundowym limitem.
    zasiej_czysta_wersje(mc_dir, minecraft, dl, on.clone()).await?;

    // Instalator mieli okolo minuty i nie raportuje postepu — mowimy o tym wprost,
    // zeby pasek stojacy w miejscu nie wygladal na zawieszenie.
    on(Progress::trwa(
        Stage::Loader,
        "Instaluję NeoForge — to potrwa około minuty",
    ));

    // Resztę plików instalator dociąga sam, tym samym krótkim limitem. Dajemy
    // trzy podejścia, a od drugiego każemy Javie trzymać się IPv4: zepsute IPv6
    // to najczęstszy powód, dla którego przeglądarka działa, a Java stoi na
    // timeoucie — sama czeka na adres, którego nie da się osiągnąć.
    const PROB: u32 = 3;
    let mut nieudane: Option<(i32, String)> = None;
    for proba in 1..=PROB {
        if proba > 1 {
            on(Progress::trwa(
                Stage::Loader,
                format!("Instaluję NeoForge — podejście {proba} z {PROB}"),
            ));
        }

        let mut polecenie = std::process::Command::new(&java.java_bin);
        if proba > 1 {
            polecenie.arg("-Djava.net.preferIPv4Stack=true");
        }
        let wyjscie = polecenie
            .arg("-jar")
            .arg(&instalator)
            .arg("--install-client")
            .arg(mc_dir)
            .current_dir(mc_dir)
            .output()
            .map_err(InstallError::Uruchomienie)?;

        if wyjscie.status.success() {
            nieudane = None;
            break;
        }

        let mut tekst = String::from_utf8_lossy(&wyjscie.stdout).into_owned();
        tekst.push_str(&String::from_utf8_lossy(&wyjscie.stderr));
        nieudane = Some((wyjscie.status.code().unwrap_or(-1), tekst));
    }

    if let Some((kod, tekst)) = nieudane {
        // Klasyfikujemy po pelnym wyjsciu, a pokazujemy juz bez ramek stosu.
        let czytelne = ogon(&bez_ramek_stosu(&tekst), 1200);
        return Err(if wyglada_na_siec(&tekst) {
            InstallError::InstalatorSiec {
                loader: profil,
                wyjscie: czytelne,
            }
        } else {
            InstallError::Instalator {
                loader: profil,
                kod,
                wyjscie: czytelne,
            }
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
    on(Progress::pliki(Stage::Loader, 1, 1, "NeoForge gotowy"));
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Prawdziwe wyjscie instalatora z komputera testera. Zwroc uwage, ze
    /// nazwa wyjatku jest urwana — do okna bledu trafia sama koncowka, wiec
    /// rozpoznanie nie moze na niej polegac.
    const TIMEOUT_INSTALATORA: &str = concat!(
        "ketImpl.timedFinishConnect(Unknown Source)\n",
        "\tat java.base/sun.nio.ch.NioSocketImpl.connect(Unknown Source)\n",
        "\tat java.base/java.net.Socket.connect(Unknown Source)\n",
        "\tat net.minecraftforge.installer.DownloadUtils.getConnection(DownloadUtils.java:128)\n",
        "\tat net.minecraftforge.installer.json.Util.getVanillaVersion(Util.java:73)\n",
        "\tat net.minecraftforge.installer.SimpleInstaller.main(SimpleInstaller.java:174)\n",
        "A problem installing was detected, install cannot continue"
    );

    #[test]
    fn timeout_instalatora_rozpoznajemy_jako_siec() {
        assert!(wyglada_na_siec(TIMEOUT_INSTALATORA));
    }

    #[test]
    fn zwykla_awaria_instalatora_to_nie_siec() {
        let inne = "java.lang.OutOfMemoryError: Java heap space\n\
                    A problem installing was detected, install cannot continue";
        assert!(!wyglada_na_siec(inne));
    }

    #[test]
    fn ramki_stosu_nie_trafiaja_do_okna_bledu() {
        let czytelne = bez_ramek_stosu(TIMEOUT_INSTALATORA);
        assert!(
            !czytelne.contains("java.base/"),
            "ramki stosu musza zniknac: {czytelne}"
        );
        assert!(czytelne.contains("A problem installing was detected"));
    }

    /// Dlatego klasyfikujemy pelny tekst, a skracamy dopiero to, co widzi
    /// gracz: caly slad sieci potrafi siedziec wylacznie w ramkach stosu,
    /// ktore z okna bledu wyrzucamy.
    #[test]
    fn po_obcieciu_ramek_slad_sieci_znika() {
        let tylko_ramki = concat!(
            "\tat java.base/sun.nio.ch.NioSocketImpl.timedFinishConnect(Unknown Source)\n",
            "A problem installing was detected, install cannot continue"
        );
        assert!(wyglada_na_siec(tylko_ramki));
        assert!(!wyglada_na_siec(&bez_ramek_stosu(tylko_ramki)));
    }
}
