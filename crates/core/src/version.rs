use crate::java::Os;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum VersionError {
    #[error("nie znaleziono profilu wersji {0}")]
    Brak(String),
    #[error("nieprawidłowy JSON profilu {0}: {1}")]
    Json(String, serde_json::Error),
    #[error("błąd odczytu: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    pub main_class: String,
    #[serde(default)]
    pub arguments: Arguments,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub asset_index: Option<AssetIndex>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Arg>,
    #[serde(default)]
    pub jvm: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Arg {
    Prosty(String),
    Warunkowy {
        #[serde(default)]
        rules: Vec<Rule>,
        value: ArgValue,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    Jeden(String),
    Wiele(Vec<String>),
}

impl ArgValue {
    pub fn lista(&self) -> Vec<String> {
        match self {
            ArgValue::Jeden(s) => vec![s.clone()],
            ArgValue::Wiele(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<RuleOs>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RuleOs {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
}

/// Biblioteka na dysku. `path` mówi, gdzie ją położyć w `libraries/`.
#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    pub url: String,
}

/// Pobranie bez ustalonego miejsca w drzewie — tak Mojang opisuje `downloads.client`.
/// Ten wpis nie ma pola `path` i próba czytania go jako `Artifact` kończy się
/// błędem „missing field path".
#[derive(Debug, Clone, Deserialize)]
pub struct Download {
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    #[serde(default)]
    pub client: Option<Download>,
}

/// Wczytuje profil i rekurencyjnie dokleja to, po czym dziedziczy.
pub fn load(versions_dir: &Path, id: &str) -> Result<VersionJson, VersionError> {
    let sciezka = versions_dir.join(id).join(format!("{id}.json"));
    if !sciezka.is_file() {
        return Err(VersionError::Brak(id.to_string()));
    }
    let tekst = std::fs::read_to_string(&sciezka)?;
    let v: VersionJson =
        serde_json::from_str(&tekst).map_err(|e| VersionError::Json(id.to_string(), e))?;

    match v.inherits_from.clone() {
        Some(rodzic_id) => {
            let rodzic = load(versions_dir, &rodzic_id)?;
            Ok(merge(v, rodzic))
        }
        None => Ok(v),
    }
}

fn bez_wersji(koordynat: &str) -> String {
    let czesci: Vec<&str> = koordynat.split(':').collect();
    if czesci.len() >= 2 {
        format!("{}:{}", czesci[0], czesci[1])
    } else {
        koordynat.to_string()
    }
}

/// Scala profil potomny z rodzicem zgodnie z semantyką `inheritsFrom`.
pub fn merge(child: VersionJson, parent: VersionJson) -> VersionJson {
    // Biblioteki rodzica idą pierwsze, ale przy kolizji współrzędnej Maven
    // wygrywa wersja z profilu potomnego — NeoForge celowo podbija np. ASM.
    let nadpisane: std::collections::BTreeSet<String> =
        child.libraries.iter().map(|l| bez_wersji(&l.name)).collect();

    let mut libraries: Vec<Library> = parent
        .libraries
        .into_iter()
        .filter(|l| !nadpisane.contains(&bez_wersji(&l.name)))
        .collect();
    libraries.extend(child.libraries);

    let mut game = parent.arguments.game;
    game.extend(child.arguments.game);
    let mut jvm = parent.arguments.jvm;
    jvm.extend(child.arguments.jvm);

    VersionJson {
        id: child.id,
        inherits_from: None,
        main_class: child.main_class,
        arguments: Arguments { game, jvm },
        libraries,
        asset_index: child.asset_index.or(parent.asset_index),
        assets: child.assets.or(parent.assets),
        downloads: child.downloads.or(parent.downloads),
    }
}

pub fn rules_allow(rules: &[Rule], os: Os) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut wynik = false;
    for r in rules {
        let pasuje = match &r.os {
            None => true,
            Some(o) => match &o.name {
                None => true,
                Some(n) => n == os.nazwa_mojang(),
            },
        };
        if pasuje {
            wynik = r.action == "allow";
        }
    }
    wynik
}

/// Podstawia `${zmienna}`. Nieznane zmienne zostawia nietknięte,
/// żeby błąd był widoczny w logu zamiast zamieniać się w pusty argument.
pub fn substitute(tpl: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(tpl.len());
    let mut reszta = tpl;
    while let Some(start) = reszta.find("${") {
        out.push_str(&reszta[..start]);
        let po = &reszta[start + 2..];
        match po.find('}') {
            Some(koniec) => {
                let nazwa = &po[..koniec];
                match vars.get(nazwa) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push_str("${");
                        out.push_str(nazwa);
                        out.push('}');
                    }
                }
                reszta = &po[koniec + 1..];
            }
            None => {
                out.push_str(&reszta[start..]);
                return out;
            }
        }
    }
    out.push_str(reszta);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn regula_allow_dla_wlasciwego_systemu() {
        let r: Vec<Rule> =
            serde_json::from_str(r#"[{"action":"allow","os":{"name":"osx"}}]"#).unwrap();
        assert!(rules_allow(&r, Os::Osx));
        assert!(!rules_allow(&r, Os::Linux));
    }

    #[test]
    fn brak_regul_znaczy_zgoda() {
        assert!(rules_allow(&[], Os::Linux));
    }

    #[test]
    fn regula_disallow_wyklucza() {
        let r: Vec<Rule> = serde_json::from_str(
            r#"[{"action":"allow"},{"action":"disallow","os":{"name":"windows"}}]"#,
        )
        .unwrap();
        assert!(rules_allow(&r, Os::Linux));
        assert!(!rules_allow(&r, Os::Windows));
    }

    #[test]
    fn scalanie_bierze_mainclass_z_dziecka_i_libki_z_obu() {
        let dziecko: VersionJson = serde_json::from_str(
            r#"{
            "id":"neoforge-21.1.249","inheritsFrom":"1.21.1",
            "mainClass":"cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments":{"game":["--fml.neoForgeVersion","21.1.249"],"jvm":["-p","x"]},
            "libraries":[{"name":"org.ow2.asm:asm:9.10.1"}]
        }"#,
        )
        .unwrap();
        let rodzic: VersionJson = serde_json::from_str(
            r#"{
            "id":"1.21.1","mainClass":"net.minecraft.client.main.Main",
            "arguments":{"game":["--username","${auth_player_name}"],"jvm":["-Xss1M"]},
            "libraries":[{"name":"com.github.oshi:oshi-core:6.4.10"}],
            "assets":"17",
            "assetIndex":{"id":"17","sha1":"aa","size":1,"totalSize":2,"url":"https://x/17.json"}
        }"#,
        )
        .unwrap();

        let w = merge(dziecko, rodzic);
        assert_eq!(w.main_class, "cpw.mods.bootstraplauncher.BootstrapLauncher");
        assert_eq!(w.libraries.len(), 2);
        assert_eq!(w.assets.unwrap(), "17");
        assert_eq!(w.asset_index.unwrap().id, "17");
        // Argumenty rodzica ida pierwsze, potem dziecka.
        assert_eq!(w.arguments.game[0], Arg::Prosty("--username".into()));
        assert_eq!(
            w.arguments.game[2],
            Arg::Prosty("--fml.neoForgeVersion".into())
        );
    }

    #[test]
    fn dziecko_nadpisuje_libke_o_tej_samej_wspolrzednej() {
        let dziecko: VersionJson = serde_json::from_str(
            r#"{
            "id":"c","inheritsFrom":"p","mainClass":"C",
            "arguments":{"game":[],"jvm":[]},
            "libraries":[{"name":"org.ow2.asm:asm:9.10.1"}]}"#,
        )
        .unwrap();
        let rodzic: VersionJson = serde_json::from_str(
            r#"{
            "id":"p","mainClass":"P",
            "arguments":{"game":[],"jvm":[]},
            "libraries":[{"name":"org.ow2.asm:asm:9.2"}]}"#,
        )
        .unwrap();
        let w = merge(dziecko, rodzic);
        assert_eq!(
            w.libraries.len(),
            1,
            "asm nie moze byc na classpathie dwa razy"
        );
        assert_eq!(w.libraries[0].name, "org.ow2.asm:asm:9.10.1");
    }

    #[test]
    fn podstawia_zmienne_w_szablonie() {
        let mut v = BTreeMap::new();
        v.insert("auth_player_name".to_string(), "Tomasz".to_string());
        v.insert("classpath_separator".to_string(), ":".to_string());
        assert_eq!(substitute("${auth_player_name}", &v), "Tomasz");
        assert_eq!(substitute("a${classpath_separator}b", &v), "a:b");
        // Nieznana zmienna zostaje jak byla — lepiej zobaczyc ja w logu
        // niz po cichu wstawic pusty ciag.
        assert_eq!(substitute("${nieznane}", &v), "${nieznane}");
    }

    #[test]
    fn czyta_downloads_client_bez_pola_path() {
        // Prawdziwy ksztalt z 1.21.1.json: downloads.client nie ma "path",
        // w przeciwienstwie do libraries[].downloads.artifact. Wczesniejsza
        // wersja wymagala path zawsze i wywracala cala instalacje.
        let v: VersionJson = serde_json::from_str(
            r#"{
            "id":"1.21.1","mainClass":"net.minecraft.client.main.Main",
            "downloads":{"client":{"sha1":"abc","size":26000000,
                                   "url":"https://piston-data.mojang.com/client.jar"}},
            "libraries":[]}"#,
        )
        .unwrap();
        let k = v.downloads.unwrap().client.unwrap();
        assert_eq!(k.sha1, "abc");
        assert_eq!(k.url, "https://piston-data.mojang.com/client.jar");
    }

    #[test]
    fn wspolrzedna_maven_bez_wersji() {
        assert_eq!(bez_wersji("org.ow2.asm:asm:9.10.1"), "org.ow2.asm:asm");
        assert_eq!(
            bez_wersji("net.minecraft:client:1.21.1:extra"),
            "net.minecraft:client"
        );
    }
}
