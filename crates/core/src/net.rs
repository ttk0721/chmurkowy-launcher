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
        let client = reqwest::Client::builder()
            .user_agent("ChmurkowyLauncher/0.1")
            .build()
            .expect("klient HTTP");
        Self {
            client,
            limit: Arc::new(tokio::sync::Semaphore::new(concurrency)),
        }
    }

    /// Zwraca `true`, jeśli plik faktycznie pobrano, `false` jeśli już był poprawny.
    pub async fn fetch_one(&self, spec: &DownloadSpec) -> Result<bool, NetError> {
        if pasuje(&spec.dest, &spec.expect) {
            return Ok(false);
        }
        let _p = self.limit.acquire().await.expect("semafor");

        if let Some(rodzic) = spec.dest.parent() {
            tokio::fs::create_dir_all(rodzic)
                .await
                .map_err(|e| NetError::Io(rodzic.display().to_string(), e))?;
        }
        let czesciowy = spec.dest.with_extension("part");
        let mut ostatni = String::from("brak prób");

        // Każdy adres dostaje pełny komplet prób, zanim przejdziemy do następnego.
        for url in &spec.urls {
            for proba in 0..PROBY {
                match self.raz(url, &czesciowy).await {
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

    async fn raz(&self, url: &str, czesciowy: &Path) -> Result<(), String> {
        use tokio::io::AsyncWriteExt;

        let odp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        let mut plik = tokio::fs::File::create(czesciowy)
            .await
            .map_err(|e| e.to_string())?;
        let mut strumien = odp.bytes_stream();
        while let Some(kawalek) = strumien.next().await {
            let kawalek = kawalek.map_err(|e| e.to_string())?;
            plik.write_all(&kawalek).await.map_err(|e| e.to_string())?;
        }
        plik.flush().await.map_err(|e| e.to_string())?;
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
        let bajty = Arc::new(AtomicU64::new(0));

        let zadania = specs.into_iter().map(|spec| {
            let done = done.clone();
            let bajty = bajty.clone();
            let on = on.clone();
            async move {
                let etykieta = spec
                    .dest
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();
                self.fetch_one(&spec).await?;
                let rozmiar = tokio::fs::metadata(&spec.dest)
                    .await
                    .map(|m| m.len())
                    .unwrap_or(0);
                let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                let b = bajty.fetch_add(rozmiar, Ordering::Relaxed) + rozmiar;
                on(Progress {
                    stage,
                    done: d,
                    total,
                    bytes: b,
                    label: etykieta,
                });
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
