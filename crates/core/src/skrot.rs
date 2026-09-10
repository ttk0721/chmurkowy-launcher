//! Dopisanie launchera do menu aplikacji systemu.
//!
//! Wydanie zawiera i instalator, i gołą binarkę — ta druga jest po to, żeby
//! launcher mógł podmienić sam siebie przy aktualizacji. Na liście wydań
//! wygląda jednak zachęcająco i to ją ludzie pobierają: klikają, program
//! działa, tylko nie ma go potem w wyszukiwarce aplikacji.
//!
//! Zamiast tłumaczyć, że „trzeba było wziąć to drugie", launcher dopisuje
//! się do menu sam — niezależnie od tego, skąd go uruchomiono.
//!
//! Na Windowsie skrótami zajmuje się instalator, więc tam nic nie robimy.

use std::path::{Path, PathBuf};

/// Gdzie i co trzeba zapisać, żeby launcher pojawił się w menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wpis {
    pub plik_desktop: PathBuf,
    pub plik_ikony: PathBuf,
    pub tresc: String,
}

/// Co zrobiliśmy — do zapisania w logu launchera.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stan {
    /// Wpis już był i wskazywał na ten sam plik.
    BezZmian,
    /// Dopisaliśmy launcher do menu (albo poprawiliśmy ścieżkę po przenosinach).
    Dopisany,
    /// Ten system załatwia skróty inaczej.
    NieDotyczy,
}

/// Układa treść wpisu. Nic nie zapisuje.
///
/// `wspolny` to katalog danych współdzielonych użytkownika
/// (`~/.local/share`), przyjmowany z zewnątrz, żeby dało się to sprawdzić
/// testem bez dotykania prawdziwego katalogu domowego.
pub fn zaplanuj(exe: &Path, wspolny: &Path) -> Wpis {
    // `Exec` musi być bezwzględne i w cudzysłowie — ścieżka bywa ze spacją,
    // a katalog „Pobrane (1)" to nie teoria, tylko codzienność.
    let tresc = format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name=Chmurkowy Launcher\n\
         Comment=Minecraft z paczką modów Chmurkowego Serwera\n\
         GenericName=Launcher Minecrafta\n\
         Exec=\"{exec}\"\n\
         Icon=chmurkowy-launcher\n\
         Terminal=false\n\
         Categories=Game;\n\
         StartupNotify=true\n\
         StartupWMClass=ChmurkowyLauncher\n\
         Keywords=minecraft;gra;mody;chmurka;\n",
        exec = exe.display()
    );

    Wpis {
        plik_desktop: wspolny
            .join("applications")
            .join("chmurkowy-launcher.desktop"),
        plik_ikony: wspolny
            .join("icons")
            .join("hicolor")
            .join("256x256")
            .join("apps")
            .join("chmurkowy-launcher.png"),
        tresc,
    }
}

/// Zapisuje wpis, jeśli to konieczne.
///
/// Zwraca `Ok(false)`, gdy wszystko było już na miejscu — wtedy nie ma o czym
/// mówić graczowi i nie ruszamy plików przy każdym uruchomieniu.
pub fn zapisz(wpis: &Wpis, ikona: &[u8]) -> Result<bool, String> {
    let aktualny = std::fs::read_to_string(&wpis.plik_desktop).unwrap_or_default();
    let ikona_jest = wpis.plik_ikony.is_file();
    if aktualny == wpis.tresc && ikona_jest {
        return Ok(false);
    }

    for plik in [&wpis.plik_desktop, &wpis.plik_ikony] {
        if let Some(rodzic) = plik.parent() {
            std::fs::create_dir_all(rodzic).map_err(|e| format!("{}: {e}", rodzic.display()))?;
        }
    }

    if !ikona_jest {
        std::fs::write(&wpis.plik_ikony, ikona)
            .map_err(|e| format!("{}: {e}", wpis.plik_ikony.display()))?;
    }
    std::fs::write(&wpis.plik_desktop, &wpis.tresc)
        .map_err(|e| format!("{}: {e}", wpis.plik_desktop.display()))?;

    Ok(true)
}

/// Odświeża spis aplikacji. Bez tego skrót bywa widoczny dopiero po
/// wylogowaniu. Nie każde środowisko ma to narzędzie, więc brak nie jest błędem.
pub fn odswiez_menu(wspolny: &Path) {
    let _ = std::process::Command::new("update-desktop-database")
        .arg(wspolny.join("applications"))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status();
}

/// Całość w jednym wywołaniu: ustala miejsce, zapisuje i odświeża menu.
pub fn zarejestruj(exe: &Path, ikona: &[u8]) -> Result<Stan, String> {
    if cfg!(target_os = "windows") {
        // Skróty w menu Start zakłada instalator — on jeden wie, co potem
        // po sobie posprzątać przy odinstalowaniu.
        return Ok(Stan::NieDotyczy);
    }
    let Some(wspolny) = crate::miejsca::katalog_wspoldzielony() else {
        return Ok(Stan::NieDotyczy);
    };

    let wpis = zaplanuj(exe, &wspolny);
    if zapisz(&wpis, ikona)? {
        odswiez_menu(&wspolny);
        Ok(Stan::Dopisany)
    } else {
        Ok(Stan::BezZmian)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IKONA: &[u8] = b"udawana-ikona";

    #[test]
    fn wpis_wskazuje_na_biezacy_plik() {
        let w = zaplanuj(
            Path::new("/dom/gracz/Pobrane/ChmurkowyLauncher-linux-x64"),
            Path::new("/dom/gracz/.local/share"),
        );
        assert!(w
            .tresc
            .contains("Exec=\"/dom/gracz/Pobrane/ChmurkowyLauncher-linux-x64\""));
        assert!(w.plik_desktop.ends_with("applications/chmurkowy-launcher.desktop"));
    }

    /// Katalog ze spacja to nie teoria — „Pobrane (1)" powstaje samo przy
    /// drugim pobraniu tego samego pliku.
    #[test]
    fn sciezka_ze_spacja_jest_w_cudzyslowie() {
        let w = zaplanuj(
            Path::new("/dom/gracz/Pobrane (1)/ChmurkowyLauncher"),
            Path::new("/dom/gracz/.local/share"),
        );
        assert!(
            w.tresc.contains("Exec=\"/dom/gracz/Pobrane (1)/ChmurkowyLauncher\""),
            "{}",
            w.tresc
        );
    }

    #[test]
    fn zapis_tworzy_katalogi_i_pliki() {
        let kat = tempfile::tempdir().unwrap();
        let w = zaplanuj(Path::new("/gdzies/ChmurkowyLauncher"), kat.path());

        assert!(zapisz(&w, IKONA).unwrap(), "pierwszy zapis ma cos zmienic");
        assert!(w.plik_desktop.is_file());
        assert!(w.plik_ikony.is_file());
        assert_eq!(std::fs::read(&w.plik_ikony).unwrap(), IKONA);
    }

    /// Przy kazdym starcie launchera przechodzimy tedy. Nie wolno wtedy
    /// przepisywac plikow ani zaczepiac gracza komunikatem.
    #[test]
    fn drugi_start_nic_nie_zmienia() {
        let kat = tempfile::tempdir().unwrap();
        let w = zaplanuj(Path::new("/gdzies/ChmurkowyLauncher"), kat.path());
        assert!(zapisz(&w, IKONA).unwrap());
        assert!(!zapisz(&w, IKONA).unwrap(), "drugi zapis ma byc bezczynny");
    }

    /// Gracz przenosi plik z „Pobrane" na pulpit — wpis ma za nim podazyc,
    /// inaczej klikniecie w menu odpalaloby nieistniejacy program.
    #[test]
    fn przeniesienie_launchera_poprawia_wpis() {
        let kat = tempfile::tempdir().unwrap();
        let stary = zaplanuj(Path::new("/dom/gracz/Pobrane/ChmurkowyLauncher"), kat.path());
        assert!(zapisz(&stary, IKONA).unwrap());

        let nowy = zaplanuj(Path::new("/dom/gracz/Pulpit/ChmurkowyLauncher"), kat.path());
        assert!(zapisz(&nowy, IKONA).unwrap(), "zmiana sciezki musi przejsc");
        let zapisane = std::fs::read_to_string(&nowy.plik_desktop).unwrap();
        assert!(zapisane.contains("/dom/gracz/Pulpit/ChmurkowyLauncher"));
        assert!(!zapisane.contains("/dom/gracz/Pobrane/"));
    }

    /// Skasowana ikona ma wrocic, nawet gdy sam wpis sie nie zmienil.
    #[test]
    fn brakujaca_ikona_jest_odtwarzana() {
        let kat = tempfile::tempdir().unwrap();
        let w = zaplanuj(Path::new("/gdzies/ChmurkowyLauncher"), kat.path());
        assert!(zapisz(&w, IKONA).unwrap());
        std::fs::remove_file(&w.plik_ikony).unwrap();
        assert!(zapisz(&w, IKONA).unwrap());
        assert!(w.plik_ikony.is_file());
    }

    /// Wpis musi byc prawidlowym plikiem .desktop, inaczej srodowisko
    /// graficzne po prostu go zignoruje i gracz znowu nic nie znajdzie.
    #[test]
    fn wpis_ma_wymagane_pola() {
        let w = zaplanuj(Path::new("/gdzies/ChmurkowyLauncher"), Path::new("/tmp"));
        assert!(w.tresc.starts_with("[Desktop Entry]\n"));
        for pole in ["Type=Application", "Name=", "Exec=", "Icon=", "Terminal=false"] {
            assert!(w.tresc.contains(pole), "brakuje {pole} w:\n{}", w.tresc);
        }
        assert!(w.tresc.ends_with('\n'));
    }
}
