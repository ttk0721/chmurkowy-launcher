use chmurka_core::auth::{msa::DeviceCode, Account};
use chmurka_core::manifest::Manifest;
use chmurka_core::progress::{Progress, Stage};
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Widok {
    Glowny,
    Logowanie,
    Ustawienia,
}

/// Wiadomości płynące z zadań w tle do wątku rysującego.
pub enum Wiadomosc {
    Manifest(Box<Manifest>),
    Postep(Progress),
    Notatka(String),
    Blad(String),
    KodUrzadzenia(Box<DeviceCode>),
    Zalogowano(Box<Account>),
    GraZakonczona(Option<i32>),
}

pub struct App {
    pub katalog: PathBuf,
    pub widok: Widok,
    pub manifest: Option<Manifest>,
    pub konto: Option<Account>,
    pub kod: Option<DeviceCode>,
    pub postep: Option<Progress>,
    pub blad: Option<String>,
    /// Krótka informacja zwrotna po akcji w ustawieniach — bez niej
    /// przyciski wyglądały, jakby nic nie robiły.
    pub komunikat: Option<String>,
    pub log: Vec<String>,
    pub pokaz_szczegoly: bool,
    pub nick_offline: String,
    pub pamiec_mb: u32,
    pub zajety: bool,
    pub nadawca: Sender<Wiadomosc>,
    pub odbiorca: Receiver<Wiadomosc>,
    pub runtime: tokio::runtime::Runtime,
}

impl App {
    pub fn nowa(katalog: PathBuf) -> Self {
        let (nadawca, odbiorca) = std::sync::mpsc::channel();
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("runtime tokio");

        let mut app = Self {
            katalog,
            // Furtka do pracy nad wygladem: CHMURKA_WIDOK=logowanie|ustawienia
            // pozwala otworzyc launcher od razu na danym ekranie.
            widok: match std::env::var("CHMURKA_WIDOK").as_deref() {
                Ok("logowanie") => Widok::Logowanie,
                Ok("ustawienia") => Widok::Ustawienia,
                _ => Widok::Glowny,
            },
            manifest: None,
            konto: None,
            kod: None,
            postep: None,
            blad: None,
            komunikat: None,
            log: Vec::new(),
            pokaz_szczegoly: false,
            nick_offline: String::new(),
            pamiec_mb: 4096,
            zajety: false,
            nadawca,
            odbiorca,
            runtime,
        };
        app.wczytaj_manifest();
        app.wznow_sesje();
        app
    }

    pub fn data(&self) -> PathBuf {
        self.katalog.join("data")
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
                    let _ = n.send(Wiadomosc::Blad(format!(
                        "Nie udało się pobrać informacji o paczce: {e}"
                    )));
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
            use chmurka_core::auth::{msa, store};
            // Manifest jeszcze się ściąga, więc używamy tego samego identyfikatora,
            // który i tak w nim stoi. Odświeżanie nie może czekać na sieć dwa razy.
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
        while let Ok(w) = self.odbiorca.try_recv() {
            match w {
                Wiadomosc::Manifest(m) => {
                    self.pamiec_mb = m.memory.max_mb;
                    self.manifest = Some(*m);
                }
                Wiadomosc::Postep(p) => {
                    if p.stage == Stage::Ready {
                        self.log.push(p.label.clone());
                    }
                    self.postep = Some(p);
                }
                Wiadomosc::Notatka(s) => self.log.push(s),
                Wiadomosc::Blad(e) => {
                    self.zajety = false;
                    self.log.push(format!("BŁĄD: {e}"));
                    self.blad = Some(e);
                }
                Wiadomosc::KodUrzadzenia(d) => self.kod = Some(*d),
                Wiadomosc::Zalogowano(k) => {
                    self.kod = None;
                    self.zajety = false;
                    self.konto = Some(*k);
                    self.widok = Widok::Glowny;
                }
                Wiadomosc::GraZakonczona(kod) => {
                    self.zajety = false;
                    self.postep = None;
                    if !matches!(kod, Some(0)) {
                        self.blad = Some(format!(
                            "Gra zakończyła się kodem {}. Zajrzyj do szczegółów.",
                            kod.map(|k| k.to_string()).unwrap_or_else(|| "nieznanym".into())
                        ));
                        self.pokaz_szczegoly = true;
                    }
                }
            }
        }
    }
}

async fn pobierz_manifest(adres: &str) -> anyhow::Result<Manifest> {
    let tekst = reqwest::get(adres).await?.text().await?;
    Ok(chmurka_core::manifest::parse(&tekst)?)
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _f: &mut eframe::Frame) {
        self.odbierz();
        // Zadania w tle nie budzą pętli rysującej same z siebie.
        if self.zajety || self.kod.is_some() || self.manifest.is_none() {
            ctx.request_repaint_after(std::time::Duration::from_millis(120));
        }
        match self.widok {
            Widok::Glowny => crate::views::main::rysuj(self, ctx),
            Widok::Logowanie => crate::views::login::rysuj(self, ctx),
            Widok::Ustawienia => crate::views::settings::rysuj(self, ctx),
        }
    }
}

/// Startuje logowanie Microsoft w tle i odpytuje o token aż do skutku.
pub fn zaloguj_microsoft(app: &mut App) {
    let Some(m) = &app.manifest else { return };
    let client_id = m.auth.msa_client_id.clone();
    let n = app.nadawca.clone();
    let sciezka_auth = app.data().join("auth.json");

    app.blad = None;
    app.zajety = true;
    app.runtime.spawn(async move {
        use chmurka_core::auth::msa;

        let kod = match msa::begin(&client_id).await {
            Ok(k) => k,
            Err(e) => {
                let _ = n.send(Wiadomosc::Blad(e.to_string()));
                return;
            }
        };
        let device_code = kod.device_code.clone();
        let mut odstep = kod.interval_s.max(1);
        let koniec = std::time::Instant::now() + std::time::Duration::from_secs(kod.expires_in_s);
        let _ = n.send(Wiadomosc::KodUrzadzenia(Box::new(kod)));

        loop {
            if std::time::Instant::now() > koniec {
                let _ = n.send(Wiadomosc::Blad("Kod wygasł — spróbuj jeszcze raz.".into()));
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(odstep)).await;
            match msa::poll_once(&client_id, &device_code).await {
                Ok(msa::PollResult::Czekamy) => {}
                // Microsoft prosi o wolniejsze odpytywanie — zignorowanie tego
                // kończy się zablokowaniem całej sesji logowania.
                Ok(msa::PollResult::Zwolnij) => odstep += 5,
                Ok(msa::PollResult::Gotowe(t)) => {
                    match msa::zaloguj_minecraft(&t).await {
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
                        Err(e) => {
                            let _ = n.send(Wiadomosc::Blad(e.to_string()));
                        }
                    }
                    return;
                }
                Err(e) => {
                    let _ = n.send(Wiadomosc::Blad(e.to_string()));
                    return;
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
    let pamiec = app.pamiec_mb;

    app.blad = None;
    app.zajety = true;
    app.log.clear();

    app.runtime.spawn(async move {
        use chmurka_core::*;
        use std::sync::Arc;

        let n2 = n.clone();
        let postep: Arc<dyn Fn(progress::Progress) + Send + Sync> = Arc::new(move |p| {
            let _ = n2.send(Wiadomosc::Postep(p));
        });

        let wynik = async {
            let dl = net::Downloader::new(8);
            let mc = data.join("mc");
            let instancja = data.join("instance");
            std::fs::create_dir_all(&instancja)?;

            let java = java::ensure(&data, m.java.major, &dl, postep.clone()).await?;
            let profil = game_install::ensure_loader(
                &mc,
                &java,
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
            let akcje = pack_sync::plan(&m, &stan, &pack_sync::DiskProbe::new(&instancja));
            let notatki =
                pack_sync::apply(&m, &instancja, &mut stan, &akcje, &dl, postep.clone()).await?;
            stan.save(&sciezka_stanu)?;
            for x in notatki {
                let _ = n.send(Wiadomosc::Notatka(x));
            }

            let mut cmd = launch::build_command(&launch::LaunchParams {
                java: &java.java_bin,
                mc_dir: &mc,
                game_dir: &instancja,
                version: &wersja,
                account: &konto,
                min_mb: m.memory.min_mb,
                max_mb: pamiec,
            })?;
            let _ = n.send(Wiadomosc::Postep(progress::Progress::pliki(
                progress::Stage::Ready,
                1,
                1,
                "Uruchamiam grę",
            )));

            // Log gry trafia do pliku i do panelu „szczegóły".
            // Testowanie paczki polega głównie na czytaniu crashy, więc
            // wyjście procesu jest najcenniejszą rzeczą, jaką launcher produkuje.
            let katalog_logow = data.join("logs");
            std::fs::create_dir_all(&katalog_logow)?;
            let wyjscie = cmd
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .output()?;

            let mut tresc = String::from_utf8_lossy(&wyjscie.stdout).into_owned();
            tresc.push_str(&String::from_utf8_lossy(&wyjscie.stderr));
            let plik_logu = katalog_logow.join("game.log");
            std::fs::write(&plik_logu, &tresc)?;

            if !wyjscie.status.success() {
                let ogon: Vec<&str> = tresc.lines().rev().take(40).collect();
                for linia in ogon.into_iter().rev() {
                    let _ = n.send(Wiadomosc::Notatka(linia.to_string()));
                }
                let _ = n.send(Wiadomosc::Notatka(format!(
                    "pełny log: {}",
                    plik_logu.display()
                )));
            }

            let _ = n.send(Wiadomosc::GraZakonczona(wyjscie.status.code()));
            Ok::<(), anyhow::Error>(())
        }
        .await;

        if let Err(e) = wynik {
            let _ = n.send(Wiadomosc::Blad(e.to_string()));
        }
    });
}
