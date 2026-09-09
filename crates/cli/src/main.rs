mod pw;

use anyhow::{bail, Context, Result};
use chmurka_core::hash::sha512_file;
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
    },
}

fn main() -> Result<()> {
    match Cli::parse().polecenie {
        Polecenie::PackBuild {
            instance,
            out,
            base_url,
            version,
        } => pack_build(&instance, &out, &base_url, &version),
    }
}

fn pack_build(instance: &Path, out: &Path, base_url: &str, version: &str) -> Result<()> {
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

    let mut files = Vec::new();

    for nazwa in &jary {
        let meta = &wpisy_meta[nazwa];
        let sciezka = mods.join(nazwa);
        let url = match (&meta.url, meta.cf_file) {
            (Some(u), _) => u.clone(),
            (None, Some(fid)) => pw::curseforge_url(fid, nazwa),
            (None, None) => bail!("mod {nazwa} (projekt CurseForge {:?}) nie ma ani adresu, ani identyfikatora pliku", meta.cf_project),
        };
        // Hash z metadanych bywa sha1 (CurseForge), wiec liczymy wlasny sha512
        // z pliku na dysku — to i tak jedyne zrodlo prawdy o tym, co gramy.
        let sha512 = sha512_file(&sciezka)?;
        files.push(serde_json::json!({
            "path": format!("mods/{nazwa}"),
            "size": std::fs::metadata(&sciezka)?.len(),
            "sha512": sha512,
            "policy": "mirror",
            "urls": [url],
        }));
    }

    // 4. Configi i options.txt trafiaja do naszego magazynu adresowanego trescia.
    let magazyn = out.join("files");
    std::fs::create_dir_all(&magazyn)?;
    let mut nasze: Vec<(String, PathBuf, &'static str)> = Vec::new();
    zbierz_configi(&instance.join("config"), "config", &mut nasze);
    let opcje = instance.join("options.txt");
    if opcje.is_file() {
        nasze.push(("options.txt".to_string(), opcje, "seed"));
    }

    for (rel, zrodlo, polityka) in nasze {
        let sha512 = sha512_file(&zrodlo)?;
        let podkatalog = magazyn.join(&sha512[0..2]);
        std::fs::create_dir_all(&podkatalog)?;
        let cel = podkatalog.join(&sha512);
        if !cel.exists() {
            std::fs::copy(&zrodlo, &cel)?;
        }
        files.push(serde_json::json!({
            "path": rel,
            "size": std::fs::metadata(&zrodlo)?.len(),
            "sha512": sha512,
            "policy": polityka,
            "urls": [format!("{}/files/{}/{}", base_url.trim_end_matches('/'), &sha512[0..2], sha512)],
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
        "launcher": { "latest_version": "0.1.0", "urls": {} },
        "mirror_dirs": ["mods"],
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

fn zbierz_configi(katalog: &Path, prefiks: &str, out: &mut Vec<(String, PathBuf, &'static str)>) {
    let Ok(wpisy) = std::fs::read_dir(katalog) else {
        return;
    };
    for w in wpisy.flatten() {
        let nazwa = w.file_name().to_string_lossy().to_string();
        if nazwa.starts_with('.') {
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
