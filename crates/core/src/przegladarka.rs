//! Otwieranie strony logowania w osobnym, małym oknie przeglądarki.
//!
//! Nowa karta w otwartej przeglądarce to dla dziecka i dla rodzica najgorszy
//! możliwy wynik: dokleja się gdzieś z boku, wśród dwudziestu innych, często
//! bez przeniesienia okna na wierzch. Gracz nie wie, gdzie patrzeć, i wraca do
//! launchera z informacją, że „nic się nie otworzyło".
//!
//! Przeglądarki z rodziny Chromium mają na to tryb aplikacji (`--app=`):
//! osobne okno, bez pasków kart i adresu, wychodzące na pierwszy plan. Jest
//! w nim tylko strona logowania Microsoftu i nic poza nią.
//!
//! Świadomie NIE dokładamy tu wbudowanej przeglądarki (webview). Na Linuksie
//! wymagałaby bibliotek systemowych `webkit2gtk`, których launcher nie ma jak
//! dostarczyć — a jego obietnicą jest jeden plik, który po prostu działa.
//! Gdy trybu aplikacji nie da się użyć, wracamy do zwykłej karty: gorzej, ale
//! zawsze.

use std::process::Command;

/// Rozmiar okna logowania. Wysokie i wąskie, bo taka jest strona Microsoftu —
/// szersze okno dokłada tylko pustego tła po bokach.
const SZEROKOSC: u32 = 520;
const WYSOKOSC: u32 = 720;

/// Nazwy binarek, po których poznajemy rodzinę Chromium.
///
/// Sprawdzamy je jako fragment nazwy pliku wykonywalnego, bo ta sama
/// przeglądarka nazywa się różnie w różnych systemach — Brave to `brave` na
/// Arch Linuksie, `brave-browser` na Debianie i `brave.exe` na Windowsie.
/// Dopasowanie po fragmencie przechodzi przez wszystkie trzy.
const RODZINA_CHROMIUM: &[&str] = &[
    "chrome",
    "chromium",
    "brave",
    "msedge",
    "microsoft-edge",
    "vivaldi",
    "opera",
    "thorium",
];

/// Kandydaci sprawdzani, gdy nie udało się odczytać domyślnej przeglądarki.
///
/// Edge jest pierwszy na Windowsie nie z sympatii, tylko dlatego, że jest tam
/// zawsze — logowanie ma zadziałać także na komputerze, na którym nikt nigdy
/// niczego nie instalował.
const KANDYDACI: &[&str] = &[
    "msedge",
    "microsoft-edge",
    "google-chrome",
    "google-chrome-stable",
    "chromium",
    "chromium-browser",
    "brave",
    "brave-browser",
    "vivaldi",
];

/// Czy ta binarka należy do rodziny Chromium, czyli czy zrozumie `--app=`.
///
/// Firefox tej opcji nie ma i wywala się na niej — a `spawn` i tak zwróciłby
/// sukces, więc launcher uznałby, że okno się otworzyło, podczas gdy gracz nie
/// zobaczyłby niczego. Dlatego bramka jest tutaj, przed uruchomieniem.
pub fn rodzina_chromium(sciezka: &str) -> bool {
    let nazwa = sciezka
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(sciezka)
        .to_ascii_lowercase();
    RODZINA_CHROMIUM.iter().any(|w| nazwa.contains(w))
}

/// Argumenty trybu aplikacji.
///
/// Osobno od uruchamiania, żeby dało się je sprawdzić testem — pomyłka w tych
/// napisach kończy się oknem, które albo się nie otwiera, albo pokazuje pustą
/// stronę startową zamiast logowania.
///
/// `profil` wskazuje własny katalog danych przeglądarki. Ma dwa skutki naraz
/// i oba są tu potrzebne:
///
/// 1. Wymusza osobną instancję przeglądarki. Bez tego żądanie trafia do już
///    działającej i `--window-size` jest **ignorowany** — okno dziedziczy
///    geometrię po niej i wychodzi na całą wysokość ekranu. Sprawdzone
///    empirycznie: z własnym profilem rozmiar jest respektowany, bez niego nie.
/// 2. Świeży profil nie ma żadnych ciasteczek, więc Microsoft nie rozpoznaje
///    nikogo i zawsze pyta, na które konto się logujemy. Tego właśnie brakowało
///    przy wchodzeniu na zapamiętaną sesję.
pub fn argumenty_okienka(adres: &str, profil: Option<&std::path::Path>) -> Vec<String> {
    let mut a = vec![
        format!("--app={adres}"),
        format!("--window-size={SZEROKOSC},{WYSOKOSC}"),
    ];
    match profil {
        Some(katalog) => {
            a.push(format!("--user-data-dir={}", katalog.display()));
            // Świeży profil inaczej wita kreatorem powitalnym i pytaniem
            // o przeglądarkę domyślną, zasłaniając stronę logowania.
            a.push("--no-first-run".to_string());
            a.push("--no-default-browser-check".to_string());
        }
        // Bez własnego profilu kolejne uruchomienie dokleja się do już otwartej
        // przeglądarki jako zwykła karta — czyli dokładnie to, czego chcemy
        // uniknąć.
        None => a.push("--new-window".to_string()),
    }
    a
}

/// Wyciąga nazwę programu z wiersza `Exec=` pliku `.desktop`.
///
/// Format dopuszcza pola `%U`, `%u`, `%f` i cudzysłowy wokół ścieżki,
/// a niektóre wpisy zaczynają się od `env ZMIENNA=... program`.
pub fn program_z_exec(wiersz: &str) -> Option<String> {
    let reszta = wiersz.strip_prefix("Exec=")?.trim();
    for slowo in reszta.split_whitespace() {
        let czyste = slowo.trim_matches('"');
        // Pomijamy `env` i przypisania zmiennych przed właściwym programem.
        if czyste == "env" || (czyste.contains('=') && !czyste.starts_with('-')) {
            continue;
        }
        if czyste.starts_with('%') || czyste.starts_with('-') {
            continue;
        }
        return Some(czyste.to_string());
    }
    None
}

/// Domyślna przeglądarka gracza, o ile da się ją ustalić i zrozumie `--app=`.
///
/// Sięgamy po nią przed listą znanych nazw z dwóch powodów. Po pierwsze gracz
/// zobaczy przeglądarkę, której używa, a nie przypadkową inną zainstalowaną
/// obok. Po drugie lista nazw jest krucha: ta sama Brave to `brave` na jednym
/// systemie i `brave-browser` na innym.
fn z_pliku_desktop() -> Option<String> {
    let wynik = Command::new("xdg-settings")
        .args(["get", "default-web-browser"])
        .output()
        .ok()?;
    let plik = String::from_utf8_lossy(&wynik.stdout).trim().to_string();
    if plik.is_empty() {
        return None;
    }

    let mut katalogi: Vec<std::path::PathBuf> = Vec::new();
    if let Some(dom) = std::env::var_os("HOME") {
        katalogi.push(std::path::Path::new(&dom).join(".local/share/applications"));
    }
    if let Some(sciezki) = std::env::var_os("XDG_DATA_DIRS") {
        for k in std::env::split_paths(&sciezki) {
            katalogi.push(k.join("applications"));
        }
    }
    katalogi.push(std::path::PathBuf::from("/usr/share/applications"));

    for katalog in katalogi {
        let Ok(tresc) = std::fs::read_to_string(katalog.join(&plik)) else {
            continue;
        };
        for wiersz in tresc.lines() {
            if let Some(program) = program_z_exec(wiersz) {
                return rodzina_chromium(&program).then_some(program);
            }
        }
    }
    None
}

/// Wyciąga ścieżkę z wyjścia `reg query ... /ve`.
///
/// Wyjście wygląda tak:
///
/// ```text
/// HKEY_LOCAL_MACHINE\SOFTWARE\...\App Paths\msedge.exe
///     (Default)    REG_SZ    C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe
/// ```
///
/// Ścieżka bywa ze spacjami, więc nie wolno ciąć jej po białych znakach —
/// bierzemy wszystko za znacznikiem typu.
pub fn sciezka_z_reg_query(tekst: &str) -> Option<String> {
    for wiersz in tekst.lines() {
        if let Some((_, reszta)) = wiersz.split_once("REG_SZ") {
            let sciezka = reszta.trim();
            if !sciezka.is_empty() {
                return Some(sciezka.to_string());
            }
        }
    }
    None
}

/// Znajduje przeglądarkę na Windowsie po pełnej ścieżce.
///
/// To NIE jest kosmetyka. `Command::new("msedge")` szuka programu w `PATH`,
/// a `msedge.exe` tam nie jest — Windows trzyma ścieżki przeglądarek w rejestrze,
/// pod `App Paths`. Bez tego odczytu uruchomienie zawsze zawodziło i gracz
/// dostawał zwykłą kartę zamiast okienka, czyli funkcja nie działała wcale
/// na systemie, na którym gra większość graczy.
///
/// `HKCU` idzie przed `HKLM`, bo instalacja „tylko dla mnie" jest częstsza
/// u dzieci, które nie mają praw administratora.
fn z_rejestru_windows() -> Option<String> {
    const GALEZIE: &[&str] = &["HKCU", "HKLM"];
    for exe in KANDYDACI {
        let plik = format!("{exe}.exe");
        for galaz in GALEZIE {
            let klucz =
                format!("{galaz}\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\{plik}");
            // Wszystkie argumenty jako `&str` — tablica musi byc jednorodna,
            // inaczej `&String` obok `&str` nie przejdzie kompilacji.
            let Ok(wynik) = Command::new("reg")
                .args(["query", klucz.as_str(), "/ve"])
                .output()
            else {
                continue;
            };
            if !wynik.status.success() {
                continue;
            }
            let tekst = String::from_utf8_lossy(&wynik.stdout);
            if let Some(sciezka) = sciezka_z_reg_query(&tekst) {
                if std::path::Path::new(&sciezka).exists() {
                    return Some(sciezka);
                }
            }
        }
    }
    // Rejestr bywa niepełny przy instalacjach przenośnych — próbujemy jeszcze
    // miejsc, w których te przeglądarki siedzą domyślnie.
    zgadnij_z_katalogow_programow()
}

/// Zapasowe, dobrze znane miejsca instalacji. Kolejność jak w [`KANDYDACI`].
fn zgadnij_z_katalogow_programow() -> Option<String> {
    const KONCOWKI: &[&str] = &[
        r"Microsoft\Edge\Application\msedge.exe",
        r"Google\Chrome\Application\chrome.exe",
        r"BraveSoftware\Brave-Browser\Application\brave.exe",
        r"Vivaldi\Application\vivaldi.exe",
    ];
    let korzenie = [
        std::env::var_os("ProgramFiles"),
        std::env::var_os("ProgramFiles(x86)"),
        std::env::var_os("LOCALAPPDATA"),
    ];
    for korzen in korzenie.into_iter().flatten() {
        for koncowka in KONCOWKI {
            let pelna = std::path::Path::new(&korzen).join(koncowka);
            if pelna.exists() {
                return Some(pelna.display().to_string());
            }
        }
    }
    None
}

/// Próbuje otworzyć adres w osobnym okienku. Zwraca `false`, gdy się nie udało
/// i trzeba wrócić do zwykłej karty.
///
/// `profil` — patrz [`argumenty_okienka`]. `None` używa przeglądarki gracza
/// wraz z jej zapamiętanymi hasłami i zalogowaną sesją.
pub fn otworz_w_okienku(adres: &str, profil: Option<&std::path::Path>) -> bool {
    let mut kolejka: Vec<String> = Vec::new();
    if let Some(domyslna) = domyslna_przegladarka() {
        kolejka.push(domyslna);
    }
    kolejka.extend(KANDYDACI.iter().map(|s| s.to_string()));
    otworz_z_kolejki(&kolejka, adres, profil)
}

/// Wszystkie sposoby ustalenia przeglądarki, po kolei.
///
/// Zaden nie jest ograniczony do jednego systemu przez `cfg`. Na obcym systemie
/// kazdy po prostu nic nie znajduje: na Windowsie nie ma `xdg-settings`,
/// na Linuksie nie ma `reg` ani katalogu `Program Files`. Kosztuje to jedno
/// nieudane uruchomienie procesu przy logowaniu, a w zamian CAŁY ten kod jest
/// sprawdzany przez kompilator i testy na obu systemach.
///
/// Poprzednia wersja miala tu `#[cfg(windows)]` i wlasnie w tej galezi siedzial
/// blad, ktorego nie dalo sie wykryc na maszynie deweloperskiej.
fn domyslna_przegladarka() -> Option<String> {
    z_pliku_desktop()
        .or_else(z_rejestru_windows)
        .or_else(zgadnij_z_katalogow_programow)
}

fn otworz_z_kolejki(kolejka: &[String], adres: &str, profil: Option<&std::path::Path>) -> bool {
    for program in kolejka {
        if !rodzina_chromium(program) {
            continue;
        }
        let mut polecenie = Command::new(program);
        polecenie.args(argumenty_okienka(adres, profil));
        ukryj_konsole(&mut polecenie);
        // Nie czekamy na zakończenie: okno przeglądarki żyje własnym życiem,
        // a launcher ma w tym czasie odpytywać Microsoft o token.
        if polecenie.spawn().is_ok() {
            return true;
        }
    }
    false
}

/// Kasuje katalog profilu, żeby następne okno nikogo nie pamiętało.
///
/// Wołane przed „Zaloguj na inne konto". Samo `--incognito` nie wystarcza,
/// bo bez własnego profilu żądanie i tak trafia do działającej przeglądarki
/// i przejmuje jej rozmiar okna.
///
/// Błąd kasowania jest tu nieszkodliwy: w najgorszym razie okno pokaże konto
/// z poprzedniego logowania, a gracz kliknie „użyj innego konta" na stronie
/// Microsoftu. Przerywanie logowania z tego powodu byłoby gorsze.
pub fn wyczysc_profil(katalog: &std::path::Path) {
    let _ = std::fs::remove_dir_all(katalog);
}

/// Na Windowsie uruchomienie procesu potrafi mrugnąć czarnym oknem konsoli.
#[cfg(windows)]
fn ukryj_konsole(polecenie: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    polecenie.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn ukryj_konsole(_polecenie: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Adres musi wejsc do `--app=` w calosci i bez spacji rozdzielajacej —
    /// Chromium czyta `--app URL` jako dwa osobne argumenty i otwiera pusta
    /// strone startowa zamiast logowania.
    #[test]
    fn adres_siedzi_w_app_bez_spacji() {
        let a = argumenty_okienka("https://www.microsoft.com/link?otc=V3REVW36", None);
        assert_eq!(a[0], "--app=https://www.microsoft.com/link?otc=V3REVW36");
        assert!(a.iter().any(|x| x == "--new-window"));
        assert_eq!(a[1], "--window-size=520,720");
    }

    /// Wlasny profil to jedyny sposob, w jaki `--window-size` jest w ogole
    /// respektowany przy juz dzialajacej przegladarce — bez niego zadanie
    /// trafia do istniejacej instancji, ktora narzuca swoja geometrie.
    /// Sprawdzone empirycznie: bez profilu okno wychodzilo na cala wysokosc
    /// ekranu, z profilem ma zadany rozmiar.
    #[test]
    fn wlasny_profil_zamiast_nowego_okna() {
        let a = argumenty_okienka("https://example.test", Some(std::path::Path::new("/tmp/p")));
        assert!(a.iter().any(|x| x == "--user-data-dir=/tmp/p"), "{a:?}");
        assert!(a.iter().any(|x| x == "--no-first-run"));
        assert!(a.iter().any(|x| x == "--no-default-browser-check"));
        // `--new-window` przy wlasnym profilu jest zbedne i myli: instancja
        // i tak jest osobna.
        assert!(!a.iter().any(|x| x == "--new-window"), "{a:?}");
        // Rozmiar musi zostac — to dla niego caly ten profil.
        assert_eq!(a[1], "--window-size=520,720");
    }

    /// Ta sama przegladarka nazywa sie roznie w roznych systemach. Brave to
    /// `brave` na Archu, `brave-browser` na Debianie i `brave.exe` na Windowsie
    /// — sztywna lista nazw przepuscilaby tylko jedna z nich.
    #[test]
    fn rozpoznaje_rodzine_chromium_mimo_roznych_nazw() {
        for dobra in [
            "brave",
            "brave-browser",
            "/usr/bin/brave",
            "C:\\Program Files\\BraveSoftware\\brave.exe",
            "google-chrome-stable",
            "chromium-browser",
            "msedge.exe",
            "microsoft-edge",
        ] {
            assert!(rodzina_chromium(dobra), "{dobra}");
        }
    }

    /// Firefox nie zna `--app=`, a `spawn` i tak zwrocilby sukces. Gdyby
    /// przeszedl przez bramke, launcher uznalby, ze okno sie otworzylo,
    /// a gracz nie zobaczylby niczego.
    #[test]
    fn firefox_nie_przechodzi_bramki() {
        for zla in [
            "firefox",
            "/usr/bin/firefox",
            "firefox-esr",
            "epiphany",
            "falkon",
        ] {
            assert!(!rodzina_chromium(zla), "{zla}");
        }
    }

    /// Na Windowsie sciezka przegladarki idzie z rejestru i prawie zawsze
    /// zawiera spacje ("C:\\Program Files (x86)\\..."). Ciecie po bialych
    /// znakach urwaloby ja po "C:\\Program" i uruchomienie by padlo — a to
    /// jedyna droga do okienka na tym systemie.
    #[test]
    fn czyta_sciezke_ze_spacjami_z_rejestru() {
        let wyjscie = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\App Paths\\msedge.exe\r\n    (Default)    REG_SZ    C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe\r\n\r\n";
        assert_eq!(
            sciezka_z_reg_query(wyjscie).as_deref(),
            Some("C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe")
        );
    }

    /// Brak klucza w rejestrze nie moze udawac sukcesu — wtedy przechodzimy
    /// do zapasowych katalogow, a na koncu do zwyklej karty.
    #[test]
    fn brak_wpisu_w_rejestrze_daje_none() {
        assert_eq!(sciezka_z_reg_query(""), None);
        assert_eq!(
            sciezka_z_reg_query("BLAD: System nie moze odnalezc okreslonego klucza rejestru."),
            None
        );
        // Wpis bez wartosci tez nie jest sciezka.
        assert_eq!(sciezka_z_reg_query("    (Default)    REG_SZ    "), None);
    }

    /// Sciezka z rejestru musi przejsc bramke rodziny Chromium — inaczej
    /// zapasowa lista nazw nigdy by nie zadzialala na Windowsie.
    #[test]
    fn pelna_sciezka_windows_przechodzi_bramke() {
        assert!(rodzina_chromium(
            "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"
        ));
        assert!(rodzina_chromium(
            "C:\\Program Files\\BraveSoftware\\Brave-Browser\\Application\\brave.exe"
        ));
        assert!(!rodzina_chromium(
            "C:\\Program Files\\Mozilla Firefox\\firefox.exe"
        ));
    }

    #[test]
    fn czyta_program_z_wiersza_exec() {
        assert_eq!(program_z_exec("Exec=brave %U").as_deref(), Some("brave"));
        assert_eq!(program_z_exec("Exec=brave").as_deref(), Some("brave"));
        assert_eq!(
            program_z_exec("Exec=/usr/bin/google-chrome-stable --incognito %U").as_deref(),
            Some("/usr/bin/google-chrome-stable")
        );
        assert_eq!(
            program_z_exec("Exec=env MOZ_ENABLE_WAYLAND=1 firefox %u").as_deref(),
            Some("firefox")
        );
        assert_eq!(
            program_z_exec("Exec=\"/opt/Moja Przeglądarka/brave\" %U").as_deref(),
            Some("/opt/Moja")
        );
        assert_eq!(program_z_exec("Name=Brave"), None);
    }
}
