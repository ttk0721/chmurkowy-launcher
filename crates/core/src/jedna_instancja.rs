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
//! Celowo bez blokad plikowych systemu operacyjnego. Wymagałyby osobnego kodu
//! na Windowsa i na Linuksa, a dzisiejszy przegląd pokazał, że to właśnie
//! w gałęziach `#[cfg]` chowają się błędy niewidoczne na maszynie, na której
//! się pracuje. `sysinfo` jest już w zależnościach i działa tak samo wszędzie.

use std::path::{Path, PathBuf};

/// Przedrostek nazwy znacznika „pokaż się".
const ZNACZNIK: &str = "chmurkowy-launcher-pokaz-";

/// Czy inny egzemplarz launchera już chodzi.
///
/// Porównujemy pełną ścieżkę pliku wykonywalnego, nie samą nazwę. Dwie różne
/// kopie launchera w dwóch katalogach to dwie osobne instalacje — przenośnego
/// launchera wolno mieć kilka i nie one są problemem.
pub fn juz_chodzi() -> bool {
    let Ok(moj_exe) = std::env::current_exe() else {
        // Bez pewności co do własnej ścieżki nie blokujemy niczego. Fałszywe
        // wykrycie zostawiłoby gracza z launcherem, którego nie da się włączyć.
        return false;
    };
    let moj_pid = std::process::id();

    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    system
        .processes()
        .iter()
        .any(|(pid, proces)| jest_innym_egzemplarzem(pid, proces, moj_pid, &moj_exe))
}

/// Czy ten wpis to naprawdę inny egzemplarz launchera.
///
/// Osobno od przeglądania listy, żeby dało się to sprawdzić testem — i dobrze,
/// bo test od razu złapał tu błąd, który uniemożliwiłby uruchomienie launchera.
///
/// `thread_kind()` jest tu kluczowe: na Linuksie `sysinfo` wylicza **wątki**
/// jako osobne wpisy z własnymi identyfikatorami i tą samą ścieżką pliku
/// wykonywalnego. Bez tego warunku launcher wykrywałby własne wątki jako
/// drugi egzemplarz i odmawiał startu — czyli lekarstwo byłoby gorsze od
/// choroby, na którą powstało.
fn jest_innym_egzemplarzem(
    pid: &sysinfo::Pid,
    proces: &sysinfo::Process,
    moj_pid: u32,
    moj_exe: &Path,
) -> bool {
    pid.as_u32() != moj_pid
        && proces.thread_kind().is_none()
        && proces.exe().is_some_and(|e| e == moj_exe)
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

    /// Ten proces testowy nie jest launcherem, wiec nie wolno go uznac za
    /// drugi egzemplarz. Gdyby wykrywanie lapalo cokolwiek, launcher nie
    /// dalby sie w ogole uruchomic.
    #[test]
    fn wlasny_proces_nie_jest_drugim_egzemplarzem() {
        let moj_exe = std::env::current_exe().unwrap();
        let moj_pid = std::process::id();
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        let znalezione: Vec<String> = system
            .processes()
            .iter()
            .filter(|(pid, p)| jest_innym_egzemplarzem(pid, p, moj_pid, &moj_exe))
            .map(|(pid, p)| format!("{pid}:{:?} rodzic={:?}", p.name(), p.parent()))
            .collect();
        assert!(
            znalezione.is_empty(),
            "moj pid {moj_pid}, exe {moj_exe:?}, znalezione: {znalezione:?}"
        );
        assert!(!juz_chodzi(), "test nie moze wykryc sam siebie");
    }
}
