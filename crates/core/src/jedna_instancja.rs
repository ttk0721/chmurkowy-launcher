//! Pilnuje, żeby launcher chodził w jednym egzemplarzu.
//!
//! Gracze zgłaszali po trzy ikony launchera naraz w zasobniku. Przyczyna nie
//! jest tajemnicza: launcher chowa się do zasobnika, więc rodzic nie widzi go
//! na pasku zadań, klika skrót jeszcze raz — i dostaje drugi egzemplarz. Nic
//! tego nie powstrzymywało.
//!
//! Sama blokada to za mało. Gdyby drugi egzemplarz po prostu cicho się
//! zamykał, kliknięcie skrótu nie robiłoby **nic**, co jest gorsze od trzech
//! ikon: nie widać ani launchera, ani reakcji. Dlatego drugi egzemplarz
//! zostawia znacznik, a ten działający pokazuje się na jego widok.
//!
//! # Dlaczego blokada pliku, a nie przeglądanie listy procesów
//!
//! Pierwsza wersja szukała innego procesu o tej samej ścieżce pliku
//! wykonywalnego. Wyglądało to na rozwiązanie bez kodu osobnego na każdy
//! system, ale okazało się zawodne: `sysinfo` wylicza **wątki** jako osobne
//! wpisy z tą samą ścieżką. Filtr po `thread_kind()` naprawiał to na jednej
//! maszynie, a na maszynie budującej już nie — tam wątek testu przeszedł
//! przez filtr i został uznany za drugi egzemplarz.
//!
//! Ta pomyłka jest niesymetryczna i to przesądza sprawę: fałszywe wykrycie
//! nie powoduje drugiej ikony w zasobniku, tylko launcher, którego **nie da
//! się w ogóle uruchomić**. Lekarstwo byłoby gorsze od choroby.
//!
//! Blokada pliku nie zgaduje. Trzyma ją jądro systemu, znika sama, gdy proces
//! ginie — także po zabiciu go czy zaniku zasilania — i nie da się jej wziąć
//! dwa razy. Kosztuje dwie krótkie gałęzie `#[cfg]`, które i tak sprawdza
//! kompilacja na obu systemach.

use std::path::{Path, PathBuf};

/// Przedrostek nazwy znacznika „pokaż się".
const ZNACZNIK: &str = "chmurkowy-launcher-pokaz-";

/// Trzymana blokada. Dopóki żyje, ten proces jest jedynym launcherem.
///
/// Plik zostaje na dysku, ale to bez znaczenia: liczy się blokada, a tę
/// system zdejmuje razem z procesem.
pub struct Blokada {
    _plik: std::fs::File,
}

/// Próbuje zająć blokadę. `None` znaczy, że inny egzemplarz już ją trzyma.
///
/// Wynik trzeba przechować do końca działania launchera — porzucenie go
/// zwalnia blokadę i wpuszcza kolejne egzemplarze.
#[must_use = "porzucenie blokady wpuszcza kolejne egzemplarze launchera"]
pub fn zajmij() -> Option<Blokada> {
    let sciezka = sciezka_blokady()?;
    let plik = otworz_wylacznie(&sciezka)?;
    Some(Blokada { _plik: plik })
}

/// Ścieżka pliku blokady — obok znacznika, liczona tak samo.
fn sciezka_blokady() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    Some(sciezka_znacznika(&exe).with_extension("blokada"))
}

/// Otwiera plik z wyłącznością. `None`, gdy trzyma go ktoś inny.
#[cfg(windows)]
fn otworz_wylacznie(sciezka: &Path) -> Option<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    // `share_mode(0)` znaczy „nikt inny nie otworzy tego pliku, dopóki go
    // trzymam". Drugi egzemplarz dostaje tu błąd i tak poznaje, że przegrał.
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .share_mode(0)
        .open(sciezka)
        .ok()
}

#[cfg(unix)]
fn otworz_wylacznie(sciezka: &Path) -> Option<std::fs::File> {
    use std::os::unix::io::AsRawFd;
    let plik = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(sciezka)
        .ok()?;
    // LOCK_NB: nie czekamy w kolejce, tylko od razu wiemy, czy ktoś trzyma.
    let wynik = unsafe { libc::flock(plik.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    (wynik == 0).then_some(plik)
}

/// Ścieżka znacznika — w katalogu tymczasowym, z nazwą liczoną ze ścieżki
/// pliku wykonywalnego.
///
/// Celowo NIE w katalogu danych launchera. Ten wylicza [`crate::miejsca::ustal`],
/// które przy okazji **przenosi katalog**, więc wołanie go drugi raz tylko po
/// to, żeby poznać ścieżkę, byłoby niebezpieczne. Tutaj obie strony liczą to
/// samo z `current_exe()` i bez żadnych skutków ubocznych.
///
/// Skrót ze ścieżki jest po to, żeby dwie kopie przenośnego launchera
/// w różnych katalogach nie podnosiły sobie nawzajem okien.
pub fn sciezka_znacznika(exe: &Path) -> PathBuf {
    use sha2::{Digest, Sha256};
    let skrot = Sha256::digest(exe.as_os_str().as_encoded_bytes());
    let krotki: String = skrot.iter().take(8).map(|b| format!("{b:02x}")).collect();
    std::env::temp_dir().join(format!("{ZNACZNIK}{krotki}"))
}

/// Ścieżka znacznika dla tego procesu.
fn moj_znacznik() -> Option<PathBuf> {
    std::env::current_exe().ok().map(|e| sciezka_znacznika(&e))
}

/// Drugi egzemplarz prosi działający, żeby się pokazał.
pub fn popros_o_pokazanie() {
    if let Some(s) = moj_znacznik() {
        let _ = std::fs::write(s, b"pokaz");
    }
}

/// Czy ktoś poprosił o pokazanie okna. Znacznik jest przy okazji kasowany,
/// żeby prośba zadziałała raz, a nie przy każdej klatce.
pub fn ktos_prosi_o_pokazanie() -> bool {
    let Some(s) = moj_znacznik() else {
        return false;
    };
    if s.exists() {
        let _ = std::fs::remove_file(&s);
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn znacznik_dziala_raz() {
        // Sprzatamy po ewentualnym wczesniejszym przebiegu.
        let _ = ktos_prosi_o_pokazanie();

        assert!(!ktos_prosi_o_pokazanie(), "na starcie nikt nie prosi");

        popros_o_pokazanie();
        assert!(ktos_prosi_o_pokazanie(), "prosba musi zostac zauwazona");
        // Druga odpowiedz musi byc juz przeczaca — inaczej okno wyskakiwaloby
        // graczowi na wierzch przy kazdej klatce.
        assert!(!ktos_prosi_o_pokazanie(), "prosba dziala tylko raz");
    }

    /// Dwie kopie przenosnego launchera w roznych katalogach nie moga podnosic
    /// sobie nawzajem okien.
    #[test]
    fn rozne_kopie_maja_rozne_znaczniki() {
        let a = sciezka_znacznika(Path::new("/opt/chmurka/ChmurkowyLauncher"));
        let b = sciezka_znacznika(Path::new("/home/kto/ChmurkowyLauncher"));
        assert_ne!(a, b);
        // Ta sama sciezka musi dawac ten sam znacznik — inaczej drugi
        // egzemplarz pisalby tam, gdzie pierwszy nie zaglada.
        assert_eq!(
            a,
            sciezka_znacznika(Path::new("/opt/chmurka/ChmurkowyLauncher"))
        );
    }

    /// Blokada musi byc wylaczna: drugie zajecie tej samej sciezki nie moze
    /// sie udac, dopoki pierwsze zyje. To jest cala istota tej poprawki.
    #[test]
    fn druga_blokada_nie_przechodzi() {
        let kat = tempfile::tempdir().unwrap();
        let s = kat.path().join("test.blokada");

        let pierwsza = otworz_wylacznie(&s).expect("pierwsza blokada musi przejsc");
        assert!(
            otworz_wylacznie(&s).is_none(),
            "druga blokada nie moze przejsc, dopoki pierwsza zyje"
        );

        // Po zwolnieniu plik znowu jest wolny — inaczej launcher nie dalby sie
        // uruchomic po zwyklym zamknieciu.
        drop(pierwsza);
        assert!(
            otworz_wylacznie(&s).is_some(),
            "po zwolnieniu blokada musi byc znowu do wziecia"
        );
    }

    /// Blokada i znacznik musza lezec obok siebie, ale nie moga byc tym samym
    /// plikiem — kasowanie znacznika zwalnialoby wtedy blokade.
    #[test]
    fn blokada_to_inny_plik_niz_znacznik() {
        let exe = Path::new("/opt/chmurka/ChmurkowyLauncher");
        let znacznik = sciezka_znacznika(exe);
        let blokada = znacznik.with_extension("blokada");
        assert_ne!(znacznik, blokada);
        assert_eq!(znacznik.parent(), blokada.parent());
    }
}
