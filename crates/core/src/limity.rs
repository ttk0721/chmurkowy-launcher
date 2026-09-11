//! Górne granice czasu dla wszystkiego, co czeka na świat zewnętrzny.
//!
//! Launcher nie miał ich wcale. `reqwest` domyślnie nie nakłada żadnego
//! limitu, więc host, który przyjmował połączenie i milkł, zatrzymywał
//! launcher na zawsze: pasek postępu stał w miejscu, przycisk GRAJ był
//! wygaszony, a jedynym wyjściem było zabicie procesu. Gorzej: cała
//! maszyneria odporności — trzy próby i zapasowe adresy — siedzi **za**
//! tym oczekiwaniem, więc przy zawieszeniu nie uruchamiała się ani razu.
//!
//! Wartości są w jednym miejscu, żeby nie rozjechały się między crate'ami.

use std::time::Duration;

/// Nawiązanie połączenia. Dziesięć sekund to dużo nawet dla wolnego łącza —
/// tu nie płyną jeszcze żadne dane, więc rozmiar pliku nie ma znaczenia.
pub const POLACZENIE: Duration = Duration::from_secs(10);

/// Całe żądanie, gdy odpowiedź jest mała: manifest, logowanie, odświeżenie
/// tokenu. Przy kilku kilobajtach limit na całość jest właściwym narzędziem.
pub const MALE_ZADANIE: Duration = Duration::from_secs(30);

/// Przerwa między kolejnymi kawałkami pobieranego pliku.
///
/// Celowo **nie** limit na całe żądanie: JRE waży 180 MB i na wolnym łączu
/// uczciwie schodzi kilkanaście minut. Limit na przerwę przerywa martwe
/// połączenie, nie karząc powolnego.
pub const BRAK_POSTEPU: Duration = Duration::from_secs(60);

/// Instalator NeoForge mieli około minuty. Kwadrans to zapas na wolny dysk
/// i wolne łącze, a jednocześnie granica, po której wiadomo, że stanął.
pub const INSTALATOR: Duration = Duration::from_secs(15 * 60);

/// Komenda wpisana przez gracza w Ustawieniach (przed startem, po wyjściu).
///
/// Limit jest tu ważniejszy niż gdziekolwiek indziej, bo treść wpisuje
/// człowiek: literówka w skrypcie albo program czekający na wciśnięcie
/// klawisza zawiesiłby start gry na zawsze, a launcher nie ma jak pokazać
/// takiemu procesowi konsoli. Dwie minuty starczą na kopię zapasową świata
/// czy podmianę konfiguracji.
pub const KOMENDA_GRACZA: Duration = Duration::from_secs(2 * 60);

/// Zapytanie Javy o wersję (`java -version`).
///
/// Zdrowa Java odpowiada w ułamku sekundy. Gdy ścieżka wskazuje na plik
/// w zawieszonym zasobie sieciowym, proces potrafi wisieć bez końca —
/// a dzieje się to w Ustawieniach, gdzie gracz patrzy na przycisk i czeka.
pub const SPRAWDZENIE_JAVY: Duration = Duration::from_secs(15);

/// Ile bajtów musi przyjść w oknie [`BRAK_POSTEPU`], żeby uznać pobieranie
/// za żywe.
///
/// Sam `read_timeout` tego nie łapie: zepsuty serwer może słać po kilka
/// bajtów co kilkanaście sekund i formalnie nigdy nie milczeć. Każdy odczyt
/// mieści się wtedy w limicie, a pobranie 180 MB trwałoby miesiącami —
/// z paskiem postępu stojącym w miejscu, bo meldujemy co 400 kB.
///
/// 64 kB na minutę to około kilobajta na sekundę. Wolniej niż modem z lat
/// dziewięćdziesiątych; to nie jest już powolne łącze, tylko zepsute.
pub const NAJMNIEJ_BAJTOW_NA_OKNO: u64 = 64 * 1024;

/// Klient do krótkich żądań: manifest, logowanie, profil gracza.
pub fn klient_maly() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(UZYTKOWNIK)
        .connect_timeout(POLACZENIE)
        .timeout(MALE_ZADANIE)
        .build()
        .expect("klient HTTP")
}

/// Klient do pobierania plików: bez limitu na całość, z limitem na ciszę.
pub fn klient_pobierania() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(UZYTKOWNIK)
        .connect_timeout(POLACZENIE)
        .read_timeout(BRAK_POSTEPU)
        .build()
        .expect("klient HTTP")
}

const UZYTKOWNIK: &str = concat!("ChmurkowyLauncher/", env!("CARGO_PKG_VERSION"));

#[cfg(test)]
mod tests {
    use super::*;

    /// Limit na cale zadanie przy pobieraniu duzych plikow bylby lekarstwem
    /// gorszym od choroby: urwalby uczciwe pobieranie 180 MB na wolnym laczu.
    /// Sprawdzamy to na wartosciach, bo samego klienta reqwest nie da sie
    /// o to zapytac.
    #[test]
    fn limit_pobierania_dotyczy_ciszy_a_nie_calosci() {
        // Gdyby ktos kiedys ustawil BRAK_POSTEPU tak nisko jak MALE_ZADANIE,
        // znaczyloby to, ze traktuje pobieranie jak krotkie zadanie.
        assert!(
            BRAK_POSTEPU >= MALE_ZADANIE,
            "przerwa w transmisji ma byc tolerowana dluzej niz cale male zadanie"
        );
    }

    #[test]
    fn polaczenie_jest_najkrotsze() {
        assert!(POLACZENIE < MALE_ZADANIE);
        assert!(POLACZENIE < BRAK_POSTEPU);
    }

    /// Instalator ma wlasny, znacznie hojniejszy limit — mieli okolo minuty,
    /// a na wolnym dysku bywa wolniejszy.
    #[test]
    fn instalator_dostaje_wyraznie_wiecej_czasu() {
        assert!(INSTALATOR > BRAK_POSTEPU * 10);
    }

    #[test]
    fn klienci_daja_sie_zbudowac() {
        let _ = klient_maly();
        let _ = klient_pobierania();
    }

    #[test]
    fn przedstawiamy_sie_z_wersja() {
        assert!(UZYTKOWNIK.starts_with("ChmurkowyLauncher/"));
        assert!(UZYTKOWNIK.len() > "ChmurkowyLauncher/".len());
    }
}
