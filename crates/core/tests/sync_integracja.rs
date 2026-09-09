use chmurka_core::hash::sha512_hex;
use chmurka_core::manifest;
use chmurka_core::net::Downloader;
use chmurka_core::pack_sync::{apply, plan, DiskProbe};
use chmurka_core::progress::Progress;
use chmurka_core::state::State;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Arc;

/// Serwer oddający ustaloną treść na każde żądanie, dopóki żyje wątek.
fn serwer(tresc: &'static [u8]) -> (u16, std::thread::JoinHandle<()>) {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = l.local_addr().unwrap().port();
    let h = std::thread::spawn(move || {
        for s in l.incoming().take(1) {
            let mut s = match s {
                Ok(s) => s,
                Err(_) => break,
            };
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let n = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                tresc.len()
            );
            let _ = s.write_all(n.as_bytes());
            let _ = s.write_all(tresc);
        }
    });
    (port, h)
}

fn manifest_json(port: u16, hash: &str) -> String {
    format!(
        r#"{{
      "schema": 1,
      "pack": {{ "name": "T", "edition": "", "version": "1", "minecraft": "1.21.1",
                 "loader": {{ "kind": "neoforge", "version": "21.1.249" }} }},
      "java": {{ "major": 21, "distribution": "temurin" }},
      "memory": {{ "min_mb": 512, "max_mb": 4096 }},
      "auth": {{ "msa_client_id": "x" }},
      "launcher": {{ "latest_version": "1", "urls": {{}} }},
      "mirror_dirs": ["mods"],
      "files": [
        {{ "path": "mods/a.jar", "size": 7, "sha512": "{hash}", "policy": "mirror",
           "urls": ["https://127.0.0.1:{port}/a.jar"] }}
      ]
    }}"#
    )
}

#[tokio::test]
async fn pobiera_mod_kasuje_obcy_i_zapisuje_stan() {
    let (port, h) = serwer(b"chmurka");
    let hash = sha512_hex(b"chmurka");
    // Manifest wymaga https, ale w tescie mamy serwer http.
    // Podmieniamy schemat dopiero po walidacji, zeby przetestowac obie rzeczy.
    let mut m = manifest::parse(&manifest_json(port, &hash)).unwrap();
    m.files[0].urls[0] = m.files[0].urls[0].replace("https://", "http://");

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("mods")).unwrap();
    std::fs::write(root.join("mods/obcy.jar"), b"nie nasz").unwrap();

    let mut st = State::new();
    let akcje = plan(&m, &st, &DiskProbe::new(root));
    let dl = Downloader::new(4);
    let cichy: Arc<dyn Fn(Progress) + Send + Sync> = Arc::new(|_| {});
    apply(&m, root, &mut st, &akcje, &dl, cichy).await.unwrap();

    assert_eq!(std::fs::read(root.join("mods/a.jar")).unwrap(), b"chmurka");
    assert!(
        !root.join("mods/obcy.jar").exists(),
        "obcy mod musi zniknac"
    );
    assert_eq!(st.written.get("mods/a.jar").unwrap(), &hash);

    // Drugi przebieg nie ma juz nic do roboty.
    let akcje2 = plan(&m, &st, &DiskProbe::new(root));
    assert!(
        akcje2.is_empty(),
        "ponowna synchronizacja nie powinna nic robic"
    );
    h.join().unwrap();
}
