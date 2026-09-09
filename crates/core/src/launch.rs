use crate::auth::Account;
use crate::java::{biezacy_os, Os};
use crate::version::{rules_allow, substitute, Arg, VersionJson};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("profil wersji nie ma indeksu zasobów, gra nie wystartuje")]
    BrakIndeksuZasobow,
}

pub struct LaunchParams<'a> {
    pub java: &'a Path,
    pub mc_dir: &'a Path,
    pub game_dir: &'a Path,
    pub version: &'a VersionJson,
    pub account: &'a Account,
    pub min_mb: u32,
    pub max_mb: u32,
}

pub fn separator(os: Os) -> &'static str {
    if os == Os::Windows {
        ";"
    } else {
        ":"
    }
}

pub fn build_classpath(v: &VersionJson, mc_dir: &Path, os: Os) -> String {
    let libs = mc_dir.join("libraries");
    let mut czesci: Vec<String> = Vec::new();
    for l in &v.libraries {
        if !rules_allow(&l.rules, os) {
            continue;
        }
        if let Some(art) = l.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            czesci.push(libs.join(&art.path).display().to_string());
        }
    }
    czesci.join(separator(os))
}

pub fn build_command(p: &LaunchParams) -> Result<std::process::Command, LaunchError> {
    let os = biezacy_os();
    let indeks = p
        .version
        .assets
        .clone()
        .or_else(|| p.version.asset_index.as_ref().map(|a| a.id.clone()))
        .ok_or(LaunchError::BrakIndeksuZasobow)?;

    let mut zmienne: BTreeMap<String, String> = BTreeMap::new();
    zmienne.insert("auth_player_name".into(), p.account.name.clone());
    zmienne.insert("auth_uuid".into(), p.account.uuid.clone());
    zmienne.insert("auth_access_token".into(), p.account.token.clone());
    zmienne.insert("auth_xuid".into(), String::new());
    zmienne.insert("clientid".into(), String::new());
    zmienne.insert("user_type".into(), p.account.kind.user_type().into());
    zmienne.insert("version_name".into(), p.version.id.clone());
    zmienne.insert("version_type".into(), "release".into());
    zmienne.insert("game_directory".into(), p.game_dir.display().to_string());
    zmienne.insert(
        "assets_root".into(),
        p.mc_dir.join("assets").display().to_string(),
    );
    zmienne.insert(
        "game_assets".into(),
        p.mc_dir.join("assets").display().to_string(),
    );
    zmienne.insert("assets_index_name".into(), indeks);
    zmienne.insert(
        "library_directory".into(),
        p.mc_dir.join("libraries").display().to_string(),
    );
    zmienne.insert("classpath_separator".into(), separator(os).into());
    zmienne.insert("classpath".into(), build_classpath(p.version, p.mc_dir, os));
    zmienne.insert(
        "natives_directory".into(),
        p.mc_dir
            .join("natives")
            .join(&p.version.id)
            .display()
            .to_string(),
    );
    zmienne.insert("launcher_name".into(), "ChmurkowyLauncher".into());
    zmienne.insert("launcher_version".into(), env!("CARGO_PKG_VERSION").into());
    zmienne.insert("resolution_width".into(), "854".into());
    zmienne.insert("resolution_height".into(), "480".into());

    let mut cmd = std::process::Command::new(p.java);
    cmd.arg(format!("-Xms{}M", p.min_mb));
    cmd.arg(format!("-Xmx{}M", p.max_mb));
    for a in rozwin(&p.version.arguments.jvm, os, &zmienne) {
        cmd.arg(a);
    }
    cmd.arg(&p.version.main_class);
    for a in rozwin(&p.version.arguments.game, os, &zmienne) {
        cmd.arg(a);
    }
    cmd.current_dir(p.game_dir);
    Ok(cmd)
}

fn rozwin(argi: &[Arg], os: Os, zmienne: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    for a in argi {
        match a {
            Arg::Prosty(s) => out.push(substitute(s, zmienne)),
            Arg::Warunkowy { rules, value } => {
                if rules_allow(rules, os) {
                    for v in value.lista() {
                        out.push(substitute(&v, zmienne));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Account, AccountKind};

    fn wersja() -> VersionJson {
        serde_json::from_str(
            r#"{
            "id":"neoforge-21.1.249",
            "mainClass":"cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments":{
              "game":["--username","${auth_player_name}","--uuid","${auth_uuid}",
                      "--accessToken","${auth_access_token}","--userType","${user_type}",
                      "--gameDir","${game_directory}"],
              "jvm":["-DlibraryDirectory=${library_directory}","-cp","${classpath}"]
            },
            "libraries":[
              {"name":"a:b:1","downloads":{"artifact":{"path":"a/b/1/b-1.jar","sha1":"x","size":1,"url":"https://x"}}},
              {"name":"c:d:1","downloads":{"artifact":{"path":"c/d/1/d-1.jar","sha1":"y","size":1,"url":"https://y"}},
               "rules":[{"action":"allow","os":{"name":"osx"}}]}
            ],
            "assets":"17",
            "assetIndex":{"id":"17","sha1":"a","size":1,"totalSize":1,"url":"https://x/17.json"}
        }"#,
        )
        .unwrap()
    }

    fn konto() -> Account {
        Account {
            name: "Tomasz".into(),
            uuid: "61a50080-80aa-3842-8df5-cd674d3a57f2".into(),
            token: "0".into(),
            kind: AccountKind::Offline,
        }
    }

    #[test]
    fn podstawia_dane_konta_do_argumentow() {
        let v = wersja();
        let k = konto();
        let mc = std::path::Path::new("/tmp/mc");
        let gra = std::path::Path::new("/tmp/gra");
        let cmd = build_command(&LaunchParams {
            java: std::path::Path::new("/tmp/java"),
            mc_dir: mc,
            game_dir: gra,
            version: &v,
            account: &k,
            min_mb: 512,
            max_mb: 4096,
        })
        .unwrap();

        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"Tomasz".to_string()));
        assert!(args.contains(&"61a50080-80aa-3842-8df5-cd674d3a57f2".to_string()));
        assert!(args.contains(&"legacy".to_string()));
        assert!(args.contains(&"-Xmx4096M".to_string()));
        assert!(args.contains(&"-Xms512M".to_string()));
        assert!(args.contains(&"cpw.mods.bootstraplauncher.BootstrapLauncher".to_string()));
        assert!(
            !args.iter().any(|a| a.contains("${")),
            "wszystkie zmienne musza byc podstawione"
        );
    }

    #[test]
    fn classpath_pomija_libki_dla_innego_systemu() {
        let v = wersja();
        let cp = build_classpath(&v, std::path::Path::new("/tmp/mc"), Os::Linux);
        assert!(cp.contains("b-1.jar"));
        assert!(
            !cp.contains("d-1.jar"),
            "libka tylko dla macOS nie moze trafic na classpath Linuksa"
        );
    }

    #[test]
    fn separator_classpatha_zalezy_od_systemu() {
        assert_eq!(separator(Os::Linux), ":");
        assert_eq!(separator(Os::Windows), ";");
    }
}
