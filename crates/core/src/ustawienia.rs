//! Ustawienia gracza zapisywane w `data/settings.json`.
//!
//! Wcześniej nic nie było zapisywane — zmiana pamięci znikała po zamknięciu
//! launchera.

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ustawienia {
    pub pamiec_mb: u32,
    /// Dodatkowe parametry przekazywane Javie, oddzielone spacjami.
    pub dodatkowe_argumenty: String,
    /// Chowanie okna launchera po starcie gry, żeby nie zabierało zasobów.
    /// Domyślnie włączone.
    pub ukryj_po_starcie: bool,
    pub nick_offline: String,
}

impl Default for Ustawienia {
    fn default() -> Self {
        Self {
            pamiec_mb: 4096,
            dodatkowe_argumenty: String::new(),
            ukryj_po_starcie: true,
            nick_offline: String::new(),
        }
    }
}

impl Ustawienia {
    /// Uszkodzony plik daje ustawienia domyślne, nie błąd — gracz nie może
    /// zostać zablokowany przez zepsuty plik konfiguracyjny.
    pub fn wczytaj(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn zapisz(&self, path: &Path) -> std::io::Result<()> {
        if let Some(rodzic) = path.parent() {
            std::fs::create_dir_all(rodzic)?;
        }
        let tresc = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, tresc)
    }

    /// Czy gracz sam ustawił rozmiar sterty. Wtedy nasz suwak ustępuje,
    /// bo dwa `-Xmx` w jednej komendzie tylko mylą.
    pub fn wlasny_rozmiar_sterty(&self) -> bool {
        podziel_argumenty(&self.dodatkowe_argumenty)
            .iter()
            .any(|a| a.starts_with("-Xmx"))
    }
}

/// Dzieli wpisane parametry na osobne argumenty, respektując cudzysłowy.
///
/// Zwykły podział po spacjach psułby ścieżki ze spacjami, a takie trafiają się
/// w parametrach typu `-Dcos="C:\Program Files\x"`.
pub fn podziel_argumenty(tekst: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut biezacy = String::new();
    let mut w_cudzyslowie: Option<char> = None;

    for znak in tekst.chars() {
        match (znak, w_cudzyslowie) {
            ('"', None) | ('\'', None) => w_cudzyslowie = Some(znak),
            (z, Some(otwarty)) if z == otwarty => w_cudzyslowie = None,
            (z, None) if z.is_whitespace() => {
                if !biezacy.is_empty() {
                    out.push(std::mem::take(&mut biezacy));
                }
            }
            (z, _) => biezacy.push(z),
        }
    }
    if !biezacy.is_empty() {
        out.push(biezacy);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domyslnie_chowa_okno_po_starcie() {
        let u = Ustawienia::default();
        assert!(u.ukryj_po_starcie, "to ma byc wlaczone domyslnie");
        assert_eq!(u.pamiec_mb, 4096);
    }

    #[test]
    fn dzieli_zwykle_argumenty() {
        assert_eq!(
            podziel_argumenty("-XX:+UseG1GC -Dfoo=bar"),
            vec!["-XX:+UseG1GC", "-Dfoo=bar"]
        );
    }

    #[test]
    fn szanuje_cudzyslowy() {
        assert_eq!(
            podziel_argumenty(r#"-Dsciezka="C:\Program Files\Java" -Xss1M"#),
            vec![r"-Dsciezka=C:\Program Files\Java", "-Xss1M"]
        );
    }

    #[test]
    fn puste_i_nadmiarowe_spacje_nie_tworza_pustych_argumentow() {
        assert!(podziel_argumenty("   ").is_empty());
        assert_eq!(podziel_argumenty("  -a    -b  "), vec!["-a", "-b"]);
    }

    #[test]
    fn wykrywa_wlasny_rozmiar_sterty() {
        let mut u = Ustawienia::default();
        assert!(!u.wlasny_rozmiar_sterty());
        u.dodatkowe_argumenty = "-XX:+UseZGC -Xmx8G".into();
        assert!(u.wlasny_rozmiar_sterty());
    }

    #[test]
    fn uszkodzony_plik_daje_domyslne_zamiast_bledu() {
        let dir = std::env::temp_dir().join("chmurka-ust-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.json");
        std::fs::write(&p, b"to nie jest json").unwrap();
        assert_eq!(Ustawienia::wczytaj(&p), Ustawienia::default());
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn zapis_i_odczyt_zachowuja_wartosci() {
        let dir = std::env::temp_dir().join("chmurka-ust-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings2.json");
        let u = Ustawienia {
            pamiec_mb: 8192,
            dodatkowe_argumenty: "-XX:+UseG1GC".into(),
            ukryj_po_starcie: false,
            nick_offline: "Tomek".into(),
        };
        u.zapisz(&p).unwrap();
        assert_eq!(Ustawienia::wczytaj(&p), u);
        std::fs::remove_file(&p).unwrap();
    }
}
