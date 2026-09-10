use crate::zasobnik::{Zasobnik, ZdarzenieZasobnika};
use chmurka_core::auth::{msa, msa::DeviceCode, Account};
use chmurka_core::bledy::{BladLaunchera, BladUzytkownika};
use chmurka_core::manifest::Manifest;
use chmurka_core::progress::{Progress, Stage};
use chmurka_core::paczki::{StanShaderow, StanZasobow};
use chmurka_core::ustawienia::{podziel_argumenty, Ustawienia};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Widok {
    Glowny,
    Logowanie,
    Ustawienia,
    Paczki,
    Blad,
}

/// Wiadomości płynące z zadań w tle do wątku rysującego.
pub enum Wiadomosc {
    Manifest(Box<Manifest>),
    Postep(Progress),
    Notatka(String),
    /// Błąd z kodem i instrukcją — pokazywany na własnym ekranie.
    BladZKodem(Box<BladUzytkownika>),
    KodUrzadzenia(Box<DeviceCode>),
    Zalogowano(Box<Account>),
    /// Gra ruszyła — czas schować okno, jeśli gracz sobie tego życzy.
    GraWystartowala,
    GraZakonczona(Option<i32>),
}

pub struct App {
    pub katalog: PathBuf,
    pub widok: Widok,
    pub manifest: Option<Manifest>,
    pub konto: Option<Account>,
    pub kod: Option<DeviceCode>,
    pub postep: Option<Progress>,
    pub blad_z_kodem: Option<BladUzytkownika>,
    /// Krótka informacja zwrotna po akcji w ustawieniach.
    pub komunikat: Option<String>,
    pub log: Vec<String>,
    pub pokaz_szczegoly: bool,
    pub ustawienia: Ustawienia,
    /// Stan paczek czytany z plików gry przy każdym wejściu na ekran —
    /// gracz mógł je pozmieniać w samej grze.
    pub zasoby: StanZasobow,
    pub shadery: StanShaderow,
    pub zajety: bool,
    /// Prawda od startu gry do jej zakończenia.
    pub gra_dziala: bool,
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
    pub runtime: tokio::runtime::Runtime,
}

impl App {
    pub fn nowa(katalog: PathBuf) -> Self {
        let (nadawca, odbiorca) = std::sync::mpsc::channel();
        let (nadawca_zas, odbiorca_zasobnika) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime tokio");

        let plik_ustawien = katalog.join("data").join("settings.json");
        let pierwsze_uruchomienie = !plik_ustawien.is_file();
        let mut ustawienia = Ustawienia::wczytaj(&plik_ustawien);

        // Przy pierwszym uruchomieniu dobieramy pamięć do komputera. Później już
        // nie ruszamy — to wybór gracza, nawet jeśli odbiega od zalecenia.
        if pierwsze_uruchomienie {
            if let Some(mb) = chmurka_core::pamiec::calkowita_mb() {
                ustawienia.pamiec_mb = chmurka_core::pamiec::zalecana_mb(mb);
            }
        }
        // ksni zaklada dzialajacy runtime tokio, wiec ikone tworzymy w jego kontekscie.
        let zasobnik = runtime.block_on(crate::zasobnik::utworz(nadawca_zas));

        let mut app = Self {
            katalog,
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
            kod: None,
            postep: None,
            blad_z_kodem: None,
            komunikat: None,
            log: Vec::new(),
            pokaz_szczegoly: false,
            ustawienia,
            zasoby: StanZasobow::default(),
            shadery: StanShaderow::default(),
            zajety: false,
            gra_dziala: false,
            schowaj_okno: false,
            przywroc_okno: false,
            zakoncz: false,
            nadawca,
            odbiorca,
            odbiorca_zasobnika,
            ma_zasobnik: zasobnik.is_some(),
            _zasobnik: zasobnik,
            runtime,
        };
        // Podgląd ekranu błędu przy pracy nad wyglądem: CHMURKA_WIDOK=blad
        if app.widok == Widok::Blad {
            app.blad_z_kodem = Some(
                BladLaunchera::GraPadla {
                    kod: Some(1),
                    ogon_logu: "java.lang.OutOfMemoryError: Java heap space\n\tat net.minecraft.client.main.Main.main(Main.java:1)".into(),
                    wlasne_argumenty: false,
                }
                .dla_uzytkownika(),
            );
        }
        if pierwsze_uruchomienie {
            app.zapisz_ustawienia();
        }
        if app.widok == Widok::Paczki {
            app.odswiez_paczki();
        }
        app.wczytaj_manifest();
        app.wznow_sesje();
        app
    }

    pub fn data(&self) -> PathBuf {
        self.katalog.join("data")
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

    fn wczytaj_manifest(&mut self) {
        let n = self.nadawca.clone();
        let adres = env!("CHMURKA_MANIFEST_URL").to_string();
        self.runtime.spawn(async move {
            match pobierz_manifest(&adres).await {
                Ok(m) => {
                    let _ = n.send(Wiadomosc::Manifest(Box::new(m)));
                }
                Err(e) => {
                    let _ = n.send(Wiadomosc::BladZKodem(Box::new(e.dla_uzytkownika())));
                }
            }
        });
    }

    /// Próbuje odtworzyć poprzednie logowanie bez pytania użytkownika.
    /// Niepowodzenie jest ciche — użytkownik po prostu zobaczy ekran logowania.
    fn wznow_sesje(&mut self) {
        let sciezka = self.data().join("auth.json");
        let Some(zapis) = chmurka_core::auth::store::load(&sciezka) else {
            return;
        };
        let n = self.nadawca.clone();
        self.runtime.spawn(async move {
            use chmurka_core::auth::store;
            let client_id = "00000000402b5328";
            let Ok(t) = msa::refresh(client_id, &zapis.refresh_token).await else {
                return;
            };
            if let Ok(konto) = msa::zaloguj_minecraft(&t).await {
                let _ = store::save(
                    &sciezka,
                    &store::Zapis {
                        refresh_token: t.refresh_token,
                        nick: konto.name.clone(),
                    },
                );
                let _ = n.send(Wiadomosc::Zalogowano(Box::new(konto)));
            }
        });
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
                Wiadomosc::Manifest(m) => self.manifest = Some(*m),
                Wiadomosc::Postep(p) => {
                    if p.stage == Stage::Ready {
                        self.log.push(p.label.clone());
                    }
                    self.postep = Some(p);
                }
                Wiadomosc::Notatka(s) => self.log.push(s),
                Wiadomosc::BladZKodem(b) => {
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
                Wiadomosc::Zalogowano(k) => {
                    self.kod = None;
                    self.zajety = false;
                    self.konto = Some(*k);
                    if self.widok == Widok::Logowanie {
                        self.widok = Widok::Glowny;
                    }
                }
                Wiadomosc::GraWystartowala => {
                    self.gra_dziala = true;
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
                    self.log.push(format!(
                        "Gra zakończyła się kodem {}",
                        kod.map(|k| k.to_string()).unwrap_or_else(|| "nieznanym".into())
                    ));
                }
            }
        }
    }
}

async fn pobierz_manifest(adres: &str) -> Result<Manifest, BladLaunchera> {
    let siec = |e: reqwest::Error| BladLaunchera::Logowanie(msa::AuthError::Siec(e.to_string()));
    let tekst = reqwest::get(adres).await.map_err(siec)?.text().await.map_err(siec)?;
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
        if self.gra_dziala {
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        } else if self.zajety || self.kod.is_some() || self.manifest.is_none() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }

        match self.widok {
            Widok::Glowny => crate::views::main::rysuj(self, ctx),
            Widok::Logowanie => crate::views::login::rysuj(self, ctx),
            Widok::Ustawienia => crate::views::settings::rysuj(self, ctx),
            Widok::Paczki => crate::views::packs::rysuj(self, ctx),
            Widok::Blad => crate::views::error::rysuj(self, ctx),
        }
    }
}

/// Startuje logowanie Microsoft w tle i odpytuje o token aż do skutku.
pub fn zaloguj_microsoft(app: &mut App) {
    let Some(m) = &app.manifest else { return };
    let client_id = m.auth.msa_client_id.clone();
    let n = app.nadawca.clone();
    let sciezka_auth = app.data().join("auth.json");

    app.blad_z_kodem = None;
    app.zajety = true;
    app.runtime.spawn(async move {
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
            tokio::time::sleep(std::time::Duration::from_secs(odstep)).await;
            match msa::poll_once(&client_id, &device_code).await {
                Ok(msa::PollResult::Czekamy) => {}
                // Microsoft prosi o wolniejsze odpytywanie — zignorowanie tego
                // kończy się zablokowaniem całej sesji logowania.
                Ok(msa::PollResult::Zwolnij) => odstep += 5,
                Ok(msa::PollResult::Gotowe(t)) => {
                    return match msa::zaloguj_minecraft(&t).await {
                        Ok(konto) => {
                            let _ = chmurka_core::auth::store::save(
                                &sciezka_auth,
                                &chmurka_core::auth::store::Zapis {
                                    refresh_token: t.refresh_token,
                                    nick: konto.name.clone(),
                                },
                            );
                            let _ = n.send(Wiadomosc::Zalogowano(Box::new(konto)));
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

    app.runtime.spawn(async move {
        let n2 = n.clone();
        let postep: Arc<dyn Fn(Progress) + Send + Sync> =
            Arc::new(move |p| {
                let _ = n2.send(Wiadomosc::Postep(p));
            });

        if let Err(e) = przygotuj_i_odpal(&data, &m, &konto, &ustawienia, postep, &n).await {
            let _ = n.send(Wiadomosc::BladZKodem(Box::new(e.dla_uzytkownika())));
        }
    });
}

async fn przygotuj_i_odpal(
    data: &Path,
    m: &Manifest,
    konto: &Account,
    ustawienia: &Ustawienia,
    postep: Arc<dyn Fn(Progress) + Send + Sync>,
    n: &Sender<Wiadomosc>,
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
    })
}
