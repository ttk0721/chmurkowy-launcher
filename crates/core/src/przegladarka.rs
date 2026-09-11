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
#[cfg(windows)]
const KANDYDACI: &[&str] = &["msedge", "chrome", "brave", "vivaldi", "opera"];

#[cfg(not(windows))]
const KANDYDACI: &[&str] = &[
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
pub fn argumenty_okienka(adres: &str) -> Vec<String> {
    vec![
        format!("--app={adres}"),
        format!("--window-size={SZEROKOSC},{WYSOKOSC}"),
        // Bez tego kolejne uruchomienie dokleja się do już otwartej
        // przeglądarki jako zwykła karta — czyli dokładnie to, czego
        // chcemy uniknąć.
        "--new-window".to_string(),
    ]
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
#[cfg(not(windows))]
fn domyslna_przegladarka() -> Option<String> {
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

#[cfg(windows)]
fn domyslna_przegladarka() -> Option<String> {
    // Na Windowsie domyślna przeglądarka siedzi w rejestrze pod identyfikatorem
    // programu, a nie pod ścieżką — odczyt jest na tyle zawiły, że nie opłaca
    // się go tu powtarzać. Lista niżej zaczyna się od Edge'a, który na tym
    // systemie jest zawsze.
    None
}

/// Próbuje otworzyć adres w osobnym okienku. Zwraca `false`, gdy się nie udało
/// i trzeba wrócić do zwykłej karty.
pub fn otworz_w_okienku(adres: &str) -> bool {
    let mut kolejka: Vec<String> = Vec::new();
    if let Some(domyslna) = domyslna_przegladarka() {
        kolejka.push(domyslna);
    }
    kolejka.extend(KANDYDACI.iter().map(|s| s.to_string()));

    for program in kolejka {
        if !rodzina_chromium(&program) {
            continue;
        }
        let mut polecenie = Command::new(&program);
        polecenie.args(argumenty_okienka(adres));
        ukryj_konsole(&mut polecenie);
        // Nie czekamy na zakończenie: okno przeglądarki żyje własnym życiem,
        // a launcher ma w tym czasie odpytywać Microsoft o token.
        if polecenie.spawn().is_ok() {
            return true;
        }
    }
    false
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
        let a = argumenty_okienka("https://www.microsoft.com/link?otc=V3REVW36");
        assert_eq!(a[0], "--app=https://www.microsoft.com/link?otc=V3REVW36");
        assert!(a.iter().any(|x| x == "--new-window"));
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
