use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::io::Read;
use std::path::Path;

pub fn sha512_hex(data: &[u8]) -> String {
    let mut h = Sha512::new();
    h.update(data);
    hex(&h.finalize())
}

pub fn sha512_file(path: &Path) -> std::io::Result<String> {
    let mut h = Sha512::new();
    stream(path, |chunk| h.update(chunk))?;
    Ok(hex(&h.finalize()))
}

pub fn sha1_file(path: &Path) -> std::io::Result<String> {
    let mut h = Sha1::new();
    stream(path, |chunk| h.update(chunk))?;
    Ok(hex(&h.finalize()))
}

/// Czyta plik kawałkami, żeby 93-megabajtowy mod nie wjeżdżał w całości do pamięci.
fn stream(path: &Path, mut sink: impl FnMut(&[u8])) -> std::io::Result<()> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        sink(&buf[..n]);
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CHMURKA_SHA512: &str = "075f74768309ab9b7cc792bda5af551e8b121c71881dcad1215fe941911e033ce89db16e97644a42995f32935c828a9ad9028f2a3a362a83e29829a3ee00c658";

    #[test]
    fn sha512_zgadza_sie_z_wektorem() {
        assert_eq!(sha512_hex(b"chmurka"), CHMURKA_SHA512);
    }

    #[test]
    fn sha512_pliku_zgadza_sie_z_buforem() {
        let dir = std::env::temp_dir().join("chmurka-test-hash");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.bin");
        std::fs::write(&p, b"chmurka").unwrap();
        assert_eq!(sha512_file(&p).unwrap(), CHMURKA_SHA512);
        std::fs::remove_file(&p).unwrap();
    }

    #[test]
    fn sha1_pliku_zgadza_sie_z_wektorem() {
        let dir = std::env::temp_dir().join("chmurka-test-hash");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("b.bin");
        std::fs::write(&p, b"abc").unwrap();
        // sha1("abc") to znany wektor z RFC 3174
        assert_eq!(
            sha1_file(&p).unwrap(),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        std::fs::remove_file(&p).unwrap();
    }
}
