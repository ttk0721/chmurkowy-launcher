//! Odinstalowanie launchera z poziomu samego launchera.
//!
//! Na Windowsie da się to zrobić przez „Aplikacje i funkcje", na Linuksie
//! przez `uninstall.sh` — ale jedno i drugie wymaga, żeby ktoś wiedział,
//! gdzie tego szukać. Jeden przycisk w programie działa wszędzie tak samo.
//!
//! Najważniejsza zasada tego modułu: **odpowiedź „nie" na pytanie o dane
//! nigdy nie może skasować świata gracza.** Wszystko inne jest drugorzędne.

use std::path::{Path, PathBuf};

/// Co odinstalowanie ma objąć.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zakres {
    /// Znika program i skróty. Światy, ustawienia i paczka zostają.
    TylkoProgram,
    /// Znika wszystko, razem ze światami z singleplayera.
    Wszystko,
}

/// Co trzeba zrobić, żeby launcher zniknął z komputera.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    /// Kasujemy sami, jeszcze przed wyjściem.
    pub do_usuniecia: Vec<PathBuf>,
    /// Program, który dokończy robotę już po naszym wyjściu — bo pliku,
    /// który się wykonuje, Windows nie pozwala usunąć.
    pub dokonczy: Option<(PathBuf, Vec<String>)>,
}

impl Plan {
    /// Czy ten plan w ogóle rusza dane gracza. Do sprawdzenia w testach
    /// i do pokazania w oknie potwierdzenia.
    pub fn rusza_dane(&self, katalog_danych: &Path) -> bool {
        self.do_usuniecia
            .iter()
            .any(|p| p == katalog_danych || katalog_danych.starts_with(p))
    }
}

/// Układa plan odinstalowania. Nic nie kasuje — to robi [`wykonaj`].
///
/// `exe` to plik launchera, `katalog_danych` — katalog z grą i światami.
/// Obie ścieżki przyjmujemy z zewnątrz, żeby dało się to sprawdzić testem.
pub fn zaplanuj(exe: &Path, katalog_danych: &Path, zakres: Zakres) -> Plan {
    let mut do_usuniecia = Vec::new();
    let mut dokonczy = None;

    if cfg!(target_os = "windows") {
        // Instalator zostawia obok programu swój deinstalator. To on wie,
        // co dopisał do rejestru i do menu Start, więc oddajemy mu robotę.
        let deinstalator = exe.with_file_name("unins000.exe");
        if deinstalator.is_file() {
            dokonczy = Some((
                deinstalator,
                vec!["/SILENT".to_string(), "/NORESTART".to_string()],
            ));
        }
        // Bez deinstalatora (kopia przenośna) zostaje sam plik programu.
        // Usunie go skrypt sprzątający — patrz `wykonaj`.
    } else {
        // Na Linuksie plik, który się wykonuje, wolno odpiąć od katalogu,
        // więc radzimy sobie sami.
        do_usuniecia.push(exe.to_path_buf());
        if let Some(dane) = katalog_wspoldzielony() {
            do_usuniecia.push(
                dane.join("applications")
                    .join("chmurkowy-launcher.desktop"),
            );
            do_usuniecia.push(
                dane.join("icons")
                    .join("hicolor")
                    .join("256x256")
                    .join("apps")
                    .join("chmurkowy-launcher.png"),
            );
        }
    }

    if zakres == Zakres::Wszystko {
        do_usuniecia.push(katalog_danych.to_path_buf());
    }

    Plan {
        do_usuniecia,
        dokonczy,
    }
}

fn katalog_wspoldzielony() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|p| p.join(".local").join("share"))
        })
}

/// Wykonuje plan. Zwraca listę rzeczy, których nie udało się usunąć —
/// pusta lista znaczy pełny sukces.
///
/// Po powrocie wołający ma zakończyć proces: dopiero wtedy deinstalator
/// albo skrypt sprzątający może zabrać się za plik programu.
pub fn wykonaj(plan: &Plan) -> Vec<String> {
    let mut potkniecia = Vec::new();

    for sciezka in &plan.do_usuniecia {
        let wynik = if sciezka.is_dir() {
            std::fs::remove_dir_all(sciezka)
        } else {
            std::fs::remove_file(sciezka)
        };
        // Brak pliku to nie potknięcie — celem jest, żeby go nie było.
        if let Err(e) = wynik {
            if e.kind() != std::io::ErrorKind::NotFound {
                potkniecia.push(format!("{}: {e}", sciezka.display()));
            }
        }
    }

    if let Some((program, argumenty)) = &plan.dokonczy {
        let mut cmd = std::process::Command::new(program);
        cmd.args(argumenty);
        if let Some(katalog) = program.parent() {
            cmd.current_dir(katalog);
        }
        if let Err(e) = cmd.spawn() {
            potkniecia.push(format!("{}: {e}", program.display()));
        }
    }

    potkniecia
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sciezki() -> (PathBuf, PathBuf) {
        (
            PathBuf::from("/dom/gracz/program/ChmurkowyLauncher"),
            PathBuf::from("/dom/gracz/dane/data"),
        )
    }

    /// Najwazniejszy test w tym pliku. Gracz, ktory odpowiedzial „nie" na
    /// pytanie o dane, ma zachowac swiaty z singleplayera. Pomylka tutaj
    /// jest nieodwracalna.
    #[test]
    fn bez_zgody_nie_ruszamy_danych_gracza() {
        let (exe, dane) = sciezki();
        let plan = zaplanuj(&exe, &dane, Zakres::TylkoProgram);
        assert!(
            !plan.rusza_dane(&dane),
            "plan bez zgody dotyka danych: {:?}",
            plan.do_usuniecia
        );
        assert!(
            !plan.do_usuniecia.iter().any(|p| p.starts_with(&dane)),
            "zadna sciezka nie moze lezec w katalogu danych: {:?}",
            plan.do_usuniecia
        );
    }

    #[test]
    fn ze_zgoda_dane_znikaja() {
        let (exe, dane) = sciezki();
        let plan = zaplanuj(&exe, &dane, Zakres::Wszystko);
        assert!(plan.rusza_dane(&dane));
        assert!(plan.do_usuniecia.contains(&dane));
    }

    /// Kazdy zakres ma usunac program — inaczej „odinstaluj" niczego
    /// nie odinstalowuje.
    #[test]
    fn program_znika_w_obu_zakresach() {
        let (exe, dane) = sciezki();
        for zakres in [Zakres::TylkoProgram, Zakres::Wszystko] {
            let plan = zaplanuj(&exe, &dane, zakres);
            let usuwa_program = plan.do_usuniecia.contains(&exe) || plan.dokonczy.is_some();
            assert!(usuwa_program, "zakres {zakres:?} nie usuwa programu");
        }
    }

    #[test]
    fn brakujacy_plik_nie_jest_potknieciem() {
        let kat = tempfile::tempdir().unwrap();
        let plan = Plan {
            do_usuniecia: vec![kat.path().join("nie-ma-mnie")],
            dokonczy: None,
        };
        assert!(wykonaj(&plan).is_empty());
    }

    #[test]
    fn usuwa_i_pliki_i_katalogi() {
        let kat = tempfile::tempdir().unwrap();
        let plik = kat.path().join("program");
        let katalog = kat.path().join("dane");
        std::fs::write(&plik, "x").unwrap();
        std::fs::create_dir_all(katalog.join("saves/swiat")).unwrap();
        std::fs::write(katalog.join("saves/swiat/level.dat"), "swiat").unwrap();

        let plan = Plan {
            do_usuniecia: vec![plik.clone(), katalog.clone()],
            dokonczy: None,
        };
        assert!(wykonaj(&plan).is_empty());
        assert!(!plik.exists());
        assert!(!katalog.exists());
    }

    /// `rusza_dane` musi lapac takze przypadek, w ktorym do usuniecia trafil
    /// katalog NADRZEDNY wobec danych — inaczej okno potwierdzenia klamaloby.
    #[test]
    fn wykrywamy_takze_katalog_nadrzedny() {
        let dane = PathBuf::from("/dom/gracz/chmurkowy-launcher/data");
        let plan = Plan {
            do_usuniecia: vec![PathBuf::from("/dom/gracz/chmurkowy-launcher")],
            dokonczy: None,
        };
        assert!(plan.rusza_dane(&dane));
    }
}
