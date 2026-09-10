mod pw;

use anyhow::{bail, Context, Result};
use chmurka_core::hash::{sha1_file, sha512_file};
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "chmurka", about = "Narzędzia Chmurkowego Launchera")]
struct Cli {
    #[command(subcommand)]
    polecenie: Polecenie,
}

#[derive(Subcommand)]
enum Polecenie {
    /// Buduje manifest i katalog plików do wypchnięcia na GitHub Pages.
    PackBuild {
        /// Katalog `minecraft` instancji PrismLaunchera.
        #[arg(long)]
        instance: PathBuf,
        /// Katalog wynikowy.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Bazowy adres, pod którym będą leżeć nasze pliki.
        #[arg(long)]
        base_url: String,
        /// Wersja paczki, np. 2026.09.09-1.
        #[arg(long)]
        version: String,
        /// Najnowsza wersja launchera — launcher porownuje ja ze swoja
        /// i informuje gracza o dostepnej aktualizacji.
        #[arg(long, default_value = "0.4.5")]
        launcher_version: String,
    },
    /// Instaluje wszystko do wskazanego katalogu danych.
    Install {
        #[arg(long)]
        manifest: String,
        #[arg(long, default_value = "data")]
        data: PathBuf,
    },
    /// Sprawdza logowanie kontem Microsoft i wypisuje nick oraz UUID.
    Login {
        /// Identyfikator aplikacji. Domyślnie ten sam, którego używa oficjalny launcher.
        #[arg(long, default_value = "00000000402b5328")]
        client_id: String,
    },
    /// Instaluje i uruchamia grę na koncie offline.
    Run {
        #[arg(long)]
        manifest: String,
        #[arg(long, default_value = "data")]
        data: PathBuf,
        #[arg(long)]
        nick: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().polecenie {
        Polecenie::PackBuild {
            instance,
            out,
            base_url,
            version,
            launcher_version,
        } => pack_build(&instance, &out, &base_url, &version, &launcher_version),
        Polecenie::Install { manifest, data } => {
            przygotuj(&manifest, &data).await?;
            println!("Gotowe.");
            Ok(())
        }
        Polecenie::Login { client_id } => logowanie(&client_id).await,
        Polecenie::Run {
            manifest,
            data,
            nick,
        } => {
            let (wersja, java) = przygotuj(&manifest, &data).await?;
            let konto = chmurka_core::auth::offline::offline_account(&nick);
            let mut cmd = chmurka_core::launch::build_command(&chmurka_core::launch::LaunchParams {
                java: &java.java_bin,
                mc_dir: &data.join("mc"),
                game_dir: &data.join("instance"),
                version: &wersja,
                account: &konto,
                min_mb: 512,
                max_mb: 4096,
                dodatkowe: &[],
            })?;
            println!("Uruchamiam grę…");
            let status = cmd.status()?;
            println!("Gra zakończyła się kodem {:?}", status.code());
            Ok(())
        }
    }
}

/// Ręczna weryfikacja logowania Microsoft. To najbardziej niepewny fragment
/// projektu — używamy identyfikatora aplikacji oficjalnego launchera, który
/// Microsoft może w każdej chwili odciąć.
async fn logowanie(client_id: &str) -> Result<()> {
    use chmurka_core::auth::msa;

    let kod = msa::begin(client_id).await?;
    println!("Wejdź na {} i wpisz kod:", kod.verification_uri);
    println!("\n    {}\n", kod.user_code);
    println!("Czekam (kod ważny {} s)…", kod.expires_in_s);

    let koniec = std::time::Instant::now() + std::time::Duration::from_secs(kod.expires_in_s);
    let mut odstep = kod.interval_s.max(1);
    loop {
        if std::time::Instant::now() > koniec {
            bail!("kod wygasł");
        }
        tokio::time::sleep(std::time::Duration::from_secs(odstep)).await;
        match msa::poll_once(client_id, &kod.device_code).await? {
            msa::PollResult::Czekamy => {}
            // Microsoft prosi o wolniejsze odpytywanie — zignorowanie tego
            // konczy sie zablokowaniem calej sesji logowania.
            msa::PollResult::Zwolnij => odstep += 5,
            msa::PollResult::Gotowe(t) => {
                let konto = msa::zaloguj_minecraft(&t).await?;
                println!("Zalogowano: {} ({})", konto.name, konto.uuid);
                return Ok(());
            }
        }
    }
}

/// Wspólna część install i run: pobiera manifest i doprowadza instalację do stanu gotowego.
async fn przygotuj(
    adres_manifestu: &str,
    data: &Path,
) -> Result<(
    chmurka_core::version::VersionJson,
    chmurka_core::java::JavaInstall,
)> {
    use chmurka_core::*;
    use std::sync::Arc;

    let postep: Arc<dyn Fn(progress::Progress) + Send + Sync> = Arc::new(|p| {
        // Co setny plik wystarczy — przy 4000 zasobach pełny log zalewa konsolę.
        let warto = match p.jednostka {
            progress::Jednostka::Pliki => p.done % 100 == 0 || p.done == p.total,
            // Co cwierc pobrania — widac ruch, a konsola nie tonie w meldunkach.
            progress::Jednostka::Bajty => {
                let cwiartka = (p.total / 4).max(1);
                p.done == p.total || p.done / cwiartka != (p.done.saturating_sub(400 * 1024)) / cwiartka
            }
            progress::Jednostka::Nieznana => true,
        };
        if warto {
            let licznik = p.licznik();
            if licznik.is_empty() {
                println!("[{}] {}", p.stage.opis(), p.label);
            } else {
                println!("[{}] {} {}", p.stage.opis(), licznik, p.label);
            }
        }
    });

    let tekst = reqwest::get(adres_manifestu).await?.text().await?;
    let m = manifest::parse(&tekst)?;
    println!("Paczka {} wersja {}", m.pack.name, m.pack.version);

    let dl = net::Downloader::new(8);
    let mc = data.join("mc");
    let instancja = data.join("instance");
    std::fs::create_dir_all(&instancja)?;

    let java = java::ensure(data, m.java.major, &dl, postep.clone()).await?;
    let profil = game_install::ensure_loader(
        &mc,
        &java,
        &m.pack.minecraft,
        &m.pack.loader.kind,
        &m.pack.loader.version,
        &dl,
        postep.clone(),
    )
    .await?;
    let wersja = version::load(&mc.join("versions"), &profil)?;

    game_install::ensure_libraries(&mc, &wersja, &dl, postep.clone()).await?;
    game_install::ensure_assets(&mc, &wersja, &dl, postep.clone()).await?;

    if !m.narzedzia.is_empty() {
        narzedzia::zapewnij(&instancja, &m.narzedzia, &dl, postep.clone()).await?;
    }

    let sciezka_stanu = data.join("state.json");
    let mut stan = state::State::load(&sciezka_stanu);
    let akcje = pack_sync::plan(&m, &stan, &pack_sync::DiskProbe::new(&instancja));
    let notatki = pack_sync::apply(&m, &instancja, &mut stan, &akcje, &dl, postep.clone()).await?;
    stan.save(&sciezka_stanu)?;
    for n in notatki {
        println!("  {n}");
    }

    Ok((wersja, java))
}

fn pack_build(
    instance: &Path,
    out: &Path,
    base_url: &str,
    version: &str,
    launcher_version: &str,
) -> Result<()> {
    let mods = instance.join("mods");
    let index = mods.join(".index");
    if !index.is_dir() {
        bail!(
            "brak {} — czy to na pewno instancja PrismLaunchera?",
            index.display()
        );
    }

    // 1. Metadane modów.
    let mut wpisy_meta: BTreeMap<String, pw::PwEntry> = BTreeMap::new();
    for w in std::fs::read_dir(&index)?.flatten() {
        let p = w.path();
        if p.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let tekst = std::fs::read_to_string(&p)?;
        let e = pw::parse_pw(&tekst).with_context(|| format!("plik {}", p.display()))?;
        wpisy_meta.insert(e.filename.clone(), e);
    }

    // 2. Jary na dysku, z pominięciem .bak i .disabled.
    let mut jary: Vec<String> = Vec::new();
    for w in std::fs::read_dir(&mods)?.flatten() {
        let p = w.path();
        if p.is_dir() {
            continue;
        }
        let nazwa = p.file_name().unwrap().to_string_lossy().to_string();
        if nazwa.ends_with(".jar") {
            jary.push(nazwa);
        }
    }
    jary.sort();

    // 3. Rozjazd między metadanymi a plikami przerywa budowanie.
    // Mod bez metadanych nie ma skad byc pobrany, wiec cicha zgoda dalaby
    // testerom niekompletna paczke.
    let brakujace: Vec<&String> = jary.iter().filter(|j| !wpisy_meta.contains_key(*j)).collect();
    if !brakujace.is_empty() {
        bail!("mody bez metadanych, nie wiadomo skąd je pobrać: {brakujace:?}");
    }

    let magazyn = out.join("files");
    std::fs::create_dir_all(&magazyn)?;

    let mut files = Vec::new();
    let mut wlasne_mody: Vec<String> = Vec::new();

    for nazwa in &jary {
        let meta = &wpisy_meta[nazwa];
        let sciezka = mods.join(nazwa);
        // Hash z metadanych bywa sha1 (CurseForge), wiec do manifestu liczymy
        // wlasny sha512 z pliku na dysku — to jedyne zrodlo prawdy o tym, co gramy.
        let sha512 = sha512_file(&sciezka)?;

        // Czy plik na dysku to naprawdę ten sam plik, który leży pod adresem
        // źródłowym? Jeśli mod został lokalnie załatany, wskazanie CDN-u dałoby
        // testerom inną paczkę niż ta, którą utrzymujący faktycznie sprawdził.
        let zgodny_ze_zrodlem = match (&meta.hash, meta.hash_format.as_deref()) {
            (Some(h), Some("sha512")) => sha512.eq_ignore_ascii_case(h),
            (Some(h), Some("sha1")) => sha1_file(&sciezka)?.eq_ignore_ascii_case(h),
            // Bez hasha nie mamy jak tego sprawdzić, więc hostujemy u siebie.
            _ => false,
        };

        let url = if zgodny_ze_zrodlem {
            match (&meta.url, meta.cf_file) {
                (Some(u), _) => u.clone(),
                (None, Some(fid)) => pw::curseforge_url(fid, nazwa),
                (None, None) => bail!(
                    "mod {nazwa} (projekt CurseForge {:?}) nie ma ani adresu, ani identyfikatora pliku",
                    meta.cf_project
                ),
            }
        } else {
            wlasne_mody.push(nazwa.clone());
            do_magazynu(&magazyn, &sciezka, &sha512)?;
            adres_wlasny(base_url, &sha512)
        };

        files.push(serde_json::json!({
            "path": format!("mods/{nazwa}"),
            "size": std::fs::metadata(&sciezka)?.len(),
            "sha512": sha512,
            "policy": "mirror",
            "urls": [url],
        }));
    }

    if !wlasne_mody.is_empty() {
        eprintln!(
            "\nUWAGA: {} mod(ów) różni się od pliku pod adresem źródłowym — hostuję je u siebie,",
            wlasne_mody.len()
        );
        eprintln!("żeby testerzy dostali dokładnie to, co masz w instancji:");
        for m in &wlasne_mody {
            eprintln!("  - {m}");
        }
        eprintln!("Jeśli to niezamierzone, pobierz te mody na nowo w PrismLauncherze.\n");
    }

    // 4. Configi i options.txt trafiaja do naszego magazynu adresowanego trescia.
    let mut nasze: Vec<(String, PathBuf, &'static str)> = Vec::new();
    zbierz_configi(&instance.join("config"), "config", &mut nasze);
    let opcje = instance.join("options.txt");
    if opcje.is_file() {
        nasze.push(("options.txt".to_string(), opcje, "seed"));
    }

    for (rel, zrodlo, polityka) in nasze {
        let sha512 = sha512_file(&zrodlo)?;
        do_magazynu(&magazyn, &zrodlo, &sha512)?;
        files.push(serde_json::json!({
            "path": rel,
            "size": std::fs::metadata(&zrodlo)?.len(),
            "sha512": sha512,
            "policy": polityka,
            "urls": [adres_wlasny(base_url, &sha512)],
        }));
    }

    let manifest = serde_json::json!({
        "schema": 1,
        "pack": {
            "name": "Chmurkowy Serwer",
            "edition": "edycja 2026/2027",
            "version": version,
            "minecraft": "1.21.1",
            "loader": { "kind": "neoforge", "version": "21.1.249" }
        },
        "java": { "major": 21, "distribution": "temurin" },
        "memory": { "min_mb": 512, "max_mb": 4096 },
        "auth": { "msa_client_id": "00000000402b5328" },
        "launcher": {
            "latest_version": launcher_version,
            "urls": {
                "windows-x64": format!("https://github.com/ttk0721/chmurkowy-launcher/releases/latest/download/ChmurkowyLauncher-windows-x64.exe"),
                "linux-x64": format!("https://github.com/ttk0721/chmurkowy-launcher/releases/latest/download/ChmurkowyLauncher-linux-x64")
            }
        },
        "mirror_dirs": ["mods"],
        // Zewnetrzne programy wymagane przez Create: Harmonics. Adresy sa te same,
        // z ktorych korzysta sam mod — niczego nie hostujemy u siebie.
        "narzedzia": [
            {
                "nazwa": "yt-dlp",
                "katalog": "audio_providers/yt-dlp",
                "pliki": { "linux-x64": "yt-dlp", "windows-x64": "yt-dlp.exe" },
                "zrodla": {
                    "linux-x64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux",
                    "windows-x64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
                }
            },
            {
                "nazwa": "ffmpeg",
                "katalog": "audio_providers/ffmpeg",
                "zrodla": {
                    "linux-x64": "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-lgpl.tar.xz",
                    "windows-x64": "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-lgpl.zip"
                }
            }
        ],
        "files": files,
    });

    let tekst = serde_json::to_string_pretty(&manifest)?;
    // Weryfikacja wlasnego wyniku: manifest musi przejsc walidacje launchera.
    chmurka_core::manifest::parse(&tekst)
        .context("wygenerowany manifest nie przechodzi walidacji")?;
    std::fs::write(out.join("manifest.json"), &tekst)?;

    println!(
        "Zapisano {} wpisów do {}",
        manifest["files"].as_array().unwrap().len(),
        out.display()
    );
    Ok(())
}

/// Kładzie plik w magazynie adresowanym treścią: `files/<2 znaki>/<pełny hash>`.
/// Ten sam plik w kolejnych wersjach paczki zajmuje miejsce tylko raz.
fn do_magazynu(magazyn: &Path, zrodlo: &Path, sha512: &str) -> Result<()> {
    let podkatalog = magazyn.join(&sha512[0..2]);
    std::fs::create_dir_all(&podkatalog)?;
    let cel = podkatalog.join(sha512);
    if !cel.exists() {
        std::fs::copy(zrodlo, &cel)?;
    }
    Ok(())
}

fn adres_wlasny(base_url: &str, sha512: &str) -> String {
    format!(
        "{}/files/{}/{}",
        base_url.trim_end_matches('/'),
        &sha512[0..2],
        sha512
    )
}

/// Czy to plik roboczy, który powstał u utrzymującego i nie ma czego szukać
/// w paczce?
///
/// NeoForge odkłada obok configów kopie `*.toml_backup1`, `*.toml_backup2`
/// i tak dalej. Trafiały do manifestu razem z resztą i tylko zajmowały miejsce,
/// a dwie kopie o wspólnym trzonie nazwy wywracały pobieranie u testerów.
fn smiec_roboczy(nazwa: &str) -> bool {
    nazwa.ends_with(".bak")
        || nazwa.ends_with(".part")
        || nazwa.ends_with('~')
        || nazwa.contains(".toml_backup")
        || nazwa.contains(".json_backup")
}

fn zbierz_configi(katalog: &Path, prefiks: &str, out: &mut Vec<(String, PathBuf, &'static str)>) {
    let Ok(wpisy) = std::fs::read_dir(katalog) else {
        return;
    };
    for w in wpisy.flatten() {
        let nazwa = w.file_name().to_string_lossy().to_string();
        if nazwa.starts_with('.') || smiec_roboczy(&nazwa) {
            continue;
        }
        let rel = format!("{prefiks}/{nazwa}");
        if w.path().is_dir() {
            zbierz_configi(&w.path(), &rel, out);
        } else {
            out.push((rel, w.path(), "smart"));
        }
    }
}

#[cfg(test)]
mod testy_paczowania {
    use super::*;

    /// Te dwa pliki naprawde pojechaly do testerow i wywrocily im instalacje.
    #[test]
    fn kopie_configow_neoforge_nie_jada_do_paczki() {
        assert!(smiec_roboczy("chloride-client.toml_backup1"));
        assert!(smiec_roboczy("chloride-client.toml_backup2"));
    }

    #[test]
    fn zwykle_configi_zostaja() {
        assert!(!smiec_roboczy("chloride-client.toml"));
        assert!(!smiec_roboczy("iris.properties"));
        assert!(!smiec_roboczy("options.txt"));
    }

    #[test]
    fn inne_pliki_robocze_tez_odpadaja() {
        assert!(smiec_roboczy("create.jar.bak"));
        assert!(smiec_roboczy("cos.part"));
        assert!(smiec_roboczy("notatki.txt~"));
    }
}
