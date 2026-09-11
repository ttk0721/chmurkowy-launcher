//! Szukanie Javy na komputerze gracza i sprawdzanie, czy dana ścieżka
//! naprawdę nią jest.
//!
//! Launcher pobiera własną Javę i to zostaje domyślnym zachowaniem — nic
//! tu tego nie zmienia. Ale są komputery, na których pobrana Java nie chce
//! działać: stara biblioteka systemowa, polityka firmowa blokująca pliki
//! w katalogu użytkownika, zbudowana od nowa dystrybucja. Do tej pory
//! takiemu graczowi nie dało się pomóc niczym poza „spróbuj na innym
//! komputerze". Teraz może wskazać Javę, którą już ma.
//!
//! [`sprawdz`] jest tu ważniejsze niż [`znajdz_kandydatow`]: wskazanie złego
//! pliku kończy się grą, która nie startuje, a komunikat JVM o tym, że plik
//! nie jest programem, nikomu nic nie mówi. Lepiej powiedzieć to w Ustawieniach,
//! zanim gracz kliknie GRAJ.

use crate::java::{biezacy_os, Os};
use std::path::{Path, PathBuf};

/// Co wiemy o wskazanej Javie po zapytaniu jej o wersję.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoJavy {
    /// Wersja tak, jak ją podała sama Java: `21.0.4`, `1.8.0_402`.
    pub wersja: String,
    /// Numer główny w postaci, w jakiej mówi o nim świat: 8, 17, 21.
    pub major: u32,
    /// Pierwsza linia odpowiedzi — nazwa dystrybucji dla ciekawskich.
    pub opis: String,
    /// Czy to maszyna 64-bitowa.
    ///
    /// 32-bitowa JVM nie zaadresuje więcej niż około 1,5 GB sterty, więc przy
    /// paczce wymagającej 4 GB nie ma z niej pożytku — a komunikat błędu mówi
    /// wtedy tylko „Could not reserve enough space for object heap".
    pub bity64: bool,
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum BladJavy {
    #[error("w tym miejscu nie ma pliku: {0}")]
    NieMaPliku(String),
    #[error("tego pliku nie da się uruchomić: {0}")]
    NieUruchomil(String),
    #[error("Java nie odpowiedziała w ciągu {0} sekund")]
    Zawiesil(u64),
    #[error("to nie wygląda na Javę — odpowiedziała: {0}")]
    NieRozpoznano(String),
}

/// Plik, którym pytamy o wersję.
///
/// Na Windowsie do uruchamiania gry używamy `javaw.exe`, żeby obok okna gry
/// nie wisiała czarna konsola. Ale `javaw.exe` nie ma gdzie wypisać odpowiedzi
/// na `-version` — odcina się od konsoli, więc pytanie wraca puste. Do pytania
/// bierzemy więc `java.exe` leżące obok.
pub fn do_pytania(sciezka: &Path) -> PathBuf {
    if sciezka.file_name().and_then(|n| n.to_str()) == Some("javaw.exe") {
        let obok = sciezka.with_file_name("java.exe");
        if obok.is_file() {
            return obok;
        }
    }
    sciezka.to_path_buf()
}

/// Plik, którym uruchamiamy grę.
///
/// Odwrotność [`do_pytania`]: gdy gracz wskazał `java.exe`, wolimy leżące obok
/// `javaw.exe`. Inaczej przy każdym uruchomieniu gry otwiera się druga, czarna
/// konsola, której nie da się zamknąć bez ubicia gry.
pub fn do_uruchamiania(sciezka: &Path) -> PathBuf {
    if biezacy_os() == Os::Windows
        && sciezka.file_name().and_then(|n| n.to_str()) == Some("java.exe")
    {
        let obok = sciezka.with_file_name("javaw.exe");
        if obok.is_file() {
            return obok;
        }
    }
    sciezka.to_path_buf()
}

/// Pyta wskazaną Javę o wersję i tłumaczy odpowiedź.
pub async fn sprawdz(sciezka: &Path) -> Result<InfoJavy, BladJavy> {
    let pytana = do_pytania(sciezka);
    if !pytana.is_file() {
        return Err(BladJavy::NieMaPliku(pytana.display().to_string()));
    }

    let mut cmd = tokio::process::Command::new(&pytana);
    cmd.arg("-version");
    // Bez tego zawieszony proces zostaje po przerwaniu oczekiwania i wisi
    // do końca życia launchera.
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    {
        // Bez tego przy każdym sprawdzeniu mignęłoby czarne okno konsoli.
        // `creation_flags` jest własną metodą tokio na Windowsie, więc nie
        // trzeba tu wciągać `CommandExt` — wciągnięty byłby martwym importem.
        const BEZ_OKNA: u32 = 0x0800_0000;
        cmd.creation_flags(BEZ_OKNA);
    }

    let limit = crate::limity::SPRAWDZENIE_JAVY;
    let wyjscie = match tokio::time::timeout(limit, cmd.output()).await {
        Err(_) => return Err(BladJavy::Zawiesil(limit.as_secs())),
        Ok(Err(e)) => return Err(BladJavy::NieUruchomil(e.to_string())),
        Ok(Ok(w)) => w,
    };

    // Java wypisuje `-version` na wyjście błędów, nie na standardowe.
    // Sprawdzamy oba, bo niektóre dystrybucje odwracają ten zwyczaj.
    let tekst = format!(
        "{}{}",
        String::from_utf8_lossy(&wyjscie.stderr),
        String::from_utf8_lossy(&wyjscie.stdout)
    );

    czytaj_odpowiedz(&tekst).ok_or_else(|| {
        let skrot: String = tekst.lines().take(2).collect::<Vec<_>>().join(" ");
        BladJavy::NieRozpoznano(if skrot.trim().is_empty() {
            "nic".into()
        } else {
            skrot.trim().to_string()
        })
    })
}

/// Wyciąga wersję z odpowiedzi `java -version`.
///
/// Wydzielone z [`sprawdz`], żeby dało się to przetestować bez uruchamiania
/// procesu — odpowiedzi różnych dystrybucji różnią się na tyle, że warto mieć
/// je spisane w testach.
pub fn czytaj_odpowiedz(tekst: &str) -> Option<InfoJavy> {
    let linia_wersji = tekst
        .lines()
        .find(|l| l.contains("version \"") || l.contains("version\u{a0}\""))?;
    let start = linia_wersji.find('"')? + 1;
    let reszta = &linia_wersji[start..];
    let koniec = reszta.find('"')?;
    let wersja = reszta[..koniec].to_string();

    let major = major_z_wersji(&wersja)?;

    Some(InfoJavy {
        wersja,
        major,
        opis: linia_wersji.trim().to_string(),
        // Java sama mówi o sobie „64-Bit Server VM". Brak tego napisu przy
        // obecności „Client VM" albo „Server VM" znaczy 32 bity.
        bity64: tekst.contains("64-Bit"),
    })
}

/// Numer główny w postaci, w jakiej mówi o nim świat.
///
/// Do Javy 8 numery wyglądały jak `1.8.0_402` i główny był drugi w kolejności;
/// od Javy 9 jest pierwszy. Bez tej reguły wskazanie Javy 8 raportowałoby
/// „wersja 1", a komunikat „paczka wymaga 21, masz 1" brzmi jak błąd launchera.
pub fn major_z_wersji(wersja: &str) -> Option<u32> {
    let mut czesci = wersja.split(['.', '_', '-', '+']);
    let pierwsza: u32 = czesci.next()?.parse().ok()?;
    if pierwsza == 1 {
        czesci.next()?.parse().ok()
    } else {
        Some(pierwsza)
    }
}

/// Miejsca, w których na tym systemie zwykle leży Java.
///
/// Zwraca tylko ścieżki, które naprawdę istnieją, posortowane i bez powtórzeń.
/// Nie pyta żadnej z nich o wersję — to robi [`sprawdz`] dopiero dla tej,
/// którą gracz wybierze; odpalanie kilkunastu procesów naraz przy otwarciu
/// Ustawień byłoby widoczne jako zacięcie.
pub fn znajdz_kandydatow() -> Vec<PathBuf> {
    let os = biezacy_os();
    let nazwa = crate::java::nazwa_javy(os);
    let mut out: Vec<PathBuf> = Vec::new();

    let mut dodaj = |sc: PathBuf| {
        if sc.is_file() && !out.contains(&sc) {
            out.push(sc);
        }
    };

    if let Some(dom) = std::env::var_os("JAVA_HOME") {
        dodaj(PathBuf::from(&dom).join("bin").join(nazwa));
    }

    if let Some(sciezka) = std::env::var_os("PATH") {
        for katalog in std::env::split_paths(&sciezka) {
            dodaj(katalog.join(nazwa));
        }
    }

    for korzen in korzenie_instalacji(os) {
        // Układ `<korzeń>/<jakaś-java>/bin/java`.
        if let Ok(wpisy) = std::fs::read_dir(&korzen) {
            for w in wpisy.flatten() {
                let k = w.path();
                dodaj(k.join("bin").join(nazwa));
                // macOS chowa binarkę o dwa poziomy głębiej.
                dodaj(k.join("Contents").join("Home").join("bin").join(nazwa));
            }
        }
    }

    out.sort();
    out
}

/// Katalogi, w których instalatory Javy zakładają swoje podkatalogi.
fn korzenie_instalacji(os: Os) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    match os {
        Os::Linux => {
            out.push(PathBuf::from("/usr/lib/jvm"));
            out.push(PathBuf::from("/usr/lib64/jvm"));
            out.push(PathBuf::from("/opt/java"));
            // Inne launchery Minecrafta pobierają własne JRE i zwykle są to
            // wersje sprawdzone w boju akurat z tą grą.
            if let Some(dane) = katalog_danych() {
                out.push(dane.join("PrismLauncher").join("java"));
                out.push(dane.join("multimc").join("java"));
            }
        }
        Os::Windows => {
            for zmienna in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
                if let Some(p) = std::env::var_os(zmienna) {
                    let p = PathBuf::from(p);
                    out.push(p.join("Java"));
                    out.push(p.join("Eclipse Adoptium"));
                    out.push(p.join("Microsoft").join("jdk"));
                    out.push(p.join("Programs").join("Eclipse Adoptium"));
                }
            }
        }
        Os::Osx => {
            out.push(PathBuf::from("/Library/Java/JavaVirtualMachines"));
            if let Some(dom) = std::env::var_os("HOME") {
                out.push(PathBuf::from(dom).join("Library/Java/JavaVirtualMachines"));
            }
        }
    }
    out
}

fn katalog_danych() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_DATA_HOME") {
        let p = PathBuf::from(x);
        if p.is_absolute() {
            return Some(p);
        }
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Odpowiedzi zebrane z prawdziwych dystrybucji. Roznia sie na tyle,
    /// ze warto miec je spisane.
    #[test]
    fn czyta_wersje_temurina() {
        let i = czytaj_odpowiedz(
            "openjdk version \"21.0.4\" 2024-07-16 LTS\n\
             OpenJDK Runtime Environment Temurin-21.0.4+7 (build 21.0.4+7-LTS)\n\
             OpenJDK 64-Bit Server VM Temurin-21.0.4+7 (build 21.0.4+7-LTS, mixed mode, sharing)",
        )
        .unwrap();
        assert_eq!(i.wersja, "21.0.4");
        assert_eq!(i.major, 21);
        assert!(i.bity64);
    }

    #[test]
    fn czyta_wersje_oracle() {
        let i = czytaj_odpowiedz(
            "java version \"17.0.12\" 2024-07-16 LTS\n\
             Java(TM) SE Runtime Environment (build 17.0.12+8-LTS-286)\n\
             Java HotSpot(TM) 64-Bit Server VM (build 17.0.12+8-LTS-286, mixed mode, sharing)",
        )
        .unwrap();
        assert_eq!(i.major, 17);
    }

    /// Java 8 numeruje sie `1.8.0_402`. Bez reguly „gdy pierwsza czesc to 1,
    /// bierz druga" launcher pisalby „paczka wymaga 21, masz 1".
    #[test]
    fn java_osiem_ma_numer_osiem_a_nie_jeden() {
        let i = czytaj_odpowiedz(
            "openjdk version \"1.8.0_402\"\n\
             OpenJDK Runtime Environment (build 1.8.0_402-b06)\n\
             OpenJDK 64-Bit Server VM (build 25.402-b06, mixed mode)",
        )
        .unwrap();
        assert_eq!(i.wersja, "1.8.0_402");
        assert_eq!(i.major, 8);
    }

    /// 32-bitowa JVM nie zaadresuje 4 GB sterty. Trzeba to rozpoznac, zanim
    /// gracz zobaczy „Could not reserve enough space for object heap".
    #[test]
    fn rozpoznaje_maszyne_32_bitowa() {
        let i = czytaj_odpowiedz(
            "java version \"1.8.0_402\"\n\
             Java(TM) SE Runtime Environment (build 1.8.0_402-b06)\n\
             Java HotSpot(TM) Client VM (build 25.402-b06, mixed mode)",
        )
        .unwrap();
        assert!(!i.bity64);
    }

    #[test]
    fn cos_co_nie_jest_java_nie_przechodzi() {
        assert!(czytaj_odpowiedz("").is_none());
        assert!(czytaj_odpowiedz("bash: java: nie znaleziono polecenia").is_none());
        assert!(czytaj_odpowiedz("Python 3.12.5").is_none());
    }

    #[test]
    fn major_z_roznych_zapisow() {
        assert_eq!(major_z_wersji("21"), Some(21));
        assert_eq!(major_z_wersji("21.0.4"), Some(21));
        assert_eq!(major_z_wersji("17.0.12+8"), Some(17));
        assert_eq!(major_z_wersji("1.8.0_402"), Some(8));
        assert_eq!(major_z_wersji("nonsens"), None);
    }

    #[tokio::test]
    async fn nieistniejaca_sciezka_mowi_wprost_ze_nie_ma_pliku() {
        let b = sprawdz(Path::new("/nie/ma/takiego/java"))
            .await
            .unwrap_err();
        assert!(matches!(b, BladJavy::NieMaPliku(_)), "{b:?}");
    }

    /// Wskazanie czegos, co istnieje, ale Java nie jest, musi dac zrozumialy
    /// komunikat, a nie wywrocic launchera.
    #[tokio::test]
    async fn plik_ktory_nie_jest_java_daje_czytelny_blad() {
        let dir = std::env::temp_dir().join("chmurka-javy-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("nie-java.txt");
        std::fs::write(&p, b"to nie jest program").unwrap();

        let b = sprawdz(&p).await.unwrap_err();
        assert!(
            matches!(b, BladJavy::NieUruchomil(_) | BladJavy::NieRozpoznano(_)),
            "{b:?}"
        );
        std::fs::remove_file(&p).unwrap();
    }

    /// Prawdziwe sprawdzenie: bierzemy Jave, ktora naprawde stoi na tej
    /// maszynie, i pytamy ja o wersje. Testy na spreparowanych odpowiedziach
    /// nie wykryja tego, ze `javaw.exe` nie ma gdzie odpowiedziec ani ze
    /// dystrybucja pisze wersje inaczej, niz zakladamy.
    ///
    /// Na maszynie bez Javy test nie ma czego sprawdzic i po prostu przechodzi.
    #[tokio::test]
    async fn prawdziwa_java_z_tej_maszyny_daje_sie_odpytac() {
        let kandydaci = znajdz_kandydatow();
        if kandydaci.is_empty() {
            eprintln!("brak Javy na tej maszynie — nie ma czego sprawdzic");
            return;
        }

        let mut udane = 0;
        for sc in &kandydaci {
            if let Ok(i) = sprawdz(sc).await {
                assert!(i.major >= 8, "{} zglosila wersje {}", sc.display(), i.major);
                assert!(!i.wersja.is_empty());
                assert!(!i.opis.is_empty());
                udane += 1;
            }
        }
        assert!(
            udane > 0,
            "zadnej z {} znalezionych Jav nie dalo sie odpytac",
            kandydaci.len()
        );
    }

    /// Szukanie kandydatow nie moze wywrocic sie na maszynie bez Javy ani
    /// zwrocic sciezek, ktorych nie ma.
    #[test]
    fn szukanie_kandydatow_zwraca_tylko_istniejace_pliki() {
        for sc in znajdz_kandydatow() {
            assert!(sc.is_file(), "{} nie istnieje", sc.display());
        }
    }
}
