use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("ścieżka jest pusta")]
    Empty,
    #[error("ścieżka absolutna nie jest dozwolona: {0}")]
    Absolute(String),
    #[error("ścieżka wychodzi poza katalog instancji: {0}")]
    Escapes(String),
    #[error("niedozwolony znak lub segment w ścieżce: {0}")]
    Illegal(String),
}

/// Sprawdza ścieżkę pochodzącą z manifestu i zwraca jej postać znormalizowaną.
///
/// Manifest jest danymi z sieci. Bez tej funkcji wpis `../../.bashrc`
/// pozwoliłby nadpisać dowolny plik na dysku użytkownika.
pub fn validate_rel(raw: &str) -> Result<String, PathError> {
    if raw.is_empty() {
        return Err(PathError::Empty);
    }
    if raw.contains('\\') {
        // Ukośnik odwrotny bywa separatorem na Windowsie, więc mógłby ominąć
        // kontrolę segmentów. Manifest zawsze używa ukośnika zwykłego.
        return Err(PathError::Illegal(raw.to_string()));
    }
    if raw.starts_with('/') {
        return Err(PathError::Absolute(raw.to_string()));
    }
    // Litera dysku, np. "C:/..." albo "C:".
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && (bytes[0] as char).is_ascii_alphabetic() {
        return Err(PathError::Absolute(raw.to_string()));
    }

    for seg in raw.split('/') {
        match seg {
            "" | "." => return Err(PathError::Illegal(raw.to_string())),
            ".." => return Err(PathError::Escapes(raw.to_string())),
            _ => {}
        }
    }
    Ok(raw.to_string())
}

/// Skleja zwalidowaną ścieżkę z katalogiem bazowym.
pub fn join_within(base: &Path, rel: &str) -> Result<PathBuf, PathError> {
    let clean = validate_rel(rel)?;
    let mut out = base.to_path_buf();
    for seg in clean.split('/') {
        out.push(seg);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn przyjmuje_zwykle_sciezki() {
        assert_eq!(validate_rel("mods/sodium.jar").unwrap(), "mods/sodium.jar");
        assert_eq!(validate_rel("options.txt").unwrap(), "options.txt");
        assert_eq!(
            validate_rel("config/create/common.toml").unwrap(),
            "config/create/common.toml"
        );
    }

    #[test]
    fn odrzuca_wyjscie_w_gore() {
        assert!(matches!(validate_rel("../secret"), Err(PathError::Escapes(_))));
        assert!(matches!(
            validate_rel("mods/../../secret"),
            Err(PathError::Escapes(_))
        ));
        assert!(matches!(validate_rel("mods/.."), Err(PathError::Escapes(_))));
    }

    #[test]
    fn odrzuca_sciezki_absolutne() {
        assert!(matches!(validate_rel("/etc/passwd"), Err(PathError::Absolute(_))));
        assert!(matches!(
            validate_rel("C:/Windows/system32"),
            Err(PathError::Absolute(_))
        ));
        // Sciezka UNC wpada w gałąź ukośnika odwrotnego, więc dostaje Illegal.
        assert!(matches!(
            validate_rel("\\\\serwer\\udzial"),
            Err(PathError::Illegal(_))
        ));
    }

    #[test]
    fn odrzuca_ukosnik_odwrotny_i_puste_segmenty() {
        assert!(matches!(
            validate_rel("mods\\sodium.jar"),
            Err(PathError::Illegal(_))
        ));
        assert!(matches!(
            validate_rel("mods//sodium.jar"),
            Err(PathError::Illegal(_))
        ));
        assert!(matches!(
            validate_rel("mods/./sodium.jar"),
            Err(PathError::Illegal(_))
        ));
        assert!(matches!(validate_rel(""), Err(PathError::Empty)));
    }

    #[test]
    fn join_within_zostaje_w_katalogu() {
        let base = std::path::Path::new("/tmp/instancja");
        assert_eq!(
            join_within(base, "mods/a.jar").unwrap(),
            std::path::Path::new("/tmp/instancja/mods/a.jar")
        );
        assert!(join_within(base, "../a.jar").is_err());
    }
}
