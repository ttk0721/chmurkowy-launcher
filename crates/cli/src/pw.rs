#[derive(Debug, thiserror::Error)]
pub enum PwError {
    #[error("brak pola filename w metadanych")]
    BrakNazwy,
}

#[derive(Debug, Clone)]
pub struct PwEntry {
    pub filename: String,
    pub url: Option<String>,
    pub sha512: Option<String>,
    pub cf_project: Option<u64>,
    pub cf_file: Option<u64>,
}

/// Wyciąga potrzebne pola z pliku `.pw.toml`.
///
/// Celowo nie używamy pełnego parsera TOML: interesuje nas pięć pól o stałym
/// kształcie, a pliki generuje PrismLauncher, więc format jest przewidywalny.
pub fn parse_pw(tekst: &str) -> Result<PwEntry, PwError> {
    let filename = pole(tekst, "filename").ok_or(PwError::BrakNazwy)?;
    let format_hasha = pole(tekst, "hash-format");
    let sha512 = match format_hasha.as_deref() {
        Some("sha512") => pole(tekst, "hash"),
        _ => None,
    };

    // PrismLauncher zapisuje dla modów z CurseForge `mode = 'metadata:curseforge'`
    // razem z pustym `url = ''`. Bez sprawdzenia trybu pusty ciąg trafiłby do
    // manifestu jako prawdziwy adres i paczka byłaby nie do pobrania.
    let url = match pole(tekst, "mode").as_deref() {
        Some("url") => pole(tekst, "url").filter(|u| !u.is_empty()),
        _ => None,
    };

    Ok(PwEntry {
        filename,
        url,
        sha512,
        cf_project: liczba(tekst, "project-id"),
        cf_file: liczba(tekst, "file-id"),
    })
}

fn pole(tekst: &str, klucz: &str) -> Option<String> {
    for linia in tekst.lines() {
        let linia = linia.trim();
        let Some(reszta) = linia.strip_prefix(klucz) else {
            continue;
        };
        // Bez tego klucz "url" pasowałby także do linii "urls = [...]".
        let reszta = reszta.trim_start();
        if let Some(wartosc) = reszta.strip_prefix('=') {
            return Some(
                wartosc
                    .trim()
                    .trim_matches('\'')
                    .trim_matches('"')
                    .to_string(),
            );
        }
    }
    None
}

fn liczba(tekst: &str, klucz: &str) -> Option<u64> {
    pole(tekst, klucz)?.parse().ok()
}

/// Adres bezpośredni do CDN-u CurseForge, składany z identyfikatora pliku.
pub fn curseforge_url(file_id: u64, filename: &str) -> String {
    format!(
        "https://mediafilez.forgecdn.net/files/{}/{}/{}",
        file_id / 1000,
        file_id % 1000,
        filename
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const MODRINTH: &str = r#"
filename = 'sodium-neoforge-0.8.13+mc1.21.1.jar'
name = 'Sodium'
side = 'client'

[download]
hash = '4f537696af95411e9daf15795fb5fcbc49913d48'
hash-format = 'sha512'
mode = 'url'
url = 'https://cdn.modrinth.com/data/AANobbMI/versions/uMOpc5uV/sodium.jar'

[update.modrinth]
mod-id = 'AANobbMI'
version = 'uMOpc5uV'
"#;

    const CURSEFORGE: &str = r#"
filename = 'spark-1.10.124-neoforge.jar'
name = 'spark'
side = 'both'

[download]
hash = 'aabbcc'
hash-format = 'sha1'
mode = 'metadata:curseforge'

[update.curseforge]
file-id = 6225208
project-id = 361579
"#;

    #[test]
    fn czyta_wpis_modrinth() {
        let e = parse_pw(MODRINTH).unwrap();
        assert_eq!(e.filename, "sodium-neoforge-0.8.13+mc1.21.1.jar");
        assert_eq!(
            e.url.unwrap(),
            "https://cdn.modrinth.com/data/AANobbMI/versions/uMOpc5uV/sodium.jar"
        );
        assert_eq!(e.sha512.unwrap(), "4f537696af95411e9daf15795fb5fcbc49913d48");
    }

    #[test]
    fn czyta_wpis_curseforge_bez_url() {
        let e = parse_pw(CURSEFORGE).unwrap();
        assert_eq!(e.filename, "spark-1.10.124-neoforge.jar");
        assert!(e.url.is_none(), "CurseForge nie podaje adresu wprost");
        assert!(
            e.sha512.is_none(),
            "hash jest sha1, wiec nie nadaje sie jako sha512"
        );
        assert_eq!(e.cf_file.unwrap(), 6225208);
        assert_eq!(e.cf_project.unwrap(), 361579);
    }

    /// Prawdziwy plik z paczki. PrismLauncher dopisuje tu puste `url = ''`,
    /// co przed poprawką trafiało do manifestu jako adres i wywracało walidację.
    const CURSEFORGE_Z_PUSTYM_URL: &str = r#"
filename = 'SkyVillages-1.0.6-1.21.x-neoforge-release.jar'
name = 'Sky Villages [Forge]'
side = ''

[download]
hash = '645eb49d4019e5550afd4055f380c26edd126611'
hash-format = 'sha1'
mode = 'metadata:curseforge'
url = ''

[update.curseforge]
file-id = 5647988
project-id = 545467
"#;

    #[test]
    fn pusty_url_nie_jest_adresem() {
        let e = parse_pw(CURSEFORGE_Z_PUSTYM_URL).unwrap();
        assert!(
            e.url.is_none(),
            "puste url = '' musi byc traktowane jak brak adresu"
        );
        assert_eq!(e.cf_file.unwrap(), 5647988);
    }

    #[test]
    fn sklada_adres_curseforge() {
        // Sprawdzone empirycznie: ten adres zwraca HTTP 200.
        assert_eq!(
            curseforge_url(6225208, "spark-1.10.124-neoforge.jar"),
            "https://mediafilez.forgecdn.net/files/6225/208/spark-1.10.124-neoforge.jar"
        );
        assert_eq!(
            curseforge_url(5647988, "SkyVillages.jar"),
            "https://mediafilez.forgecdn.net/files/5647/988/SkyVillages.jar"
        );
        // CurseForge nie dopelnia reszty zerami.
        assert_eq!(
            curseforge_url(1234005, "x.jar"),
            "https://mediafilez.forgecdn.net/files/1234/5/x.jar"
        );
    }
}
