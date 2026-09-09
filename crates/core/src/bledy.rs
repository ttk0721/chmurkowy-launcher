//! Katalog błędów pisany dla rodzica, który nie zna się na komputerach.
//!
//! Każdy błąd ma krótki kod (łatwy do podyktowania przez telefon), zdanie
//! opisujące co się stało bez żargonu, i listę konkretnych kroków do wykonania.
//! Szczegóły techniczne są osobno — dla administracji, nie dla gracza.

use crate::auth::msa::AuthError;
use crate::game_install::InstallError;
use crate::java::JavaError;
use crate::launch::LaunchError;
use crate::manifest::ManifestError;
use crate::net::NetError;
use crate::pack_sync::SyncError;
use crate::version::VersionError;

/// Zdanie doklejane do każdego błędu jako ostatnia deska ratunku.
pub const OSTATNIA_DESKA: &str = "Jeśli nic z powyższego nie pomogło, napisz do administracji \
serwera i podaj kod błędu. Administracja prowadzi serwer po godzinach, więc odpowiedź może \
zająć dzień lub dwa — to normalne, nikt o Tobie nie zapomniał.";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BladUzytkownika {
    pub kod: String,
    pub tytul: String,
    pub co_sie_stalo: String,
    pub co_zrobic: Vec<String>,
    /// Treść techniczna — do skopiowania administracji, nie do czytania przez gracza.
    pub szczegoly: String,
}

impl BladUzytkownika {
    fn nowy(
        kod: &str,
        tytul: &str,
        co_sie_stalo: impl Into<String>,
        co_zrobic: &[&str],
        szczegoly: impl Into<String>,
    ) -> Self {
        Self {
            kod: kod.to_string(),
            tytul: tytul.to_string(),
            co_sie_stalo: co_sie_stalo.into(),
            co_zrobic: co_zrobic.iter().map(|s| s.to_string()).collect(),
            szczegoly: szczegoly.into(),
        }
    }

    /// Tekst do wklejenia administracji.
    pub fn do_schowka(&self) -> String {
        format!(
            "Chmurkowy Launcher {}\nKod błędu: {}\n{}\n\nSzczegóły techniczne:\n{}",
            env!("CARGO_PKG_VERSION"),
            self.kod,
            self.tytul,
            self.szczegoly
        )
    }
}

/// Wszystko, co może pójść nie tak po drodze do uruchomionej gry.
#[derive(Debug, thiserror::Error)]
pub enum BladLaunchera {
    #[error(transparent)]
    Siec(#[from] NetError),
    #[error(transparent)]
    Java(#[from] JavaError),
    #[error(transparent)]
    Instalacja(#[from] InstallError),
    #[error(transparent)]
    Synchronizacja(#[from] SyncError),
    #[error(transparent)]
    Wersja(#[from] VersionError),
    #[error(transparent)]
    Start(#[from] LaunchError),
    #[error(transparent)]
    Paczka(#[from] ManifestError),
    #[error(transparent)]
    Logowanie(#[from] AuthError),
    #[error("operacja na pliku {0}: {1}")]
    Plik(String, std::io::Error),
    /// Siatka bezpieczeństwa: coś, czego nie potrafimy zaklasyfikować.
    /// Nie powinno się zdarzyć — jeśli się zdarza, brakuje nam kategorii.
    #[error("nierozpoznany błąd: {0}")]
    Nieznany(String),
    /// Gra wystartowała, ale zakończyła się szybko i nienormalnie.
    #[error("gra zakończyła się kodem {kod:?}")]
    GraPadla {
        kod: Option<i32>,
        ogon_logu: String,
        wlasne_argumenty: bool,
    },
}

impl BladLaunchera {
    pub fn dla_uzytkownika(&self) -> BladUzytkownika {
        match self {
            BladLaunchera::Siec(e) => z_sieci(e),
            BladLaunchera::Java(e) => z_javy(e),
            BladLaunchera::Instalacja(e) => z_instalacji(e),
            BladLaunchera::Synchronizacja(e) => z_synchronizacji(e),
            BladLaunchera::Wersja(e) => BladUzytkownika::nowy(
                "INST-03",
                "Pliki gry są niekompletne",
                "Launcher zainstalował grę, ale brakuje w niej pliku opisującego wersję.",
                &[
                    "Wejdź w Ustawienia i kliknij „Napraw instalację”.",
                    "Uruchom launcher ponownie — pliki gry pobiorą się od nowa.",
                    "Twoje światy i ustawienia gry zostaną nietknięte.",
                ],
                e.to_string(),
            ),
            BladLaunchera::Start(e) => BladUzytkownika::nowy(
                "INST-04",
                "Nie da się uruchomić gry",
                "Coś jest nie tak z plikami gry i launcher nie potrafi jej wystartować.",
                &[
                    "Wejdź w Ustawienia i kliknij „Napraw instalację”.",
                    "Uruchom launcher ponownie.",
                ],
                e.to_string(),
            ),
            BladLaunchera::Paczka(e) => z_paczki(e),
            BladLaunchera::Logowanie(e) => z_logowania(e),
            BladLaunchera::Plik(sciezka, e) => z_pliku(sciezka, e),
            BladLaunchera::Nieznany(tresc) => BladUzytkownika::nowy(
                "INNY-01",
                "Coś poszło nie tak, ale nie wiemy co",
                "Launcher natknął się na problem, którego nie potrafi rozpoznać. \
                 To znaczy, że trafiłeś na coś naprawdę rzadkiego.",
                &[
                    "Uruchom launcher ponownie — czasem to wystarcza.",
                    "Wejdź w Ustawienia i kliknij „Napraw instalację”.",
                    "Skopiuj szczegóły przyciskiem poniżej i wyślij je administracji. \
                     Ten błąd nie ma jeszcze własnego opisu, więc Twoje zgłoszenie \
                     realnie pomoże go dodać.",
                ],
                tresc.clone(),
            ),
            BladLaunchera::GraPadla {
                kod,
                ogon_logu,
                wlasne_argumenty,
            } => z_gry(*kod, ogon_logu, *wlasne_argumenty),
        }
    }
}

// --- SIEĆ ---

fn z_sieci(e: &NetError) -> BladUzytkownika {
    match e {
        NetError::Wyczerpano { plik, ostatni, .. } => {
            if ostatni.contains("niezgodny hash") {
                BladUzytkownika::nowy(
                    "SIE-03",
                    "Pobrany plik był uszkodzony",
                    "Launcher pobrał plik, ale okazał się uszkodzony i został odrzucony. \
                     Najczęściej winne jest niestabilne połączenie.",
                    &[
                        "Uruchom launcher ponownie — pobierze brakujący plik jeszcze raz.",
                        "Jeśli masz Wi-Fi, podejdź bliżej routera albo podłącz kabel.",
                        "Wyłącz na chwilę program antywirusowy — czasem psuje pobierane pliki.",
                    ],
                    format!("{plik}\n{ostatni}"),
                )
            } else {
                BladUzytkownika::nowy(
                    "SIE-02",
                    "Nie udało się pobrać pliku",
                    "Launcher kilka razy próbował pobrać jeden z plików gry i za każdym razem \
                     się nie udało.",
                    &[
                        "Sprawdź, czy internet działa — otwórz dowolną stronę w przeglądarce.",
                        "Uruchom launcher ponownie. To, co już się pobrało, nie pobierze się drugi raz.",
                        "Jeśli korzystasz z sieci szkolnej albo firmowej, może ona blokować pobieranie.",
                    ],
                    format!("{plik}\n{ostatni}"),
                )
            }
        }
        NetError::Io(sciezka, io) => z_pliku(sciezka, io),
    }
}

// --- PLIKI ---

fn z_pliku(sciezka: &str, e: &std::io::Error) -> BladUzytkownika {
    // ENOSPC na Linuksie i macOS, ERROR_DISK_FULL na Windowsie.
    let brak_miejsca = matches!(e.raw_os_error(), Some(28) | Some(112));

    if brak_miejsca {
        return BladUzytkownika::nowy(
            "PLIK-01",
            "Skończyło się miejsce na dysku",
            "Gra z modami zajmuje około 2 GB. Na dysku zabrakło miejsca w trakcie pobierania.",
            &[
                "Zwolnij miejsce na dysku — usuń niepotrzebne pliki albo opróżnij kosz.",
                "Potrzeba co najmniej 3 GB wolnego miejsca.",
                "Możesz też przenieść cały folder launchera na inny dysk i uruchomić go stamtąd.",
            ],
            format!("{sciezka}: {e}"),
        );
    }

    match e.kind() {
        std::io::ErrorKind::PermissionDenied => BladUzytkownika::nowy(
            "PLIK-02",
            "Launcher nie ma prawa zapisywać w tym miejscu",
            "System nie pozwala launcherowi zapisywać plików w folderze, w którym się znajduje.",
            &[
                "Przenieś folder z launcherem na Pulpit albo do folderu Dokumenty.",
                "Nie trzymaj launchera w „Program Files” ani na dysku systemowym poza folderem użytkownika.",
                "Uruchom launcher ponownie z nowego miejsca.",
            ],
            format!("{sciezka}: {e}"),
        ),
        _ => BladUzytkownika::nowy(
            "PLIK-03",
            "Nie udało się zapisać pliku",
            "Launcher nie mógł zapisać jednego z plików gry. Zwykle znaczy to, że plik \
             jest w tej chwili używany przez inny program.",
            &[
                "Sprawdź, czy gra nie jest już uruchomiona — jeśli tak, zamknij ją.",
                "Zamknij launcher i uruchom go ponownie.",
                "Program antywirusowy potrafi blokować pliki gry — dodaj folder launchera do wyjątków.",
            ],
            format!("{sciezka}: {e}"),
        ),
    }
}

// --- INSTALACJA ---

fn z_javy(e: &JavaError) -> BladUzytkownika {
    match e {
        JavaError::Net(n) => z_sieci(n),
        JavaError::Io(io) => z_pliku("katalog Javy", io),
        JavaError::Rozpakowanie(s) => BladUzytkownika::nowy(
            "INST-01",
            "Nie udało się rozpakować Javy",
            "Launcher pobrał Javę potrzebną do uruchomienia gry, ale nie dał rady jej rozpakować. \
             Zwykle znaczy to, że pobieranie zostało przerwane.",
            &[
                "Uruchom launcher ponownie — Java pobierze się jeszcze raz.",
                "Sprawdź, czy na dysku jest co najmniej 1 GB wolnego miejsca.",
                "Wyłącz na chwilę program antywirusowy i spróbuj ponownie.",
            ],
            s.clone(),
        ),
        JavaError::BrakBinarki(s) => BladUzytkownika::nowy(
            "INST-01",
            "Java rozpakowała się niekompletnie",
            "Launcher rozpakował Javę, ale nie znalazł w niej programu, który uruchamia grę.",
            &[
                "Usuń folder „data/java” z katalogu launchera.",
                "Uruchom launcher ponownie — Java pobierze się od nowa.",
            ],
            s.clone(),
        ),
    }
}

fn z_instalacji(e: &InstallError) -> BladUzytkownika {
    match e {
        InstallError::Net(n) => z_sieci(n),
        InstallError::Io(io) => z_pliku("pliki gry", io),
        InstallError::Uruchomienie(io) => BladUzytkownika::nowy(
            "INST-02",
            "Nie udało się uruchomić instalatora gry",
            "Launcher nie mógł uruchomić programu instalującego modyfikacje do Minecrafta.",
            &[
                "Wyłącz na chwilę program antywirusowy — często blokuje takie programy.",
                "Wejdź w Ustawienia i kliknij „Napraw instalację”, potem uruchom launcher ponownie.",
            ],
            io.to_string(),
        ),
        InstallError::Instalator { kod, wyjscie, .. } => BladUzytkownika::nowy(
            "INST-02",
            "Instalacja modyfikacji nie powiodła się",
            "Program instalujący modyfikacje do Minecrafta zakończył pracę błędem.",
            &[
                "Wejdź w Ustawienia i kliknij „Napraw instalację”.",
                "Uruchom launcher ponownie — instalacja zacznie się od zera.",
                "Sprawdź, czy na dysku jest co najmniej 3 GB wolnego miejsca.",
            ],
            format!("kod wyjścia {kod}\n{wyjscie}"),
        ),
        InstallError::IndeksZasobow(s) => BladUzytkownika::nowy(
            "INST-03",
            "Spis plików gry jest uszkodzony",
            "Launcher pobrał listę plików Minecrafta, ale nie potrafi jej odczytać.",
            &[
                "Wejdź w Ustawienia i kliknij „Napraw instalację”.",
                "Uruchom launcher ponownie.",
            ],
            s.clone(),
        ),
    }
}

fn z_synchronizacji(e: &SyncError) -> BladUzytkownika {
    match e {
        SyncError::Net(n) => z_sieci(n),
        SyncError::Io(sciezka, io) => z_pliku(sciezka, io),
        SyncError::Path(p) => BladUzytkownika::nowy(
            "PACZKA-02",
            "Paczka modów zawiera błąd",
            "Opis paczki modów jest nieprawidłowy i launcher odmówił jego użycia. \
             To błąd po stronie serwera, nie Twój.",
            &[
                "Poczekaj chwilę i uruchom launcher ponownie — administracja mogła właśnie wgrywać zmiany.",
                "Jeśli błąd się powtarza, zgłoś go administracji.",
            ],
            p.to_string(),
        ),
    }
}

// --- PACZKA ---

fn z_paczki(e: &ManifestError) -> BladUzytkownika {
    BladUzytkownika::nowy(
        "PACZKA-01",
        "Nie udało się odczytać informacji o paczce",
        "Launcher pobrał opis paczki modów, ale nie potrafi go zrozumieć. \
         To błąd po stronie serwera, nie Twój.",
        &[
            "Poczekaj kilka minut i uruchom launcher ponownie.",
            "Sprawdź, czy masz najnowszą wersję launchera.",
            "Jeśli błąd się powtarza, zgłoś go administracji.",
        ],
        e.to_string(),
    )
}

// --- LOGOWANIE ---

fn z_logowania(e: &AuthError) -> BladUzytkownika {
    match e {
        AuthError::Siec(s) => BladUzytkownika::nowy(
            "SIE-01",
            "Brak połączenia z internetem",
            "Launcher nie mógł połączyć się z serwerami Microsoftu, żeby Cię zalogować.",
            &[
                "Sprawdź, czy internet działa — otwórz dowolną stronę w przeglądarce.",
                "Jeśli masz Wi-Fi, podejdź bliżej routera.",
                "Spróbuj ponownie za chwilę.",
            ],
            s.clone(),
        ),
        AuthError::Wygasl => BladUzytkownika::nowy(
            "KONTO-01",
            "Kod logowania wygasł",
            "Kod jest ważny 15 minut. Ten już się przeterminował.",
            &["Kliknij „Zaloguj przez Microsoft” jeszcze raz i wpisz nowy kod."],
            "device code expired",
        ),
        AuthError::BrakGry => BladUzytkownika::nowy(
            "KONTO-04",
            "To konto nie ma kupionego Minecrafta",
            "Zalogowałeś się poprawnie, ale na tym koncie Microsoft nie ma gry Minecraft.",
            &[
                "Sprawdź, czy logujesz się na właściwe konto — to, na którym kupiliście grę.",
                "Jeśli w domu jest kilka kont Microsoft, spróbuj innego.",
                "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
            ],
            "profil Minecrafta nie istnieje (HTTP 404)",
        ),
        AuthError::Xbox(s) => {
            let (kod, tytul, co, kroki): (&str, &str, &str, &[&str]) = if s.contains("dziecka") {
                (
                    "KONTO-03",
                    "To konto dziecka i wymaga zgody rodzica",
                    "Microsoft blokuje logowanie kont dziecięcych, dopóki nie zostaną dodane \
                     do rodziny Microsoft.",
                    &[
                        "Rodzic powinien wejść na account.microsoft.com/family i dodać to konto do rodziny.",
                        "Po dodaniu spróbuj zalogować się jeszcze raz.",
                        "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                    ],
                )
            } else if s.contains("Xbox") {
                (
                    "KONTO-02",
                    "To konto nie ma profilu Xbox",
                    "Minecraft wymaga profilu Xbox, a to konto Microsoft jeszcze go nie ma.",
                    &[
                        "Wejdź na xbox.com, zaloguj się tym kontem i załóż profil — to darmowe i zajmuje minutę.",
                        "Wróć do launchera i zaloguj się ponownie.",
                    ],
                )
            } else {
                (
                    "KONTO-05",
                    "Microsoft odmówił logowania",
                    "Serwery Microsoftu odrzuciły logowanie i nie podały powodu, który \
                     launcher potrafi rozpoznać.",
                    &[
                        "Spróbuj zalogować się jeszcze raz za kilka minut.",
                        "Sprawdź, czy możesz zalogować się na minecraft.net w przeglądarce.",
                        "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                    ],
                )
            };
            BladUzytkownika::nowy(kod, tytul, co, kroki, s.clone())
        }
        AuthError::Odmowa(s) => BladUzytkownika::nowy(
            "KONTO-05",
            "Logowanie nie powiodło się",
            "Microsoft przerwał logowanie. Zdarza się, gdy okno logowania zostanie zamknięte \
             albo kod wpisany źle.",
            &[
                "Kliknij „Zaloguj przez Microsoft” jeszcze raz.",
                "Uważnie przepisz kod — najprościej użyć przycisku „Kopiuj kod i otwórz przeglądarkę”.",
                "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
            ],
            s.clone(),
        ),
    }
}

// --- GRA ---

fn z_gry(kod: Option<i32>, ogon_logu: &str, wlasne_argumenty: bool) -> BladUzytkownika {
    let log = ogon_logu.to_lowercase();

    if log.contains("outofmemoryerror") || log.contains("java heap space") {
        return BladUzytkownika::nowy(
            "GRA-02",
            "Grze zabrakło pamięci",
            "Paczka z 249 modami potrzebuje dużo pamięci. Tyle, ile jej przydzielono, nie wystarczyło.",
            &[
                "Wejdź w Ustawienia i przesuń suwak pamięci na 6144 MB albo więcej.",
                "Zamknij przeglądarkę i inne programy przed uruchomieniem gry.",
                "Jeśli komputer ma 8 GB pamięci lub mniej, nie ustawiaj więcej niż 4096 MB — zabraknie jej systemowi.",
            ],
            format!("kod wyjścia {kod:?}\n{ogon_logu}"),
        );
    }

    // Zle wlasne parametry Javy zabijaja proces natychmiast i z bardzo krotkim logiem.
    if wlasne_argumenty
        && (log.contains("unrecognized option")
            || log.contains("could not create the java virtual machine")
            || log.contains("invalid maximum heap size")
            || log.contains("unrecognized vm option"))
    {
        return BladUzytkownika::nowy(
            "GRA-03",
            "Java odrzuciła Twoje dodatkowe parametry",
            "W Ustawieniach są wpisane dodatkowe parametry Javy, których Java nie rozumie, \
             więc gra w ogóle nie wystartowała.",
            &[
                "Wejdź w Ustawienia i wyczyść pole „Dodatkowe parametry Javy”.",
                "Uruchom grę ponownie — powinna wystartować.",
                "Jeśli chcesz używać własnych parametrów, dodawaj je po jednym i sprawdzaj po każdym.",
            ],
            format!("kod wyjścia {kod:?}\n{ogon_logu}"),
        );
    }

    if log.contains("mixin") || log.contains("modloadingexception") || log.contains("crash report")
    {
        return BladUzytkownika::nowy(
            "GRA-04",
            "Jeden z modów spowodował błąd",
            "Gra wystartowała, ale jedna z modyfikacji przerwała jej uruchamianie. \
             To najczęściej problem z samą paczką, a nie z Twoim komputerem.",
            &[
                "Wejdź w Ustawienia i kliknij „Napraw instalację”, potem uruchom grę ponownie.",
                "Jeśli to nie pomoże, skopiuj szczegóły przyciskiem poniżej i wyślij je administracji.",
                "Prawdopodobnie ten sam błąd mają inni gracze i administracja już o nim wie.",
            ],
            format!("kod wyjścia {kod:?}\n{ogon_logu}"),
        );
    }

    BladUzytkownika::nowy(
        "GRA-01",
        "Gra zamknęła się zaraz po uruchomieniu",
        "Minecraft wystartował, ale zakończył się po kilku sekundach.",
        &[
            "Uruchom grę jeszcze raz — czasem wystarczy druga próba.",
            "Wejdź w Ustawienia i sprawdź, czy pamięć jest ustawiona na co najmniej 4096 MB.",
            "Zaktualizuj sterowniki karty graficznej.",
            "Wejdź w Ustawienia i kliknij „Napraw instalację”.",
        ],
        format!("kod wyjścia {kod:?}\n{ogon_logu}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn io(kind: std::io::ErrorKind) -> std::io::Error {
        std::io::Error::new(kind, "test")
    }

    #[test]
    fn brak_uprawnien_ma_wlasny_kod() {
        let b = BladLaunchera::Plik("data/mods".into(), io(std::io::ErrorKind::PermissionDenied))
            .dla_uzytkownika();
        assert_eq!(b.kod, "PLIK-02");
        assert!(b.co_zrobic.iter().any(|k| k.contains("Pulpit")));
    }

    #[test]
    fn brak_miejsca_rozpoznany_po_kodzie_systemowym() {
        let b = BladLaunchera::Plik("data".into(), std::io::Error::from_raw_os_error(28))
            .dla_uzytkownika();
        assert_eq!(b.kod, "PLIK-01");
    }

    #[test]
    fn zerwane_pobieranie_to_siec_a_uszkodzony_plik_to_inny_kod() {
        let zerwane = BladLaunchera::Siec(NetError::Wyczerpano {
            plik: "a.jar".into(),
            adresy: vec![],
            ostatni: "connection refused".into(),
        });
        assert_eq!(zerwane.dla_uzytkownika().kod, "SIE-02");

        let uszkodzony = BladLaunchera::Siec(NetError::Wyczerpano {
            plik: "a.jar".into(),
            adresy: vec![],
            ostatni: "niezgodny hash pobranej treści z https://x".into(),
        });
        assert_eq!(uszkodzony.dla_uzytkownika().kod, "SIE-03");
    }

    #[test]
    fn brak_pamieci_w_logu_daje_rade_o_suwaku() {
        let b = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: "java.lang.OutOfMemoryError: Java heap space".into(),
            wlasne_argumenty: false,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-02");
        assert!(b.co_zrobic[0].contains("6144"));
    }

    #[test]
    fn zle_parametry_rozpoznane_tylko_gdy_uzytkownik_je_ustawil() {
        let log = "Unrecognized option: -XXbzdura";
        let z_wlasnymi = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: log.into(),
            wlasne_argumenty: true,
        }
        .dla_uzytkownika();
        assert_eq!(z_wlasnymi.kod, "GRA-03");

        // Bez wlasnych parametrow ta sama tresc nie moze obwiniac uzytkownika.
        let bez = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: log.into(),
            wlasne_argumenty: false,
        }
        .dla_uzytkownika();
        assert_ne!(bez.kod, "GRA-03");
    }

    #[test]
    fn crash_moda_ma_wlasna_kategorie() {
        let b = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: "net.neoforged.fml.ModLoadingException: something".into(),
            wlasne_argumenty: false,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-04");
    }

    #[test]
    fn konto_dziecka_i_brak_xboxa_to_rozne_kody() {
        let dziecko =
            BladLaunchera::Logowanie(AuthError::Xbox("to konto dziecka — musi".into()))
                .dla_uzytkownika();
        assert_eq!(dziecko.kod, "KONTO-03");

        let xbox = BladLaunchera::Logowanie(AuthError::Xbox(
            "to konto Microsoft nie ma profilu Xbox".into(),
        ))
        .dla_uzytkownika();
        assert_eq!(xbox.kod, "KONTO-02");
    }

    #[test]
    fn kazdy_blad_ma_kod_tytul_i_przynajmniej_jedna_rade() {
        let przypadki = vec![
            BladLaunchera::Plik("x".into(), io(std::io::ErrorKind::Other)),
            BladLaunchera::Logowanie(AuthError::Wygasl),
            BladLaunchera::Logowanie(AuthError::BrakGry),
            BladLaunchera::Start(LaunchError::BrakIndeksuZasobow),
            BladLaunchera::GraPadla {
                kod: None,
                ogon_logu: String::new(),
                wlasne_argumenty: false,
            },
        ];
        for p in przypadki {
            let b = p.dla_uzytkownika();
            assert!(!b.kod.is_empty(), "brak kodu dla {p:?}");
            assert!(!b.tytul.is_empty(), "brak tytulu dla {p:?}");
            assert!(!b.co_sie_stalo.is_empty(), "brak opisu dla {p:?}");
            assert!(!b.co_zrobic.is_empty(), "brak rad dla {p:?}");
            // Zargon nie ma prawa trafic do tytulu widzianego przez rodzica.
            let t = b.tytul.to_lowercase();
            for slowo in ["error", "exception", "null", "http", "socket"] {
                assert!(!t.contains(slowo), "zargon '{slowo}' w tytule: {}", b.tytul);
            }
        }
    }

    #[test]
    fn nierozpoznany_blad_tez_dostaje_kod() {
        let b = BladLaunchera::Nieznany("cos zupelnie nowego".into()).dla_uzytkownika();
        assert_eq!(b.kod, "INNY-01");
        assert!(!b.co_zrobic.is_empty());
        // Tresc techniczna musi przetrwac, bo to jedyny slad dla administracji.
        assert!(b.szczegoly.contains("cos zupelnie nowego"));
        assert!(b.do_schowka().contains("cos zupelnie nowego"));
    }

    #[test]
    fn schowek_zawiera_kod_i_wersje() {
        let b = BladLaunchera::Logowanie(AuthError::Wygasl).dla_uzytkownika();
        let s = b.do_schowka();
        assert!(s.contains("KONTO-01"));
        assert!(s.contains(env!("CARGO_PKG_VERSION")));
    }
}
