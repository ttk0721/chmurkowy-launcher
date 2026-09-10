//! Launcher zadomawia się na komputerze przy pierwszym uruchomieniu.
//!
//! Wydanie zawiera instalator i gołą binarkę. Ta druga jest po to, żeby
//! launcher mógł podmienić sam siebie przy aktualizacji — ale na liście
//! wydań wygląda zachęcająco i to ją ludzie pobierają. Klikają, program
//! działa, tylko zostaje jednym plikiem w „Pobranych": nie ma go w menu,
//! nie da się go znaleźć wyszukiwarką, a przy każdej aktualizacji dochodzi
//! kolejna kopia z numerkiem w nazwie.
//!
//! Zamiast tłumaczyć, że „trzeba było wziąć to drugie", launcher przenosi
//! się tam, gdzie jego miejsce, i dopisuje do menu. Raz, po cichu.

use std::path::{Path, PathBuf};

/// Nazwa pliku programu po zadomowieniu.
fn nazwa_pliku() -> &'static str {
    if cfg!(target_os = "windows") {
        "ChmurkowyLauncher.exe"
    } else {
        "ChmurkowyLauncher"
    }
}

/// Gdzie launcher powinien mieszkać.
///
/// Obok leży katalog `data`, więc program i jego dane trzymają się razem,
/// a odinstalowanie sprowadza się do usunięcia jednego drzewa.
pub fn docelowy_plik() -> Option<PathBuf> {
    crate::miejsca::katalog_uzytkownika().map(|k| k.join(nazwa_pliku()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Wynik {
    /// Launcher jest już tam, gdzie ma być.
    JuzNaMiejscu,
    /// Przeniesiony — trzeba uruchomić plik spod tej ścieżki i zakończyć siebie.
    Zainstalowany(PathBuf),
    /// Nie udało się; działamy dalej stamtąd, skąd nas uruchomiono.
    NieUdalo(String),
}

/// Czy launcher uruchomiono spoza swojego miejsca.
///
/// Porównujemy ścieżki po skanonizowaniu — inaczej dowiązanie albo `./plik`
/// wyglądałyby na coś innego i launcher instalowałby się w kółko.
pub fn poza_miejscem(exe: &Path, docelowy: &Path) -> bool {
    let kanoniczny = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    kanoniczny(exe) != kanoniczny(docelowy)
}

/// Kopiuje launcher tam, gdzie jego miejsce.
///
/// Nie kasuje pliku źródłowego: gracz sam go pobrał i to jego rzecz, co
/// z nim zrobi. Kasowanie cudzych plików z „Pobranych" bez pytania byłoby
/// przekroczeniem uprawnień.
pub fn zadomow(exe: &Path) -> Wynik {
    let Some(docelowy) = docelowy_plik() else {
        return Wynik::NieUdalo("nie wiadomo, gdzie jest katalog użytkownika".into());
    };
    if !poza_miejscem(exe, &docelowy) {
        return Wynik::JuzNaMiejscu;
    }

    if let Some(rodzic) = docelowy.parent() {
        if let Err(e) = std::fs::create_dir_all(rodzic) {
            return Wynik::NieUdalo(format!("{}: {e}", rodzic.display()));
        }
    }

    // Pliku, który się wykonuje, nie da się nadpisać na Windowsie, a na
    // Linuksie da się, ale lepiej tego nie robić. Kasujemy stary wpis
    // katalogu i wstawiamy nowy — to działa wszędzie.
    let _ = std::fs::remove_file(&docelowy);
    if let Err(e) = std::fs::copy(exe, &docelowy) {
        return Wynik::NieUdalo(format!("{}: {e}", docelowy.display()));
    }
    if let Err(e) = nadaj_prawo_uruchamiania(&docelowy) {
        return Wynik::NieUdalo(e);
    }

    Wynik::Zainstalowany(docelowy)
}

fn nadaj_prawo_uruchamiania(p: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut u = std::fs::metadata(p)
            .map_err(|e| format!("{}: {e}", p.display()))?
            .permissions();
        u.set_mode(0o755);
        std::fs::set_permissions(p, u).map_err(|e| format!("{}: {e}", p.display()))?;
    }
    #[cfg(not(unix))]
    let _ = p;
    Ok(())
}

impl Wynik {
    /// Zdanie do logu launchera. Pusty ciąg, gdy nie ma o czym mówić.
    pub fn opis(&self) -> String {
        match self {
            Wynik::JuzNaMiejscu => String::new(),
            Wynik::Zainstalowany(p) => format!(
                "Launcher zainstalował się w {} i dopisał do menu aplikacji. \
                 Plik, który pobrałeś, możesz skasować.",
                p.display()
            ),
            Wynik::NieUdalo(powod) => format!(
                "Nie udało się zainstalować launchera ({powod}). Działa stąd, \
                 skąd go uruchomiono."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_sam_plik_to_juz_na_miejscu() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("ChmurkowyLauncher");
        std::fs::write(&p, "x").unwrap();
        assert!(!poza_miejscem(&p, &p));
    }

    /// Sciezka wzgledna i dowiazanie wskazuja ten sam plik. Bez kanonizacji
    /// launcher instalowalby sie przy kazdym starcie w kolko.
    #[test]
    fn rozne_zapisy_tej_samej_sciezki_to_nie_przeprowadzka() {
        let kat = tempfile::tempdir().unwrap();
        let p = kat.path().join("ChmurkowyLauncher");
        std::fs::write(&p, "x").unwrap();
        let okrezna = kat.path().join(".").join("ChmurkowyLauncher");
        assert!(!poza_miejscem(&okrezna, &p));
    }

    #[test]
    fn inny_plik_to_przeprowadzka() {
        let kat = tempfile::tempdir().unwrap();
        let skad = kat.path().join("Pobrane").join("ChmurkowyLauncher");
        let dokad = kat.path().join("miejsce").join("ChmurkowyLauncher");
        std::fs::create_dir_all(skad.parent().unwrap()).unwrap();
        std::fs::write(&skad, "x").unwrap();
        assert!(poza_miejscem(&skad, &dokad));
    }

    /// Gracz sam pobral ten plik i to jego rzecz, co z nim zrobi.
    /// Kasowanie cudzych plikow z „Pobranych" bez pytania to za duzo.
    #[test]
    fn zrodlo_zostaje_nietkniete() {
        let kat = tempfile::tempdir().unwrap();
        let skad = kat.path().join("ChmurkowyLauncher");
        std::fs::write(&skad, "program").unwrap();

        let dokad = kat.path().join("miejsce").join("ChmurkowyLauncher");
        std::fs::create_dir_all(dokad.parent().unwrap()).unwrap();
        std::fs::copy(&skad, &dokad).unwrap();

        assert!(skad.is_file(), "plik zrodlowy nie moze zniknac");
        assert_eq!(std::fs::read_to_string(&dokad).unwrap(), "program");
    }

    #[test]
    fn kazdy_wynik_ma_zrozumiale_zdanie() {
        assert!(Wynik::JuzNaMiejscu.opis().is_empty());
        let z = Wynik::Zainstalowany(PathBuf::from("/dom/gracz/.local/share/x/ChmurkowyLauncher"));
        assert!(z.opis().contains("menu aplikacji"));
        let n = Wynik::NieUdalo("brak miejsca".into());
        assert!(n.opis().contains("brak miejsca"));
        assert!(n.opis().contains("Działa stąd"));
    }

    #[test]
    fn nazwa_pliku_pasuje_do_systemu() {
        if cfg!(target_os = "windows") {
            assert!(nazwa_pliku().ends_with(".exe"));
        } else {
            assert!(!nazwa_pliku().contains('.'));
        }
    }
}
