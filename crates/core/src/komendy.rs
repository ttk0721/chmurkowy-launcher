//! Własne komendy gracza: przed uruchomieniem, opakowująca i po zakończeniu.
//!
//! Trzy pola tekstowe w Ustawieniach, w których można wpisać cokolwiek.
//! Najczęstsze zastosowania to kopia zapasowa świata przed graniem
//! i `gamemoderun` albo `prime-run` jako opakowanie na Linuksie.
//!
//! Dwie rzeczy są tu nieoczywiste i obie wynikają z tego, że treść pisze
//! człowiek:
//!
//! 1. **Limit czasu.** Komenda czekająca na wciśnięcie klawisza albo pytająca
//!    o hasło wisiałaby w nieskończoność, a launcher nie ma jak pokazać jej
//!    konsoli. Pasek postępu stałby w miejscu bez wyjaśnienia.
//! 2. **Powłoka.** Ludzie piszą `cp swiat swiat.bak && echo gotowe`, a nie
//!    listę argumentów. Bez powłoki `&&` byłoby zwykłym argumentem programu
//!    `cp`, który zgłosiłby niezrozumiały błąd.

use crate::limity;
use std::path::{Path, PathBuf};

/// Kiedy komenda się uruchamia. Nazwy są takie, jakich użyjemy w komunikacie
/// dla gracza.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Etap {
    Przed,
    Po,
}

impl Etap {
    pub fn opis(&self) -> &'static str {
        match self {
            Etap::Przed => "przed uruchomieniem gry",
            Etap::Po => "po zakończeniu gry",
        }
    }
}

#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum BladKomendy {
    #[error("nie udało się uruchomić komendy {} ({komenda}): {powod}", etap.opis())]
    NieUruchomil {
        etap: Etap,
        komenda: String,
        powod: String,
    },
    #[error("komenda {} ({komenda}) nie skończyła się w ciągu {sekundy} s", etap.opis())]
    Zawiesila {
        etap: Etap,
        komenda: String,
        sekundy: u64,
    },
    #[error("komenda {} ({komenda}) zakończyła się błędem {kod:?}", etap.opis())]
    Zwrocila {
        etap: Etap,
        komenda: String,
        kod: Option<i32>,
        wyjscie: String,
    },
}

/// Co komenda wie o uruchamianej grze.
///
/// Te same nazwy zmiennych, których używa Prism Launcher — gracze przenoszą
/// między launcherami gotowe skrypty i nie ma powodu, żeby wymyślać własne.
#[derive(Debug, Clone)]
pub struct Kontekst {
    pub nazwa: String,
    pub id: String,
    pub katalog_instancji: PathBuf,
    pub katalog_mc: PathBuf,
    pub java: PathBuf,
    pub argumenty_javy: String,
}

impl Kontekst {
    pub fn zmienne(&self) -> Vec<(String, String)> {
        vec![
            ("INST_NAME".into(), self.nazwa.clone()),
            ("INST_ID".into(), self.id.clone()),
            (
                "INST_DIR".into(),
                self.katalog_instancji.display().to_string(),
            ),
            ("INST_MC_DIR".into(), self.katalog_mc.display().to_string()),
            ("INST_JAVA".into(), self.java.display().to_string()),
            ("INST_JAVA_ARGS".into(), self.argumenty_javy.clone()),
        ]
    }
}

/// Buduje wywołanie komendy przez powłokę systemu.
///
/// Wydzielone, żeby dało się to sprawdzić testem bez uruchamiania czegokolwiek.
pub fn przez_powloke(komenda: &str) -> (String, Vec<String>) {
    if cfg!(windows) {
        ("cmd".into(), vec!["/C".into(), komenda.to_string()])
    } else {
        ("/bin/sh".into(), vec!["-c".into(), komenda.to_string()])
    }
}

/// Uruchamia komendę i czeka na jej koniec.
///
/// Pusta komenda to nie błąd, tylko brak komendy — większość graczy nigdy
/// niczego tu nie wpisze.
pub async fn uruchom(
    etap: Etap,
    komenda: &str,
    katalog_roboczy: &Path,
    kontekst: &Kontekst,
    dodatkowe_srodowisko: &[(String, String)],
) -> Result<(), BladKomendy> {
    let komenda = komenda.trim();
    if komenda.is_empty() {
        return Ok(());
    }

    let (program, argumenty) = przez_powloke(komenda);
    let mut cmd = tokio::process::Command::new(program);
    cmd.args(argumenty);
    cmd.current_dir(katalog_roboczy);
    for (k, v) in kontekst.zmienne() {
        cmd.env(k, v);
    }
    for (k, v) in dodatkowe_srodowisko {
        cmd.env(k, v);
    }
    // Bez tego komenda przerwana limitem czasu zostaje jako sierota i wisi
    // do końca życia launchera.
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    {
        // Komenda gracza ma się wykonać po cichu, a nie mignąć konsolą.
        const BEZ_OKNA: u32 = 0x0800_0000;
        cmd.creation_flags(BEZ_OKNA);
    }

    let limit = limity::KOMENDA_GRACZA;
    let wyjscie = match tokio::time::timeout(limit, cmd.output()).await {
        Err(_) => {
            return Err(BladKomendy::Zawiesila {
                etap,
                komenda: komenda.to_string(),
                sekundy: limit.as_secs(),
            })
        }
        Ok(Err(e)) => {
            return Err(BladKomendy::NieUruchomil {
                etap,
                komenda: komenda.to_string(),
                powod: e.to_string(),
            })
        }
        Ok(Ok(w)) => w,
    };

    if wyjscie.status.success() {
        return Ok(());
    }

    Err(BladKomendy::Zwrocila {
        etap,
        komenda: komenda.to_string(),
        kod: wyjscie.status.code(),
        wyjscie: ogon(&wyjscie.stdout, &wyjscie.stderr),
    })
}

/// Ostatnie linie tego, co komenda wypisała — tyle, ile zmieści się
/// w oknie błędu bez przewijania.
fn ogon(stdout: &[u8], stderr: &[u8]) -> String {
    let razem = format!(
        "{}{}",
        String::from_utf8_lossy(stdout),
        String::from_utf8_lossy(stderr)
    );
    let linie: Vec<&str> = razem.lines().collect();
    let od = linie.len().saturating_sub(12);
    linie[od..].join("\n").trim().to_string()
}

/// Rozbija komendę opakowującą na program i jego argumenty.
///
/// Opakowanie **nie** idzie przez powłokę: wstawiamy je przed ścieżką do Javy
/// w tej samej komendzie, w której jest gra. Gdyby szło przez powłokę,
/// trzeba by scalić kilkaset argumentów gry z powrotem w jeden łańcuch
/// i zadbać o cudzysłowy w każdym z nich — a wśród nich są ścieżki ze spacjami
/// i cały classpath.
pub fn rozbij_opakowanie(tekst: &str) -> Vec<String> {
    crate::ustawienia::podziel_argumenty(tekst)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kontekst() -> Kontekst {
        Kontekst {
            nazwa: "Chmurkowy Serwer".into(),
            id: "chmurka".into(),
            katalog_instancji: PathBuf::from("/tmp/instancja"),
            katalog_mc: PathBuf::from("/tmp/mc"),
            java: PathBuf::from("/tmp/java/bin/java"),
            argumenty_javy: "-Xmx4096M".into(),
        }
    }

    #[test]
    fn zmienne_maja_nazwy_takie_jak_w_prismie() {
        let z = kontekst().zmienne();
        let nazwy: Vec<&str> = z.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            nazwy,
            vec![
                "INST_NAME",
                "INST_ID",
                "INST_DIR",
                "INST_MC_DIR",
                "INST_JAVA",
                "INST_JAVA_ARGS"
            ],
            "gracze przenosza miedzy launcherami gotowe skrypty"
        );
    }

    #[test]
    fn komenda_idzie_przez_powloke_systemu() {
        let (program, argi) = przez_powloke("echo raz && echo dwa");
        if cfg!(windows) {
            assert_eq!(program, "cmd");
            assert_eq!(argi[0], "/C");
        } else {
            assert_eq!(program, "/bin/sh");
            assert_eq!(argi[0], "-c");
        }
        assert_eq!(argi[1], "echo raz && echo dwa");
    }

    #[test]
    fn opakowanie_rozbija_sie_na_program_i_argumenty() {
        assert_eq!(rozbij_opakowanie("gamemoderun"), vec!["gamemoderun"]);
        assert_eq!(
            rozbij_opakowanie("env DRI_PRIME=1 gamemoderun"),
            vec!["env", "DRI_PRIME=1", "gamemoderun"]
        );
        assert!(rozbij_opakowanie("   ").is_empty());
    }

    #[tokio::test]
    async fn pusta_komenda_to_nie_blad() {
        let k = kontekst();
        assert!(uruchom(Etap::Przed, "", Path::new("."), &k, &[])
            .await
            .is_ok());
        assert!(uruchom(Etap::Po, "   \n ", Path::new("."), &k, &[])
            .await
            .is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn udana_komenda_przechodzi() {
        let k = kontekst();
        assert!(uruchom(Etap::Przed, "true", Path::new("/tmp"), &k, &[])
            .await
            .is_ok());
    }

    /// Komenda, ktora zwraca blad PRZED gra, musi przerwac uruchamianie —
    /// inaczej „zrob kopie swiata przed graniem" cicho nie robi kopii.
    #[cfg(unix)]
    #[tokio::test]
    async fn nieudana_komenda_zglasza_kod_i_wyjscie() {
        let k = kontekst();
        let b = uruchom(
            Etap::Przed,
            "echo nie ma takiego pliku >&2; exit 3",
            Path::new("/tmp"),
            &k,
            &[],
        )
        .await
        .unwrap_err();
        match b {
            BladKomendy::Zwrocila { kod, wyjscie, .. } => {
                assert_eq!(kod, Some(3));
                assert!(wyjscie.contains("nie ma takiego pliku"), "{wyjscie}");
            }
            inny => panic!("spodziewalem sie Zwrocila, jest {inny:?}"),
        }
    }

    /// Zmienne `$INST_*` maja byc widoczne w srodowisku komendy.
    #[cfg(unix)]
    #[tokio::test]
    async fn komenda_widzi_zmienne_instancji() {
        let k = kontekst();
        // Komenda konczy sie bledem, gdy zmienna nie ma wlasciwej wartosci.
        let wynik = uruchom(
            Etap::Przed,
            r#"test "$INST_ID" = chmurka && test "$INST_JAVA_ARGS" = "-Xmx4096M""#,
            Path::new("/tmp"),
            &k,
            &[],
        )
        .await;
        assert!(wynik.is_ok(), "{wynik:?}");
    }

    /// Wlasne zmienne srodowiskowe gracza tez maja dochodzic do komend,
    /// a nie tylko do samej gry.
    #[cfg(unix)]
    #[tokio::test]
    async fn komenda_widzi_wlasne_zmienne_gracza() {
        let k = kontekst();
        let wynik = uruchom(
            Etap::Przed,
            r#"test "$MOJA" = wartosc"#,
            Path::new("/tmp"),
            &k,
            &[("MOJA".to_string(), "wartosc".to_string())],
        )
        .await;
        assert!(wynik.is_ok(), "{wynik:?}");
    }

    /// Bez limitu czasu literowka w skrypcie zawiesza start gry na zawsze.
    #[cfg(unix)]
    #[tokio::test(start_paused = true)]
    async fn zawieszona_komenda_konczy_sie_limitem_czasu() {
        let k = kontekst();
        let b = uruchom(Etap::Przed, "sleep 100000", Path::new("/tmp"), &k, &[])
            .await
            .unwrap_err();
        assert!(
            matches!(b, BladKomendy::Zawiesila { .. }),
            "bez limitu launcher wisi w nieskonczonosc, a jest {b:?}"
        );
    }
}
