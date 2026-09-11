//! Ustawienia gracza zapisywane w `data/settings.json`.
//!
//! Wcześniej nic nie było zapisywane — zmiana pamięci znikała po zamknięciu
//! launchera.
//!
//! Pola z pierwszych wersji zostają na najwyższym poziomie pliku, a nowe
//! grupy siedzą w zagnieżdżonych obiektach. To nie jest estetyka: przeniesienie
//! istniejącego pola w głąb sprawiłoby, że stary plik wczytuje się bez niego
//! i gracz po aktualizacji zastaje domyślne 4096 MB zamiast swoich ośmiu.

use std::path::{Path, PathBuf};

/// Najmniejsze okno, jakie ma sens. Minecraft zejdzie niżej, ale interfejs
/// gry robi się wtedy nieczytelny, a menu nie mieści przycisków.
pub const NAJMNIEJSZA_SZEROKOSC: u32 = 320;
pub const NAJMNIEJSZA_WYSOKOSC: u32 = 240;
/// Górna granica to 8K. Powyżej i tak decyduje sterownik grafiki.
pub const NAJWIEKSZA_SZEROKOSC: u32 = 7680;
pub const NAJWIEKSZA_WYSOKOSC: u32 = 4320;

/// Rozmiar okna, z jakim Minecraft startuje, gdy nikt mu nic nie narzuca.
pub const DOMYSLNA_SZEROKOSC: u32 = 854;
pub const DOMYSLNA_WYSOKOSC: u32 = 480;

/// Najmniejsza sterta startowa. Poniżej JVM i tak ją podnosi, a wpisanie
/// zera w polu nie może skończyć się komendą `-Xms0M`.
pub const NAJMNIEJSZE_MINIMUM_MB: u32 = 256;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ustawienia {
    pub pamiec_mb: u32,
    /// Dodatkowe parametry przekazywane Javie, oddzielone spacjami.
    pub dodatkowe_argumenty: String,
    /// Chowanie okna launchera po starcie gry, żeby nie zabierało zasobów.
    /// Domyślnie włączone.
    pub ukryj_po_starcie: bool,
    /// Pobieranie yt-dlp i ffmpeg wymaganych przez Create: Harmonics.
    /// Domyślnie włączone — bez nich mod pyta gracza o instalację w trakcie gry.
    pub biblioteki_dzwieku: bool,
    pub nick_offline: String,

    /// Zakładka „Gra": okno Minecrafta i co launcher robi wokół niego.
    pub gra: Gra,
    /// Zakładka „Java": którą Javą uruchamiać i ile dać jej pamięci.
    pub java: UstawieniaJavy,
    /// Zakładka „Zaawansowane": własne komendy i zmienne środowiskowe.
    pub zaawansowane: Zaawansowane,
}

impl Default for Ustawienia {
    fn default() -> Self {
        Self {
            pamiec_mb: 4096,
            dodatkowe_argumenty: String::new(),
            ukryj_po_starcie: true,
            biblioteki_dzwieku: true,
            nick_offline: String::new(),
            gra: Gra::default(),
            java: UstawieniaJavy::default(),
            zaawansowane: Zaawansowane::default(),
        }
    }
}

/// Okno gry i zachowanie launchera wokół niej.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Gra {
    /// Czy narzucamy grze rozmiar okna. Gdy wyłączone, Minecraft używa
    /// tego, co sam zapamiętał w `options.txt`.
    pub wlasny_rozmiar_okna: bool,
    pub szerokosc: u32,
    pub wysokosc: u32,
    /// Start na pełnym ekranie (`--fullscreen`).
    pub pelny_ekran: bool,
    /// Zamknięcie launchera, gdy gra się skończy.
    pub zamknij_po_grze: bool,
}

impl Default for Gra {
    fn default() -> Self {
        Self {
            wlasny_rozmiar_okna: false,
            szerokosc: DOMYSLNA_SZEROKOSC,
            wysokosc: DOMYSLNA_WYSOKOSC,
            pelny_ekran: false,
            zamknij_po_grze: false,
        }
    }
}

impl Gra {
    /// Rozmiar do przekazania grze, albo `None`, gdy zostawiamy jej decyzję.
    ///
    /// Wartości są przycinane do sensownego zakresu, bo pole tekstowe przyjmie
    /// każdą liczbę, a `--width 0` kończy się oknem, którego nie widać.
    pub fn rozmiar(&self) -> Option<(u32, u32)> {
        if !self.wlasny_rozmiar_okna {
            return None;
        }
        Some((
            self.szerokosc
                .clamp(NAJMNIEJSZA_SZEROKOSC, NAJWIEKSZA_SZEROKOSC),
            self.wysokosc
                .clamp(NAJMNIEJSZA_WYSOKOSC, NAJWIEKSZA_WYSOKOSC),
        ))
    }

    /// Czy wpisany rozmiar wykracza poza zakres, który i tak przytniemy.
    /// UI mówi o tym wprost, zamiast po cichu zmieniać liczbę pod palcami.
    pub fn rozmiar_poza_zakresem(&self) -> bool {
        self.wlasny_rozmiar_okna
            && (self.szerokosc < NAJMNIEJSZA_SZEROKOSC
                || self.szerokosc > NAJWIEKSZA_SZEROKOSC
                || self.wysokosc < NAJMNIEJSZA_WYSOKOSC
                || self.wysokosc > NAJWIEKSZA_WYSOKOSC)
    }
}

/// Która Java uruchamia grę i ile dostaje pamięci na starcie.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct UstawieniaJavy {
    /// Ścieżka do własnego pliku wykonywalnego Javy. Pusta znaczy
    /// „użyj tej, którą launcher pobrał sam" — i tak jest domyślnie.
    pub sciezka: String,
    /// Pominięcie sprawdzenia, czy wersja Javy pasuje do paczki.
    /// Furtka dla kogoś, kto wie, co robi; domyślnie zamknięta.
    pub pomin_sprawdzanie: bool,
    /// Startowy rozmiar sterty (`-Xms`). 512 MB to wartość, którą do tej pory
    /// launcher brał z manifestu — domyślne ustawienie niczego nie zmienia.
    pub pamiec_min_mb: u32,
    /// Czy gracz widział już ostrzeżenie o tej zakładce.
    pub przyjeto_ostrzezenie: bool,
}

impl Default for UstawieniaJavy {
    fn default() -> Self {
        Self {
            sciezka: String::new(),
            pomin_sprawdzanie: false,
            pamiec_min_mb: 512,
            przyjeto_ostrzezenie: false,
        }
    }
}

impl UstawieniaJavy {
    /// Ścieżka do własnej Javy, o ile gracz ją podał.
    pub fn wlasna(&self) -> Option<PathBuf> {
        let s = self.sciezka.trim();
        if s.is_empty() {
            None
        } else {
            Some(PathBuf::from(s))
        }
    }
}

/// Własne komendy i zmienne środowiskowe — zakładka dla zaawansowanych.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Zaawansowane {
    /// Uruchamiana przed startem gry. Niepowodzenie przerywa uruchamianie.
    pub komenda_przed: String,
    /// Program opakowujący, np. `gamemoderun` albo `prime-run`.
    /// Wstawia się przed ścieżką do Javy.
    pub komenda_wrapper: String,
    /// Uruchamiana po zamknięciu gry. Niepowodzenie niczego nie przerywa —
    /// gra i tak już się skończyła.
    pub komenda_po: String,
    pub zmienne: Vec<Zmienna>,
    /// Czy gracz widział już ostrzeżenie o tej zakładce.
    pub przyjeto_ostrzezenie: bool,
}

impl Zaawansowane {
    /// Zmienne gotowe do wstawienia w środowisko procesu gry.
    ///
    /// Puste i uszkodzone wpisy odpadają, a przy powtórzonej nazwie wygrywa
    /// ostatnia — tak samo, jak zachowuje się powłoka.
    pub fn srodowisko(&self) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        for z in &self.zmienne {
            if !z.poprawna() {
                continue;
            }
            let nazwa = z.nazwa.trim().to_string();
            match out.iter_mut().find(|(n, _)| *n == nazwa) {
                Some(wpis) => wpis.1 = z.wartosc.clone(),
                None => out.push((nazwa, z.wartosc.clone())),
            }
        }
        out
    }

    /// Czy gracz cokolwiek tu ustawił. Decyduje o tym, czy zakładka dostaje
    /// znacznik „coś tu jest" i czy warto proponować przywrócenie domyślnych.
    pub fn cokolwiek_ustawione(&self) -> bool {
        !self.komenda_przed.trim().is_empty()
            || !self.komenda_wrapper.trim().is_empty()
            || !self.komenda_po.trim().is_empty()
            || self.zmienne.iter().any(|z| !z.pusta())
    }
}

/// Jedna zmienna środowiskowa dokładana do procesu gry.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Zmienna {
    pub nazwa: String,
    pub wartosc: String,
}

impl Zmienna {
    /// Świeży, pusty wiersz w tabeli.
    pub fn nowa() -> Self {
        Self::default()
    }

    pub fn pusta(&self) -> bool {
        self.nazwa.trim().is_empty() && self.wartosc.is_empty()
    }

    /// Czy nazwa nadaje się na zmienną środowiskową.
    ///
    /// Znak `=` i bajt zerowy w nazwie są nie do przekazania systemowi —
    /// na Uniksie `execve` przyjmuje środowisko jako `NAZWA=wartość`, więc
    /// `=` w nazwie rozjechałby podział. Rust w takiej sytuacji panikuje
    /// przy `Command::env`, a panika w launcherze to okno, które znika.
    pub fn poprawna(&self) -> bool {
        let n = self.nazwa.trim();
        !n.is_empty()
            && !n.contains('=')
            && !n.contains('\0')
            && !self.wartosc.contains('\0')
            && !n.chars().any(char::is_whitespace)
    }

    /// Dlaczego wpis jest odrzucany. `None`, gdy wszystko gra.
    pub fn powod_odrzucenia(&self) -> Option<&'static str> {
        if self.pusta() || self.poprawna() {
            return None;
        }
        let n = self.nazwa.trim();
        if n.is_empty() {
            Some("brak nazwy")
        } else if n.contains('=') {
            Some("nazwa nie może zawierać znaku =")
        } else if n.chars().any(char::is_whitespace) {
            Some("nazwa nie może zawierać spacji")
        } else {
            Some("nazwa zawiera znak nie do przekazania systemowi")
        }
    }
}

impl Ustawienia {
    /// Uszkodzony plik daje ustawienia domyślne, nie błąd — gracz nie może
    /// zostać zablokowany przez zepsuty plik konfiguracyjny.
    pub fn wczytaj(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn zapisz(&self, path: &Path) -> std::io::Result<()> {
        if let Some(rodzic) = path.parent() {
            std::fs::create_dir_all(rodzic)?;
        }
        let tresc = serde_json::to_vec_pretty(self)?;
        std::fs::write(path, tresc)
    }

    /// Czy gracz sam ustawił rozmiar sterty. Wtedy nasz suwak ustępuje,
    /// bo dwa `-Xmx` w jednej komendzie tylko mylą.
    pub fn wlasny_rozmiar_sterty(&self) -> bool {
        podziel_argumenty(&self.dodatkowe_argumenty)
            .iter()
            .any(|a| a.starts_with("-Xmx"))
    }

    /// Startowy rozmiar sterty, jaki naprawdę trafi do komendy.
    ///
    /// `-Xms` większe od `-Xmx` to nie jest ostrzeżenie, tylko koniec:
    /// JVM odmawia startu komunikatem „Initial heap size set to a larger value
    /// than the maximum heap size" i gra nie rusza wcale. Suwak pamięci
    /// i pole minimum stoją w dwóch różnych zakładkach, więc ustawienie ich
    /// sprzecznie jest łatwiejsze, niż się wydaje — przycinamy w locie.
    pub fn minimum_sterty_mb(&self) -> u32 {
        self.java
            .pamiec_min_mb
            .clamp(NAJMNIEJSZE_MINIMUM_MB, self.pamiec_mb)
    }

    /// Czy wpisane minimum przekracza maksimum. UI mówi o tym wprost,
    /// zamiast po cichu podmieniać liczbę.
    pub fn minimum_przekracza_maksimum(&self) -> bool {
        self.java.pamiec_min_mb > self.pamiec_mb
    }
}

/// Dzieli wpisane parametry na osobne argumenty, respektując cudzysłowy.
///
/// Zwykły podział po spacjach psułby ścieżki ze spacjami, a takie trafiają się
/// w parametrach typu `-Dcos="C:\Program Files\x"`.
///
/// Nowa linia liczy się jak spacja — pole argumentów Javy jest wieloliniowe,
/// bo ludzie wklejają tam gotowe listy parametrów po jednym w wierszu.
pub fn podziel_argumenty(tekst: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut biezacy = String::new();
    let mut w_cudzyslowie: Option<char> = None;

    for znak in tekst.chars() {
        match (znak, w_cudzyslowie) {
            ('"', None) | ('\'', None) => w_cudzyslowie = Some(znak),
            (z, Some(otwarty)) if z == otwarty => w_cudzyslowie = None,
            (z, None) if z.is_whitespace() => {
                if !biezacy.is_empty() {
                    out.push(std::mem::take(&mut biezacy));
                }
            }
            (z, _) => biezacy.push(z),
        }
    }
    if !biezacy.is_empty() {
        out.push(biezacy);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domyslnie_chowa_okno_po_starcie() {
        let u = Ustawienia::default();
        assert!(u.ukryj_po_starcie, "to ma byc wlaczone domyslnie");
        assert!(
            u.biblioteki_dzwieku,
            "biblioteki dzwieku tez domyslnie wlaczone"
        );
        assert_eq!(u.pamiec_mb, 4096);
    }

    /// Domyslne ustawienia nowych zakladek nie moga zmienic ani jednego
    /// argumentu komendy wzgledem tego, co launcher robil wczesniej.
    #[test]
    fn nowe_zakladki_domyslnie_nic_nie_zmieniaja() {
        let u = Ustawienia::default();
        assert_eq!(u.gra.rozmiar(), None, "domyslnie nie narzucamy rozmiaru");
        assert!(!u.gra.pelny_ekran);
        assert!(!u.gra.zamknij_po_grze);
        assert_eq!(u.java.wlasna(), None, "domyslnie Java pobrana przez nas");
        assert!(!u.java.pomin_sprawdzanie);
        assert_eq!(
            u.minimum_sterty_mb(),
            512,
            "512 MB to wartosc, ktora launcher bral z manifestu"
        );
        assert!(u.zaawansowane.srodowisko().is_empty());
        assert!(!u.zaawansowane.cokolwiek_ustawione());
    }

    #[test]
    fn dzieli_zwykle_argumenty() {
        assert_eq!(
            podziel_argumenty("-XX:+UseG1GC -Dfoo=bar"),
            vec!["-XX:+UseG1GC", "-Dfoo=bar"]
        );
    }

    /// Pole argumentow Javy jest wieloliniowe — ludzie wklejaja tam listy
    /// parametrow po jednym w wierszu.
    #[test]
    fn dzieli_argumenty_wpisane_w_kilku_wierszach() {
        assert_eq!(
            podziel_argumenty("-XX:+UseG1GC\n-Dfoo=bar\n\n-Xss1M"),
            vec!["-XX:+UseG1GC", "-Dfoo=bar", "-Xss1M"]
        );
    }

    #[test]
    fn szanuje_cudzyslowy() {
        assert_eq!(
            podziel_argumenty(r#"-Dsciezka="C:\Program Files\Java" -Xss1M"#),
            vec![r"-Dsciezka=C:\Program Files\Java", "-Xss1M"]
        );
    }

    #[test]
    fn puste_i_nadmiarowe_spacje_nie_tworza_pustych_argumentow() {
        assert!(podziel_argumenty("   ").is_empty());
        assert_eq!(podziel_argumenty("  -a    -b  "), vec!["-a", "-b"]);
    }

    #[test]
    fn wykrywa_wlasny_rozmiar_sterty() {
        let mut u = Ustawienia::default();
        assert!(!u.wlasny_rozmiar_sterty());
        u.dodatkowe_argumenty = "-XX:+UseZGC -Xmx8G".into();
        assert!(u.wlasny_rozmiar_sterty());
    }

    #[test]
    fn uszkodzony_plik_daje_domyslne_zamiast_bledu() {
        let dir = std::env::temp_dir().join("chmurka-ust-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings.json");
        std::fs::write(&p, b"to nie jest json").unwrap();
        assert_eq!(Ustawienia::wczytaj(&p), Ustawienia::default());
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn zapis_i_odczyt_zachowuja_wartosci() {
        let dir = std::env::temp_dir().join("chmurka-ust-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings2.json");
        let u = Ustawienia {
            pamiec_mb: 8192,
            dodatkowe_argumenty: "-XX:+UseG1GC".into(),
            ukryj_po_starcie: false,
            biblioteki_dzwieku: false,
            nick_offline: "Tomek".into(),
            gra: Gra {
                wlasny_rozmiar_okna: true,
                szerokosc: 1920,
                wysokosc: 1080,
                pelny_ekran: true,
                zamknij_po_grze: true,
            },
            java: UstawieniaJavy {
                sciezka: "/usr/lib/jvm/java-21/bin/java".into(),
                pomin_sprawdzanie: true,
                pamiec_min_mb: 1024,
                przyjeto_ostrzezenie: true,
            },
            zaawansowane: Zaawansowane {
                komenda_przed: "echo start".into(),
                komenda_wrapper: "gamemoderun".into(),
                komenda_po: "echo koniec".into(),
                zmienne: vec![Zmienna {
                    nazwa: "MESA_GL_VERSION_OVERRIDE".into(),
                    wartosc: "4.6".into(),
                }],
                przyjeto_ostrzezenie: true,
            },
        };
        u.zapisz(&p).unwrap();
        assert_eq!(Ustawienia::wczytaj(&p), u);
        std::fs::remove_file(&p).unwrap();
    }

    /// Najwazniejszy test migracji: plik zapisany przez wersje sprzed zakladek
    /// ma zachowac KAZDA swoja wartosc. Przeniesienie ktoregokolwiek ze starych
    /// pol w glab grupy sprawiloby, ze gracz z 8192 MB dostaje po aktualizacji
    /// domyslne 4096 — i nie ma pojecia dlaczego.
    #[test]
    fn stary_plik_ustawien_nie_traci_ani_jednej_wartosci() {
        let dir = std::env::temp_dir().join("chmurka-ust-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings-stare.json");
        std::fs::write(
            &p,
            br#"{
              "pamiec_mb": 8192,
              "dodatkowe_argumenty": "-XX:+UseZGC",
              "ukryj_po_starcie": false,
              "biblioteki_dzwieku": false,
              "nick_offline": "Chmurka"
            }"#,
        )
        .unwrap();

        let u = Ustawienia::wczytaj(&p);
        assert_eq!(u.pamiec_mb, 8192);
        assert_eq!(u.dodatkowe_argumenty, "-XX:+UseZGC");
        assert!(!u.ukryj_po_starcie);
        assert!(!u.biblioteki_dzwieku);
        assert_eq!(u.nick_offline, "Chmurka");
        // A nowe grupy dostaja swoje domyslne wartosci.
        assert_eq!(u.gra, Gra::default());
        assert_eq!(u.java, UstawieniaJavy::default());
        assert_eq!(u.zaawansowane, Zaawansowane::default());
        std::fs::remove_file(&p).unwrap();
    }

    /// Niekompletna grupa tez nie moze wywrocic wczytywania.
    #[test]
    fn czesciowo_wypelniona_grupa_uzupelnia_sie_domyslnymi() {
        let dir = std::env::temp_dir().join("chmurka-ust-test");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("settings-czesciowe.json");
        std::fs::write(&p, br#"{"gra": {"pelny_ekran": true}}"#).unwrap();
        let u = Ustawienia::wczytaj(&p);
        assert!(u.gra.pelny_ekran);
        assert_eq!(u.gra.szerokosc, DOMYSLNA_SZEROKOSC);
        assert_eq!(u.pamiec_mb, 4096);
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn rozmiar_okna_tylko_gdy_wlaczony() {
        let mut g = Gra {
            szerokosc: 1280,
            wysokosc: 720,
            ..Gra::default()
        };
        assert_eq!(g.rozmiar(), None);
        g.wlasny_rozmiar_okna = true;
        assert_eq!(g.rozmiar(), Some((1280, 720)));
    }

    #[test]
    fn rozmiar_okna_przycinany_do_sensownego_zakresu() {
        let g = Gra {
            wlasny_rozmiar_okna: true,
            szerokosc: 0,
            wysokosc: 99_999,
            ..Gra::default()
        };
        assert_eq!(
            g.rozmiar(),
            Some((NAJMNIEJSZA_SZEROKOSC, NAJWIEKSZA_WYSOKOSC)),
            "--width 0 daloby okno, ktorego nie widac"
        );
        assert!(g.rozmiar_poza_zakresem());
    }

    /// `-Xms` wieksze od `-Xmx` to nie ostrzezenie, tylko odmowa startu JVM.
    /// Pola stoja w dwoch roznych zakladkach, wiec ustawienie ich sprzecznie
    /// jest latwe.
    #[test]
    fn minimum_sterty_nigdy_nie_przekracza_maksimum() {
        let u = Ustawienia {
            pamiec_mb: 2048,
            java: UstawieniaJavy {
                pamiec_min_mb: 8192,
                ..UstawieniaJavy::default()
            },
            ..Ustawienia::default()
        };
        assert!(u.minimum_przekracza_maksimum());
        assert_eq!(u.minimum_sterty_mb(), 2048);
    }

    #[test]
    fn minimum_sterty_nie_schodzi_do_zera() {
        let u = Ustawienia {
            java: UstawieniaJavy {
                pamiec_min_mb: 0,
                ..UstawieniaJavy::default()
            },
            ..Ustawienia::default()
        };
        assert_eq!(u.minimum_sterty_mb(), NAJMNIEJSZE_MINIMUM_MB);
    }

    #[test]
    fn wlasna_java_tylko_gdy_sciezka_niepusta() {
        let mut j = UstawieniaJavy::default();
        assert_eq!(j.wlasna(), None);
        j.sciezka = "   ".into();
        assert_eq!(j.wlasna(), None, "same spacje to nadal brak sciezki");
        j.sciezka = "/usr/bin/java".into();
        assert_eq!(j.wlasna(), Some(PathBuf::from("/usr/bin/java")));
    }

    /// `Command::env` panikuje przy nazwie ze znakiem `=`, a panika
    /// w launcherze to okno, ktore po prostu znika.
    #[test]
    fn odrzuca_nazwy_nie_do_przekazania_systemowi() {
        let zle = [
            Zmienna {
                nazwa: "".into(),
                wartosc: "x".into(),
            },
            Zmienna {
                nazwa: "A=B".into(),
                wartosc: "x".into(),
            },
            Zmienna {
                nazwa: "MOJA ZMIENNA".into(),
                wartosc: "x".into(),
            },
            Zmienna {
                nazwa: "A\0B".into(),
                wartosc: "x".into(),
            },
        ];
        for z in &zle {
            assert!(!z.poprawna(), "{:?} nie moze przejsc", z.nazwa);
            assert!(z.powod_odrzucenia().is_some());
        }
        let dobra = Zmienna {
            nazwa: "MESA_GL_VERSION_OVERRIDE".into(),
            wartosc: "4.6".into(),
        };
        assert!(dobra.poprawna());
        assert_eq!(dobra.powod_odrzucenia(), None);
    }

    /// Pusty wiersz w tabeli to nie blad — to wiersz, ktorego gracz jeszcze
    /// nie wypelnil. Nie ma go czym straszyc.
    #[test]
    fn pusty_wiersz_nie_jest_bledem() {
        let z = Zmienna::nowa();
        assert!(z.pusta());
        assert_eq!(z.powod_odrzucenia(), None);
    }

    #[test]
    fn srodowisko_pomija_zepsute_i_zostawia_ostatnia_przy_powtorce() {
        let z = Zaawansowane {
            zmienne: vec![
                Zmienna {
                    nazwa: "DRI_PRIME".into(),
                    wartosc: "0".into(),
                },
                Zmienna {
                    nazwa: "ZLA=NAZWA".into(),
                    wartosc: "x".into(),
                },
                Zmienna::nowa(),
                Zmienna {
                    nazwa: "DRI_PRIME".into(),
                    wartosc: "1".into(),
                },
            ],
            ..Zaawansowane::default()
        };
        assert_eq!(
            z.srodowisko(),
            vec![("DRI_PRIME".to_string(), "1".to_string())]
        );
    }

    #[test]
    fn wie_czy_w_zaawansowanych_cokolwiek_ustawiono() {
        let mut z = Zaawansowane::default();
        assert!(!z.cokolwiek_ustawione());
        z.zmienne.push(Zmienna::nowa());
        assert!(
            !z.cokolwiek_ustawione(),
            "sam pusty wiersz to jeszcze nie ustawienie"
        );
        z.komenda_wrapper = "gamemoderun".into();
        assert!(z.cokolwiek_ustawione());
    }
}
