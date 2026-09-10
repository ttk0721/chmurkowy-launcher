use crate::zasobnik::{Zasobnik, ZdarzenieZasobnika};
use chmurka_core::auth::{msa, msa::DeviceCode, Account};
use chmurka_core::bledy::{BladLaunchera, BladUzytkownika};
use chmurka_core::manifest::Manifest;
use chmurka_core::progress::{Progress, Stage};
use chmurka_core::paczki::{StanShaderow, StanZasobow};
use chmurka_core::ustawienia::{podziel_argumenty, Ustawienia};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Widok {
    Glowny,
    Logowanie,
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
        let docelowy = chmurka_core::miejsca::katalog_uzytkownika()
            .unwrap_or_else(|| katalog.clone());
        let (katalog_danych, przeprowadzka) =
            chmurka_core::miejsca::ustal(&katalog, &docelowy);
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
            ustawienia,
            zasoby: StanZasobow::default(),
            shadery: StanShaderow::default(),
            zajety: false,
            gra_dziala: false,
            konsola: chmurka_core::konsola::Konsola::nowa(
                sciezka_logu,
            ),
            schowaj_okno: false,
            przywroc_okno: false,
            zakoncz: false,
            nadawca,
            odbiorca,
            odbiorca_zasobnika,
            ma_zasobnik: zasobnik.is_some(),
            _zasobnik: zasobnik,
            runtime: Some(runtime),
        };
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
    fn w_tle<F>(&self, zadanie: F)
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
        let n = self.nadawca.clone();
        let adres = env!("CHMURKA_MANIFEST_URL").to_string();
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
                let _ = n.send(Wiadomosc::BladZKodem(Box::new(e.dla_uzytkownika())));
            }
        });
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
        let url = m
            .launcher
            .urls
            .get(aktualizacja::klucz_systemu())
            .cloned();
        let data = self.data();

        match aktualizacja::zdecyduj(biezaca, &najnowsza, url.as_deref(), &data) {
            Decyzja::Aktualna => return,
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
                    self.zaktualizuj_sie();
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
                        kod.map(|k| k.to_string()).unwrap_or_else(|| "nieznanym".into())
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

        match self.widok {
            Widok::Glowny => crate::views::main::rysuj(self, ctx),
            Widok::Logowanie => crate::views::login::rysuj(self, ctx),
            Widok::Ustawienia => crate::views::settings::rysuj(self, ctx),
            Widok::Paczki => crate::views::packs::rysuj(self, ctx),
            Widok::Blad => crate::views::error::rysuj(self, ctx),
            Widok::Konsola => crate::views::console::rysuj(self, ctx),
            Widok::Aktualizacja => crate::views::update::rysuj(self, ctx),
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
                Err(e) => return zglos(e, &n),
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
        let postep: Arc<dyn Fn(Progress) + Send + Sync> =
            Arc::new(move |p| {
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
        let e = match przygotuj_i_odpal(
            data,
            m,
            konto,
            ustawienia,
            postep.clone(),
            n,
            &ruszyla,
        )
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

    let java = java::ensure(data, m.java.major, &dl, postep.clone()).await?;
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
        if let Err(e) =
            narzedzia::zapewnij(&instancja, &m.narzedzia, &dl, postep.clone()).await
        {
            let _ = n.send(Wiadomosc::Notatka(format!(
                "Nie udało się przygotować bibliotek dźwięku ({e}). Gra ruszy, a mod \
                 zapyta o nie sam, gdy będą potrzebne."
            )));
        }
    }

    let dodatkowe = podziel_argumenty(&ustawienia.dodatkowe_argumenty);
    let mut cmd = launch::build_command(&launch::LaunchParams {
        java: &java.java_bin,
        mc_dir: &mc,
        game_dir: &instancja,
        version: &wersja,
        account: konto,
        min_mb: m.memory.min_mb,
        max_mb: ustawienia.pamiec_mb,
        dodatkowe: &dodatkowe,
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
