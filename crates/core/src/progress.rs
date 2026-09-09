#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Manifest,
    Java,
    Loader,
    Libraries,
    Assets,
    Pack,
    Ready,
}

impl Stage {
    /// Tekst pokazywany użytkownikowi w linii stanu.
    pub fn opis(&self) -> &'static str {
        match self {
            Stage::Manifest => "Sprawdzam paczkę",
            Stage::Java => "Pobieram Javę",
            Stage::Loader => "Instaluję NeoForge",
            Stage::Libraries => "Pobieram biblioteki",
            Stage::Assets => "Pobieram zasoby gry",
            Stage::Pack => "Pobieram mody",
            Stage::Ready => "Gotowe",
        }
    }
}

/// W czym liczony jest postęp. Bez tego interfejs nie wie, czy „45/249"
/// to pliki, czy bajty, i pokazywał surowe liczby.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jednostka {
    Pliki,
    Bajty,
    /// Nie da się zmierzyć — np. instalator NeoForge mieli przez minutę
    /// i nie ma jak zajrzeć do środka. Interfejs pokazuje wtedy kręciołek.
    Nieznana,
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub stage: Stage,
    pub done: u64,
    pub total: u64,
    pub label: String,
    pub jednostka: Jednostka,
}

impl Progress {
    pub fn pliki(stage: Stage, done: u64, total: u64, label: impl Into<String>) -> Self {
        Self {
            stage,
            done,
            total,
            label: label.into(),
            jednostka: Jednostka::Pliki,
        }
    }

    pub fn bajty(stage: Stage, pobrane: u64, calosc: u64, label: impl Into<String>) -> Self {
        Self {
            stage,
            done: pobrane,
            total: calosc,
            label: label.into(),
            jednostka: if calosc > 0 {
                Jednostka::Bajty
            } else {
                Jednostka::Nieznana
            },
        }
    }

    /// Etap bez mierzalnego postępu.
    pub fn trwa(stage: Stage, label: impl Into<String>) -> Self {
        Self {
            stage,
            done: 0,
            total: 0,
            label: label.into(),
            jednostka: Jednostka::Nieznana,
        }
    }

    /// Ułamek do paska postępu albo `None`, gdy nie da się go policzyć.
    pub fn ulamek(&self) -> Option<f32> {
        if self.jednostka == Jednostka::Nieznana || self.total == 0 {
            None
        } else {
            Some((self.done as f32 / self.total as f32).clamp(0.0, 1.0))
        }
    }

    /// Opis ilościowy dopasowany do jednostki.
    pub fn licznik(&self) -> String {
        match self.jednostka {
            Jednostka::Pliki => format!("{}/{}", self.done, self.total),
            Jednostka::Bajty => format!("{} / {}", mb(self.done), mb(self.total)),
            Jednostka::Nieznana => String::new(),
        }
    }
}

fn mb(bajty: u64) -> String {
    let m = bajty as f64 / 1_048_576.0;
    if m >= 100.0 {
        format!("{m:.0} MB")
    } else {
        format!("{m:.1} MB")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pliki_licza_sie_jak_pliki() {
        let p = Progress::pliki(Stage::Pack, 12, 249, "a.jar");
        assert_eq!(p.licznik(), "12/249");
        assert!((p.ulamek().unwrap() - 12.0 / 249.0).abs() < 1e-6);
    }

    #[test]
    fn bajty_pokazuja_megabajty() {
        let p = Progress::bajty(Stage::Java, 10 * 1_048_576, 45 * 1_048_576, "Java 21");
        assert_eq!(p.licznik(), "10.0 MB / 45.0 MB");
        assert!((p.ulamek().unwrap() - 10.0 / 45.0).abs() < 1e-6);
    }

    #[test]
    fn bez_znanej_wielkosci_nie_ma_ulamka() {
        // Serwer nie podal Content-Length — pasek musi przejsc w tryb nieokreslony,
        // zamiast pokazywac 0% przez cale pobieranie.
        let p = Progress::bajty(Stage::Java, 5000, 0, "Java 21");
        assert_eq!(p.jednostka, Jednostka::Nieznana);
        assert!(p.ulamek().is_none());
        assert_eq!(p.licznik(), "");
    }

    #[test]
    fn etap_bez_pomiaru() {
        let p = Progress::trwa(Stage::Loader, "Instaluję NeoForge");
        assert!(p.ulamek().is_none());
    }
}
