//! Katalog błędów pisany dla rodzica, który nie zna się na komputerach.
//!
//! Każdy błąd ma krótki kod (łatwy do podyktowania przez telefon), zdanie
//! opisujące co się stało bez żargonu, i listę konkretnych kroków do wykonania.
//! Szczegóły techniczne są osobno — dla administracji, nie dla gracza.

use crate::auth::msa::{AuthError, PowodOdmowy, PowodXbox};
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

    /// Raport do wklejenia administracji serwera.
    ///
    /// Wersję przyjmujemy z zewnątrz, bo `env!("CARGO_PKG_VERSION")` zwróciłby
    /// tutaj wersję biblioteki `chmurka-core`, a nie launchera — administracja
    /// dostawała przez to „0.1.0" niezależnie od tego, co gracz uruchomił.
    pub fn do_schowka(&self, wersja_launchera: &str) -> String {
        format!(
            "Chmurkowy Launcher {}\nKod błędu: {}\n{}\n\nSzczegóły techniczne:\n{}",
            wersja_launchera, self.kod, self.tytul, self.szczegoly
        )
    }
}

/// Co launcher może zrobić sam, zanim w ogóle pokaże graczowi błąd.
///
/// Rady w katalogu błędów są poprawne, ale ludzie nie czytają okien —
/// „wejdź w Ustawienia i kliknij Napraw instalację" jest dla dziesięciolatka
/// jedną wielką ścianą tekstu. Więc launcher wykonuje tę radę sam i pokazuje
/// błąd dopiero wtedy, gdy to nie pomogło.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Samonaprawa {
    /// Nic nie ruszamy, po prostu próbujemy jeszcze raz. Chwilowy problem
    /// z siecią albo z zapisem pliku często mija sam.
    Ponow,
    /// Kasujemy pobraną Javę i bierzemy ją od nowa.
    JavaOdNowa,
    /// Kasujemy pliki gry i instalujemy je od nowa. Światy, ustawienia gry
    /// i paczka modów leżą w osobnym katalogu i zostają nietknięte.
    GraOdNowa,
    /// Tego launcher sam nie naprawi — dopiero tu ma sens pokazanie błędu.
    Nic,
}

impl BladUzytkownika {
    /// Lekarstwo dobrane po kodzie błędu.
    ///
    /// Celowo po kodzie, a nie po typie wyjątku: kod jest tym, co widzi gracz
    /// i co trafia do administracji, więc reguła i komunikat nie rozjadą się
    /// przy kolejnej zmianie w środku.
    pub fn samonaprawa(&self) -> Samonaprawa {
        match self.kod.as_str() {
            // Sieć i zapis pliku — powtórka bywa wszystkim, czego trzeba.
            "SIE-01" | "SIE-02" | "SIE-03" | "SIE-04" | "PLIK-03" => Samonaprawa::Ponow,
            // Java pobrała się niekompletnie.
            "INST-01" => Samonaprawa::JavaOdNowa,
            // Pliki gry są niekompletne albo popsute.
            "INST-02" | "INST-03" | "INST-04" => Samonaprawa::GraOdNowa,
            // Dysk pełny, brak uprawnień, logowanie, padnięta gra, zła paczka
            // oraz wszystko, co gracz sam wpisał w Ustawieniach (USTAW-*):
            // powtórka nic nie zmieni, a kasowanie plików tylko zaszkodzi.
            // Przy własnej komendzie ponowienie byłoby wręcz szkodliwe —
            // uruchomiłoby ją trzy razy pod rząd.
            _ => Samonaprawa::Nic,
        }
    }

    /// Zdanie pokazywane graczowi, gdy launcher naprawia coś sam.
    /// Ma uspokajać, a nie tłumaczyć — szczegóły i tak nikt nie czyta.
    pub fn opis_samonaprawy(&self) -> &'static str {
        match self.samonaprawa() {
            Samonaprawa::Ponow => "Coś się nie udało. Próbuję jeszcze raz…",
            Samonaprawa::JavaOdNowa => "Coś się nie udało. Pobieram Javę od nowa…",
            Samonaprawa::GraOdNowa => "Coś się nie udało. Pobieram pliki gry od nowa…",
            Samonaprawa::Nic => "",
        }
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
    /// Własna komenda gracza z zakładki „Zaawansowane".
    #[error(transparent)]
    Komenda(#[from] crate::komendy::BladKomendy),
    /// Java wskazana ręcznie w zakładce „Java" nie nadaje się do gry.
    #[error("Java wskazana w Ustawieniach nie nadaje się do gry: {powod}")]
    WlasnaJava { sciezka: String, powod: String },
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
        /// Gry nie ubił błąd, tylko system — zabrakło mu pamięci.
        zabita_przez_system: bool,
        /// Ile pamięci gra dostała. Bez tego nie da się odróżnić „podnieś
        /// suwak" od „ten komputer dał już wszystko, co miał".
        sterta_mb: u32,
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
            BladLaunchera::Komenda(e) => z_komendy(e),
            BladLaunchera::WlasnaJava { sciezka, powod } => BladUzytkownika::nowy(
                "USTAW-01",
                "Java wskazana w Ustawieniach nie działa",
                "W Ustawieniach, w zakładce „Java”, wpisana jest własna ścieżka do Javy.                  Launcher nie potrafi jej użyć, więc gra nie ruszy.",
                &[
                    "Wejdź w Ustawienia → Java i kliknij „Przywróć domyślne”.                      Launcher wróci wtedy do Javy, którą pobiera sam, i to zwykle                      załatwia sprawę.",
                    "Jeśli chcesz zostać przy własnej Javie, kliknij obok pola                      „Sprawdź” — launcher powie, co jest z nią nie tak.",
                ],
                format!("{sciezka}\n{powod}"),
            ),
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
                zabita_przez_system,
                sterta_mb,
            } => z_gry(
                *kod,
                ogon_logu,
                *wlasne_argumenty,
                *zabita_przez_system,
                *sterta_mb,
            ),
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

/// Własna komenda gracza — z zakładki „Zaawansowane".
///
/// Te błędy są inne od wszystkich pozostałych: przyczyna nie leży ani
/// w launcherze, ani w plikach gry, tylko w tekście, który ktoś sam wpisał.
/// Rada „napraw instalację" byłaby tu myląca, bo instalacja jest w porządku.
fn z_komendy(e: &crate::komendy::BladKomendy) -> BladUzytkownika {
    use crate::komendy::{BladKomendy, Etap};

    let (etap, komenda) = match e {
        BladKomendy::NieUruchomil { etap, komenda, .. }
        | BladKomendy::Zawiesila { etap, komenda, .. }
        | BladKomendy::Zwrocila { etap, komenda, .. } => (*etap, komenda.clone()),
    };

    // Komenda „po zakończeniu gry" nie ma prawa nikomu popsuć grania —
    // gra już się skończyła. Mówimy o tym spokojniej.
    let gra_ucierpiala = etap == Etap::Przed;

    let (kod, tytul, co) = match e {
        BladKomendy::Zawiesila { sekundy, .. } => (
            "USTAW-03",
            "Twoja komenda się zawiesiła",
            format!(
                "Komenda, którą wpisałeś w Ustawieniach ({}), nie skończyła się                  w ciągu {sekundy} sekund, więc launcher ją przerwał. Najczęściej                  znaczy to, że komenda na coś czeka — na hasło albo na wciśnięcie                  klawisza — a nie ma jak o to zapytać.",
                etap.opis()
            ),
        ),
        _ => (
            "USTAW-02",
            if gra_ucierpiala {
                "Twoja komenda nie wykonała się poprawnie"
            } else {
                "Komenda po zakończeniu gry nie wykonała się poprawnie"
            },
            format!(
                "Komenda, którą wpisałeś w Ustawieniach ({}), zakończyła się błędem.",
                etap.opis()
            ),
        ),
    };

    let mut kroki: Vec<&str> = vec![
        "Wejdź w Ustawienia → Zaawansowane i sprawdź wpisaną komendę.          Szczegóły poniżej zawierają to, co sama wypisała.",
        "Wyczyść pole, jeśli nie wiesz, skąd się tam wzięło — puste jest bezpieczne          i niczego nie psuje.",
    ];
    if gra_ucierpiala {
        kroki.push(
            "Dopóki komenda się nie wykona, launcher nie uruchomi gry — tak ma być,              bo komendy przed startem zwykle coś przygotowują.",
        );
    }

    BladUzytkownika::nowy(kod, tytul, co, &kroki, format!("{komenda}\n{e}"))
}

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
        InstallError::InstalatorSiec { wyjscie, .. } => BladUzytkownika::nowy(
            "SIE-04",
            "Instalator gry nie mógł pobrać plików",
            "Program instalujący modyfikacje próbował pobrać pliki z internetu i nie zdążył \
             się połączyć. Dzieje się tak na wolnym łączu albo gdy sieć przepuszcza tylko \
             część połączeń — nawet jeśli strony w przeglądarce otwierają się normalnie.",
            &[
                "Uruchom launcher ponownie — to, co już się pobrało, zostaje na dysku.",
                "Jeśli masz Wi-Fi, podejdź bliżej routera albo podłącz kabel.",
                "Jeśli to sieć szkolna albo firmowa, spróbuj na domowej lub na telefonie.",
            ],
            wyjscie.clone(),
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
        // Decyzja idzie po rozpoznanym powodzie, a nie po treści komunikatu.
        // Dopóki szukaliśmy w nim słowa „Xbox", zdanie „Xbox Live odrzucił
        // logowanie, kod 2148916235" trafiało w radę o zakładaniu profilu Xbox,
        // choć chodziło o niedostępność usługi w danym kraju.
        AuthError::Xbox { powod, szczegoly } => {
            let (kod, tytul, co, kroki): (&str, &str, &str, &[&str]) = match powod {
                PowodXbox::KontoDziecka => (
                    "KONTO-03",
                    "To konto dziecka i wymaga zgody rodzica",
                    "Microsoft blokuje logowanie kont dziecięcych, dopóki nie zostaną dodane \
                     do rodziny Microsoft.",
                    &[
                        "Rodzic powinien wejść na account.microsoft.com/family i dodać to konto do rodziny.",
                        "Po dodaniu spróbuj zalogować się jeszcze raz.",
                        "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                    ],
                ),
                PowodXbox::BrakProfiluXbox => (
                    "KONTO-02",
                    "To konto nie ma profilu Xbox",
                    "Minecraft wymaga profilu Xbox, a to konto Microsoft jeszcze go nie ma.",
                    &[
                        "Wejdź na xbox.com, zaloguj się tym kontem i załóż profil — to darmowe i zajmuje minutę.",
                        "Wróć do launchera i zaloguj się ponownie.",
                    ],
                ),
                PowodXbox::RegionNiedostepny => (
                    "KONTO-06",
                    "Xbox Live nie działa w kraju ustawionym na tym koncie",
                    "Konto Microsoft ma ustawiony kraj, w którym Xbox Live nie jest dostępny. \
                     To ustawienie samego konta, nie komputera ani gry.",
                    &[
                        "Wejdź na account.microsoft.com, otwórz „Twoje dane” i sprawdź kraj lub region.",
                        "Po poprawieniu kraju zaloguj się w launcherze jeszcze raz.",
                        "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                    ],
                ),
                PowodXbox::WymaganaWeryfikacja => (
                    "KONTO-07",
                    "Microsoft chce potwierdzenia na stronie konta",
                    "Zanim to konto zaloguje się do gry, Microsoft wymaga dokończenia czegoś \
                     na stronie konta — zwykle potwierdzenia wieku albo zgody rodzica.",
                    &[
                        "Wejdź na account.microsoft.com i zaloguj się tym samym kontem.",
                        "Wykonaj to, o co poprosi strona, a potem wróć do launchera.",
                        "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                    ],
                ),
                PowodXbox::KontoZablokowane => (
                    "KONTO-08",
                    "To konto jest zablokowane przez Microsoft",
                    "Microsoft zablokował temu kontu dostęp do Xbox Live. Launcher nie ma jak \
                     tego obejść.",
                    &[
                        "Wejdź na account.microsoft.com i sprawdź, czy Microsoft czegoś nie wymaga.",
                        "Zaloguj się innym kontem, jeśli masz do niego dostęp.",
                        "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                    ],
                ),
                PowodXbox::Inny => (
                    "KONTO-05",
                    "Microsoft odmówił logowania",
                    "Serwery Microsoftu odrzuciły logowanie i nie podały powodu, który \
                     launcher potrafi rozpoznać. Szczegóły poniżej są dla administracji — \
                     to z nich wynika, co poszło nie tak.",
                    &[
                        "Spróbuj zalogować się jeszcze raz za kilka minut.",
                        "Sprawdź, czy możesz zalogować się na minecraft.net w przeglądarce.",
                        "Skopiuj szczegóły przyciskiem poniżej i wyślij je administracji.",
                        "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                    ],
                ),
            };
            BladUzytkownika::nowy(kod, tytul, co, kroki, szczegoly.clone())
        }
        AuthError::Odmowa { powod, szczegoly } => match powod {
            // Gracz kliknął „Nie". Rada o przepisywaniu kodu byłaby tu
            // myląca — kod przepisał dobrze, tylko nie potwierdził zgody.
            PowodOdmowy::Odrzucone => BladUzytkownika::nowy(
                "KONTO-09",
                "Logowanie zostało odrzucone",
                "Na stronie Microsoftu kliknięto „Nie” albo okno zostało zamknięte \
                 przed potwierdzeniem.",
                &[
                    "Kliknij „Zaloguj przez Microsoft” jeszcze raz.",
                    "Na stronie Microsoftu potwierdź, że zgadzasz się zalogować — \
                     trzeba kliknąć „Tak”.",
                    "Do grania na własnym świecie możesz na razie użyć trybu offline.",
                ],
                szczegoly.clone(),
            ),
            // Kod urządzenia jest jednorazowy i ma kilkanaście minut ważności.
            PowodOdmowy::KodNiewazny => BladUzytkownika::nowy(
                "KONTO-10",
                "Kod stracił ważność",
                "Kod do wpisania na stronie Microsoftu jest jednorazowy i ważny tylko \
                 kilkanaście minut. Ten już się przeterminował albo został wcześniej użyty.",
                &[
                    "Kliknij „Zaloguj przez Microsoft” jeszcze raz — dostaniesz nowy kod.",
                    "Wpisz go od razu; nie odświeżaj strony z kodem i nie otwieraj jej dwa razy.",
                    "Jeśli logujesz się z telefonu, miej launcher otwarty do końca — \
                     on czeka na potwierdzenie.",
                ],
                szczegoly.clone(),
            ),
            // Kod był jeszcze ważny, a Microsoft i tak odmówił. Rada
            // „weź nowy kod" jest tu najgorsza z możliwych: gracz ponawia,
            // dostaje to samo i ma prawo uznać, że launcher jest zepsuty.
            PowodOdmowy::KontoWymagaDzialania => BladUzytkownika::nowy(
                "KONTO-11",
                "Microsoft nie wpuszcza tego konta",
                "Kod został wpisany poprawnie i jeszcze nie wygasł, ale Microsoft nie wydał \
                 dostępu. Tak odpowiada, gdy konto wymaga czegoś, czego okienko z kodem nie \
                 potrafi pokazać: potwierdzenia tożsamości, zgody rodzica albo akceptacji \
                 nowego regulaminu. Ponawianie tutaj nic nie zmieni.",
                &[
                    "Otwórz w przeglądarce account.microsoft.com i zaloguj się na to samo konto — \
                     Microsoft pokaże tam, czego mu brakuje.",
                    "Jeśli to konto dziecka w rodzinie Microsoft, zgodę musi kliknąć rodzic \
                     ze swojego konta.",
                    "Gdy przeglądarka przestanie o cokolwiek pytać, wróć tutaj i kliknij \
                     „Zaloguj przez Microsoft” jeszcze raz.",
                    "Do tego czasu możesz grać w trybie offline — świat i postępy zostaną.",
                ],
                szczegoly.clone(),
            ),
            // Odświeżanie zapisanego logowania, nie kod urządzenia. Gracz nic
            // nie przepisywał, więc rada o przepisywaniu byłaby tu dziwaczna.
            PowodOdmowy::ZapisaneLogowanieWygaslo => BladUzytkownika::nowy(
                "KONTO-12",
                "Zapisane logowanie wygasło",
                "Launcher miał zapamiętane logowanie do tego konta, ale Microsoft już go nie \
                 przyjmuje. Dzieje się tak po zmianie hasła, po dłuższej przerwie w graniu \
                 albo gdy ktoś wylogował urządzenia w ustawieniach konta.",
                &[
                    "Kliknij „Zaloguj przez Microsoft” — wystarczy zalogować się jeszcze raz.",
                    "Nic nie przepadło: świat, ustawienia i paczka modów zostają na miejscu.",
                    "Jeśli logowanie znów się nie uda, skopiuj szczegóły przyciskiem poniżej \
                     i wyślij je administracji.",
                ],
                szczegoly.clone(),
            ),
            PowodOdmowy::Inny => BladUzytkownika::nowy(
                "KONTO-05",
                "Logowanie nie powiodło się",
                "Microsoft przerwał logowanie. Zdarza się, gdy okno logowania zostanie zamknięte \
                 albo kod wpisany źle.",
                &[
                    "Kliknij „Zaloguj przez Microsoft” jeszcze raz.",
                    "Uważnie przepisz kod — najprościej użyć przycisku „Kopiuj kod i otwórz przeglądarkę”.",
                    "Do testów możesz na razie użyć trybu offline na ekranie logowania.",
                ],
                szczegoly.clone(),
            ),
        },
    }
}

// --- GRA ---

fn z_gry(
    kod: Option<i32>,
    ogon_logu: &str,
    wlasne_argumenty: bool,
    zabita_przez_system: bool,
    sterta_mb: u32,
) -> BladUzytkownika {
    let log = ogon_logu.to_lowercase();

    // Ten przypadek sprawdzamy pierwszy, bo log nic o nim nie mówi. System
    // ubija grę sygnałem, którego nie da się przechwycić, więc zapis urywa się
    // w pół zdania i po samym logu wygląda to jak przypadkowa awaria.
    if zabita_przez_system {
        return BladUzytkownika::nowy(
            "GRA-05",
            "Komputerowi zabrakło pamięci i zamknął grę",
            "Gra nie zawiesiła się sama — to system ją zamknął, bo zabrakło mu pamięci \
             na wszystko naraz. Zwykle znaczy to, że grze przydzielono jej za dużo: \
             gra zajmuje o półtora do dwóch gigabajtów więcej, niż pokazuje suwak.",
            &[
                "Wejdź w Ustawienia i kliknij „Dobierz automatycznie” przy suwaku pamięci.",
                "Zamknij przeglądarkę i inne programy przed uruchomieniem gry.",
                "Jeśli to się powtarza, zejdź suwakiem jeszcze o 512 MB niżej.",
            ],
            format!("gra zabita przez system, kod wyjścia {kod:?}\n{ogon_logu}"),
        );
    }

    if log.contains("outofmemoryerror") || log.contains("java heap space") {
        // Czy suwak ma jeszcze cokolwiek do oddania? Jeśli gra dostała już
        // tyle, ile ten komputer bezpiecznie mieści, rada „podnieś suwak"
        // jest ślepą uliczką: wyżej system i tak zamknie grę. Trzeba wtedy
        // powiedzieć wprost, że to komputer jest za mały — inaczej rodzic
        // z dzieckiem będą w kółko przesuwać suwak i wracać do tego samego.
        let na_maksa = crate::pamiec::calkowita_mb()
            .is_some_and(|c| sterta_mb >= crate::pamiec::zalecana_mb(c));

        if na_maksa {
            return BladUzytkownika::nowy(
                "GRA-06",
                "Ten komputer ma za mało pamięci na tę paczkę",
                "Grze zabrakło pamięci, a dostała już tyle, ile ten komputer może jej dać. \
                 Przydzielenie jej więcej odebrałoby pamięć systemowi i gra zostałaby \
                 zamknięta jeszcze wcześniej. To nie jest wina ustawień ani instalacji.",
                &[
                    "Zamknij wszystkie inne programy — zwłaszcza przeglądarkę — i spróbuj raz jeszcze.",
                    "Wejdź w „Paczki” i wyłącz shadery, jeśli są włączone. Na słabszej grafice \
                     zajmują bardzo dużo pamięci.",
                    "Jeśli masz do wyboru inny komputer, zagraj na nim.",
                    "Napisz do administracji i podaj ten kod — może przygotować lżejszą paczkę.",
                ],
                format!("sterta {sterta_mb} MB, kod wyjścia {kod:?}\n{ogon_logu}"),
            );
        }

        return BladUzytkownika::nowy(
            "GRA-02",
            "Grze zabrakło pamięci",
            "Ta paczka modów potrzebuje dużo pamięci. Tyle, ile jej przydzielono, nie wystarczyło.",
            &[
                "Wejdź w Ustawienia i kliknij „Dobierz automatycznie” przy suwaku pamięci.",
                "Zamknij przeglądarkę i inne programy przed uruchomieniem gry.",
                "Nie podnoś suwaka na siłę — gra zajmuje o półtora do dwóch gigabajtów \
                 więcej, niż on pokazuje, i przy zbyt wysokim ustawieniu system ją zamknie.",
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

    /// Samonaprawa uruchamia akcje po trzy razy. Przy wlasnej komendzie
    /// gracza byloby to wrecz szkodliwe — „zrob kopie swiata" wykonaloby sie
    /// trzykrotnie, a literowka w skrypcie kazalaby czekac trzy limity czasu.
    #[test]
    fn wlasne_ustawienia_gracza_nigdy_nie_ida_do_samonaprawy() {
        use crate::komendy::{BladKomendy, Etap};

        let komenda = BladLaunchera::Komenda(BladKomendy::Zwrocila {
            etap: Etap::Przed,
            komenda: "cp swiat swiat.bak".into(),
            kod: Some(1),
            wyjscie: "cp: nie ma takiego pliku".into(),
        });
        let b = komenda.dla_uzytkownika();
        assert_eq!(b.kod, "USTAW-02");
        assert_eq!(b.samonaprawa(), Samonaprawa::Nic);

        let java = BladLaunchera::WlasnaJava {
            sciezka: "/usr/bin/nie-java".into(),
            powod: "to nie wyglada na Jave".into(),
        };
        let b = java.dla_uzytkownika();
        assert_eq!(b.kod, "USTAW-01");
        assert_eq!(b.samonaprawa(), Samonaprawa::Nic);
    }

    /// Zawieszona komenda ma wlasny kod, bo rada jest inna: nie „popraw
    /// komende", tylko „ona na cos czeka".
    #[test]
    fn zawieszona_komenda_ma_wlasny_kod() {
        use crate::komendy::{BladKomendy, Etap};

        let b = BladLaunchera::Komenda(BladKomendy::Zawiesila {
            etap: Etap::Przed,
            komenda: "sudo cos".into(),
            sekundy: 120,
        })
        .dla_uzytkownika();
        assert_eq!(b.kod, "USTAW-03");
        assert!(b.co_sie_stalo.contains("120"));
        assert_eq!(b.samonaprawa(), Samonaprawa::Nic);
    }

    /// Komenda po zakonczeniu gry niczego juz nie psuje — komunikat nie moze
    /// straszyc gracza, ze cos sie nie uruchomilo.
    #[test]
    fn komenda_po_grze_mowi_lagodniej_niz_ta_przed() {
        use crate::komendy::{BladKomendy, Etap};

        let zrob = |etap| {
            BladLaunchera::Komenda(BladKomendy::Zwrocila {
                etap,
                komenda: "echo x".into(),
                kod: Some(1),
                wyjscie: String::new(),
            })
            .dla_uzytkownika()
        };
        let przed = zrob(Etap::Przed);
        let po = zrob(Etap::Po);

        assert_ne!(przed.tytul, po.tytul);
        assert!(przed
            .co_zrobic
            .iter()
            .any(|k| k.contains("nie uruchomi gry")));
        assert!(
            !po.co_zrobic.iter().any(|k| k.contains("nie uruchomi gry")),
            "gra juz sie skonczyla, nie ma czego nie uruchomic"
        );
    }

    /// Kazdy blad, ktory pokazujemy graczowi, musi miec co najmniej jeden
    /// konkretny krok do wykonania. Okno z samym „cos poszlo nie tak" jest
    /// bezuzyteczne dla rodzica, ktory nie zna sie na komputerach.
    #[test]
    fn nowe_bledy_ustawien_maja_konkretne_kroki() {
        use crate::komendy::{BladKomendy, Etap};

        let bledy = [
            BladLaunchera::WlasnaJava {
                sciezka: "/x".into(),
                powod: "y".into(),
            },
            BladLaunchera::Komenda(BladKomendy::NieUruchomil {
                etap: Etap::Przed,
                komenda: "x".into(),
                powod: "y".into(),
            }),
        ];
        for e in &bledy {
            let b = e.dla_uzytkownika();
            assert!(!b.co_zrobic.is_empty(), "{} bez krokow", b.kod);
            assert!(
                b.co_zrobic.iter().any(|k| k.contains("Ustawienia")),
                "{} ma odsylac tam, gdzie lezy przyczyna",
                b.kod
            );
        }
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
            zabita_przez_system: false,
            sterta_mb: 4096,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-02");
        assert!(b.co_zrobic[0].contains("Dobierz automatycznie"));
        // Rada „podnies suwak do 6144" zabijala gre na maszynie z 8 GB:
        // sam suwak nie widzi poltora giga narzutu poza sterta.
        assert!(
            b.co_zrobic.iter().all(|r| !r.contains("6144")),
            "nie wolno podawac sztywnej liczby megabajtow: {:?}",
            b.co_zrobic
        );
        assert!(
            b.co_zrobic
                .iter()
                .any(|r| r.contains("więcej, niż on pokazuje")),
            "gracz musi uslyszec, ze gra bierze wiecej niz sterta"
        );
    }

    /// Prawdziwy przypadek z komputera testera: 8 GB pamieci, zintegrowana
    /// grafika, sterta juz na maksimum i mimo to OutOfMemoryError przy
    /// pieczeniu modeli. Rada „podnies suwak" byla wtedy slepa uliczka —
    /// wyzej system i tak zamknalby gre.
    #[test]
    fn brak_pamieci_przy_maksymalnej_stercie_to_za_slaby_komputer() {
        let Some(calkowita) = crate::pamiec::calkowita_mb() else {
            return; // bez odczytu pamieci nie ma czego sprawdzac
        };
        let na_maksa = crate::pamiec::zalecana_mb(calkowita);
        let b = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: "java.lang.OutOfMemoryError: Java heap space".into(),
            wlasne_argumenty: false,
            zabita_przez_system: false,
            sterta_mb: na_maksa,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-06");
        assert!(
            b.co_zrobic.iter().any(|r| r.contains("shadery")),
            "wylaczenie shaderow to jedyna realna dzwignia na takiej maszynie"
        );
        // Zadna rada nie moze kazac podnosic suwaka — nie ma juz czego podnosic.
        assert!(
            b.co_zrobic
                .iter()
                .all(|r| !r.contains("Dobierz automatycznie")),
            "to slepa uliczka na maszynie, ktora dala juz wszystko: {:?}",
            b.co_zrobic
        );
    }

    /// Gdy suwak stoi nisko, rada ma nadal odsylac do doboru pamieci —
    /// tam jeszcze jest co zyskac.
    #[test]
    fn brak_pamieci_przy_niskiej_stercie_wciaz_odsyla_do_suwaka() {
        let b = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: "java.lang.OutOfMemoryError: Java heap space".into(),
            wlasne_argumenty: false,
            zabita_przez_system: false,
            sterta_mb: 1024,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-02");
    }

    /// System ubija gre sygnalem, ktorego nie da sie przechwycic — w logu nie
    /// ma po tym sladu. Bez osobnego rozpoznania gracz dostawal „gra padla,
    /// nie wiemy czemu", choc przyczyne da sie naprawic jednym suwakiem.
    #[test]
    fn ubicie_przez_system_ma_wlasny_kod_mimo_pustego_logu() {
        let b = BladLaunchera::GraPadla {
            kod: None,
            ogon_logu: "[16:20:03] [Render thread/INFO]: Ładowanie tekstur".into(),
            wlasne_argumenty: false,
            zabita_przez_system: true,
            sterta_mb: 4096,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-05");
        assert!(b.tytul.contains("pamięci"));
        assert!(b.co_zrobic[0].contains("Dobierz automatycznie"));
    }

    /// Ubicie przez system rozpoznajemy przed czytaniem logu: log moglby
    /// zawierac stary, niezwiazany wpis o pamieci i wskazac zly kod.
    #[test]
    fn ubicie_przez_system_wygrywa_z_trescia_logu() {
        let b = BladLaunchera::GraPadla {
            kod: Some(137),
            ogon_logu: "java.lang.OutOfMemoryError: Java heap space".into(),
            wlasne_argumenty: false,
            zabita_przez_system: true,
            sterta_mb: 4096,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-05");
    }

    /// Gry ubitej przez system nie wolno probowac ponownie ani kasowac po niej
    /// plikow — nic z tego nie doda pamieci komputerowi.
    #[test]
    fn ubicia_przez_system_launcher_nie_naprawia_sam() {
        let b = BladLaunchera::GraPadla {
            kod: None,
            ogon_logu: String::new(),
            wlasne_argumenty: false,
            zabita_przez_system: true,
            sterta_mb: 4096,
        }
        .dla_uzytkownika();
        assert_eq!(b.samonaprawa(), Samonaprawa::Nic);
    }

    #[test]
    fn zle_parametry_rozpoznane_tylko_gdy_uzytkownik_je_ustawil() {
        let log = "Unrecognized option: -XXbzdura";
        let z_wlasnymi = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: log.into(),
            wlasne_argumenty: true,
            zabita_przez_system: false,
            sterta_mb: 4096,
        }
        .dla_uzytkownika();
        assert_eq!(z_wlasnymi.kod, "GRA-03");

        // Bez wlasnych parametrow ta sama tresc nie moze obwiniac uzytkownika.
        let bez = BladLaunchera::GraPadla {
            kod: Some(1),
            ogon_logu: log.into(),
            wlasne_argumenty: false,
            zabita_przez_system: false,
            sterta_mb: 4096,
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
            zabita_przez_system: false,
            sterta_mb: 4096,
        }
        .dla_uzytkownika();
        assert_eq!(b.kod, "GRA-04");
    }

    fn xbox(powod: PowodXbox) -> BladUzytkownika {
        BladLaunchera::Logowanie(AuthError::Xbox {
            powod,
            szczegoly: "szczegóły techniczne".into(),
        })
        .dla_uzytkownika()
    }

    /// Kazdy powod odmowy ma dostac WLASNA rade. Wczesniej wszystkie trafialy
    /// pod KONTO-05 z rada „uwaznie przepisz kod", ktora przy odrzuconej
    /// zgodzie albo przeterminowanym kodzie jest nietrafiona.
    #[test]
    fn kazdy_powod_odmowy_ma_wlasny_kod_i_rade() {
        use crate::auth::msa::PowodOdmowy;

        let zrob = |powod| {
            BladLaunchera::Logowanie(AuthError::Odmowa {
                powod,
                szczegoly: "invalid_grant: AADSTS70008".into(),
            })
            .dla_uzytkownika()
        };

        let odrzucone = zrob(PowodOdmowy::Odrzucone);
        let niewazny = zrob(PowodOdmowy::KodNiewazny);
        let konto = zrob(PowodOdmowy::KontoWymagaDzialania);
        let inny = zrob(PowodOdmowy::Inny);

        assert_eq!(odrzucone.kod, "KONTO-09");
        assert_eq!(niewazny.kod, "KONTO-10");
        assert_eq!(konto.kod, "KONTO-11");
        assert_eq!(inny.kod, "KONTO-05");

        // Cztery rozne tytuly — inaczej podzial nie ma sensu.
        assert_ne!(odrzucone.tytul, niewazny.tytul);
        assert_ne!(niewazny.tytul, konto.tytul);
        assert_ne!(konto.tytul, inny.tytul);

        // Sedno podzialu: przy odmowie dla konta ponawianie nie pomaga, wiec
        // nie wolno tu napisac, ze kod stracil waznosc ani kazac brac nowego.
        // To wlasnie ta rada zapetlila testera przy 0.4.26.
        assert!(
            !konto.co_zrobic.iter().any(|k| k.contains("nowy kod")),
            "{:?}",
            konto.co_zrobic
        );
        assert!(
            !konto.tytul.contains("ważnoś"),
            "tytul nie moze obiecywac wygasniecia: {}",
            konto.tytul
        );
        // Odwrotnie niz przy KONTO-10: tutaj trzeba graczowi wprost napisac,
        // ze kod byl dobry. Inaczej bedzie go przepisywal jeszcze staranniej.
        assert!(
            konto.co_sie_stalo.contains("nie wygasł"),
            "{}",
            konto.co_sie_stalo
        );
        // ...i odwrotnie: gracz musi dostac namiar na przegladarke, bo tylko
        // tam Microsoft pokaze, czego mu brakuje.
        assert!(
            konto
                .co_zrobic
                .iter()
                .any(|k| k.contains("account.microsoft.com")),
            "{:?}",
            konto.co_zrobic
        );

        // Rada o przepisywaniu kodu nie ma sensu, gdy kod przepisano dobrze,
        // tylko nie potwierdzono zgody.
        assert!(
            !odrzucone.co_zrobic.iter().any(|k| k.contains("przepisz")),
            "{:?}",
            odrzucone.co_zrobic
        );

        // Szczegoly techniczne musza doniesc opis od Microsoftu do administracji.
        for b in [&odrzucone, &niewazny, &konto, &inny] {
            assert!(b.szczegoly.contains("AADSTS70008"), "{}", b.szczegoly);
        }
    }

    /// Odswiezanie zapisanego logowania nie pokazuje graczowi zadnego kodu,
    /// wiec rady o przepisywaniu kodu sa tam bez sensu. Wczesniej odmowa przy
    /// odswiezaniu ladowala pod KONTO-05 wlasnie z taka rada.
    #[test]
    fn wygasle_zapisane_logowanie_nie_kaze_przepisywac_kodu() {
        use crate::auth::msa::PowodOdmowy;

        let b = BladLaunchera::Logowanie(AuthError::Odmowa {
            powod: PowodOdmowy::ZapisaneLogowanieWygaslo,
            szczegoly: "invalid_grant: The user must sign in again".into(),
        })
        .dla_uzytkownika();

        assert_eq!(b.kod, "KONTO-12");
        for krok in &b.co_zrobic {
            assert!(
                !krok.contains("przepisz") && !krok.contains("kod urządzenia"),
                "{krok}"
            );
        }
        // Gracz ma sie bac, ze straci swiat — trzeba to uprzedzic wprost.
        assert!(
            b.co_zrobic.iter().any(|k| k.contains("Nic nie przepadło")),
            "{:?}",
            b.co_zrobic
        );
    }

    /// Zadna z nowych odmow nie moze isc do samonaprawy — ponawianie logowania
    /// w kolko nie pomoze, a gracz ma do wykonania konkretny ruch.
    #[test]
    fn odmowy_logowania_nie_ida_do_samonaprawy() {
        use crate::auth::msa::PowodOdmowy;

        for powod in [
            PowodOdmowy::Odrzucone,
            PowodOdmowy::KodNiewazny,
            PowodOdmowy::KontoWymagaDzialania,
            PowodOdmowy::ZapisaneLogowanieWygaslo,
            PowodOdmowy::Inny,
        ] {
            let b = BladLaunchera::Logowanie(AuthError::Odmowa {
                powod,
                szczegoly: "x".into(),
            })
            .dla_uzytkownika();
            assert_eq!(b.samonaprawa(), Samonaprawa::Nic, "{}", b.kod);
        }
    }

    #[test]
    fn kazdy_powod_odmowy_xbox_ma_wlasny_kod() {
        assert_eq!(xbox(PowodXbox::KontoDziecka).kod, "KONTO-03");
        assert_eq!(xbox(PowodXbox::BrakProfiluXbox).kod, "KONTO-02");
        assert_eq!(xbox(PowodXbox::RegionNiedostepny).kod, "KONTO-06");
        assert_eq!(xbox(PowodXbox::WymaganaWeryfikacja).kod, "KONTO-07");
        assert_eq!(xbox(PowodXbox::KontoZablokowane).kod, "KONTO-08");
        assert_eq!(xbox(PowodXbox::Inny).kod, "KONTO-05");
    }

    /// Dopoki rozpoznawalismy powod po tresci komunikatu, zdanie „Xbox Live
    /// odrzucil logowanie, kod 2148916235" trafialo w rade o zakladaniu
    /// profilu Xbox — bo zawieralo slowo „Xbox". Decyzja musi stac na
    /// rozpoznanym powodzie, a nie na tekscie dla czlowieka.
    #[test]
    fn tresc_komunikatu_nie_decyduje_o_kodzie() {
        let b = BladLaunchera::Logowanie(AuthError::Xbox {
            powod: PowodXbox::RegionNiedostepny,
            szczegoly: "XSTS: HTTP 401, XErr 2148916235 (Xbox Live niedostępny w tym kraju)".into(),
        })
        .dla_uzytkownika();
        assert_eq!(
            b.kod, "KONTO-06",
            "slowo Xbox w tresci nie moze przewazyc o kodzie"
        );
    }

    /// Szczegoly to jedyny slad dla administracji — musza przetrwac
    /// przejscie przez katalog bledow w calosci.
    #[test]
    fn szczegoly_odmowy_trafiaja_do_raportu() {
        let b = BladLaunchera::Logowanie(AuthError::Xbox {
            powod: PowodXbox::Inny,
            szczegoly: "XSTS: HTTP 503, treść: usługa niedostępna".into(),
        })
        .dla_uzytkownika();
        assert!(b.szczegoly.contains("HTTP 503"), "{}", b.szczegoly);
        assert!(b.do_schowka("0.5.0").contains("HTTP 503"));
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
                zabita_przez_system: false,
                sterta_mb: 4096,
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
        assert!(b.do_schowka("0.4.3").contains("cos zupelnie nowego"));
    }

    #[test]
    fn schowek_zawiera_kod_i_wersje() {
        let b = BladLaunchera::Logowanie(AuthError::Wygasl).dla_uzytkownika();
        let s = b.do_schowka("0.4.3");
        assert!(s.contains("KONTO-01"));
        assert!(s.contains("0.4.3"));
    }

    /// Raport trafia do administracji, wiec musi podawac wersje launchera,
    /// a nie biblioteki. Przez pomylke pokazywal „0.1.0" kazdemu graczowi,
    /// niezaleznie od tego, co naprawde uruchomil.
    #[test]
    fn schowek_podaje_wersje_z_zewnatrz_a_nie_biblioteki() {
        let b = BladLaunchera::Logowanie(AuthError::Wygasl).dla_uzytkownika();
        let s = b.do_schowka("0.9.7");
        assert!(s.contains("Chmurkowy Launcher 0.9.7"), "{s}");
        assert!(
            !s.contains("0.1.0"),
            "wersja biblioteki nie ma prawa trafic do raportu: {s}"
        );
    }

    /// Kasowanie plikow to najostrzejszy lek, jaki launcher ma. Nie wolno go
    /// zastosowac tam, gdzie problem lezy poza plikami gry: przy pelnym dysku
    /// skasowalby 2 GB i nadal nie mial gdzie ich zapisac, a przy braku
    /// uprawnien albo wygaslym logowaniu nie zmienilby zupelnie nic.
    #[test]
    fn samonaprawa_nie_kasuje_plikow_gdy_to_nie_pomoze() {
        for kod in [
            "PLIK-01",
            "PLIK-02",
            "KONTO-01",
            "KONTO-02",
            "KONTO-03",
            "KONTO-04",
            "KONTO-05",
            "GRA-01",
            "GRA-02",
            "GRA-03",
            "GRA-04",
            "PACZKA-01",
            "PACZKA-02",
            "INNY-01",
        ] {
            let b = BladUzytkownika::nowy(kod, "t", "c", &["r"], "s");
            assert_eq!(
                b.samonaprawa(),
                Samonaprawa::Nic,
                "{kod} nie moze uruchamiac zadnej samonaprawy"
            );
        }
    }

    #[test]
    fn problemy_z_plikami_gry_launcher_naprawia_sam() {
        for kod in ["INST-02", "INST-03", "INST-04"] {
            let b = BladUzytkownika::nowy(kod, "t", "c", &["r"], "s");
            assert_eq!(b.samonaprawa(), Samonaprawa::GraOdNowa, "{kod}");
        }
        let java = BladUzytkownika::nowy("INST-01", "t", "c", &["r"], "s");
        assert_eq!(java.samonaprawa(), Samonaprawa::JavaOdNowa);
    }

    #[test]
    fn problemy_z_siecia_konczy_sie_zwykla_powtorka() {
        for kod in ["SIE-01", "SIE-02", "SIE-03", "SIE-04", "PLIK-03"] {
            let b = BladUzytkownika::nowy(kod, "t", "c", &["r"], "s");
            assert_eq!(b.samonaprawa(), Samonaprawa::Ponow, "{kod}");
        }
    }

    /// Kazdy lek musi miec co pokazac graczowi, inaczej pasek postepu
    /// zamilkby w polowie naprawy i wygladal na zawieszenie.
    #[test]
    fn kazda_samonaprawa_ma_swoje_zdanie() {
        for kod in ["SIE-02", "INST-01", "INST-03"] {
            let b = BladUzytkownika::nowy(kod, "t", "c", &["r"], "s");
            assert!(!b.opis_samonaprawy().is_empty(), "{kod}");
        }
    }

    /// Instalator, ktory nie dosiegnal serwerow, to problem z siecia,
    /// a nie z plikami — rada „napraw instalacje" niczego tu nie zmieni.
    #[test]
    fn instalator_bez_polaczenia_to_blad_sieci() {
        let b = BladLaunchera::Instalacja(InstallError::InstalatorSiec {
            loader: "neoforge-21.1.249".into(),
            wyjscie: "A problem installing was detected".into(),
        })
        .dla_uzytkownika();
        assert_eq!(b.kod, "SIE-04");
        assert!(b.co_zrobic.iter().any(|r| r.contains("router")));
    }
}
