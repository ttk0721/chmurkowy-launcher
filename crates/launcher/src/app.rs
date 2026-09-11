use crate::zasobnik::{Zasobnik, ZdarzenieZasobnika};
use chmurka_core::auth::{msa, msa::DeviceCode, Account};
use chmurka_core::bledy::{BladLaunchera, BladUzytkownika};
use chmurka_core::manifest::Manifest;
use chmurka_core::paczki::{StanShaderow, StanZasobow};
use chmurka_core::progress::{Progress, Stage};
use chmurka_core::ustawienia::{podziel_argumenty, Ustawienia};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Widok {
    Glowny,
    Logowanie,
    Konta,
    Ustawienia,
    Paczki,
    Blad,
    Konsola,
    Aktualizacja,
}

/// Wiadomości płynące z zadań w tle do wątku rysującego.
pub enum Wiadomosc {
    Manifest(Box<Manifest>),
    Postep(Progress),
    Notatka(String),
    /// Błąd z kodem i instrukcją — pokazywany na własnym ekranie.
    BladZKodem(Box<BladUzytkownika>),
    KodUrzadzenia(Box<DeviceCode>),
    /// Konto zalogowane i gotowe do zapamiętania na liście.
    ZapamietajKonto {
        konto: Box<Account>,
        refresh_token: String,
    },
    /// Gra ruszyła — czas schować okno, jeśli gracz sobie tego życzy.
    GraWystartowala,
    GraZakonczona(Option<i32>),
    /// Nowa wersja launchera leży już pod tą ścieżką. Trzeba ją odpalić
    /// i zakończyć bieżący proces.
    ZrestartujDo(PathBuf),
    /// Robota w tle skończona, interfejs znów jest do dyspozycji gracza.
    Wolny,
    /// Odpowiedź Javy wskazanej w Ustawieniach na pytanie o wersję.
    JavaSprawdzona(Box<Result<chmurka_core::javy::InfoJavy, String>>),
    /// Javy znalezione na tym komputerze.
    JavyZnalezione(Vec<PathBuf>),
    /// Plik wskazany w oknie wyboru „Przeglądaj".
    JavaWybrana(PathBuf),
}

pub struct App {
    /// Gdzie leżą gra, paczka, ustawienia i logi. Od 0.4.14 to katalog
    /// użytkownika, a nie katalog obok pliku launchera — instalacja nie jest
    /// miejscem na dwugigabajtowe światy gracza.
    katalog_danych: PathBuf,
    pub widok: Widok,
    pub manifest: Option<Manifest>,
    pub konto: Option<Account>,
    /// Wszystkie zapamiętane konta — online i offline naraz.
    pub konta: chmurka_core::auth::store::Konta,
    pub kod: Option<DeviceCode>,
    pub postep: Option<Progress>,
    pub blad_z_kodem: Option<BladUzytkownika>,
    /// Krótka informacja zwrotna po akcji w ustawieniach.
    pub komunikat: Option<String>,
    pub log: Vec<String>,
    pub pokaz_szczegoly: bool,
    /// Czy pokazać okno potwierdzenia odinstalowania.
    pub pyta_o_odinstalowanie: bool,
    /// Która zakładka Ustawień jest otwarta.
    pub zakladka: crate::views::settings::Zakladka,
    /// Zakładka, przed którą czeka jeszcze ostrzeżenie „to dla zaawansowanych".
    pub ostrzezenie_zakladki: Option<crate::views::settings::Zakladka>,
    /// Wynik ostatniego „Sprawdź" w zakładce Java. `None`, gdy nie pytano.
    pub wynik_javy: Option<Result<chmurka_core::javy::InfoJavy, String>>,
    /// Javy znalezione na komputerze po kliknięciu „Wykryj". `None`, gdy
    /// jeszcze nie szukano — to co innego niż „szukano i nic nie ma".
    pub kandydaci_javy: Option<Vec<PathBuf>>,
    /// Kiedy ostatnio udało się pobrać manifest. Przycisk „Sprawdź ponownie"
    /// odpowiada z pamięci, dopóki ten czas jest świeży.
    pub ostatnie_sprawdzenie: Option<std::time::Instant>,
    /// Kiedy launcher ostatni raz sam z siebie zajrzał po manifest.
    ///
    /// Liczone od PRÓBY, a nie od powodzenia. Gdyby liczyć od powodzenia,
    /// komputer bez sieci pytałby przy każdej klatce — czyli kilkadziesiąt
    /// razy na sekundę.
    ostatnie_pilnowanie: Option<std::time::Instant>,
    /// Czy manifest, na który czekamy, zamówiło pilnowanie.
    ///
    /// Od tego zależy, co się stanie po jego odebraniu: przy starcie launcher
    /// podmienia się sam, a w trakcie pracy pyta, bo ktoś może być w połowie
    /// logowania albo wpisywania nicku.
    pilnowanie_w_toku: bool,
    /// Wersja zaproponowana graczowi w okienku.
    pub proponowana_wersja: Option<String>,
    /// Wersja odłożona przyciskiem „Nie teraz". Do końca tej sesji cisza.
    pub odlozona_wersja: Option<String>,
    pub ustawienia: Ustawienia,
    /// Stan paczek czytany z plików gry przy każdym wejściu na ekran —
    /// gracz mógł je pozmieniać w samej grze.
    pub zasoby: StanZasobow,
    pub shadery: StanShaderow,
    pub zajety: bool,
    /// Prawda od startu gry do jej zakończenia.
    pub gra_dziala: bool,
    /// Podgląd logu gry — jedyny sposób, żeby gracz sprawdził, czy paczka
    /// się jeszcze ładuje, czy proces dawno stanął.
    pub konsola: chmurka_core::konsola::Konsola,
    schowaj_okno: bool,
    przywroc_okno: bool,
    zakoncz: bool,
    /// Furtka do pracy nad wyglądem: gdzie zapisać zawartość okna.
    /// Patrz `CHMURKA_ZRZUT` niżej.
    zrzut: Option<PathBuf>,
    /// Ile klatek narysowano. Liczy się tylko przy robieniu zrzutu.
    klatki: u32,
    pub nadawca: Sender<Wiadomosc>,
    odbiorca: Receiver<Wiadomosc>,
    odbiorca_zasobnika: Receiver<ZdarzenieZasobnika>,
    /// Ikona żyje tak długo, jak ten uchwyt.
    _zasobnik: Option<Zasobnik>,
    /// Czy udało się założyć ikonę. Bez niej chowanie okna odcięłoby graczowi
    /// dostęp do launchera, więc wtedy tylko minimalizujemy.
    ma_zasobnik: bool,
    /// W `Option`, żeby dało się go oddać w `Drop` — patrz `impl Drop for App`.
    runtime: Option<tokio::runtime::Runtime>,
}

impl Drop for App {
    /// Runtime tokio przy zwykłym porzuceniu czeka **bez limitu** na zadania
    /// blokujące, a jednym z nich jest pilnowanie procesu gry. Zamknięcie
    /// okna w trakcie rozgrywki zostawiało więc działający proces launchera:
    /// druga ikona w zasobniku po ponownym uruchomieniu, a na Windowsie
    /// zajęty własny plik `.exe`, co miesza się z podmianą przy aktualizacji.
    ///
    /// `shutdown_background` nie czeka na nic. Proces gry żyje dalej sam
    /// i o to właśnie chodzi.
    fn drop(&mut self) {
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_background();
        }
    }
}

impl App {
    pub fn nowa(katalog: PathBuf) -> Self {
        let (nadawca, odbiorca) = std::sync::mpsc::channel();
        let (nadawca_zas, odbiorca_zasobnika) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime tokio");

        // Stare instalacje trzymaly dane obok pliku launchera. Przenosimy je
        // raz, przy pierwszym uruchomieniu tej wersji — nikt nie pobiera
        // paczki drugi raz i nikt nie traci swiata z singleplayera.
        let docelowy =
            chmurka_core::miejsca::katalog_uzytkownika().unwrap_or_else(|| katalog.clone());
        let (katalog_danych, przeprowadzka) = chmurka_core::miejsca::ustal(&katalog, &docelowy);
        let plik_ustawien = katalog_danych.join("settings.json");
        let pierwsze_uruchomienie = !plik_ustawien.is_file();
        let mut ustawienia = Ustawienia::wczytaj(&plik_ustawien);

        // Przy pierwszym uruchomieniu dobieramy pamięć do komputera. Później już
        // nie ruszamy — to wybór gracza, nawet jeśli odbiega od zalecenia.
        if pierwsze_uruchomienie {
            if let Some(mb) = chmurka_core::pamiec::calkowita_mb() {
                ustawienia.pamiec_mb = chmurka_core::pamiec::zalecana_mb(mb);
            }
        }
        let sciezka_logu = katalog_danych.join("logs").join("game.log");
        // ksni zaklada dzialajacy runtime tokio, wiec ikone tworzymy w jego kontekscie.
        // Bez limitu zamrozony host zasobnika (rozszerzenie GNOME, tray KDE)
        // zatrzymywal launcher PRZED pierwsza klatka — gracz klikal ikone
        // i nie dostawal zadnego okna. Brak zasobnika juz obslugujemy:
        // launcher wtedy minimalizuje okno zamiast je chowac.
        let zasobnik = runtime.block_on(async {
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                crate::zasobnik::utworz(nadawca_zas),
            )
            .await
            .ok()
            .flatten()
        });

        let mut app = Self {
            katalog_danych,
            // Furtka do pracy nad wyglądem: CHMURKA_WIDOK=logowanie|ustawienia
            widok: match std::env::var("CHMURKA_WIDOK").as_deref() {
                Ok("logowanie") => Widok::Logowanie,
                Ok("ustawienia") => Widok::Ustawienia,
                Ok("blad") => Widok::Blad,
                Ok("paczki") => Widok::Paczki,
                _ => Widok::Glowny,
            },
            manifest: None,
            konto: None,
            konta: chmurka_core::auth::store::Konta::default(),
            kod: None,
            postep: None,
            blad_z_kodem: None,
            komunikat: None,
            log: Vec::new(),
            pokaz_szczegoly: false,
            pyta_o_odinstalowanie: false,
            zakladka: crate::views::settings::Zakladka::Ogolne,
            ostrzezenie_zakladki: None,
            wynik_javy: None,
            kandydaci_javy: None,
            ostatnie_sprawdzenie: None,
            // Manifest i tak pobiera się przy starcie, więc odliczanie rusza
            // od teraz — inaczej pierwsza klatka zamówiłaby drugie, zbędne
            // zapytanie o ten sam plik.
            ostatnie_pilnowanie: Some(std::time::Instant::now()),
            pilnowanie_w_toku: false,
            proponowana_wersja: None,
            odlozona_wersja: None,
            ustawienia,
            zasoby: StanZasobow::default(),
            shadery: StanShaderow::default(),
            zajety: false,
            gra_dziala: false,
            konsola: chmurka_core::konsola::Konsola::nowa(sciezka_logu),
            schowaj_okno: false,
            przywroc_okno: false,
            zrzut: std::env::var_os("CHMURKA_ZRZUT").map(PathBuf::from),
            klatki: 0,
            zakoncz: false,
            nadawca,
            odbiorca,
            odbiorca_zasobnika,
            ma_zasobnik: zasobnik.is_some(),
            _zasobnik: zasobnik,
            runtime: Some(runtime),
        };
        // Furtka do pracy nad wyglądem: CHMURKA_PROPOZYCJA=0.9.9 pokazuje
        // okienko z propozycją aktualizacji. Inaczej dałoby się je obejrzeć
        // tylko czekając dziesięć minut na prawdziwe wydanie.
        if let Ok(w) = std::env::var("CHMURKA_PROPOZYCJA") {
            app.proponowana_wersja = Some(w);
        }

        // Furtka do pracy nad wyglądem: CHMURKA_ZAKLADKA=gra|java|zaawansowane.
        // Idzie tą samą drogą co kliknięcie w pasek zakładek, więc przy świeżych
        // ustawieniach pokaże też okno ostrzeżenia — inaczej nie dałoby się go
        // obejrzeć bez ręcznego klikania.
        if let Ok(nazwa) = std::env::var("CHMURKA_ZAKLADKA") {
            if let Some(z) = crate::views::settings::Zakladka::z_nazwy(&nazwa) {
                crate::views::settings::otworz_zakladke(&mut app, z);
            }
        }

        // Podgląd ekranu błędu przy pracy nad wyglądem: CHMURKA_WIDOK=blad
        if app.widok == Widok::Blad {
            app.blad_z_kodem = Some(
                BladLaunchera::GraPadla {
                    kod: Some(1),
                    ogon_logu: "java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.main.Main.main(Main.java:1)".into(),
                    wlasne_argumenty: false,
                    zabita_przez_system: false,
                    sterta_mb: 4096,
                }
                .dla_uzytkownika(),
            );
        }
        let opis = przeprowadzka.opis();
        if !opis.is_empty() {
            app.log.push(opis);
        }
        if pierwsze_uruchomienie {
            app.zapisz_ustawienia();
        }
        app.zbij_zabojcze_ustawienie_pamieci();
        if app.widok == Widok::Paczki {
            app.odswiez_paczki();
        }
        // Poprzednia wersja launchera leży obok pod nazwą z „.stary".
        // Teraz na pewno już nie jest uruchomiona, więc można ją skasować.
        chmurka_core::aktualizacja::posprzataj_po_podmianie();
        app.wczytaj_manifest();
        app.wznow_sesje();
        app
    }

    /// Uruchamia zadanie w tle. Cicho odpuszcza, gdy launcher jest już
    /// w trakcie zamykania — wtedy nie ma po co niczego zaczynać.
    pub(crate) fn w_tle<F>(&self, zadanie: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        if let Some(rt) = &self.runtime {
            rt.spawn(zadanie);
        }
    }

    /// Usuwa launcher z komputera i kończy pracę.
    ///
    /// Sam plik programu na Windowsie usuwa deinstalator zostawiony przez
    /// instalator — pliku, który się wykonuje, system nie pozwala skasować.
    /// Na Linuksie radzimy sobie sami. W obu przypadkach dane gracza znikają
    /// tylko wtedy, gdy wprost o to poprosił.
    pub fn odinstaluj(&mut self, zakres: chmurka_core::odinstaluj::Zakres) {
        use chmurka_core::odinstaluj;

        self.pyta_o_odinstalowanie = false;

        let Ok(exe) = std::env::current_exe() else {
            self.komunikat =
                Some("Nie udało się ustalić, gdzie leży launcher. Odinstaluj go ręcznie.".into());
            return;
        };

        let plan = odinstaluj::zaplanuj(&exe, &self.data(), zakres);
        let potkniecia = odinstaluj::wykonaj(&plan);

        if potkniecia.is_empty() {
            self.zakoncz = true;
            return;
        }
        // Cokolwiek zostało, gracz musi o tym usłyszeć — inaczej uznałby,
        // że launcher zniknął, a on siedziałby dalej na dysku.
        self.komunikat = Some(format!(
            "Nie wszystko udało się usunąć: {}. Resztę skasuj ręcznie.",
            potkniecia.join("; ")
        ));
    }

    pub fn data(&self) -> PathBuf {
        self.katalog_danych.clone()
    }

    /// Przeładowuje stan paczek z plików gry.
    pub fn odswiez_paczki(&mut self) {
        let instancja = self.data().join("instance");
        self.zasoby = chmurka_core::paczki::wczytaj_zasoby(&instancja);
        self.shadery = chmurka_core::paczki::wczytaj_shadery(&instancja);
    }

    pub fn zapisz_paczki(&mut self) {
        let instancja = self.data().join("instance");
        if let Err(e) = chmurka_core::paczki::zapisz_zasoby(&instancja, &self.zasoby) {
            self.komunikat = Some(format!("Nie udało się zapisać paczek zasobów: {e}"));
            return;
        }
        if let Err(e) = chmurka_core::paczki::zapisz_shadery(
            &instancja,
            self.shadery.wlaczone,
            self.shadery.wybrany.as_deref(),
        ) {
            self.komunikat = Some(format!("Nie udało się zapisać shadera: {e}"));
        }
    }

    pub fn zapisz_ustawienia(&mut self) {
        let sciezka = self.data().join("settings.json");
        if let Err(e) = self.ustawienia.zapisz(&sciezka) {
            self.komunikat = Some(format!("Nie udało się zapisać ustawień: {e}"));
        }
    }

    /// Pobiera manifest — jedyna brama do wszystkiego, co launcher robi.
    ///
    /// Trzy podejścia z narastającą przerwą: chwilowe mrugnięcie hostingu
    /// nie może kończyć się ekranem błędu, skoro wystarczy poczekać sekundę.
    /// Dopiero gdy wszystkie padną, gracz cokolwiek widzi.
    pub fn wczytaj_manifest(&mut self) {
        // Sprawdzenie na życzenie — przy starcie albo z przycisku. Po nim
        // launcher zachowuje się jak dotąd, czyli podmienia się sam.
        self.pilnowanie_w_toku = false;
        self.pobierz_manifest_w_tle();
    }

    fn pobierz_manifest_w_tle(&mut self) {
        let n = self.nadawca.clone();
        let adres = env!("CHMURKA_MANIFEST_URL").to_string();
        // Czy to sprawdzenie, o które nikt nie prosił.
        let cicho = self.pilnowanie_w_toku;
        self.w_tle(async move {
            const PROBY: u32 = 3;
            let mut ostatni = None;
            for proba in 0..PROBY {
                match pobierz_manifest(&adres).await {
                    Ok(m) => {
                        let _ = n.send(Wiadomosc::Manifest(Box::new(m)));
                        return;
                    }
                    Err(e) => ostatni = Some(e),
                }
                if proba + 1 < PROBY {
                    tokio::time::sleep(std::time::Duration::from_millis(800 * (1 << proba))).await;
                }
            }
            if let Some(e) = ostatni {
                if cicho {
                    // Samodzielne sprawdzenie w tle. Gdy sieć akurat nie
                    // działa, nie ma o czym mówić: gracz o nic nie prosił,
                    // a wyrzucenie go na ekran błędu w środku zabawy byłoby
                    // karą za nasz własny pomysł. Co dziesięć minut, przy
                    // zerwanym Wi-Fi, byłaby to kara dotkliwa.
                    let _ = n.send(Wiadomosc::Notatka(format!(
                        "Nie udało się sprawdzić aktualizacji ({e}). Spróbuję później."
                    )));
                } else {
                    let _ = n.send(Wiadomosc::BladZKodem(Box::new(e.dla_uzytkownika())));
                }
            }
        });
    }

    /// Sprawdza raz na jakiś czas, czy nie wyszła nowsza wersja launchera.
    ///
    /// Dotąd launcher patrzył na to tylko przy starcie. Kto zostawia go
    /// otwartym na całe popołudnie, dowiadywał się o poprawce następnego dnia.
    ///
    /// W trakcie gry nie zaczepiamy ani serwera, ani gracza: launcher siedzi
    /// wtedy schowany w zasobniku, więc okienko nie miałoby się gdzie pokazać,
    /// a podmiana pliku spod działającej gry to ostatnia rzecz, jakiej trzeba.
    /// Po jej zakończeniu odstęp jest już dawno przekroczony i sprawdzenie
    /// dzieje się samo.
    fn przypilnuj_aktualizacji(&mut self, ctx: &egui::Context) {
        if self.gra_dziala || self.zajety {
            return;
        }
        let minelo = self.ostatnie_pilnowanie.map(|t| t.elapsed());
        match chmurka_core::aktualizacja::do_nastepnego_pilnowania(minelo) {
            None => {
                self.ostatnie_pilnowanie = Some(std::time::Instant::now());
                self.pilnowanie_w_toku = true;
                self.pobierz_manifest_w_tle();
            }
            // Bezczynne okno przestaje się przerysowywać, więc bez zamówionego
            // przebudzenia nie byłoby komu zauważyć, że dziesięć minut minęło.
            Some(za) => ctx.request_repaint_after(za),
        }
    }

    /// Czy jest o czym mówić graczowi po samodzielnym sprawdzeniu.
    fn rozwaz_propozycje(&mut self) {
        use chmurka_core::aktualizacja;

        let Some(m) = &self.manifest else { return };
        let najnowsza = m.launcher.latest_version.clone();
        let jest_plik = m.launcher.urls.contains_key(aktualizacja::klucz_systemu());

        if aktualizacja::warto_zaproponowac(
            env!("CARGO_PKG_VERSION"),
            &najnowsza,
            jest_plik,
            self.odlozona_wersja.as_deref(),
        ) {
            self.proponowana_wersja = Some(najnowsza);
        }
    }

    /// Gracz zgodził się na aktualizację zaproponowaną w okienku.
    pub fn przyjmij_propozycje(&mut self) {
        self.proponowana_wersja = None;
        self.zaktualizuj_recznie();
    }

    /// Gracz kliknął „Nie teraz" albo zamknął okienko krzyżykiem.
    pub fn odloz_propozycje(&mut self) {
        self.odlozona_wersja = self.proponowana_wersja.take();
    }

    /// Czy okienko z propozycją ma się teraz pokazać.
    ///
    /// Nie w trakcie gry, nie w trakcie roboty i nie na ekranach, na których
    /// i tak dzieje się coś ważniejszego — na ekranie błędu gracz czyta, co
    /// się stało, a na ekranie aktualizacji jest już w trakcie aktualizowania.
    pub fn pora_na_propozycje(&self) -> bool {
        self.proponowana_wersja.is_some()
            && !self.gra_dziala
            && !self.zajety
            && !matches!(self.widok, Widok::Aktualizacja | Widok::Blad)
    }

    /// Zbija przydział pamięci, jeśli zapisane ustawienie nie mieści się
    /// w tym komputerze.
    ///
    /// Suwak pokazuje stertę Javy, a gra bierze o półtora do dwóch gigabajtów
    /// więcej — metaspace, cache kodu i bufory sterownika grafiki leżą poza
    /// stertą. Ustawienie 4096 MB na maszynie z 8 GB wygląda więc niewinnie,
    /// a kończy się tym, że jądro zamyka grę w trakcie zabawy. Nikt tego nie
    /// odgadnie z samego suwaka, więc launcher poprawia to za gracza.
    ///
    /// Własnego `-Xmx` nie ruszamy — kto go wpisał, wie co robi.
    fn zbij_zabojcze_ustawienie_pamieci(&mut self) {
        use chmurka_core::pamiec;

        if self.ustawienia.wlasny_rozmiar_sterty() {
            return;
        }
        let Some(calkowita) = pamiec::calkowita_mb() else {
            return;
        };
        if !pamiec::grozi_brakiem_pamieci(self.ustawienia.pamiec_mb, calkowita) {
            return;
        }
        let bezpieczna = pamiec::zalecana_mb(calkowita);
        if bezpieczna >= self.ustawienia.pamiec_mb {
            // Komputer jest po prostu za słaby — zbijanie w dół niczego
            // nie załatwi, a gracz straciłby ustawienie bez powodu.
            return;
        }
        self.log.push(format!(
            "Pamięć dla gry zmieniona z {} na {bezpieczna} MB — przy poprzednim ustawieniu \
             gra zajmowałaby około {} MB i system mógłby ją zamknąć.",
            self.ustawienia.pamiec_mb,
            pamiec::szacowany_proces_mb(self.ustawienia.pamiec_mb)
        ));
        self.ustawienia.pamiec_mb = bezpieczna;
        self.zapisz_ustawienia();
    }

    /// Podmienia launcher na najnowszy, jeśli manifest podaje nowszy.
    ///
    /// Poprawki wychodzą czasem po kilka na godzinę i nie da się prosić
    /// dziesięciolatka, żeby pobierał plik z GitHuba. Wystarczy powiedzieć
    /// „zamknij i odpal ponownie" — resztę launcher robi sam.
    ///
    /// Cokolwiek pójdzie nie tak, zostaje przy starej wersji i wpuszcza
    /// gracza do gry. Nieudana aktualizacja nie może być powodem, dla którego
    /// ktoś nie zagra.
    /// Wywoływane samo, gdy przyjdzie manifest.
    fn zaktualizuj_sie(&mut self) {
        self.zaktualizuj(false);
    }

    /// Pyta wskazaną Javę o wersję i pokazuje odpowiedź w zakładce Java.
    ///
    /// Idzie w tło, bo `java -version` na zawieszonym zasobie sieciowym potrafi
    /// trwać — a robi się to pod przyciskiem, przy którym gracz stoi i patrzy.
    pub fn sprawdz_jave(&mut self, sciezka: std::path::PathBuf) {
        self.zajety = true;
        self.wynik_javy = None;
        let n = self.nadawca.clone();
        self.w_tle(async move {
            let wynik = chmurka_core::javy::sprawdz(&sciezka)
                .await
                .map_err(|e| e.to_string());
            let _ = n.send(Wiadomosc::JavaSprawdzona(Box::new(wynik)));
        });
    }

    /// Otwiera systemowe okno wyboru pliku dla ścieżki do Javy.
    ///
    /// Okno idzie w tło, bo wariant portalowy `rfd` czeka na odpowiedź systemu
    /// po D-Bus. Wywołane wprost zamroziłoby rysowanie launchera na cały czas,
    /// gdy okno stoi otwarte.
    pub fn wybierz_jave(&mut self) {
        self.zajety = true;
        let n = self.nadawca.clone();
        let start = self
            .ustawienia
            .java
            .wlasna()
            .and_then(|p| p.parent().map(std::path::Path::to_path_buf));
        self.w_tle(async move {
            let wybor = tokio::task::spawn_blocking(move || {
                let mut okno = rfd::FileDialog::new().set_title("Wskaż plik wykonywalny Javy");
                if let Some(k) = start {
                    okno = okno.set_directory(k);
                }
                okno.pick_file()
            })
            .await
            .ok()
            .flatten();

            match wybor {
                Some(p) => {
                    let _ = n.send(Wiadomosc::JavaWybrana(p));
                }
                // Zamknięcie okna bez wyboru to nie błąd — trzeba tylko oddać
                // graczowi przyciski, bo inaczej zostają wyszarzone na zawsze.
                None => {
                    let _ = n.send(Wiadomosc::Wolny);
                }
            }
        });
    }

    /// Szuka Javy w typowych miejscach na tym komputerze.
    pub fn wykryj_javy(&mut self) {
        self.zajety = true;
        let n = self.nadawca.clone();
        self.w_tle(async move {
            // Chodzenie po katalogach to praca blokująca — na wolnym dysku
            // albo przy zamontowanym zasobie sieciowym zablokowałaby wątek,
            // na którym stoi cała reszta zadań w tle.
            let lista = tokio::task::spawn_blocking(chmurka_core::javy::znajdz_kandydatow)
                .await
                .unwrap_or_default();
            let _ = n.send(Wiadomosc::JavyZnalezione(lista));
        });
    }

    /// Odpowiada na „Sprawdź ponownie".
    ///
    /// Pyta serwer tylko wtedy, gdy od ostatniego sprawdzenia minęło dość
    /// czasu. Wcześniej odpowiada tym, co już wie — kilka kliknięć pod rząd
    /// nie ma po co obciążać hostingu, skoro odpowiedź będzie ta sama.
    pub fn sprawdz_aktualizacje(&mut self) {
        let od_ostatniego = self.ostatnie_sprawdzenie.map(|t| t.elapsed());
        if chmurka_core::aktualizacja::warto_sprawdzic(od_ostatniego) {
            self.komunikat = Some("Sprawdzam…".into());
            self.wczytaj_manifest();
        } else {
            self.komunikat = Some(format!(
                "Sprawdzano przed chwilą — masz najnowszą wersję ({}).",
                env!("CARGO_PKG_VERSION")
            ));
        }
    }

    /// Wywoływane z Ustawień, gdy gracz kliknie „Zaktualizuj teraz”.
    ///
    /// Ręczne kliknięcie omija znacznik nieudanej próby: skoro gracz prosi
    /// wprost, to nie nam mu odmawiać, bo poprzednim razem coś nie wyszło.
    pub fn zaktualizuj_recznie(&mut self) {
        self.zaktualizuj(true);
    }

    fn zaktualizuj(&mut self, recznie: bool) {
        use chmurka_core::aktualizacja::{self, Decyzja};

        // W trakcie instalacji albo gry nie ma o czym mówić — podmiana pliku
        // spod działającego procesu to ostatnia rzecz, jakiej wtedy trzeba.
        if self.zajety {
            return;
        }
        let Some(m) = &self.manifest else { return };

        let biezaca = env!("CARGO_PKG_VERSION");
        let najnowsza = m.launcher.latest_version.clone();
        let url = m.launcher.urls.get(aktualizacja::klucz_systemu()).cloned();
        let data = self.data();

        match aktualizacja::zdecyduj(biezaca, &najnowsza, url.as_deref(), &data) {
            Decyzja::Aktualna => {
                // Klikniecie, po ktorym nic sie nie dzieje, wyglada na zepsuty
                // przycisk. Przy sprawdzeniu automatycznym milczymy, bo to
                // stan normalny przy kazdym starcie.
                if recznie {
                    self.komunikat = Some(format!("Masz już najnowszą wersję ({biezaca})."));
                }
                return;
            }
            Decyzja::BrakAdresu => {
                self.log.push(format!(
                    "Jest launcher {najnowsza}, ale manifest nie podaje pliku dla tego systemu."
                ));
                return;
            }
            Decyzja::JuzProbowano(w) if !recznie => {
                self.log.push(format!(
                    "Aktualizacja do {w} raz się nie powiodła — zostaję przy {biezaca}."
                ));
                return;
            }
            Decyzja::JuzProbowano(_) | Decyzja::Nowsza { .. } => {}
        }

        let Some(url) = url else { return };
        self.zajety = true;
        // Bez własnego ekranu cała aktualizacja wyglądała tak: okno znika
        // i po chwili wraca. Teraz widać, co się dzieje i że nic nie trzeba
        // klikać ani niczego pobierać z przeglądarki.
        self.widok = Widok::Aktualizacja;
        self.log
            .push(format!("Aktualizuję launcher do {najnowsza}…"));

        let n = self.nadawca.clone();
        let n2 = n.clone();
        let postep: Arc<dyn Fn(Progress) + Send + Sync> = Arc::new(move |p| {
            let _ = n2.send(Wiadomosc::Postep(p));
        });

        self.w_tle(async move {
            let dl = chmurka_core::net::Downloader::new(4);
            match aktualizacja::pobierz_i_podmien(&url, &najnowsza, &data, &dl, postep).await {
                Ok(exe) => {
                    let _ = n.send(Wiadomosc::ZrestartujDo(exe));
                }
                Err(e) => {
                    let _ = n.send(Wiadomosc::Notatka(format!(
                        "Nie udało się zaktualizować launchera ({e}). Gram na tej wersji."
                    )));
                    // Nieudana aktualizacja nie może zostawić gracza na ekranie
                    // z napisem „za chwilę się zamknę" — wracamy tam, skąd przyszedł.
                    let _ = n.send(Wiadomosc::Wolny);
                }
            }
        });
    }

    /// Wraca na wybrane konto bez pytania gracza.
    ///
    /// Konto offline odtwarzamy od ręki — nick wystarczy, sieć niepotrzebna.
    /// Przy koncie Microsoft odświeżamy token w tle; niepowodzenie jest ciche,
    /// gracz po prostu zobaczy ekran kont.
    fn wznow_sesje(&mut self) {
        use chmurka_core::auth::store;

        let sciezka = self.data().join("auth.json");
        self.konta = store::wczytaj(&sciezka);

        let Some(wybrane) = self.konta.wybrane().cloned() else {
            return;
        };

        let token = match &wybrane.rodzaj {
            store::Rodzaj::Offline => {
                self.konto = Some(store::na_konto_offline(&wybrane));
                return;
            }
            store::Rodzaj::Microsoft { refresh_token } => refresh_token.clone(),
        };

        let n = self.nadawca.clone();
        self.w_tle(async move {
            let client_id = "00000000402b5328";
            let Ok(t) = msa::refresh(client_id, &token).await else {
                return;
            };
            if let Ok(konto) = msa::zaloguj_minecraft(&t).await {
                let _ = n.send(Wiadomosc::ZapamietajKonto {
                    konto: Box::new(konto),
                    refresh_token: t.refresh_token,
                });
            }
        });
    }

    /// Dopisuje konto do listy, zapisuje ją i przełącza się na nie.
    pub fn zapamietaj_konto(&mut self, konto: Account, refresh_token: Option<String>) {
        use chmurka_core::auth::store;

        self.konta.dodaj(store::ZapisaneKonto {
            nick: konto.name.clone(),
            uuid: konto.uuid.clone(),
            rodzaj: match refresh_token {
                Some(t) => store::Rodzaj::Microsoft { refresh_token: t },
                None => store::Rodzaj::Offline,
            },
        });
        self.zapisz_konta();
        self.konto = Some(konto);
    }

    pub fn zapisz_konta(&mut self) {
        let sciezka = self.data().join("auth.json");
        if let Err(e) = chmurka_core::auth::store::zapisz(&sciezka, &self.konta) {
            self.komunikat = Some(format!("Nie udało się zapisać kont: {e}"));
        }
    }

    /// Przełącza się na zapamiętane konto.
    ///
    /// Offline działa od ręki. Przy koncie Microsoft odświeżamy token w tle,
    /// więc przez chwilę gra jeszcze nie ruszy — ale gracz nie musi nic wpisywać.
    pub fn przelacz_konto(&mut self, klucz: &str) {
        use chmurka_core::auth::store;

        if !self.konta.wybierz(klucz) {
            return;
        }
        self.zapisz_konta();
        let Some(wybrane) = self.konta.wybrane().cloned() else {
            return;
        };

        match &wybrane.rodzaj {
            store::Rodzaj::Offline => {
                self.konto = Some(store::na_konto_offline(&wybrane));
                self.widok = Widok::Glowny;
            }
            store::Rodzaj::Microsoft { refresh_token } => {
                // Do czasu odświeżenia nie mamy ważnego tokenu do gry.
                self.konto = None;
                let token = refresh_token.clone();
                let n = self.nadawca.clone();
                self.zajety = true;
                self.w_tle(async move {
                    let client_id = "00000000402b5328";
                    match msa::refresh(client_id, &token).await {
                        Ok(t) => match msa::zaloguj_minecraft(&t).await {
                            Ok(konto) => {
                                let _ = n.send(Wiadomosc::ZapamietajKonto {
                                    konto: Box::new(konto),
                                    refresh_token: t.refresh_token,
                                });
                            }
                            Err(e) => {
                                let _ = n.send(Wiadomosc::BladZKodem(Box::new(
                                    BladLaunchera::Logowanie(e).dla_uzytkownika(),
                                )));
                            }
                        },
                        Err(e) => {
                            let _ = n.send(Wiadomosc::BladZKodem(Box::new(
                                BladLaunchera::Logowanie(e).dla_uzytkownika(),
                            )));
                        }
                    }
                });
            }
        }
    }

    /// Usuwa konto z listy i przechodzi na następne, jeśli jakieś zostało.
    pub fn zapomnij_konto(&mut self, klucz: &str) {
        let bylo_wybrane = self.konta.wybrane.as_deref() == Some(klucz);
        self.konta.usun(klucz);
        self.zapisz_konta();
        if bylo_wybrane {
            self.konto = None;
            if let Some(nastepne) = self.konta.wybrane().map(|k| k.klucz()) {
                self.przelacz_konto(&nastepne);
            }
        }
    }

    fn odbierz(&mut self) {
        while let Ok(z) = self.odbiorca_zasobnika.try_recv() {
            match z {
                ZdarzenieZasobnika::Pokaz => self.przywroc_okno = true,
                ZdarzenieZasobnika::Zakoncz => self.zakoncz = true,
            }
        }

        while let Ok(w) = self.odbiorca.try_recv() {
            match w {
                Wiadomosc::Manifest(m) => {
                    self.manifest = Some(*m);
                    self.ostatnie_sprawdzenie = Some(std::time::Instant::now());
                    if std::mem::take(&mut self.pilnowanie_w_toku) {
                        // Launcher już chodzi i ktoś może być w połowie czegoś.
                        // Pytamy, zamiast podmieniać się pod rękami.
                        self.rozwaz_propozycje();
                    } else {
                        self.zaktualizuj_sie();
                    }
                }
                Wiadomosc::Postep(p) => {
                    if p.stage == Stage::Ready {
                        self.log.push(p.label.clone());
                    }
                    self.postep = Some(p);
                }
                Wiadomosc::Notatka(s) => self.log.push(s),
                Wiadomosc::BladZKodem(b) => {
                    // Bez tego po nieudanym logowaniu ekran zostawal na kodzie,
                    // ktory dawno wygasl, a przycisk „Zaloguj przez Microsoft"
                    // rysuje sie tylko wtedy, gdy kodu nie ma. Gracz nie mial
                    // jak zaczac od nowa.
                    self.kod = None;
                    self.zajety = false;
                    self.gra_dziala = false;
                    self.postep = None;
                    self.log.push(format!("BŁĄD {}: {}", b.kod, b.tytul));
                    self.blad_z_kodem = Some(*b);
                    self.widok = Widok::Blad;
                    // Gra padła, gdy launcher siedział w zasobniku — okno musi
                    // wrócić samo, inaczej gracz zobaczyłby tylko znikającą grę.
                    self.przywroc_okno = true;
                }
                Wiadomosc::KodUrzadzenia(d) => self.kod = Some(*d),
                Wiadomosc::ZapamietajKonto {
                    konto,
                    refresh_token,
                } => {
                    self.zapamietaj_konto(*konto, Some(refresh_token));
                    self.kod = None;
                    self.zajety = false;
                    self.widok = Widok::Glowny;
                    self.komunikat = None;
                }
                Wiadomosc::JavaSprawdzona(wynik) => {
                    self.wynik_javy = Some(*wynik);
                    self.zajety = false;
                }
                Wiadomosc::JavyZnalezione(lista) => {
                    self.kandydaci_javy = Some(lista);
                    self.zajety = false;
                }
                Wiadomosc::JavaWybrana(sciezka) => {
                    self.ustawienia.java.sciezka = sciezka.display().to_string();
                    self.zapisz_ustawienia();
                    self.zajety = false;
                    // Od razu sprawdzamy, co gracz wybrał. Wskazanie złego pliku
                    // ma się wyjaśnić tutaj, a nie dopiero po kliknięciu GRAJ.
                    self.sprawdz_jave(sciezka);
                }
                Wiadomosc::GraWystartowala => {
                    self.gra_dziala = true;
                    // Miedzy klknieciem GRAJ a pojawieniem sie okna gry mija
                    // ze dwie minuty, w ktorych na ekranie nie ma nic. Kto
                    // przywroci launcher z zasobnika, ma od razu zobaczyc,
                    // ze paczka sie laduje — a nie pusty ekran glowny.
                    self.widok = Widok::Konsola;
                    if self.ustawienia.ukryj_po_starcie {
                        self.schowaj_okno = true;
                    }
                }
                Wiadomosc::GraZakonczona(kod) => {
                    // Gracz mógł w grze włączyć albo wyłączyć paczki — czytamy od nowa.
                    self.odswiez_paczki();
                    // Tylko po normalnym wyjściu z gry. Gdy gra pada, wiadomość
                    // idzie inną drogą i launcher zostaje, żeby pokazać błąd —
                    // zamknięcie się w takiej chwili zabrałoby graczowi jedyne
                    // wyjaśnienie, jakie ma.
                    if self.ustawienia.gra.zamknij_po_grze {
                        self.zakoncz = true;
                    }
                    self.zajety = false;
                    self.gra_dziala = false;
                    self.postep = None;
                    self.przywroc_okno = true;
                    // Po normalnym wyjściu z gry wracamy na ekran główny.
                    // Konsola przydawała się w trakcie ładowania; teraz gracz
                    // chce zobaczyć przycisk GRAJ, a nie ścianę logów.
                    if self.widok == Widok::Konsola {
                        self.widok = Widok::Glowny;
                    }
                    self.log.push(format!(
                        "Gra zakończyła się kodem {}",
                        kod.map(|k| k.to_string())
                            .unwrap_or_else(|| "nieznanym".into())
                    ));
                }
                Wiadomosc::Wolny => {
                    self.zajety = false;
                    self.postep = None;
                    if self.widok == Widok::Aktualizacja {
                        self.widok = Widok::Glowny;
                    }
                }
                Wiadomosc::ZrestartujDo(exe) => {
                    // Nowy plik leży już pod nazwą, spod której wystartowaliśmy.
                    // Odpalamy go i schodzimy z drogi — gracz zobaczy okno,
                    // które na moment znika i wraca w nowej wersji.
                    match chmurka_core::aktualizacja::uruchom_ponownie(&exe) {
                        Ok(()) => self.zakoncz = true,
                        Err(e) => {
                            self.zajety = false;
                            self.postep = None;
                            self.log.push(format!(
                                "Nowy launcher jest już na dysku, ale nie dał się uruchomić \
                                 ({e}). Zamknij i odpal launcher ponownie."
                            ));
                        }
                    }
                }
            }
        }
    }
}

async fn pobierz_manifest(adres: &str) -> Result<Manifest, BladLaunchera> {
    let siec = |e: reqwest::Error| BladLaunchera::Logowanie(msa::AuthError::Siec(e.to_string()));
    // Skrot `reqwest::get` buduje klienta bez zadnych limitow. Manifest to
    // pierwsza rzecz po starcie i jedyna brama do wszystkiego — milczacy
    // serwer zostawial gracza z wiecznym kreciolkiem „Sprawdzam paczke".
    let tekst = chmurka_core::limity::klient_maly()
        .get(adres)
        .send()
        .await
        .map_err(siec)?
        .text()
        .await
        .map_err(siec)?;
    Ok(chmurka_core::manifest::parse(&tekst)?)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        self.odbierz();

        if self.zakoncz {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if std::mem::take(&mut self.schowaj_okno) {
            if self.ma_zasobnik {
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            } else {
                // Bez ikony w zasobniku ukryte okno byłoby nie do odzyskania.
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
        }
        if std::mem::take(&mut self.przywroc_okno) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        if self.zrzut.is_some() {
            zrob_zrzut(self, ctx);
        }

        // W trakcie gry nic się nie zmienia aż do jej zakończenia, więc odpytujemy
        // raz na sekundę — inaczej schowany launcher wciąż mieliłby procesor.
        if self.gra_dziala || self.widok == Widok::Konsola {
            // Na konsoli odswiezamy czesciej — inaczej log doganialby okno
            // z sekundowym opoznieniem i wygladal na zamarly.
            let odstep = if self.widok == Widok::Konsola {
                std::time::Duration::from_millis(400)
            } else {
                std::time::Duration::from_secs(1)
            };
            ctx.request_repaint_after(odstep);
        } else if self.zajety || self.kod.is_some() || self.manifest.is_none() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        self.przypilnuj_aktualizacji(ctx);

        match self.widok {
            Widok::Glowny => crate::views::main::rysuj(self, ctx),
            Widok::Logowanie => crate::views::login::rysuj(self, ctx),
            Widok::Konta => crate::views::accounts::rysuj(self, ctx),
            Widok::Ustawienia => crate::views::settings::rysuj(self, ctx),
            Widok::Paczki => crate::views::packs::rysuj(self, ctx),
            Widok::Blad => crate::views::error::rysuj(self, ctx),
            Widok::Konsola => crate::views::console::rysuj(self, ctx),
            Widok::Aktualizacja => crate::views::update::rysuj(self, ctx),
        }

        // Na wierzchu, po narysowaniu ekranu — inaczej okienko schowałoby się
        // pod panelami widoku.
        if self.pora_na_propozycje() {
            crate::views::update::okno_propozycji(self, ctx);
        }
    }
}

/// Startuje logowanie Microsoft w tle i odpytuje o token aż do skutku.
pub fn zaloguj_microsoft(app: &mut App) {
    let Some(m) = &app.manifest else { return };
    let client_id = m.auth.msa_client_id.clone();
    let n = app.nadawca.clone();

    app.blad_z_kodem = None;
    app.zajety = true;
    app.w_tle(async move {
        let zglos = |e: msa::AuthError, n: &Sender<Wiadomosc>| {
            let _ = n.send(Wiadomosc::BladZKodem(Box::new(
                BladLaunchera::Logowanie(e).dla_uzytkownika(),
            )));
        };

        let kod = match msa::begin(&client_id).await {
            Ok(k) => k,
            Err(e) => return zglos(e, &n),
        };
        let device_code = kod.device_code.clone();
        let mut odstep = kod.interval_s.max(1);
        let koniec = std::time::Instant::now() + std::time::Duration::from_secs(kod.expires_in_s);
        let _ = n.send(Wiadomosc::KodUrzadzenia(Box::new(kod)));

        loop {
            if std::time::Instant::now() > koniec {
                return zglos(msa::AuthError::Wygasl, &n);
            }
            // Śpimy najwyżej do terminu. Bez tego przy odstępie urosłym przez
            // „slow_down" launcher spał jeszcze długo po wygaśnięciu kodu,
            // pokazując graczowi kręciołek i kod, który już nie działa.
            let zostalo = koniec.saturating_duration_since(std::time::Instant::now());
            tokio::time::sleep(zostalo.min(std::time::Duration::from_secs(odstep))).await;

            match msa::poll_once(&client_id, &device_code).await {
                Ok(msa::PollResult::Czekamy) => {}
                // Microsoft prosi o wolniejsze odpytywanie — zignorowanie tego
                // kończy się zablokowaniem całej sesji logowania. Sufit, bo bez
                // niego po kilkunastu takich odpowiedziach odstęp rósł ponad
                // minutę i gracz czekał w ciszy.
                Ok(msa::PollResult::Zwolnij) => odstep = (odstep + 5).min(60),
                // Chwilowa awaria sieci nie może przerwać logowania w połowie —
                // gracz ma wpisany kod na stronie Microsoftu i nie ma pojęcia,
                // że coś mrugnęło. Termin i tak kiedyś zamknie sprawę.
                Err(msa::AuthError::Siec(_)) => {}
                Ok(msa::PollResult::Gotowe(t)) => {
                    return match msa::zaloguj_minecraft(&t).await {
                        Ok(konto) => {
                            let _ = n.send(Wiadomosc::ZapamietajKonto {
                                konto: Box::new(konto),
                                refresh_token: t.refresh_token,
                            });
                        }
                        Err(e) => zglos(e, &n),
                    };
                }
                // Zanim ogłosimy graczowi powód, dokładamy to, czego sam
                // `poll_once` nie wie: ile zostało do terminu. Bez tego każde
                // `invalid_grant` szło jako „kod stracił ważność", także wtedy
                // gdy odmowa przyszła w drugiej sekundzie logowania — a wtedy
                // rada „weź nowy kod” zapętla gracza na amen.
                //
                // Zapas jest na rozjazd zegarów i czas przelotu odpowiedzi:
                // przy terminie tuż-tuż uczciwiej przyznać, że kod mógł wygasnąć.
                Err(e) => {
                    const ZAPAS: std::time::Duration = std::time::Duration::from_secs(30);
                    let mogl_wygasnac =
                        koniec.saturating_duration_since(std::time::Instant::now()) < ZAPAS;
                    return zglos(msa::doprecyzuj_odmowe(e, mogl_wygasnac), &n);
                }
            }
        }
    });
}

/// Instaluje wszystko, czego brakuje, i uruchamia grę.
pub fn uruchom(app: &mut App) {
    let (Some(m), Some(konto)) = (app.manifest.clone(), app.konto.clone()) else {
        return;
    };
    let data = app.data();
    let n = app.nadawca.clone();
    let ustawienia = app.ustawienia.clone();

    app.blad_z_kodem = None;
    app.zajety = true;
    app.log.clear();

    app.w_tle(async move {
        let n2 = n.clone();
        let postep: Arc<dyn Fn(Progress) + Send + Sync> = Arc::new(move |p| {
            let _ = n2.send(Wiadomosc::Postep(p));
        });

        odpal_z_samonaprawa(&data, &m, &konto, &ustawienia, postep, &n).await;
    });
}

/// Uruchamia grę, a po drodze naprawia to, co potrafi naprawić sam.
///
/// Gracz ma kliknąć „GRAJ" i tyle. Rada „wejdź w Ustawienia i kliknij Napraw
/// instalację" jest poprawna, ale nikt jej nie wykonuje — więc launcher robi
/// to za niego. Okno błędu zostaje na to, czego sam nie ruszy: pełny dysk,
/// brak uprawnień, wygasłe logowanie.
///
/// Gdy gra już wystartowała, nie ponawiamy niczego — drugie okno Minecrafta
/// byłoby gorsze od każdego błędu.
async fn odpal_z_samonaprawa(
    data: &Path,
    m: &Manifest,
    konto: &Account,
    ustawienia: &Ustawienia,
    postep: Arc<dyn Fn(Progress) + Send + Sync>,
    n: &Sender<Wiadomosc>,
) {
    use chmurka_core::bledy::Samonaprawa;

    const PODEJSCIA: u32 = 3;
    for podejscie in 1..=PODEJSCIA {
        let ruszyla = Arc::new(AtomicBool::new(false));
        let e = match przygotuj_i_odpal(data, m, konto, ustawienia, postep.clone(), n, &ruszyla)
            .await
        {
            Ok(()) => return,
            Err(e) => e,
        };

        let b = e.dla_uzytkownika();
        let lek = b.samonaprawa();
        let ostatnie = podejscie == PODEJSCIA;

        if ruszyla.load(Ordering::SeqCst) || lek == Samonaprawa::Nic || ostatnie {
            let _ = n.send(Wiadomosc::BladZKodem(Box::new(b)));
            return;
        }

        let _ = n.send(Wiadomosc::Notatka(format!(
            "{} (kod {}, podejście {} z {})",
            b.opis_samonaprawy(),
            b.kod,
            podejscie + 1,
            PODEJSCIA
        )));
        postep(Progress::trwa(Stage::Loader, b.opis_samonaprawy()));

        match lek {
            Samonaprawa::JavaOdNowa => {
                let _ = std::fs::remove_dir_all(data.join("java"));
            }
            Samonaprawa::GraOdNowa => {
                let _ = std::fs::remove_dir_all(data.join("mc"));
            }
            // Powtórka bez kasowania czegokolwiek. Chwila przerwy, bo problem
            // bywa chwilowy i natychmiastowy nawrót trafiłby w to samo.
            Samonaprawa::Ponow => {
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            }
            Samonaprawa::Nic => unreachable!("obsłużone wyżej"),
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn przygotuj_i_odpal(
    data: &Path,
    m: &Manifest,
    konto: &Account,
    ustawienia: &Ustawienia,
    postep: Arc<dyn Fn(Progress) + Send + Sync>,
    n: &Sender<Wiadomosc>,
    ruszyla: &AtomicBool,
) -> Result<(), BladLaunchera> {
    use chmurka_core::*;

    let plik = |sc: &Path| {
        let sc = sc.display().to_string();
        move |e: std::io::Error| BladLaunchera::Plik(sc.clone(), e)
    };

    let dl = net::Downloader::new(8);
    let mc = data.join("mc");
    let instancja = data.join("instance");
    std::fs::create_dir_all(&instancja).map_err(plik(&instancja))?;

    // Gracz mógł w Ustawieniach wskazać własną Javę. Sprawdzamy ją, zanim
    // cokolwiek pobierzemy — zła ścieżka ma się wyjaśnić od razu, a nie po
    // kwadransie instalowania.
    let java = match ustawienia.java.wlasna() {
        Some(sciezka) => {
            postep(Progress::trwa(Stage::Java, "Sprawdzam wskazaną Javę…"));
            wlasna_java(&sciezka, m.java.major, ustawienia.java.pomin_sprawdzanie).await?
        }
        None => java::ensure(data, m.java.major, &dl, postep.clone()).await?,
    };
    let profil = game_install::ensure_loader(
        &mc,
        &java,
        &m.pack.minecraft,
        &m.pack.loader.kind,
        &m.pack.loader.version,
        &dl,
        postep.clone(),
    )
    .await?;
    let wersja = version::load(&mc.join("versions"), &profil)?;
    game_install::ensure_libraries(&mc, &wersja, &dl, postep.clone()).await?;
    game_install::ensure_assets(&mc, &wersja, &dl, postep.clone()).await?;

    let sciezka_stanu = data.join("state.json");
    let mut stan = state::State::load(&sciezka_stanu);
    let akcje = pack_sync::plan(m, &stan, &pack_sync::DiskProbe::new(&instancja));
    let notatki = pack_sync::apply(m, &instancja, &mut stan, &akcje, &dl, postep.clone()).await?;
    stan.save(&sciezka_stanu).map_err(plik(&sciezka_stanu))?;
    for x in notatki {
        let _ = n.send(Wiadomosc::Notatka(x));
    }

    // Biblioteki dźwięku są dodatkiem, nie warunkiem działania paczki —
    // gdy pobranie się nie uda, gra i tak ma wystartować, a mod poradzi sobie sam.
    if ustawienia.biblioteki_dzwieku && !m.narzedzia.is_empty() {
        if let Err(e) = narzedzia::zapewnij(&instancja, &m.narzedzia, &dl, postep.clone()).await {
            let _ = n.send(Wiadomosc::Notatka(format!(
                "Nie udało się przygotować bibliotek dźwięku ({e}). Gra ruszy, a mod \
                 zapyta o nie sam, gdy będą potrzebne."
            )));
        }
    }

    let dodatkowe = podziel_argumenty(&ustawienia.dodatkowe_argumenty);
    let srodowisko = ustawienia.zaawansowane.srodowisko();
    let opakowanie = komendy::rozbij_opakowanie(&ustawienia.zaawansowane.komenda_wrapper);
    let minimum = ustawienia.minimum_sterty_mb();

    // Komendy gracza dostają te same zmienne, których używa Prism Launcher —
    // gotowe skrypty przenoszą się między launcherami bez przeróbek.
    let kontekst = komendy::Kontekst {
        nazwa: m.pack.name.clone(),
        id: "chmurka".into(),
        katalog_instancji: instancja.clone(),
        katalog_mc: mc.clone(),
        java: java.java_bin.clone(),
        argumenty_javy: {
            let mut a = vec![
                format!("-Xms{minimum}M"),
                format!("-Xmx{}M", ustawienia.pamiec_mb),
            ];
            a.extend(dodatkowe.iter().cloned());
            a.join(" ")
        },
    };

    // Komenda przed startem ma prawo przerwać uruchamianie. Tak ma być:
    // wpisuje się tu rzeczy typu „zrób kopię świata", a granie na świecie,
    // którego kopia się nie udała, jest dokładnie tym, przed czym ta komenda
    // miała chronić.
    if !ustawienia.zaawansowane.komenda_przed.trim().is_empty() {
        let _ = n.send(Wiadomosc::Postep(Progress::trwa(
            Stage::Ready,
            "Wykonuję Twoją komendę przed startem…",
        )));
        komendy::uruchom(
            komendy::Etap::Przed,
            &ustawienia.zaawansowane.komenda_przed,
            &instancja,
            &kontekst,
            &srodowisko,
        )
        .await?;
    }

    let mut cmd = launch::build_command(&launch::LaunchParams {
        java: &java.java_bin,
        mc_dir: &mc,
        game_dir: &instancja,
        version: &wersja,
        account: konto,
        min_mb: minimum,
        max_mb: ustawienia.pamiec_mb,
        dodatkowe: &dodatkowe,
        okno: ustawienia.gra.rozmiar(),
        pelny_ekran: ustawienia.gra.pelny_ekran,
        srodowisko: &srodowisko,
        opakowanie: &opakowanie,
    })?;

    // Log gry leci prosto do pliku, a nie do pamięci launchera. Dzięki temu
    // launcher może schować się do zasobnika, a log i tak powstaje w całości.
    let katalog_logow = data.join("logs");
    std::fs::create_dir_all(&katalog_logow).map_err(plik(&katalog_logow))?;
    let plik_logu = katalog_logow.join("game.log");
    let uchwyt = std::fs::File::create(&plik_logu).map_err(plik(&plik_logu))?;
    let uchwyt2 = uchwyt.try_clone().map_err(plik(&plik_logu))?;

    let _ = n.send(Wiadomosc::Postep(Progress::trwa(
        Stage::Ready,
        "Uruchamiam grę…",
    )));

    let mut dziecko = cmd
        .stdout(std::process::Stdio::from(uchwyt))
        .stderr(std::process::Stdio::from(uchwyt2))
        .spawn()
        .map_err(|e| BladLaunchera::Plik("uruchomienie gry".into(), e))?;

    // Od tego momentu nie wolno juz niczego ponawiac ani kasowac — proces gry
    // dziala i drugie okno Minecrafta byloby gorsze od kazdego bledu.
    ruszyla.store(true, Ordering::SeqCst);
    let _ = n.send(Wiadomosc::GraWystartowala);

    let status = tokio::task::spawn_blocking(move || dziecko.wait())
        .await
        .map_err(|e| BladLaunchera::Plik("oczekiwanie na grę".into(), std::io::Error::other(e)))?
        .map_err(|e| BladLaunchera::Plik("oczekiwanie na grę".into(), e))?;

    // Komenda po zakończeniu — w przeciwieństwie do tej przed startem NIE
    // przerywa niczego. Gra już się skończyła, nie ma czego chronić, a okno
    // błędu po udanej rozgrywce tylko by przestraszyło.
    if let Err(e) = komendy::uruchom(
        komendy::Etap::Po,
        &ustawienia.zaawansowane.komenda_po,
        &instancja,
        &kontekst,
        &srodowisko,
    )
    .await
    {
        let _ = n.send(Wiadomosc::Notatka(format!(
            "Twoja komenda po zakończeniu gry nie wykonała się poprawnie ({e}).              Sama gra zakończyła się normalnie."
        )));
    }

    if status.success() {
        let _ = n.send(Wiadomosc::GraZakonczona(status.code()));
        return Ok(());
    }

    let tresc = std::fs::read_to_string(&plik_logu).unwrap_or_default();
    let ogon: String = tresc
        .lines()
        .rev()
        .take(60)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    Err(BladLaunchera::GraPadla {
        kod: status.code(),
        ogon_logu: ogon,
        wlasne_argumenty: !dodatkowe.is_empty(),
        zabita_przez_system: zabita_przez_system(&status),
        sterta_mb: ustawienia.pamiec_mb,
    })
}

/// Furtka do pracy nad wyglądem: `CHMURKA_ZRZUT=/ścieżka/okno.ppm`.
///
/// Zapisuje zawartość **okna launchera**, a nie ekranu — to jedyny sposób,
/// żeby obejrzeć układ bez wchodzenia komukolwiek na pulpit, i jedyny, który
/// działa tak samo pod Waylandem, pod X11 i na Windowsie.
///
/// PPM, bo to dwie linijki nagłówka i surowe bajty. Dokładanie biblioteki
/// do PNG-ów po to, żeby rozejrzeć się po własnym oknie, byłoby przesadą —
/// `magick` przerobi to na cokolwiek.
fn zrob_zrzut(app: &mut App, ctx: &egui::Context) {
    let Some(cel) = app.zrzut.clone() else {
        return;
    };
    app.klatki += 1;
    ctx.request_repaint();

    // Okienka rozjaśniają się płynnie przez ułamek sekundy i zrzut z tego
    // czasu pokazuje je półprzezroczyste. Nie da się na to poczekać —
    // przysłonięte okno przestaje dostawać przerysowania, więc nie ma komu
    // odliczyć upływu czasu. Wyłączamy więc animacje i pierwszy narysowany
    // stan jest od razu docelowym.
    if app.klatki == 1 {
        ctx.style_mut(|s| s.animation_time = 0.0);
    }

    // Kilka klatek na ustabilizowanie układu: pierwsza jest rysowana, zanim
    // panele poznają swoje rozmiary, więc zrzut z niej pokazuje coś innego
    // niż to, co gracz zobaczy.
    const PO_ILU_KLATKACH: u32 = 8;
    if app.klatki == PO_ILU_KLATKACH {
        ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        return;
    }
    if app.klatki <= PO_ILU_KLATKACH {
        return;
    }

    let obraz = ctx.input(|i| {
        i.events.iter().find_map(|e| match e {
            egui::Event::Screenshot { image, .. } => Some(image.clone()),
            _ => None,
        })
    });
    let Some(obraz) = obraz else {
        // Nie przyszedł jeszcze. Po kilkunastu klatkach dajemy spokój, żeby
        // launcher nie został otwarty na zawsze.
        if app.klatki > PO_ILU_KLATKACH + 60 {
            app.zakoncz = true;
        }
        return;
    };

    let [szer, wys] = obraz.size;
    let mut bajty = format!("P6\n{szer} {wys}\n255\n").into_bytes();
    for p in &obraz.pixels {
        bajty.extend_from_slice(&[p.r(), p.g(), p.b()]);
    }
    match std::fs::write(&cel, &bajty) {
        Ok(()) => eprintln!("zrzut zapisany: {} ({szer}x{wys})", cel.display()),
        Err(e) => eprintln!("nie udało się zapisać zrzutu: {e}"),
    }
    app.zakoncz = true;
}

/// Sprawdza Javę wskazaną ręcznie w Ustawieniach.
///
/// Wskazanie złego pliku kończy się grą, która nie startuje, a komunikat JVM
/// o tym, że coś nie jest programem, nie mówi nic. Dlatego pytamy Javę o wersję
/// zanim zaczniemy cokolwiek pobierać — i mówimy wprost, co jest nie tak.
async fn wlasna_java(
    sciezka: &Path,
    wymagany: u32,
    pomin_sprawdzanie: bool,
) -> Result<chmurka_core::java::JavaInstall, BladLaunchera> {
    use chmurka_core::javy;

    let blad = |powod: String| BladLaunchera::WlasnaJava {
        sciezka: sciezka.display().to_string(),
        powod,
    };

    let info = javy::sprawdz(sciezka)
        .await
        .map_err(|e| blad(e.to_string()))?;

    if !pomin_sprawdzanie {
        if info.major != wymagany {
            return Err(blad(format!(
                "paczka potrzebuje Javy {wymagany}, a ta jest w wersji {} ({})",
                info.major, info.wersja
            )));
        }
        // 32-bitowa JVM nie zaadresuje 4 GB sterty, których wymaga paczka.
        // Bez tego sprawdzenia gracz dostaje „Could not reserve enough space
        // for object heap" i nie ma pojęcia, co z tym zrobić.
        if !info.bity64 {
            return Err(blad(
                "to Java 32-bitowa, a paczka potrzebuje więcej pamięci, niż taka potrafi objąć"
                    .into(),
            ));
        }
    }

    Ok(chmurka_core::java::JavaInstall {
        java_bin: javy::do_uruchamiania(sciezka),
    })
}

/// Czy to system ubił grę, bo zabrakło mu pamięci?
///
/// Jądro wysyła wtedy SIGKILL i proces nie ma jak niczego zapisać — w logu
/// gry nie ma śladu, urywa się w połowie zdania. Bez tego rozpoznania gracz
/// dostawał „gra padła, nie wiemy czemu", choć przyczyna jest konkretna
/// i da się ją naprawić suwakiem pamięci.
fn zabita_przez_system(status: &std::process::ExitStatus) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        // 9 to SIGKILL — tym zabija zarowno jadro, jak i systemd-oomd.
        if status.signal() == Some(9) {
            return true;
        }
    }
    // Powloki i menedzery procesow raportuja zabicie sygnalem jako 128 + numer.
    status.code() == Some(137)
}
