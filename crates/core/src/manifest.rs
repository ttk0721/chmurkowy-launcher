use crate::paths::{validate_rel, PathError};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const SCHEMA: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("nieprawidłowy JSON manifestu: {0}")]
    Json(#[from] serde_json::Error),
    #[error("nieobsługiwana wersja schematu manifestu: {0} (launcher rozumie 1)")]
    Schema(u32),
    #[error("wpis {path}: {source}")]
    Path { path: String, source: PathError },
    #[error("wpis {path}: adres musi być https, jest: {url}")]
    NotHttps { path: String, url: String },
    #[error("wpis {path}: brak jakiegokolwiek adresu do pobrania")]
    NoUrls { path: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub pack: Pack,
    pub java: JavaReq,
    pub memory: Memory,
    pub auth: AuthCfg,
    pub launcher: LauncherInfo,
    #[serde(default)]
    pub mirror_dirs: Vec<String>,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pack {
    pub name: String,
    #[serde(default)]
    pub edition: String,
    pub version: String,
    pub minecraft: String,
    pub loader: Loader,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Loader {
    pub kind: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavaReq {
    pub major: u32,
    #[serde(default = "domyslna_dystrybucja")]
    pub distribution: String,
}

fn domyslna_dystrybucja() -> String {
    "temurin".to_string()
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Memory {
    pub min_mb: u32,
    pub max_mb: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthCfg {
    pub msa_client_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LauncherInfo {
    pub latest_version: String,
    #[serde(default)]
    pub urls: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Policy {
    /// Katalog musi się zgadzać co do pliku. Obce pliki są kasowane.
    Mirror,
    /// Nadpisujemy tylko wtedy, gdy gracz pliku nie ruszał.
    Smart,
    /// Wgrywamy raz i nigdy więcej nie dotykamy.
    Seed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha512: String,
    pub policy: Policy,
    pub urls: Vec<String>,
}

pub fn parse(s: &str) -> Result<Manifest, ManifestError> {
    let m: Manifest = serde_json::from_str(s)?;
    if m.schema != SCHEMA {
        return Err(ManifestError::Schema(m.schema));
    }
    for f in &m.files {
        validate_rel(&f.path).map_err(|source| ManifestError::Path {
            path: f.path.clone(),
            source,
        })?;
        if f.urls.is_empty() {
            return Err(ManifestError::NoUrls {
                path: f.path.clone(),
            });
        }
        for u in &f.urls {
            if !u.starts_with("https://") {
                return Err(ManifestError::NotHttps {
                    path: f.path.clone(),
                    url: u.clone(),
                });
            }
        }
    }
    Ok(m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wzorcowy() -> String {
        r#"{
          "schema": 1,
          "pack": { "name": "Chmurkowy Serwer", "edition": "edycja 2026/2027",
                    "version": "2026.09.09-1", "minecraft": "1.21.1",
                    "loader": { "kind": "neoforge", "version": "21.1.249" } },
          "java": { "major": 21, "distribution": "temurin" },
          "memory": { "min_mb": 512, "max_mb": 4096 },
          "auth": { "msa_client_id": "00000000402b5328" },
          "launcher": { "latest_version": "1.0.0", "urls": { "linux-x64": "https://example.test/l" } },
          "mirror_dirs": ["mods"],
          "files": [
            { "path": "mods/a.jar", "size": 10, "sha512": "ab", "policy": "mirror",
              "urls": ["https://cdn.modrinth.com/a.jar"] },
            { "path": "options.txt", "size": 5, "sha512": "cd", "policy": "seed",
              "urls": ["https://example.test/o"] }
          ]
        }"#
        .to_string()
    }

    #[test]
    fn wczytuje_poprawny_manifest() {
        let m = parse(&wzorcowy()).unwrap();
        assert_eq!(m.pack.minecraft, "1.21.1");
        assert_eq!(m.pack.loader.version, "21.1.249");
        assert_eq!(m.java.major, 21);
        assert_eq!(m.mirror_dirs, vec!["mods".to_string()]);
        assert_eq!(m.files.len(), 2);
        assert_eq!(m.files[0].policy, Policy::Mirror);
        assert_eq!(m.files[1].policy, Policy::Seed);
    }

    #[test]
    fn odrzuca_nieznana_wersje_schematu() {
        let zly = wzorcowy().replace("\"schema\": 1", "\"schema\": 2");
        assert!(matches!(parse(&zly), Err(ManifestError::Schema(2))));
    }

    #[test]
    fn odrzuca_wroga_sciezke() {
        let zly = wzorcowy().replace("mods/a.jar", "../../.bashrc");
        assert!(matches!(parse(&zly), Err(ManifestError::Path { .. })));
    }

    #[test]
    fn odrzuca_adres_bez_https() {
        let zly = wzorcowy().replace(
            "https://cdn.modrinth.com/a.jar",
            "http://cdn.modrinth.com/a.jar",
        );
        assert!(matches!(parse(&zly), Err(ManifestError::NotHttps { .. })));
    }

    #[test]
    fn odrzuca_wpis_bez_zrodla() {
        let zly = wzorcowy().replace(r#"["https://cdn.modrinth.com/a.jar"]"#, "[]");
        assert!(matches!(parse(&zly), Err(ManifestError::NoUrls { .. })));
    }
}
