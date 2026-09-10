//! Gdzie leżą dane launchera: gra, paczka modów, ustawienia i logi.
//!
//! Do wersji 0.4.13 wszystko leżało w katalogu `data` obok pliku launchera.
//! Było to wygodne przy jednym przenośnym pliku, ale przestaje działać, gdy
//! program instaluje się raz i potem aktualizuje: katalog instalacji nie jest
//! miejscem na dwugigabajtowe światy gracza.
//!
//! Dane przenoszą się więc do katalogu użytkownika. Tym, którzy już mają
//! launcher, przenosimy je raz, przy pierwszym uruchomieniu nowej wersji —
//! nikt nie pobiera paczki drugi raz i nikt nie traci świata z singleplayera.

use std::path::{Path, PathBuf};

/// Nazwa katalogu, w którym trzymamy wszystko swoje.
const NAZWA: &str = "ChmurkowyLauncher";

/// Katalog danych użytkownika, zgodny ze zwyczajem systemu.
///
/// Windows: `%LOCALAPPDATA%\ChmurkowyLauncher`
/// Linux: `$XDG_DATA_HOME/chmurkowy-launcher`, a bez niego `~/.local/share/…`
///
/// Celowo bez dodatkowej biblioteki — to kilka linii, a każda zależność
/// w programie, który ma działać na cudzych komputerach, to jedna rzecz
/// więcej do pilnowania.
pub fn katalog_uzytkownika() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .map(|p| p.join(NAZWA))
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .map(|p| p.join(".local").join("share"))
            })
            .map(|p| p.join("chmurkowy-launcher"))
    }
}

/// Katalog danych współdzielonych użytkownika (`~/.local/share`).
///
/// Tu trafiają rzeczy widoczne dla systemu: wpis w menu aplikacji i ikona.
/// Nie mylić z [`katalog_uzytkownika`], który wskazuje nasz własny podkatalog.
pub fn katalog_wspoldzielony() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|p| p.join(".local").join("share"))
        })
}

/// Co launcher zrobił z katalogiem danych — do zapisania w logu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wynik {
    /// Nic się nie działo: dane były już na miejscu albo powstaną od zera.
    BezZmian,
    /// Dane przeniesione ze starej lokalizacji.
    Przeniesione { skad: PathBuf },
    /// Przeniesienie się nie udało — zostajemy przy starym katalogu.
    /// Lepiej działać w nietypowym miejscu niż zgubić komuś dwa gigabajty.
    ZostajemyPrzyStarym { powod: String },
}

/// Ustala katalog danych i w razie potrzeby przenosi stary.
///
/// `obok_programu` to katalog z plikiem launchera, `docelowy` — katalog
/// użytkownika. Obie ścieżki przyjmujemy z zewnątrz, żeby dało się to
/// sprawdzić testem bez dotykania prawdziwego katalogu domowego.
pub fn ustal(obok_programu: &Path, docelowy: &Path) -> (PathBuf, Wynik) {
    let stary = obok_programu.join("data");
    let nowy = docelowy.join("data");

    // Kiedy nowy katalog już istnieje, przeprowadzka jest za nami.
    // Starego nie ruszamy — mógł zostać po nieudanej próbie i to jedyna
    // kopia, jaką gracz ma.
    if nowy.is_dir() {
        return (nowy, Wynik::BezZmian);
    }

    if !stary.is_dir() {
        return (nowy, Wynik::BezZmian);
    }

    match przenies(&stary, &nowy) {
        Ok(()) => (
            nowy,
            Wynik::Przeniesione {
                skad: stary.clone(),
            },
        ),
        Err(e) => (stary, Wynik::ZostajemyPrzyStarym { powod: e }),
    }
}

/// Przenosi katalog. Najpierw tanio, przez zmianę nazwy; gdy to nie wyjdzie
/// (katalog domowy bywa na innym dysku niż pendrive z launcherem) — kopiuje
/// i dopiero po udanym kopiowaniu kasuje oryginał.
fn przenies(skad: &Path, dokad: &Path) -> Result<(), String> {
    if let Some(rodzic) = dokad.parent() {
        std::fs::create_dir_all(rodzic).map_err(|e| format!("{}: {e}", rodzic.display()))?;
    }

    if std::fs::rename(skad, dokad).is_ok() {
        return Ok(());
    }

    kopiuj_katalog(skad, dokad).inspect_err(|_| {
        // Nieudane kopiowanie zostawia śmieci — sprzątamy, żeby przy
        // następnym starcie nie wyglądały na gotową przeprowadzkę.
        let _ = std::fs::remove_dir_all(dokad);
    })?;

    // Oryginał kasujemy dopiero teraz. Gdyby to się nie udało, trudno —
    // dane są już w nowym miejscu, a stary katalog nikomu nie przeszkadza.
    let _ = std::fs::remove_dir_all(skad);
    Ok(())
}

fn kopiuj_katalog(skad: &Path, dokad: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dokad).map_err(|e| format!("{}: {e}", dokad.display()))?;
    let wpisy = std::fs::read_dir(skad).map_err(|e| format!("{}: {e}", skad.display()))?;
    for wpis in wpisy {
        let wpis = wpis.map_err(|e| format!("{}: {e}", skad.display()))?;
        let z = wpis.path();
        let do_ = dokad.join(wpis.file_name());
        // `file_type` zamiast `is_dir`, żeby dowiązanie do katalogu
        // skopiować jako plik, a nie wejść w nie i ciągnąć cudze dane.
        let typ = wpis
            .file_type()
            .map_err(|e| format!("{}: {e}", z.display()))?;
        if typ.is_dir() {
            kopiuj_katalog(&z, &do_)?;
        } else {
            std::fs::copy(&z, &do_).map_err(|e| format!("{}: {e}", z.display()))?;
        }
    }
    Ok(())
}

impl Wynik {
    /// Zdanie do logu launchera. Pusty ciąg, gdy nie ma o czym mówić.
    pub fn opis(&self) -> String {
        match self {
            Wynik::BezZmian => String::new(),
            Wynik::Przeniesione { skad } => format!(
                "Dane gry przeniesione z {} do katalogu użytkownika. Światy i ustawienia zostały.",
                skad.display()
            ),
            Wynik::ZostajemyPrzyStarym { powod } => format!(
                "Nie udało się przenieść danych gry ({powod}). Launcher korzysta ze starego \
                 katalogu — nic nie zginęło."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plik(p: &Path, tresc: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, tresc).unwrap();
    }

    #[test]
    fn swieza_instalacja_uzywa_katalogu_uzytkownika() {
        let k = tempfile::tempdir().unwrap();
        let obok = k.path().join("program");
        let cel = k.path().join("uzytkownik");
        std::fs::create_dir_all(&obok).unwrap();

        let (wybrany, wynik) = ustal(&obok, &cel);
        assert_eq!(wybrany, cel.join("data"));
        assert_eq!(wynik, Wynik::BezZmian);
    }

    /// Sedno przeprowadzki: dwa gigabajty gry i swiaty gracza maja przejsc
    /// w nowe miejsce same, bez pobierania czegokolwiek od nowa.
    #[test]
    fn stare_dane_przenosza_sie_przy_pierwszym_starcie() {
        let k = tempfile::tempdir().unwrap();
        let obok = k.path().join("program");
        let cel = k.path().join("uzytkownik");

        plik(&obok.join("data/settings.json"), "{\"pamiec_mb\":8192}");
        plik(&obok.join("data/instance/saves/swiat/level.dat"), "swiat");
        plik(&obok.join("data/mc/versions/1.21.1/1.21.1.json"), "{}");

        let (wybrany, wynik) = ustal(&obok, &cel);

        assert_eq!(wybrany, cel.join("data"));
        assert!(matches!(wynik, Wynik::Przeniesione { .. }));
        assert_eq!(
            std::fs::read_to_string(cel.join("data/settings.json")).unwrap(),
            "{\"pamiec_mb\":8192}"
        );
        assert_eq!(
            std::fs::read_to_string(cel.join("data/instance/saves/swiat/level.dat")).unwrap(),
            "swiat"
        );
        assert!(
            !obok.join("data").exists(),
            "stary katalog ma zniknac po udanej przeprowadzce"
        );
    }

    /// Druga przeprowadzka nie ma prawa sie zdarzyc. Gdyby sie zdarzyla,
    /// nadpisalaby swiezy stan tym, co zostalo w starym katalogu.
    #[test]
    fn przenosimy_tylko_raz() {
        let k = tempfile::tempdir().unwrap();
        let obok = k.path().join("program");
        let cel = k.path().join("uzytkownik");

        plik(&cel.join("data/settings.json"), "nowe");
        plik(&obok.join("data/settings.json"), "stare");

        let (wybrany, wynik) = ustal(&obok, &cel);
        assert_eq!(wybrany, cel.join("data"));
        assert_eq!(wynik, Wynik::BezZmian);
        assert_eq!(
            std::fs::read_to_string(cel.join("data/settings.json")).unwrap(),
            "nowe",
            "istniejace dane nie moga zostac nadpisane starymi"
        );
        assert!(
            obok.join("data").exists(),
            "starego katalogu nie kasujemy, gdy nic z niego nie brali\u{15b}my"
        );
    }

    /// Gdy przeprowadzka padnie, wracamy do starego katalogu. Utrata swiatow
    /// bylaby gorsza od dzialania w nietypowym miejscu.
    #[test]
    fn nieudana_przeprowadzka_zostawia_dane_tam_gdzie_byly() {
        let k = tempfile::tempdir().unwrap();
        let obok = k.path().join("program");
        plik(&obok.join("data/settings.json"), "moje");

        // Cel nie do utworzenia: w miejscu katalogu lezy plik.
        let przeszkoda = k.path().join("zajete");
        std::fs::write(&przeszkoda, "nie katalog").unwrap();
        let cel = przeszkoda.join("gdzies");

        let (wybrany, wynik) = ustal(&obok, &cel);
        assert_eq!(wybrany, obok.join("data"));
        assert!(matches!(wynik, Wynik::ZostajemyPrzyStarym { .. }));
        assert_eq!(
            std::fs::read_to_string(obok.join("data/settings.json")).unwrap(),
            "moje",
            "dane musza przetrwac nieudana przeprowadzke"
        );
    }

    #[test]
    fn kazdy_wynik_ma_zrozumiale_zdanie() {
        assert!(Wynik::BezZmian.opis().is_empty());
        let p = Wynik::Przeniesione {
            skad: PathBuf::from("/stare/data"),
        };
        assert!(p.opis().contains("/stare/data"));
        assert!(p.opis().contains("Światy"));
        let z = Wynik::ZostajemyPrzyStarym {
            powod: "brak miejsca".into(),
        };
        assert!(z.opis().contains("brak miejsca"));
        assert!(z.opis().contains("nic nie zginęło"));
    }

    #[test]
    fn katalog_uzytkownika_jest_bezwzgledny() {
        if let Some(p) = katalog_uzytkownika() {
            assert!(p.is_absolute(), "{p:?}");
        }
    }
}
