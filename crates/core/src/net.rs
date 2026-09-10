use crate::hash::{sha1_file, sha512_file};
use crate::progress::{Progress, Stage};
use futures_util::StreamExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const PROBY: u32 = 3;

#[derive(Debug, thiserror::Error)]
pub enum NetError {
    #[error("nie udało się pobrać {plik}: wyczerpano próby dla adresów {adresy:?}; ostatni błąd: {ostatni}")]
    Wyczerpano {
        plik: String,
        adresy: Vec<String>,
        ostatni: String,
    },
    #[error("błąd zapisu {0}: {1}")]
    Io(String, std::io::Error),
}

#[derive(Debug, Clone)]
pub enum Expect {
    Sha512(String),
    Sha1(String),
    Any,
}

#[derive(Debug, Clone)]
pub struct DownloadSpec {
    pub urls: Vec<String>,
    pub dest: PathBuf,
    pub expect: Expect,
}

pub struct Downloader {
    client: reqwest::Client,
    limit: Arc<tokio::sync::Semaphore>,
}

impl Downloader {
    pub fn new(concurrency: usize) -> Self {
        // Bez limitow host, ktory przyjmowal polaczenie i milkl, zatrzymywal
        // launcher na zawsze — a petla ponowien i lista zapasowych adresow
        // siedza ZA tym oczekiwaniem, wiec nie ruszaly ani razu.
        let client = crate::limity::klient_pobierania();
        Self {
            client,
            limit: Arc::new(tokio::sync::Semaphore::new(concurrency)),
        }
    }

    /// Zwraca `true`, jeśli plik faktycznie pobrano, `false` jeśli już był poprawny.
    pub async fn fetch_one(&self, spec: &DownloadSpec) -> Result<bool, NetError> {
        self.fetch_one_obserwowane(spec, None).await
    }

    /// Jak `fetch_one`, ale melduje postęp w bajtach.
    ///
    /// Potrzebne przy pojedynczych dużych plikach — JRE waży 45 MB, a licznik
    /// „0 z 1" stał na zerze przez całe pobieranie i skakał od razu na koniec.
    pub async fn fetch_one_obserwowane(
        &self,
        spec: &DownloadSpec,
        obserwator: Option<(Stage, String, Arc<dyn Fn(Progress) + Send + Sync>)>,
    ) -> Result<bool, NetError> {
        if pasuje(&spec.dest, &spec.expect) {
            return Ok(false);
        }
        let _p = self.limit.acquire().await.expect("semafor");

        if let Some(rodzic) = spec.dest.parent() {
            tokio::fs::create_dir_all(rodzic)
                .await
                .map_err(|e| NetError::Io(rodzic.display().to_string(), e))?;
        }
        let czesciowy = plik_czesciowy(&spec.dest);
        let mut ostatni = String::from("brak prób");

        // Każdy adres dostaje pełny komplet prób, zanim przejdziemy do następnego.
        // Meldunek co ~400 kB zamiast co kawałek — inaczej jedno pobranie
        // wysyłałoby do interfejsu tysiące komunikatów na sekundę.
        let melduj: Box<dyn Fn(u64, Option<u64>) + Send + Sync> = match &obserwator {
            None => Box::new(|_, _| {}),
            Some((stage, etykieta, on)) => {
                let stage = *stage;
                let etykieta = etykieta.clone();
                let on = on.clone();
                Box::new(move |pobrane, calosc| {
                    on(Progress::bajty(
                        stage,
                        pobrane,
                        calosc.unwrap_or(0),
                        etykieta.clone(),
                    ));
                })
            }
        };

        for url in &spec.urls {
            for proba in 0..PROBY {
                match self.raz(url, &czesciowy, &melduj).await {
                    Ok(()) => {
                        if pasuje(&czesciowy, &spec.expect) {
                            tokio::fs::rename(&czesciowy, &spec.dest)
                                .await
                                .map_err(|e| NetError::Io(spec.dest.display().to_string(), e))?;
                            return Ok(true);
                        }
                        ostatni = format!("niezgodny hash pobranej treści z {url}");
                    }
                    Err(e) => ostatni = format!("{url}: {e}"),
                }
                let _ = tokio::fs::remove_file(&czesciowy).await;
                if proba + 1 < PROBY {
                    tokio::time::sleep(std::time::Duration::from_millis(400 * (1 << proba))).await;
                }
            }
        }
        Err(NetError::Wyczerpano {
            plik: spec.dest.display().to_string(),
            adresy: spec.urls.clone(),
            ostatni,
        })
    }

    async fn raz(
        &self,
        url: &str,
        czesciowy: &Path,
        melduj: &(dyn Fn(u64, Option<u64>) + Send + Sync),
    ) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;

        const CO_ILE: u64 = 400 * 1024;

        let odp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;

        // Adoptium przekierowuje na GitHub, ktory podaje Content-Length.
        // Gdyby go zabraklo, interfejs przechodzi w tryb nieokreslony.
        let calosc = odp.content_length();
        let mut pobrane: u64 = 0;
        let mut ostatni_meldunek: u64 = 0;
        melduj(0, calosc);

        let mut plik = tokio::fs::File::create(czesciowy)
            .await
            .map_err(|e| e.to_string())?;
        // Strażnik braku postępu. `read_timeout` pilnuje przerw między
        // odczytami, ale serwer sączący po kilka bajtów co kilkanaście sekund
        // formalnie nigdy nie milczy — każdy odczyt mieści się w limicie,
        // a pobieranie stoi. Wymagamy, żeby w każdym oknie przyszła
        // sensowna porcja danych; inaczej kończymy próbę błędem, który
        // trafia w istniejącą pętlę ponowień i listę zapasowych adresów.
        let mut okno_start = tokio::time::Instant::now();
        let mut okno_bajty: u64 = 0;

        let mut strumien = odp.bytes_stream();
        while let Some(kawalek) = strumien.next().await {
            let kawalek = kawalek.map_err(|e| e.to_string())?;
            plik.write_all(&kawalek).await.map_err(|e| e.to_string())?;
            pobrane += kawalek.len() as u64;

            okno_bajty += kawalek.len() as u64;
            if okno_start.elapsed() >= crate::limity::BRAK_POSTEPU {
                if okno_bajty < crate::limity::NAJMNIEJ_BAJTOW_NA_OKNO {
                    return Err(format!(
                        "pobieranie stoi w miejscu: {okno_bajty} B w ciągu {} s",
                        crate::limity::BRAK_POSTEPU.as_secs()
                    ));
                }
                okno_start = tokio::time::Instant::now();
                okno_bajty = 0;
            }

            if pobrane - ostatni_meldunek >= CO_ILE {
                ostatni_meldunek = pobrane;
                melduj(pobrane, calosc);
            }
        }
        plik.flush().await.map_err(|e| e.to_string())?;
        melduj(pobrane, calosc.or(Some(pobrane)));
        Ok(())
    }

    pub async fn fetch_many(
        &self,
        specs: Vec<DownloadSpec>,
        stage: Stage,
        on: Arc<dyn Fn(Progress) + Send + Sync>,
    ) -> Result<(), NetError> {
        let total = specs.len() as u64;
        let done = Arc::new(AtomicU64::new(0));

        let zadania = specs.into_iter().map(|spec| {
            let done = done.clone();
            let on = on.clone();
            async move {
                let etykieta = spec
                    .dest
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.fetch_one(&spec).await?;
                let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                on(Progress::pliki(stage, d, total, etykieta));
                Ok::<(), NetError>(())
            }
        });

        // buffer_unordered pilnuje równoległości razem z semaforem w fetch_one.
        let mut strumien = futures_util::stream::iter(zadania).buffer_unordered(16);
        while let Some(wynik) = strumien.next().await {
            wynik?;
        }
        Ok(())
    }
}

/// Nazwa pliku roboczego dla pobierania: pełna nazwa docelowa plus `.part`.
///
/// Kiedyś było tu `with_extension("part")`, które **podmienia** rozszerzenie.
/// Dwa pliki w jednym katalogu o tym samym trzonie dostawały przez to wspólny
/// plik roboczy: `chloride-client.toml_backup1` i `…_backup2` walczyły oba
/// o `chloride-client.part`. Pobierają się równolegle, więc jedno zadanie
/// przenosiło plik na miejsce, a drugie chwilę później próbowało przenieść coś,
/// czego już nie było — gracz dostawał PLIK-03 „No such file or directory”.
fn plik_czesciowy(cel: &Path) -> PathBuf {
    let nazwa = cel
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "pobieranie".to_string());
    cel.with_file_name(format!("{nazwa}.part"))
}

fn pasuje(sciezka: &Path, oczekiwane: &Expect) -> bool {
    if !sciezka.exists() {
        return false;
    }
    match oczekiwane {
        Expect::Any => true,
        Expect::Sha512(h) => sha512_file(sciezka)
            .map(|x| x.eq_ignore_ascii_case(h))
            .unwrap_or(false),
        Expect::Sha1(h) => sha1_file(sciezka)
            .map(|x| x.eq_ignore_ascii_case(h))
            .unwrap_or(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// Dokladnie ta para wywrocila instalacje u testera: dwa pliki NeoForge'a
    /// w jednym katalogu, roznica tylko w koncowce. Pobieraja sie rownolegle,
    /// wiec wspolny plik roboczy znaczyl, ze jedno zadanie przenosi go na
    /// miejsce, a drugie trafia w pustke — PLIK-03 „No such file or directory".
    #[test]
    fn pliki_o_tym_samym_trzonie_nie_dziela_pliku_roboczego() {
        let a = plik_czesciowy(Path::new("config/chloride-client.toml_backup1"));
        let b = plik_czesciowy(Path::new("config/chloride-client.toml_backup2"));
        assert_ne!(a, b, "wspolny plik roboczy to wyscig przy pobieraniu");
        assert_eq!(a, Path::new("config/chloride-client.toml_backup1.part"));
    }

    /// Zwykle rozszerzenia tez nie moga sie zlewac — `x.json` i `x.toml`
    /// w jednym katalogu to w paczce sytuacja codzienna.
    #[test]
    fn rozne_rozszerzenia_daja_rozne_pliki_roboczne() {
        assert_ne!(
            plik_czesciowy(Path::new("config/x.json")),
            plik_czesciowy(Path::new("config/x.toml"))
        );
    }

    #[test]
    fn plik_roboczy_lezy_obok_celu() {
        assert_eq!(
            plik_czesciowy(Path::new("/a/b/mod.jar")),
            Path::new("/a/b/mod.jar.part")
        );
    }

    /// Minimalny serwer HTTP na jedno żądanie. Nie chcemy ciągnąć frameworka
    /// tylko po to, żeby oddać kilka bajtów w teście.
    fn serwer(tresc: &'static [u8], status: &'static str) -> (String, std::thread::JoinHandle<()>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let h = std::thread::spawn(move || {
            if let Ok((mut s, _)) = l.accept() {
                let mut buf = [0u8; 1024];
                let _ = s.read(&mut buf);
                let naglowek = format!(
                    "HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    tresc.len()
                );
                let _ = s.write_all(naglowek.as_bytes());
                let _ = s.write_all(tresc);
            }
        });
        (format!("http://127.0.0.1:{port}/plik"), h)
    }

    #[tokio::test]
    async fn pobiera_i_weryfikuje_hash() {
        let (url, h) = serwer(b"chmurka", "200 OK");
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("a.bin");
        let dl = Downloader::new(2);
        let spec = DownloadSpec {
            urls: vec![url],
            dest: dest.clone(),
            expect: Expect::Sha512(crate::hash::sha512_hex(b"chmurka")),
        };
        assert!(dl.fetch_one(&spec).await.unwrap());
        assert_eq!(std::fs::read(&dest).unwrap(), b"chmurka");
        h.join().unwrap();
    }

    #[tokio::test]
    async fn pomija_plik_ktory_juz_jest_poprawny() {
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("a.bin");
        std::fs::write(&dest, b"chmurka").unwrap();
        let dl = Downloader::new(2);
        let spec = DownloadSpec {
            // Adres jest celowo nieosiągalny: gdyby launcher próbował pobierać,
            // test by padł. Chcemy udowodnić, że nawet nie zaczyna.
            urls: vec!["https://127.0.0.1:1/nie-istnieje".into()],
            dest: dest.clone(),
            expect: Expect::Sha512(crate::hash::sha512_hex(b"chmurka")),
        };
        assert!(!dl.fetch_one(&spec).await.unwrap());
    }

    #[tokio::test]
    async fn zly_hash_konczy_sie_bledem_i_nie_zostawia_pliku() {
        let (url, h) = serwer(b"zepsute", "200 OK");
        let dir = tempfile::tempdir().unwrap();
        let dest = dir.path().join("a.bin");
        let dl = Downloader::new(2);
        let spec = DownloadSpec {
            urls: vec![url],
            dest: dest.clone(),
            expect: Expect::Sha512(crate::hash::sha512_hex(b"chmurka")),
        };
        assert!(dl.fetch_one(&spec).await.is_err());
        assert!(!dest.exists(), "uszkodzony plik nie może zostać na dysku");
        assert!(
            !dest.with_extension("part").exists(),
            "plik .part musi być posprzątany"
        );
        h.join().unwrap();
    }
}
