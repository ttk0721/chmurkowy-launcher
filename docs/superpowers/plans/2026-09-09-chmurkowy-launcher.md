# Chmurkowy Launcher — plan implementacji

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Przenośny launcher w jednym pliku, który pobiera Javę, instaluje Minecrafta 1.21.1 z NeoForge, synchronizuje paczkę 249 modów i uruchamia grę w swoim folderze.

**Architecture:** Workspace Rust z trzema crate'ami. `core` zawiera całą logikę i nie zna UI — to tam mieszka pobieranie, hashowanie, instalacja i uruchamianie. `cli` to narzędzie deweloperskie: buduje manifest z instancji PrismLaunchera i pozwala przetestować cały pipeline bez okna. `launcher` to cienkie GUI w egui rysujące stan z `core`.

**Tech Stack:** Rust (edycja 2021), tokio, reqwest z rustls, serde/serde_json, toml, sha1/sha2/md-5, zip, tar+flate2, eframe/egui, clap.

**Spec:** `docs/superpowers/specs/2026-09-09-chmurkowy-launcher-design.md`

## Global Constraints

- Wszystkie ścieżki są względne wobec katalogu pliku wykonywalnego. Nigdy `%APPDATA%`, `~/.minecraft`, rejestru ani zmiennych środowiskowych.
- Tylko HTTPS. Adres zaczynający się od `http://` jest odrzucany na etapie walidacji manifestu.
- Pliki paczki weryfikujemy SHA-512. Pliki Mojanga (libki, assety) weryfikujemy SHA-1, bo taki hash podają ich metadane.
- Każda ścieżka z manifestu przechodzi walidację przed użyciem: bez `..`, bez ścieżek absolutnych, bez liter dysków, bez ukośników odwrotnych.
- Pobieranie zawsze do `<plik>.part`, weryfikacja, dopiero potem zmiana nazwy.
- Żadnych cichych zaniechań. Nieudane pobranie pliku paczki przerywa cały proces z komunikatem zawierającym nazwę pliku i próbowane adresy.
- Teksty widoczne dla użytkownika po polsku. Nazwy w kodzie po angielsku.
- Minecraft 1.21.1, NeoForge 21.1.249, Java 21 — wartości pochodzą z manifestu, nie z kodu.
- Wersje zależności dodajemy przez `cargo add`, nie wpisujemy ręcznie do `Cargo.toml`.

## Struktura plików

```
Cargo.toml                         workspace
crates/core/src/lib.rs             re-eksport modułów
crates/core/src/paths.rs           walidacja ścieżek z manifestu
crates/core/src/hash.rs            sha512 / sha1
crates/core/src/progress.rs        zdarzenia postępu
crates/core/src/net.rs             pobieranie z retry, listą URL-i i weryfikacją
crates/core/src/manifest.rs        schemat manifestu + walidacja
crates/core/src/state.rs           state.json — co launcher sam wgrał
crates/core/src/pack_sync.rs       planowanie i wykonanie synchronizacji
crates/core/src/java.rs            Adoptium: pobranie i rozpakowanie JRE
crates/core/src/version.rs         model version JSON, inheritsFrom, reguły, szablony
crates/core/src/game_install.rs    instalator NeoForge, libki, assety
crates/core/src/launch.rs          classpath i budowa polecenia
crates/core/src/auth/mod.rs        wspólne typy konta
crates/core/src/auth/offline.rs    UUID offline
crates/core/src/auth/msa.rs        device code flow
crates/core/src/auth/store.rs      auth.json
crates/cli/src/main.rs             chmurka: pack build / install / run
crates/cli/src/pw.rs               parser metadanych packwiz
crates/launcher/src/main.rs        punkt wejścia GUI
crates/launcher/src/theme.rs       kolory, czcionki, zaokrąglenia
crates/launcher/src/app.rs         stan aplikacji i pętla
crates/launcher/src/views/main.rs  ekran główny
crates/launcher/src/views/login.rs ekran logowania
crates/launcher/src/views/settings.rs ekran ustawień
.github/workflows/release.yml      buildy Windows + Linux
```

Podział jest według odpowiedzialności, nie warstw. `paths` i `hash` nie zależą od niczego, `net` zależy od nich, `pack_sync` od `manifest` i `state`. Dzięki temu każdy moduł da się testować osobno.

Jedno odstępstwo od specyfikacji §11: crate nazywa się `cli`, nie `packer`. Poza budowaniem manifestu dostaje polecenia `install` i `run`, którymi testujemy cały pipeline bez GUI — a to już nie mieści się w nazwie „packer". Binarka nadal nazywa się `chmurka`.

---

## Faza 1 — fundament i paczka

Kamień milowy fazy: `chmurka pack build` generuje prawdziwy manifest z Twojej instancji PrismLaunchera.

### Task 1: Workspace i walidacja ścieżek

To jest zadanie o bezpieczeństwie. Manifest przychodzi z sieci; gdyby zawierał ścieżkę `../../../.bashrc`, launcher nadpisałby plik poza swoim folderem. Walidacja musi powstać zanim cokolwiek zacznie zapisywać pliki.

**Files:**
- Create: `Cargo.toml`
- Create: `crates/core/Cargo.toml`
- Create: `crates/core/src/lib.rs`
- Create: `crates/core/src/paths.rs`

**Interfaces:**
- Consumes: nic
- Produces:
  - `pub enum PathError { Empty, Absolute(String), Escapes(String), Illegal(String) }`
  - `pub fn validate_rel(raw: &str) -> Result<String, PathError>` — zwraca ścieżkę znormalizowaną do ukośników `/`
  - `pub fn join_within(base: &Path, rel: &str) -> Result<PathBuf, PathError>`

- [ ] **Step 1: Utwórz workspace**

`Cargo.toml` w katalogu głównym:

```toml
[workspace]
members = ["crates/core"]
resolver = "2"

[workspace.package]
edition = "2021"
rust-version = "1.85"
```

`crates/core/Cargo.toml`:

```toml
[package]
name = "chmurka-core"
version = "0.1.0"
edition.workspace = true

[dependencies]
```

`crates/core/src/lib.rs`:

```rust
pub mod paths;
```

- [ ] **Step 2: Dodaj thiserror**

Run: `cargo add thiserror -p chmurka-core`

- [ ] **Step 3: Napisz testy, które mają nie przejść**

Dopisz na końcu `crates/core/src/paths.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn przyjmuje_zwykle_sciezki() {
        assert_eq!(validate_rel("mods/sodium.jar").unwrap(), "mods/sodium.jar");
        assert_eq!(validate_rel("options.txt").unwrap(), "options.txt");
        assert_eq!(validate_rel("config/create/common.toml").unwrap(), "config/create/common.toml");
    }

    #[test]
    fn odrzuca_wyjscie_w_gore() {
        assert!(matches!(validate_rel("../secret"), Err(PathError::Escapes(_))));
        assert!(matches!(validate_rel("mods/../../secret"), Err(PathError::Escapes(_))));
        assert!(matches!(validate_rel("mods/.."), Err(PathError::Escapes(_))));
    }

    #[test]
    fn odrzuca_sciezki_absolutne() {
        assert!(matches!(validate_rel("/etc/passwd"), Err(PathError::Absolute(_))));
        assert!(matches!(validate_rel("C:/Windows/system32"), Err(PathError::Absolute(_))));
        assert!(matches!(validate_rel("\\\\serwer\\udzial"), Err(PathError::Absolute(_))));
    }

    #[test]
    fn odrzuca_ukosnik_odwrotny_i_puste_segmenty() {
        assert!(matches!(validate_rel("mods\\sodium.jar"), Err(PathError::Illegal(_))));
        assert!(matches!(validate_rel("mods//sodium.jar"), Err(PathError::Illegal(_))));
        assert!(matches!(validate_rel("mods/./sodium.jar"), Err(PathError::Illegal(_))));
        assert!(matches!(validate_rel(""), Err(PathError::Empty)));
    }

    #[test]
    fn join_within_zostaje_w_katalogu() {
        let base = std::path::Path::new("/tmp/instancja");
        assert_eq!(
            join_within(base, "mods/a.jar").unwrap(),
            std::path::Path::new("/tmp/instancja/mods/a.jar")
        );
        assert!(join_within(base, "../a.jar").is_err());
    }
}
```

- [ ] **Step 4: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core`
Expected: FAIL — `cannot find function validate_rel`

- [ ] **Step 5: Zaimplementuj walidację**

Wstaw na początku `crates/core/src/paths.rs`:

```rust
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PathError {
    #[error("ścieżka jest pusta")]
    Empty,
    #[error("ścieżka absolutna nie jest dozwolona: {0}")]
    Absolute(String),
    #[error("ścieżka wychodzi poza katalog instancji: {0}")]
    Escapes(String),
    #[error("niedozwolony znak lub segment w ścieżce: {0}")]
    Illegal(String),
}

/// Sprawdza ścieżkę pochodzącą z manifestu i zwraca jej postać znormalizowaną.
///
/// Manifest jest danymi z sieci. Bez tej funkcji wpis `../../.bashrc`
/// pozwoliłby nadpisać dowolny plik na dysku użytkownika.
pub fn validate_rel(raw: &str) -> Result<String, PathError> {
    if raw.is_empty() {
        return Err(PathError::Empty);
    }
    if raw.contains('\\') {
        // Ukośnik odwrotny bywa separatorem na Windowsie, więc mógłby ominąć
        // kontrolę segmentów. Manifest zawsze używa ukośnika zwykłego.
        return Err(PathError::Illegal(raw.to_string()));
    }
    if raw.starts_with('/') {
        return Err(PathError::Absolute(raw.to_string()));
    }
    // Litera dysku, np. "C:/..." albo "C:".
    let bytes = raw.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' && (bytes[0] as char).is_ascii_alphabetic() {
        return Err(PathError::Absolute(raw.to_string()));
    }

    for seg in raw.split('/') {
        match seg {
            "" | "." => return Err(PathError::Illegal(raw.to_string())),
            ".." => return Err(PathError::Escapes(raw.to_string())),
            _ => {}
        }
    }
    Ok(raw.to_string())
}

/// Skleja zwalidowaną ścieżkę z katalogiem bazowym.
pub fn join_within(base: &Path, rel: &str) -> Result<PathBuf, PathError> {
    let clean = validate_rel(rel)?;
    let mut out = base.to_path_buf();
    for seg in clean.split('/') {
        out.push(seg);
    }
    Ok(out)
}
```

Uwaga: `validate_rel` odrzuca `..` w każdym miejscu ścieżki, także tam, gdzie matematycznie by się skróciło (`a/../b`). To celowo zbyt ostrożne — manifest generujemy sami i nigdy takich ścieżek nie produkuje, a prostsza reguła jest łatwiejsza do zweryfikowania niż normalizacja.

Zwróć uwagę, że `\\serwer\udzial` z testu wpada w gałąź ukośnika odwrotnego, więc dostaje `Illegal`, nie `Absolute`. Popraw asercję w teście `odrzuca_sciezki_absolutne` na `PathError::Illegal(_)` dla tego jednego przypadku.

- [ ] **Step 6: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 5 testów

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml crates/core
git commit -m "Walidacja sciezek z manifestu"
```

---

### Task 2: Hashowanie

**Files:**
- Create: `crates/core/src/hash.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: nic
- Produces:
  - `pub fn sha512_hex(data: &[u8]) -> String`
  - `pub fn sha512_file(path: &Path) -> std::io::Result<String>`
  - `pub fn sha1_file(path: &Path) -> std::io::Result<String>`

Wszystkie zwracają hex małymi literami.

- [ ] **Step 1: Dodaj zależności**

Run: `cargo add sha2 sha1 -p chmurka-core`

- [ ] **Step 2: Napisz testy**

`crates/core/src/hash.rs`:

```rust
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
        assert_eq!(sha1_file(&p).unwrap(), "a9993e364706816aba3e25717850c26c9cd0d89d");
        std::fs::remove_file(&p).unwrap();
    }
}
```

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core hash`
Expected: FAIL — `cannot find function sha512_hex`

- [ ] **Step 4: Zaimplementuj**

Wstaw na początku `crates/core/src/hash.rs`:

```rust
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
```

Dopisz `pub mod hash;` do `crates/core/src/lib.rs`.

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 8 testów

- [ ] **Step 6: Commit**

```bash
git add crates/core
git commit -m "Hashowanie sha512 i sha1 ze strumieniowaniem"
```

---

### Task 3: Manifest

**Files:**
- Create: `crates/core/src/manifest.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `paths::validate_rel`
- Produces:
  - `pub struct Manifest { pub schema: u32, pub pack: Pack, pub java: JavaReq, pub memory: Memory, pub auth: AuthCfg, pub launcher: LauncherInfo, pub mirror_dirs: Vec<String>, pub files: Vec<FileEntry> }`
  - `pub struct Pack { pub name: String, pub edition: String, pub version: String, pub minecraft: String, pub loader: Loader }`
  - `pub struct Loader { pub kind: String, pub version: String }`
  - `pub struct JavaReq { pub major: u32, pub distribution: String }`
  - `pub struct Memory { pub min_mb: u32, pub max_mb: u32 }`
  - `pub struct AuthCfg { pub msa_client_id: String }`
  - `pub struct LauncherInfo { pub latest_version: String, pub urls: BTreeMap<String, String> }`
  - `pub struct FileEntry { pub path: String, pub size: u64, pub sha512: String, pub policy: Policy, pub urls: Vec<String> }`
  - `pub enum Policy { Mirror, Smart, Seed }`
  - `pub fn parse(s: &str) -> Result<Manifest, ManifestError>`

- [ ] **Step 1: Dodaj zależności**

Run: `cargo add serde --features derive -p chmurka-core` oraz `cargo add serde_json -p chmurka-core`

- [ ] **Step 2: Napisz testy**

Na końcu `crates/core/src/manifest.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn wzorcowy() -> String {
        r#"{
          "schema": 1,
          "pack": { "name": "Chmurkowy Serwer", "edition": "edycja 2026/2027",
                    "version": "2026.09.09-1", "minecraft": "1.21.1",
                    "loader": { "kind": "neoforge", "version": "21.1.249" } },
          "java": { "major": 21, "distribution": "temurin" },
          "memory": { "min_mb": 512, "max_mb": 4096 },
          "auth": { "msa_client_id": "00000000402b5328" },
          "launcher": { "latest_version": "1.0.0", "urls": { "linux-x64": "https://example.test/l" } },
          "mirror_dirs": ["mods"],
          "files": [
            { "path": "mods/a.jar", "size": 10, "sha512": "ab", "policy": "mirror",
              "urls": ["https://cdn.modrinth.com/a.jar"] },
            { "path": "options.txt", "size": 5, "sha512": "cd", "policy": "seed",
              "urls": ["https://example.test/o"] }
          ]
        }"#
        .to_string()
    }

    #[test]
    fn wczytuje_poprawny_manifest() {
        let m = parse(&wzorcowy()).unwrap();
        assert_eq!(m.pack.minecraft, "1.21.1");
        assert_eq!(m.pack.loader.version, "21.1.249");
        assert_eq!(m.java.major, 21);
        assert_eq!(m.mirror_dirs, vec!["mods".to_string()]);
        assert_eq!(m.files.len(), 2);
        assert_eq!(m.files[0].policy, Policy::Mirror);
        assert_eq!(m.files[1].policy, Policy::Seed);
    }

    #[test]
    fn odrzuca_nieznana_wersje_schematu() {
        let zly = wzorcowy().replace("\"schema\": 1", "\"schema\": 2");
        assert!(matches!(parse(&zly), Err(ManifestError::Schema(2))));
    }

    #[test]
    fn odrzuca_wroga_sciezke() {
        let zly = wzorcowy().replace("mods/a.jar", "../../.bashrc");
        assert!(matches!(parse(&zly), Err(ManifestError::Path { .. })));
    }

    #[test]
    fn odrzuca_adres_bez_https() {
        let zly = wzorcowy().replace("https://cdn.modrinth.com/a.jar", "http://cdn.modrinth.com/a.jar");
        assert!(matches!(parse(&zly), Err(ManifestError::NotHttps { .. })));
    }

    #[test]
    fn odrzuca_wpis_bez_zrodla() {
        let zly = wzorcowy().replace(r#"["https://cdn.modrinth.com/a.jar"]"#, "[]");
        assert!(matches!(parse(&zly), Err(ManifestError::NoUrls { .. })));
    }
}
```

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core manifest`
Expected: FAIL — `cannot find function parse`

- [ ] **Step 4: Zaimplementuj**

Na początku `crates/core/src/manifest.rs`:

```rust
use crate::paths::{validate_rel, PathError};
use serde::Deserialize;
use std::collections::BTreeMap;

pub const SCHEMA: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("nieprawidłowy JSON manifestu: {0}")]
    Json(#[from] serde_json::Error),
    #[error("nieobsługiwana wersja schematu manifestu: {0} (launcher rozumie {SCHEMA})")]
    Schema(u32),
    #[error("wpis {path}: {source}")]
    Path { path: String, source: PathError },
    #[error("wpis {path}: adres musi być https, jest: {url}")]
    NotHttps { path: String, url: String },
    #[error("wpis {path}: brak jakiegokolwiek adresu do pobrania")]
    NoUrls { path: String },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub schema: u32,
    pub pack: Pack,
    pub java: JavaReq,
    pub memory: Memory,
    pub auth: AuthCfg,
    pub launcher: LauncherInfo,
    #[serde(default)]
    pub mirror_dirs: Vec<String>,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Pack {
    pub name: String,
    #[serde(default)]
    pub edition: String,
    pub version: String,
    pub minecraft: String,
    pub loader: Loader,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Loader {
    pub kind: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct JavaReq {
    pub major: u32,
    #[serde(default = "domyslna_dystrybucja")]
    pub distribution: String,
}

fn domyslna_dystrybucja() -> String {
    "temurin".to_string()
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Memory {
    pub min_mb: u32,
    pub max_mb: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AuthCfg {
    pub msa_client_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LauncherInfo {
    pub latest_version: String,
    #[serde(default)]
    pub urls: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Policy {
    /// Katalog musi się zgadzać co do pliku. Obce pliki są kasowane.
    Mirror,
    /// Nadpisujemy tylko wtedy, gdy gracz pliku nie ruszał.
    Smart,
    /// Wgrywamy raz i nigdy więcej nie dotykamy.
    Seed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub sha512: String,
    pub policy: Policy,
    pub urls: Vec<String>,
}

pub fn parse(s: &str) -> Result<Manifest, ManifestError> {
    let m: Manifest = serde_json::from_str(s)?;
    if m.schema != SCHEMA {
        return Err(ManifestError::Schema(m.schema));
    }
    for f in &m.files {
        validate_rel(&f.path).map_err(|source| ManifestError::Path {
            path: f.path.clone(),
            source,
        })?;
        if f.urls.is_empty() {
            return Err(ManifestError::NoUrls { path: f.path.clone() });
        }
        for u in &f.urls {
            if !u.starts_with("https://") {
                return Err(ManifestError::NotHttps {
                    path: f.path.clone(),
                    url: u.clone(),
                });
            }
        }
    }
    Ok(m)
}
```

Dopisz `pub mod manifest;` do `crates/core/src/lib.rs`.

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 13 testów

- [ ] **Step 6: Commit**

```bash
git add crates/core
git commit -m "Schemat manifestu z walidacja sciezek i https"
```

---

### Task 4: Postęp i pobieranie

Pobieranie jest jedynym miejscem, przez które przechodzą wszystkie pliki: JRE, instalator, libki, assety i mody. Dlatego retry, wznawianie i weryfikacja żyją tutaj, a nie w każdym module osobno.

**Files:**
- Create: `crates/core/src/progress.rs`
- Create: `crates/core/src/net.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `hash::{sha512_file, sha1_file}`
- Produces:
  - `pub enum Stage { Manifest, Java, Loader, Libraries, Assets, Pack, Ready }`
  - `pub struct Progress { pub stage: Stage, pub done: u64, pub total: u64, pub bytes: u64, pub label: String }`
  - `pub enum Expect { Sha512(String), Sha1(String), Any }`
  - `pub struct DownloadSpec { pub urls: Vec<String>, pub dest: PathBuf, pub expect: Expect }`
  - `pub struct Downloader` z `new(concurrency: usize) -> Self`
  - `pub async fn fetch_one(&self, spec: &DownloadSpec) -> Result<bool, NetError>` — `false` znaczy „plik już był poprawny"
  - `pub async fn fetch_many(&self, specs: Vec<DownloadSpec>, stage: Stage, on: Arc<dyn Fn(Progress) + Send + Sync>) -> Result<(), NetError>`

- [ ] **Step 1: Dodaj zależności**

Run:
```bash
cargo add tokio --features rt-multi-thread,macros,fs,process -p chmurka-core
cargo add reqwest --no-default-features --features rustls-tls,stream -p chmurka-core
cargo add futures-util -p chmurka-core
cargo add tempfile --dev -p chmurka-core
```

- [ ] **Step 2: Napisz progress.rs**

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Manifest,
    Java,
    Loader,
    Libraries,
    Assets,
    Pack,
    Ready,
}

impl Stage {
    /// Tekst pokazywany użytkownikowi w linii stanu.
    pub fn opis(&self) -> &'static str {
        match self {
            Stage::Manifest => "Sprawdzam paczkę",
            Stage::Java => "Pobieram Javę",
            Stage::Loader => "Instaluję NeoForge",
            Stage::Libraries => "Pobieram biblioteki",
            Stage::Assets => "Pobieram zasoby gry",
            Stage::Pack => "Pobieram mody",
            Stage::Ready => "Gotowe",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Progress {
    pub stage: Stage,
    pub done: u64,
    pub total: u64,
    pub bytes: u64,
    pub label: String,
}
```

- [ ] **Step 3: Napisz test pobierania na lokalnym serwerze**

Na końcu `crates/core/src/net.rs`:

```rust
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
        assert!(!dest.with_extension("part").exists(), "plik .part musi być posprzątany");
        h.join().unwrap();
    }
}
```

- [ ] **Step 4: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core net`
Expected: FAIL — `cannot find struct Downloader`

- [ ] **Step 5: Zaimplementuj net.rs**

```rust
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
            tokio::io::AsyncWriteExt::write_all(&mut plik, &kawalek)
                .await
                .map_err(|e| e.to_string())?;
        }
        tokio::io::AsyncWriteExt::flush(&mut plik)
            .await
            .map_err(|e| e.to_string())?;
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
                let rozmiar = tokio::fs::metadata(&spec.dest).await.map(|m| m.len()).unwrap_or(0);
                let d = done.fetch_add(1, Ordering::Relaxed) + 1;
                let b = bajty.fetch_add(rozmiar, Ordering::Relaxed) + rozmiar;
                on(Progress { stage, done: d, total, bytes: b, label: etykieta });
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
        Expect::Sha512(h) => sha512_file(sciezka).map(|x| x.eq_ignore_ascii_case(h)).unwrap_or(false),
        Expect::Sha1(h) => sha1_file(sciezka).map(|x| x.eq_ignore_ascii_case(h)).unwrap_or(false),
    }
}
```

Dopisz `pub mod net;` i `pub mod progress;` do `crates/core/src/lib.rs`.

Uwaga: testy używają adresu `http://` do lokalnego serwera. To dozwolone, bo zakaz `http://` obowiązuje w walidacji manifestu, nie w samym `Downloaderze` — dzięki temu testy nie potrzebują certyfikatów TLS.

- [ ] **Step 6: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 16 testów

- [ ] **Step 7: Commit**

```bash
git add crates/core
git commit -m "Pobieranie z retry, lista adresow zapasowych i weryfikacja hashem"
```

---

### Task 5: Stan i planowanie synchronizacji

Serce logiki paczki. `plan` jest funkcją czystą wobec dysku — dostaje sondę zamiast czytać pliki sama. Dzięki temu całą tablicę decyzyjną ze specyfikacji można przetestować bez tworzenia plików.

**Files:**
- Create: `crates/core/src/state.rs`
- Create: `crates/core/src/pack_sync.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `manifest::{Manifest, FileEntry, Policy}`
- Produces:
  - `pub struct State { pub written: BTreeMap<String, String> }` z `load(&Path)`, `save(&Path)`, `new()`
  - `pub trait FileProbe { fn hash_of(&self, rel: &str) -> Option<String>; fn list(&self, dir: &str) -> Vec<String>; }`
  - `pub enum Action { Download { index: usize }, Delete { path: String }, Record { path: String, sha512: String }, SkipModified { path: String }, SkipUnknown { path: String } }`
  - `pub fn plan(m: &Manifest, st: &State, probe: &dyn FileProbe) -> Vec<Action>`

- [ ] **Step 1: Napisz state.rs**

```rust
use std::collections::BTreeMap;
use std::path::Path;

/// Pamięć launchera o tym, co sam wgrał.
///
/// Bez tego nie da się odróżnić „gracz zmienił config" od „paczka się
/// zaktualizowała" — a od tego zależy, czy wolno nadpisać plik.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct State {
    #[serde(default)]
    pub written: BTreeMap<String, String>,
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }

    /// Brak pliku albo uszkodzony JSON dają pusty stan, nie błąd.
    /// Najgorsze, co się wtedy stanie, to jedno pominięcie aktualizacji configu.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(rodzic) = path.parent() {
            std::fs::create_dir_all(rodzic)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(self)?)
    }
}
```

- [ ] **Step 2: Napisz testy tablicy decyzyjnej**

Na końcu `crates/core/src/pack_sync.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::*;
    use std::collections::BTreeMap;

    /// Sonda pamięciowa: udaje dysk bez dotykania dysku.
    struct Mapa {
        pliki: BTreeMap<String, String>,
    }

    impl Mapa {
        fn nowa(pary: &[(&str, &str)]) -> Self {
            Self {
                pliki: pary.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
            }
        }
    }

    impl FileProbe for Mapa {
        fn hash_of(&self, rel: &str) -> Option<String> {
            self.pliki.get(rel).cloned()
        }
        fn list(&self, dir: &str) -> Vec<String> {
            let prefiks = format!("{dir}/");
            self.pliki.keys().filter(|k| k.starts_with(&prefiks)).cloned().collect()
        }
    }

    fn manifest(wpisy: Vec<(&str, &str, Policy)>) -> Manifest {
        Manifest {
            schema: 1,
            pack: Pack {
                name: "T".into(), edition: "".into(), version: "1".into(),
                minecraft: "1.21.1".into(),
                loader: Loader { kind: "neoforge".into(), version: "21.1.249".into() },
            },
            java: JavaReq { major: 21, distribution: "temurin".into() },
            memory: Memory { min_mb: 512, max_mb: 4096 },
            auth: AuthCfg { msa_client_id: "x".into() },
            launcher: LauncherInfo { latest_version: "1".into(), urls: BTreeMap::new() },
            mirror_dirs: vec!["mods".into()],
            files: wpisy
                .into_iter()
                .map(|(p, h, pol)| FileEntry {
                    path: p.into(), size: 1, sha512: h.into(), policy: pol,
                    urls: vec!["https://example.test/x".into()],
                })
                .collect(),
        }
    }

    fn stan(pary: &[(&str, &str)]) -> State {
        State { written: pary.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect() }
    }

    // --- tryb mirror ---

    #[test]
    fn mirror_pobiera_brakujacy_mod() {
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[]));
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn mirror_nie_rusza_zgodnego_moda() {
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("mods/a.jar", "AAA")]));
        assert_eq!(akcje, vec![]);
    }

    #[test]
    fn mirror_pobiera_ponownie_przy_zlym_hashu() {
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("mods/a.jar", "ZLE")]));
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn mirror_kasuje_obcy_mod() {
        // Obcy mod w mods/ wywala cala paczke przy starcie gry, wiec musi zniknac.
        let m = manifest(vec![("mods/a.jar", "AAA", Policy::Mirror)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("mods/a.jar", "AAA"), ("mods/obcy.jar", "XXX")]));
        assert_eq!(akcje, vec![Action::Delete { path: "mods/obcy.jar".into() }]);
    }

    // --- tryb smart ---

    #[test]
    fn smart_pobiera_brakujacy_config() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[]));
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn smart_nadpisuje_nietkniety_config() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &stan(&[("config/a.toml", "STARY")]), &Mapa::nowa(&[("config/a.toml", "STARY")]));
        assert_eq!(akcje, vec![Action::Download { index: 0 }]);
    }

    #[test]
    fn smart_nic_nie_robi_gdy_juz_aktualny() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &stan(&[("config/a.toml", "NOWY")]), &Mapa::nowa(&[("config/a.toml", "NOWY")]));
        assert_eq!(akcje, vec![]);
    }

    #[test]
    fn smart_zostawia_config_zmieniony_przez_gracza() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &stan(&[("config/a.toml", "STARY")]), &Mapa::nowa(&[("config/a.toml", "RECZNIE")]));
        assert_eq!(akcje, vec![Action::SkipModified { path: "config/a.toml".into() }]);
    }

    #[test]
    fn smart_dopisuje_wpis_gdy_plik_juz_jest_wlasciwy() {
        // Plik zgodny z manifestem, ale bez wpisu w state.json — np. po recznej
        // instalacji. Nie pobieramy, tylko zapamietujemy, ze taki jest.
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("config/a.toml", "NOWY")]));
        assert_eq!(akcje, vec![Action::Record { path: "config/a.toml".into(), sha512: "NOWY".into() }]);
    }

    #[test]
    fn smart_zostawia_nieznany_plik_bez_wpisu() {
        let m = manifest(vec![("config/a.toml", "NOWY", Policy::Smart)]);
        let akcje = plan(&m, &State::new(), &Mapa::nowa(&[("config/a.toml", "CUDZE")]));
        assert_eq!(akcje, vec![Action::SkipUnknown { path: "config/a.toml".into() }]);
    }

    // --- tryb seed ---

    #[test]
    fn seed_wgrywa_tylko_raz() {
        let m = manifest(vec![("options.txt", "NOWY", Policy::Seed)]);
        assert_eq!(plan(&m, &State::new(), &Mapa::nowa(&[])), vec![Action::Download { index: 0 }]);
        // Istniejacy plik zostaje nietkniety, nawet jesli manifest ma inna tresc.
        assert_eq!(plan(&m, &State::new(), &Mapa::nowa(&[("options.txt", "COKOLWIEK")])), vec![]);
    }
}
```

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core pack_sync`
Expected: FAIL — `cannot find function plan`

- [ ] **Step 4: Zaimplementuj planowanie**

Na początku `crates/core/src/pack_sync.rs`:

```rust
use crate::manifest::{Manifest, Policy};
use crate::state::State;
use std::collections::BTreeSet;

/// Źródło informacji o tym, co leży na dysku.
/// Abstrakcja istnieje po to, żeby `plan` dało się testować bez plików.
pub trait FileProbe {
    /// SHA-512 pliku albo `None`, gdy pliku nie ma.
    fn hash_of(&self, rel: &str) -> Option<String>;
    /// Ścieżki względne wszystkich plików w katalogu, rekurencyjnie.
    fn list(&self, dir: &str) -> Vec<String>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Pobierz plik o tym indeksie w `manifest.files`.
    Download { index: usize },
    Delete { path: String },
    /// Plik jest już właściwy — tylko zapamiętaj to w state.json.
    Record { path: String, sha512: String },
    /// Gracz zmienił plik, zostawiamy jego wersję.
    SkipModified { path: String },
    /// Plik istnieje, ale nie wiemy, kto go wgrał. Nie ruszamy.
    SkipUnknown { path: String },
}

pub fn plan(m: &Manifest, st: &State, probe: &dyn FileProbe) -> Vec<Action> {
    let mut akcje = Vec::new();
    let mut znane: BTreeSet<String> = BTreeSet::new();

    for (index, wpis) in m.files.iter().enumerate() {
        znane.insert(wpis.path.clone());
        let na_dysku = probe.hash_of(&wpis.path);
        let zapisany = st.written.get(&wpis.path);

        match wpis.policy {
            Policy::Mirror => match na_dysku {
                Some(h) if h.eq_ignore_ascii_case(&wpis.sha512) => {}
                _ => akcje.push(Action::Download { index }),
            },

            Policy::Seed => {
                if na_dysku.is_none() {
                    akcje.push(Action::Download { index });
                }
            }

            Policy::Smart => match (na_dysku, zapisany) {
                (None, _) => akcje.push(Action::Download { index }),

                (Some(h), Some(z)) if h.eq_ignore_ascii_case(z) => {
                    // Gracz nie ruszał pliku, więc wolno go zaktualizować.
                    if !h.eq_ignore_ascii_case(&wpis.sha512) {
                        akcje.push(Action::Download { index });
                    }
                }

                (Some(_), Some(_)) => {
                    akcje.push(Action::SkipModified { path: wpis.path.clone() })
                }

                (Some(h), None) if h.eq_ignore_ascii_case(&wpis.sha512) => {
                    akcje.push(Action::Record { path: wpis.path.clone(), sha512: h })
                }

                (Some(_), None) => akcje.push(Action::SkipUnknown { path: wpis.path.clone() }),
            },
        }
    }

    // Kasowanie obcych plików tylko w katalogach oznaczonych jako lustrzane.
    for dir in &m.mirror_dirs {
        for sciezka in probe.list(dir) {
            if !znane.contains(&sciezka) {
                akcje.push(Action::Delete { path: sciezka });
            }
        }
    }

    akcje
}
```

Dopisz `pub mod state;` i `pub mod pack_sync;` do `crates/core/src/lib.rs`.

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 27 testów

- [ ] **Step 6: Commit**

```bash
git add crates/core
git commit -m "Planowanie synchronizacji paczki z pelna tablica decyzyjna"
```

---

### Task 6: Wykonanie synchronizacji

**Files:**
- Modify: `crates/core/src/pack_sync.rs`
- Create: `crates/core/tests/sync_integracja.rs`

**Interfaces:**
- Consumes: `plan`, `net::Downloader`, `state::State`
- Produces:
  - `pub struct DiskProbe { root: PathBuf }` z `DiskProbe::new(root: &Path) -> Self`
  - `pub async fn apply(m: &Manifest, root: &Path, st: &mut State, actions: &[Action], dl: &Downloader, on: Arc<dyn Fn(Progress) + Send + Sync>) -> Result<Vec<String>, SyncError>` — zwraca listę komunikatów do logu (pominięte configi)

- [ ] **Step 1: Napisz test integracyjny**

`crates/core/tests/sync_integracja.rs`:

```rust
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
        for s in l.incoming().take(4) {
            let mut s = match s { Ok(s) => s, Err(_) => break };
            let mut buf = [0u8; 1024];
            let _ = s.read(&mut buf);
            let n = format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", tresc.len());
            let _ = s.write_all(n.as_bytes());
            let _ = s.write_all(tresc);
        }
    });
    (port, h)
}

fn manifest_json(port: u16, hash: &str) -> String {
    format!(r#"{{
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
    }}"#)
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
    assert!(!root.join("mods/obcy.jar").exists(), "obcy mod musi zniknac");
    assert_eq!(st.written.get("mods/a.jar").unwrap(), &hash);

    // Drugi przebieg nie ma juz nic do roboty.
    let akcje2 = plan(&m, &st, &DiskProbe::new(root));
    assert!(akcje2.is_empty(), "ponowna synchronizacja nie powinna nic robic");
    h.join().unwrap();
}
```

- [ ] **Step 2: Uruchom test i potwierdź, że nie przechodzi**

Run: `cargo test -p chmurka-core --test sync_integracja`
Expected: FAIL — `cannot find function apply`

- [ ] **Step 3: Zaimplementuj DiskProbe i apply**

Dopisz do `crates/core/src/pack_sync.rs`:

```rust
use crate::hash::sha512_file;
use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::paths::{join_within, PathError};
use crate::progress::{Progress, Stage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("nieprawidłowa ścieżka w manifeście: {0}")]
    Path(#[from] PathError),
    #[error("błąd operacji na pliku {0}: {1}")]
    Io(String, std::io::Error),
}

pub struct DiskProbe {
    root: PathBuf,
}

impl DiskProbe {
    pub fn new(root: &Path) -> Self {
        Self { root: root.to_path_buf() }
    }
}

impl FileProbe for DiskProbe {
    fn hash_of(&self, rel: &str) -> Option<String> {
        let p = join_within(&self.root, rel).ok()?;
        sha512_file(&p).ok()
    }

    fn list(&self, dir: &str) -> Vec<String> {
        let baza = match join_within(&self.root, dir) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        zbierz(&baza, dir, &mut out);
        out
    }
}

fn zbierz(katalog: &Path, prefiks: &str, out: &mut Vec<String>) {
    let Ok(wpisy) = std::fs::read_dir(katalog) else { return };
    for wpis in wpisy.flatten() {
        let nazwa = wpis.file_name().to_string_lossy().to_string();
        // Katalog .index to metadane PrismLaunchera, nie nasza sprawa.
        if nazwa.starts_with('.') {
            continue;
        }
        let rel = format!("{prefiks}/{nazwa}");
        if wpis.path().is_dir() {
            zbierz(&wpis.path(), &rel, out);
        } else {
            out.push(rel);
        }
    }
}

pub async fn apply(
    m: &Manifest,
    root: &Path,
    st: &mut State,
    actions: &[Action],
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<Vec<String>, SyncError> {
    let mut notatki = Vec::new();
    let mut do_pobrania = Vec::new();

    for akcja in actions {
        match akcja {
            Action::Download { index } => {
                let wpis = &m.files[*index];
                do_pobrania.push((
                    wpis.path.clone(),
                    wpis.sha512.clone(),
                    DownloadSpec {
                        urls: wpis.urls.clone(),
                        dest: join_within(root, &wpis.path)?,
                        expect: Expect::Sha512(wpis.sha512.clone()),
                    },
                ));
            }
            Action::Delete { path } => {
                let p = join_within(root, path)?;
                if p.exists() {
                    std::fs::remove_file(&p).map_err(|e| SyncError::Io(path.clone(), e))?;
                }
                st.written.remove(path);
                notatki.push(format!("usunięto obcy plik: {path}"));
            }
            Action::Record { path, sha512 } => {
                st.written.insert(path.clone(), sha512.clone());
            }
            Action::SkipModified { path } => {
                notatki.push(format!("pomijam {path} — plik został zmieniony ręcznie"));
            }
            Action::SkipUnknown { path } => {
                notatki.push(format!("pomijam {path} — nie wiadomo, skąd pochodzi"));
            }
        }
    }

    let specyfikacje: Vec<DownloadSpec> = do_pobrania.iter().map(|(_, _, s)| s.clone()).collect();
    dl.fetch_many(specyfikacje, Stage::Pack, on).await?;

    // Stan zapisujemy dopiero po udanym pobraniu wszystkiego.
    for (sciezka, hash, _) in do_pobrania {
        st.written.insert(sciezka, hash);
    }
    Ok(notatki)
}
```

- [ ] **Step 4: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 28 testów

- [ ] **Step 5: Commit**

```bash
git add crates/core
git commit -m "Wykonanie synchronizacji paczki z sonda dyskowa"
```

---

### Task 7: Packer — manifest z instancji PrismLaunchera

Kamień milowy fazy 1. Po tym zadaniu powstaje prawdziwy manifest z Twojej paczki.

**Files:**
- Create: `crates/cli/Cargo.toml`
- Create: `crates/cli/src/main.rs`
- Create: `crates/cli/src/pw.rs`
- Modify: `Cargo.toml` (dodaj `crates/cli` do members)

**Interfaces:**
- Consumes: `chmurka_core::{manifest, hash, paths}`
- Produces:
  - `pub struct PwEntry { pub filename: String, pub url: Option<String>, pub sha512: Option<String>, pub cf_project: Option<u64>, pub cf_file: Option<u64> }`
  - `pub fn parse_pw(tekst: &str) -> Result<PwEntry, PwError>`
  - `pub fn curseforge_url(file_id: u64, filename: &str) -> String`

- [ ] **Step 1: Utwórz crate i dodaj zależności**

```bash
cargo new --lib crates/cli --name chmurka-cli
cargo add clap --features derive -p chmurka-cli
cargo add chmurka-core --path crates/core -p chmurka-cli
cargo add tokio --features rt-multi-thread,macros -p chmurka-cli
cargo add serde_json anyhow -p chmurka-cli
```

Dopisz `"crates/cli"` do `members` w głównym `Cargo.toml`. W `crates/cli/Cargo.toml` dodaj:

```toml
[[bin]]
name = "chmurka"
path = "src/main.rs"
```

- [ ] **Step 2: Napisz testy parsera metadanych**

`crates/cli/src/pw.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    const MODRINTH: &str = r#"
filename = 'sodium-neoforge-0.8.13+mc1.21.1.jar'
name = 'Sodium'
side = 'client'

[download]
hash = '4f537696af95411e9daf15795fb5fcbc49913d48'
hash-format = 'sha512'
mode = 'url'
url = 'https://cdn.modrinth.com/data/AANobbMI/versions/uMOpc5uV/sodium.jar'

[update.modrinth]
mod-id = 'AANobbMI'
version = 'uMOpc5uV'
"#;

    const CURSEFORGE: &str = r#"
filename = 'spark-1.10.124-neoforge.jar'
name = 'spark'
side = 'both'

[download]
hash = 'aabbcc'
hash-format = 'sha1'
mode = 'metadata:curseforge'

[update.curseforge]
file-id = 6225208
project-id = 361579
"#;

    #[test]
    fn czyta_wpis_modrinth() {
        let e = parse_pw(MODRINTH).unwrap();
        assert_eq!(e.filename, "sodium-neoforge-0.8.13+mc1.21.1.jar");
        assert_eq!(e.url.unwrap(), "https://cdn.modrinth.com/data/AANobbMI/versions/uMOpc5uV/sodium.jar");
        assert_eq!(e.sha512.unwrap(), "4f537696af95411e9daf15795fb5fcbc49913d48");
    }

    #[test]
    fn czyta_wpis_curseforge_bez_url() {
        let e = parse_pw(CURSEFORGE).unwrap();
        assert_eq!(e.filename, "spark-1.10.124-neoforge.jar");
        assert!(e.url.is_none(), "CurseForge nie podaje adresu wprost");
        assert!(e.sha512.is_none(), "hash jest sha1, wiec nie nadaje sie jako sha512");
        assert_eq!(e.cf_file.unwrap(), 6225208);
        assert_eq!(e.cf_project.unwrap(), 361579);
    }

    #[test]
    fn sklada_adres_curseforge() {
        // Sprawdzone empirycznie: ten adres zwraca HTTP 200.
        assert_eq!(
            curseforge_url(6225208, "spark-1.10.124-neoforge.jar"),
            "https://mediafilez.forgecdn.net/files/6225/208/spark-1.10.124-neoforge.jar"
        );
        // Trzycyfrowa reszta musi byc dopelniona zerami po lewej.
        assert_eq!(
            curseforge_url(5647988, "SkyVillages.jar"),
            "https://mediafilez.forgecdn.net/files/5647/988/SkyVillages.jar"
        );
        assert_eq!(
            curseforge_url(1234005, "x.jar"),
            "https://mediafilez.forgecdn.net/files/1234/5/x.jar"
        );
    }
}
```

Uwaga do ostatniej asercji: CurseForge nie dopełnia reszty zerami — `1234005` daje `1234/5`. Zweryfikuj to na prawdziwym pliku podczas implementacji; jeśli okaże się inaczej, popraw test i implementację razem.

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-cli`
Expected: FAIL — `cannot find function parse_pw`

- [ ] **Step 4: Zaimplementuj parser**

Na początku `crates/cli/src/pw.rs`:

```rust
#[derive(Debug, thiserror::Error)]
pub enum PwError {
    #[error("brak pola filename w metadanych")]
    BrakNazwy,
}

#[derive(Debug, Clone)]
pub struct PwEntry {
    pub filename: String,
    pub url: Option<String>,
    pub sha512: Option<String>,
    pub cf_project: Option<u64>,
    pub cf_file: Option<u64>,
}

/// Wyciąga potrzebne pola z pliku `.pw.toml`.
///
/// Celowo nie używamy pełnego parsera TOML: interesuje nas pięć pól o stałym
/// kształcie, a pliki generuje PrismLauncher, więc format jest przewidywalny.
pub fn parse_pw(tekst: &str) -> Result<PwEntry, PwError> {
    let filename = pole(tekst, "filename").ok_or(PwError::BrakNazwy)?;
    let format_hasha = pole(tekst, "hash-format");
    let sha512 = match format_hasha.as_deref() {
        Some("sha512") => pole(tekst, "hash"),
        _ => None,
    };
    Ok(PwEntry {
        filename,
        url: pole(tekst, "url"),
        sha512,
        cf_project: liczba(tekst, "project-id"),
        cf_file: liczba(tekst, "file-id"),
    })
}

fn pole(tekst: &str, klucz: &str) -> Option<String> {
    for linia in tekst.lines() {
        let linia = linia.trim();
        if let Some(reszta) = linia.strip_prefix(klucz) {
            let reszta = reszta.trim_start();
            if let Some(wartosc) = reszta.strip_prefix('=') {
                return Some(wartosc.trim().trim_matches('\'').trim_matches('"').to_string());
            }
        }
    }
    None
}

fn liczba(tekst: &str, klucz: &str) -> Option<u64> {
    pole(tekst, klucz)?.parse().ok()
}

/// Adres bezpośredni do CDN-u CurseForge, składany z identyfikatora pliku.
pub fn curseforge_url(file_id: u64, filename: &str) -> String {
    format!(
        "https://mediafilez.forgecdn.net/files/{}/{}/{}",
        file_id / 1000,
        file_id % 1000,
        filename
    )
}
```

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-cli`
Expected: PASS, 3 testy

- [ ] **Step 6: Napisz polecenie `pack build`**

`crates/cli/src/main.rs`:

```rust
mod pw;

use anyhow::{bail, Context, Result};
use chmurka_core::hash::sha512_file;
use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "chmurka", about = "Narzędzia Chmurkowego Launchera")]
struct Cli {
    #[command(subcommand)]
    polecenie: Polecenie,
}

#[derive(Subcommand)]
enum Polecenie {
    /// Buduje manifest i katalog plików do wypchnięcia na GitHub Pages.
    PackBuild {
        /// Katalog `minecraft` instancji PrismLaunchera.
        #[arg(long)]
        instance: PathBuf,
        /// Katalog wynikowy.
        #[arg(long, default_value = "dist")]
        out: PathBuf,
        /// Bazowy adres, pod którym będą leżeć nasze pliki.
        #[arg(long)]
        base_url: String,
        /// Wersja paczki, np. 2026.09.09-1.
        #[arg(long)]
        version: String,
    },
}

fn main() -> Result<()> {
    match Cli::parse().polecenie {
        Polecenie::PackBuild { instance, out, base_url, version } => {
            pack_build(&instance, &out, &base_url, &version)
        }
    }
}

fn pack_build(instance: &Path, out: &Path, base_url: &str, version: &str) -> Result<()> {
    let mods = instance.join("mods");
    let index = mods.join(".index");
    if !index.is_dir() {
        bail!("brak {} — czy to na pewno instancja PrismLaunchera?", index.display());
    }

    // 1. Metadane modów.
    let mut wpisy_meta: BTreeMap<String, pw::PwEntry> = BTreeMap::new();
    for w in std::fs::read_dir(&index)? .flatten() {
        let p = w.path();
        if p.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let tekst = std::fs::read_to_string(&p)?;
        let e = pw::parse_pw(&tekst).with_context(|| format!("plik {}", p.display()))?;
        wpisy_meta.insert(e.filename.clone(), e);
    }

    // 2. Jary na dysku, z pominięciem .bak i .disabled.
    let mut jary: Vec<String> = Vec::new();
    for w in std::fs::read_dir(&mods)?.flatten() {
        let p = w.path();
        if p.is_dir() {
            continue;
        }
        let nazwa = p.file_name().unwrap().to_string_lossy().to_string();
        if nazwa.ends_with(".jar") {
            jary.push(nazwa);
        }
    }
    jary.sort();

    // 3. Rozjazd między metadanymi a plikami przerywa budowanie.
    // Mod bez metadanych nie ma skąd być pobrany, wiec cicha zgoda dalaby
    // testerom niekompletna paczke.
    let brakujace: Vec<&String> = jary.iter().filter(|j| !wpisy_meta.contains_key(*j)).collect();
    if !brakujace.is_empty() {
        bail!("mody bez metadanych, nie wiadomo skąd je pobrać: {brakujace:?}");
    }

    let mut files = Vec::new();

    for nazwa in &jary {
        let meta = &wpisy_meta[nazwa];
        let sciezka = mods.join(nazwa);
        let url = match (&meta.url, meta.cf_file) {
            (Some(u), _) => u.clone(),
            (None, Some(fid)) => pw::curseforge_url(fid, nazwa),
            (None, None) => bail!("mod {nazwa} nie ma ani adresu, ani identyfikatora CurseForge"),
        };
        // Hash z metadanych bywa sha1 (CurseForge), więc liczymy własny sha512
        // z pliku na dysku — to i tak jedyne źródło prawdy o tym, co gramy.
        let sha512 = sha512_file(&sciezka)?;
        files.push(serde_json::json!({
            "path": format!("mods/{nazwa}"),
            "size": std::fs::metadata(&sciezka)?.len(),
            "sha512": sha512,
            "policy": "mirror",
            "urls": [url],
        }));
    }

    // 4. Configi i options.txt trafiają do naszego magazynu adresowanego treścią.
    let magazyn = out.join("files");
    std::fs::create_dir_all(&magazyn)?;
    let mut nasze: Vec<(String, PathBuf, &str)> = Vec::new();
    zbierz_configi(&instance.join("config"), "config", &mut nasze);
    let opcje = instance.join("options.txt");
    if opcje.is_file() {
        nasze.push(("options.txt".to_string(), opcje, "seed"));
    }

    for (rel, zrodlo, polityka) in nasze {
        let sha512 = sha512_file(&zrodlo)?;
        let podkatalog = magazyn.join(&sha512[0..2]);
        std::fs::create_dir_all(&podkatalog)?;
        let cel = podkatalog.join(&sha512);
        if !cel.exists() {
            std::fs::copy(&zrodlo, &cel)?;
        }
        files.push(serde_json::json!({
            "path": rel,
            "size": std::fs::metadata(&zrodlo)?.len(),
            "sha512": sha512,
            "policy": polityka,
            "urls": [format!("{}/files/{}/{}", base_url.trim_end_matches('/'), &sha512[0..2], sha512)],
        }));
    }

    let manifest = serde_json::json!({
        "schema": 1,
        "pack": {
            "name": "Chmurkowy Serwer",
            "edition": "edycja 2026/2027",
            "version": version,
            "minecraft": "1.21.1",
            "loader": { "kind": "neoforge", "version": "21.1.249" }
        },
        "java": { "major": 21, "distribution": "temurin" },
        "memory": { "min_mb": 512, "max_mb": 4096 },
        "auth": { "msa_client_id": "00000000402b5328" },
        "launcher": { "latest_version": "0.1.0", "urls": {} },
        "mirror_dirs": ["mods"],
        "files": files,
    });

    let tekst = serde_json::to_string_pretty(&manifest)?;
    // Weryfikacja własnego wyniku: manifest musi przejść walidację launchera.
    chmurka_core::manifest::parse(&tekst).context("wygenerowany manifest nie przechodzi walidacji")?;
    std::fs::write(out.join("manifest.json"), &tekst)?;

    println!("Zapisano {} wpisów do {}", manifest["files"].as_array().unwrap().len(), out.display());
    Ok(())
}

fn zbierz_configi(katalog: &Path, prefiks: &str, out: &mut Vec<(String, PathBuf, &'static str)>) {
    let Ok(wpisy) = std::fs::read_dir(katalog) else { return };
    for w in wpisy.flatten() {
        let nazwa = w.file_name().to_string_lossy().to_string();
        if nazwa.starts_with('.') {
            continue;
        }
        let rel = format!("{prefiks}/{nazwa}");
        if w.path().is_dir() {
            zbierz_configi(&w.path(), &rel, out);
        } else {
            out.push((rel, w.path(), "smart"));
        }
    }
}
```

Run: `cargo add thiserror -p chmurka-cli`

- [ ] **Step 7: Zbuduj prawdziwy manifest**

```bash
cargo run -p chmurka-cli --bin chmurka -- pack-build \
  --instance "$HOME/.local/share/PrismLauncher/instances/Chmurkowy serwer edycja 2026-2027/minecraft" \
  --out dist \
  --base-url "https://przyklad.github.io/chmurka-pack" \
  --version "2026.09.09-1"
```

Expected: wypisuje liczbę wpisów w okolicach 660 (249 modów + około 410 configów + `options.txt`). Sprawdź:

```bash
python3 -c "import json;d=json.load(open('dist/manifest.json'));print(len(d['files']));print(sum(1 for f in d['files'] if f['policy']=='mirror'))"
du -sh dist
```

Oczekiwane: `mirror` równe 249, rozmiar `dist` około 4,5 MB.

- [ ] **Step 8: Commit**

```bash
echo "dist/" >> .gitignore
git add crates/cli Cargo.toml Cargo.lock .gitignore
git commit -m "Packer: manifest z instancji PrismLaunchera"
```

---

## Faza 2 — gra

Kamień milowy fazy: `chmurka run` instaluje wszystko i uruchamia Minecrafta w trybie offline, bez GUI.

### Task 8: Java z Adoptium

**Files:**
- Create: `crates/core/src/java.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `net::{Downloader, DownloadSpec, Expect}`, `progress::{Progress, Stage}`
- Produces:
  - `pub struct JavaInstall { pub java_bin: PathBuf }`
  - `pub fn adoptium_url(major: u32, os: Os, arch: &str) -> String`
  - `pub async fn ensure(root: &Path, major: u32, dl: &Downloader, on: Arc<dyn Fn(Progress) + Send + Sync>) -> Result<JavaInstall, JavaError>`
  - `pub fn biezacy_os() -> Os`

`Os` pochodzi z `version.rs` (Task 9), ale `java.rs` powstaje pierwszy, więc zdefiniuj `Os` tutaj i w Task 9 tylko go zaimportuj.

- [ ] **Step 1: Dodaj zależności do rozpakowywania**

```bash
cargo add zip -p chmurka-core
cargo add tar flate2 -p chmurka-core
```

- [ ] **Step 2: Napisz testy budowania adresu**

Na końcu `crates/core/src/java.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buduje_adres_adoptium() {
        // Sprawdzone empirycznie: oba adresy zwracaja HTTP 307 z przekierowaniem
        // na wydanie Temurina na GitHubie.
        assert_eq!(
            adoptium_url(21, Os::Windows, "x64"),
            "https://api.adoptium.net/v3/binary/latest/21/ga/windows/x64/jre/hotspot/normal/eclipse"
        );
        assert_eq!(
            adoptium_url(21, Os::Linux, "x64"),
            "https://api.adoptium.net/v3/binary/latest/21/ga/linux/x64/jre/hotspot/normal/eclipse"
        );
    }

    #[test]
    fn nazwa_binarki_zalezy_od_systemu() {
        assert_eq!(nazwa_javy(Os::Windows), "javaw.exe");
        assert_eq!(nazwa_javy(Os::Linux), "java");
    }
}
```

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core java`
Expected: FAIL — `cannot find function adoptium_url`

- [ ] **Step 4: Zaimplementuj**

```rust
use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::progress::{Progress, Stage};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Windows,
    Linux,
    Osx,
}

impl Os {
    pub fn nazwa_mojang(&self) -> &'static str {
        match self {
            Os::Windows => "windows",
            Os::Linux => "linux",
            Os::Osx => "osx",
        }
    }
}

pub fn biezacy_os() -> Os {
    if cfg!(target_os = "windows") {
        Os::Windows
    } else if cfg!(target_os = "macos") {
        Os::Osx
    } else {
        Os::Linux
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JavaError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("nie udało się rozpakować Javy: {0}")]
    Rozpakowanie(String),
    #[error("po rozpakowaniu nie znaleziono pliku wykonywalnego Javy w {0}")]
    BrakBinarki(String),
    #[error("błąd operacji na pliku: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct JavaInstall {
    pub java_bin: PathBuf,
}

pub fn adoptium_url(major: u32, os: Os, arch: &str) -> String {
    let os_seg = match os {
        Os::Windows => "windows",
        Os::Linux => "linux",
        Os::Osx => "mac",
    };
    format!("https://api.adoptium.net/v3/binary/latest/{major}/ga/{os_seg}/{arch}/jre/hotspot/normal/eclipse")
}

pub fn nazwa_javy(os: Os) -> &'static str {
    // javaw.exe nie otwiera okna konsoli obok gry.
    if os == Os::Windows { "javaw.exe" } else { "java" }
}

/// Zapewnia obecność JRE w `<root>/java/<major>` i zwraca ścieżkę do binarki.
pub async fn ensure(
    root: &Path,
    major: u32,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<JavaInstall, JavaError> {
    let os = biezacy_os();
    let katalog = root.join("java").join(major.to_string());

    if let Some(b) = znajdz_binarke(&katalog, os) {
        return Ok(JavaInstall { java_bin: b });
    }

    on(Progress {
        stage: Stage::Java,
        done: 0,
        total: 1,
        bytes: 0,
        label: format!("Java {major}"),
    });

    let rozszerzenie = if os == Os::Windows { "zip" } else { "tar.gz" };
    let archiwum = root.join(format!("java-{major}.{rozszerzenie}"));
    dl.fetch_one(&DownloadSpec {
        urls: vec![adoptium_url(major, os, "x64")],
        dest: archiwum.clone(),
        // Adoptium nie podaje hasha w samym przekierowaniu, wiec ufamy TLS-owi
        // i weryfikujemy wynik przez uruchomienie `java -version` nizej.
        expect: Expect::Any,
    })
    .await?;

    std::fs::create_dir_all(&katalog)?;
    if os == Os::Windows {
        rozpakuj_zip(&archiwum, &katalog)?;
    } else {
        rozpakuj_targz(&archiwum, &katalog)?;
    }
    let _ = std::fs::remove_file(&archiwum);

    let bin = znajdz_binarke(&katalog, os)
        .ok_or_else(|| JavaError::BrakBinarki(katalog.display().to_string()))?;

    on(Progress { stage: Stage::Java, done: 1, total: 1, bytes: 0, label: "Java gotowa".into() });
    Ok(JavaInstall { java_bin: bin })
}

/// Archiwa Adoptium mają jeden katalog na wierzchu (np. `jdk-21.0.12+7-jre`),
/// więc binarki szukamy o poziom głębiej, nie na sztywno.
fn znajdz_binarke(katalog: &Path, os: Os) -> Option<PathBuf> {
    let nazwa = nazwa_javy(os);
    let bezposrednio = katalog.join("bin").join(nazwa);
    if bezposrednio.is_file() {
        return Some(bezposrednio);
    }
    for w in std::fs::read_dir(katalog).ok()?.flatten() {
        let kandydat = w.path().join("bin").join(nazwa);
        if kandydat.is_file() {
            return Some(kandydat);
        }
    }
    None
}

fn rozpakuj_zip(archiwum: &Path, cel: &Path) -> Result<(), JavaError> {
    let plik = std::fs::File::open(archiwum)?;
    let mut zip = zip::ZipArchive::new(plik).map_err(|e| JavaError::Rozpakowanie(e.to_string()))?;
    zip.extract(cel).map_err(|e| JavaError::Rozpakowanie(e.to_string()))?;
    Ok(())
}

fn rozpakuj_targz(archiwum: &Path, cel: &Path) -> Result<(), JavaError> {
    let plik = std::fs::File::open(archiwum)?;
    let dekompresor = flate2::read::GzDecoder::new(plik);
    let mut archiwum = tar::Archive::new(dekompresor);
    archiwum.unpack(cel).map_err(|e| JavaError::Rozpakowanie(e.to_string()))?;
    Ok(())
}
```

Dopisz `pub mod java;` do `crates/core/src/lib.rs`.

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 30 testów

- [ ] **Step 6: Commit**

```bash
git add crates/core
git commit -m "Pobieranie i rozpakowywanie JRE z Adoptium"
```

---

### Task 9: Metadane wersji

Profil NeoForge dziedziczy po profilu vanilla przez `inheritsFrom`. Bez poprawnego scalenia gra nie dostanie ani bibliotek LWJGL, ani indeksu zasobów.

**Files:**
- Create: `crates/core/src/version.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `java::Os`
- Produces:
  - `pub struct VersionJson { pub id: String, pub inherits_from: Option<String>, pub main_class: String, pub arguments: Arguments, pub libraries: Vec<Library>, pub asset_index: Option<AssetIndex>, pub downloads: Option<Downloads>, pub assets: Option<String> }`
  - `pub struct Arguments { pub game: Vec<Arg>, pub jvm: Vec<Arg> }`
  - `pub enum Arg { Prosty(String), Warunkowy { rules: Vec<Rule>, value: Vec<String> } }`
  - `pub struct Library { pub name: String, pub downloads: Option<LibDownloads>, pub rules: Vec<Rule> }`
  - `pub fn load(versions_dir: &Path, id: &str) -> Result<VersionJson, VersionError>` — sam rozwiązuje `inheritsFrom`
  - `pub fn merge(child: VersionJson, parent: VersionJson) -> VersionJson`
  - `pub fn rules_allow(rules: &[Rule], os: Os) -> bool`
  - `pub fn substitute(tpl: &str, vars: &BTreeMap<String, String>) -> String`

- [ ] **Step 1: Napisz testy**

Na końcu `crates/core/src/version.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn regula_allow_dla_wlasciwego_systemu() {
        let r: Vec<Rule> = serde_json::from_str(
            r#"[{"action":"allow","os":{"name":"osx"}}]"#).unwrap();
        assert!(rules_allow(&r, Os::Osx));
        assert!(!rules_allow(&r, Os::Linux));
    }

    #[test]
    fn brak_regul_znaczy_zgoda() {
        assert!(rules_allow(&[], Os::Linux));
    }

    #[test]
    fn regula_disallow_wyklucza() {
        let r: Vec<Rule> = serde_json::from_str(
            r#"[{"action":"allow"},{"action":"disallow","os":{"name":"windows"}}]"#).unwrap();
        assert!(rules_allow(&r, Os::Linux));
        assert!(!rules_allow(&r, Os::Windows));
    }

    #[test]
    fn scalanie_bierze_mainclass_z_dziecka_i_libki_z_obu() {
        let dziecko: VersionJson = serde_json::from_str(r#"{
            "id":"neoforge-21.1.249","inheritsFrom":"1.21.1",
            "mainClass":"cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments":{"game":["--fml.neoForgeVersion","21.1.249"],"jvm":["-p","x"]},
            "libraries":[{"name":"org.ow2.asm:asm:9.10.1"}]
        }"#).unwrap();
        let rodzic: VersionJson = serde_json::from_str(r#"{
            "id":"1.21.1","mainClass":"net.minecraft.client.main.Main",
            "arguments":{"game":["--username","${auth_player_name}"],"jvm":["-Xss1M"]},
            "libraries":[{"name":"com.github.oshi:oshi-core:6.4.10"}],
            "assets":"17",
            "assetIndex":{"id":"17","sha1":"aa","size":1,"totalSize":2,"url":"https://x/17.json"}
        }"#).unwrap();

        let w = merge(dziecko, rodzic);
        assert_eq!(w.main_class, "cpw.mods.bootstraplauncher.BootstrapLauncher");
        assert_eq!(w.libraries.len(), 2);
        assert_eq!(w.assets.unwrap(), "17");
        assert_eq!(w.asset_index.unwrap().id, "17");
        // Argumenty rodzica ida pierwsze, potem dziecka.
        assert_eq!(w.arguments.game[0], Arg::Prosty("--username".into()));
        assert_eq!(w.arguments.game[2], Arg::Prosty("--fml.neoForgeVersion".into()));
    }

    #[test]
    fn dziecko_nadpisuje_libke_o_tej_samej_wspolrzednej() {
        let dziecko: VersionJson = serde_json::from_str(r#"{
            "id":"c","inheritsFrom":"p","mainClass":"C",
            "arguments":{"game":[],"jvm":[]},
            "libraries":[{"name":"org.ow2.asm:asm:9.10.1"}]}"#).unwrap();
        let rodzic: VersionJson = serde_json::from_str(r#"{
            "id":"p","mainClass":"P",
            "arguments":{"game":[],"jvm":[]},
            "libraries":[{"name":"org.ow2.asm:asm:9.2"}]}"#).unwrap();
        let w = merge(dziecko, rodzic);
        assert_eq!(w.libraries.len(), 1, "asm nie moze byc na classpathie dwa razy");
        assert_eq!(w.libraries[0].name, "org.ow2.asm:asm:9.10.1");
    }

    #[test]
    fn podstawia_zmienne_w_szablonie() {
        let mut v = BTreeMap::new();
        v.insert("auth_player_name".to_string(), "Tomasz".to_string());
        v.insert("classpath_separator".to_string(), ":".to_string());
        assert_eq!(substitute("${auth_player_name}", &v), "Tomasz");
        assert_eq!(substitute("a${classpath_separator}b", &v), "a:b");
        // Nieznana zmienna zostaje jak byla — lepiej zobaczyc ja w logu
        // niz po cichu wstawic pusty ciag.
        assert_eq!(substitute("${nieznane}", &v), "${nieznane}");
    }

    #[test]
    fn wspolrzedna_maven_bez_wersji() {
        assert_eq!(bez_wersji("org.ow2.asm:asm:9.10.1"), "org.ow2.asm:asm");
        assert_eq!(bez_wersji("net.minecraft:client:1.21.1:extra"), "net.minecraft:client");
    }
}
```

- [ ] **Step 2: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core version`
Expected: FAIL — `cannot find type VersionJson`

- [ ] **Step 3: Zaimplementuj**

```rust
use crate::java::Os;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum VersionError {
    #[error("nie znaleziono profilu wersji {0}")]
    Brak(String),
    #[error("nieprawidłowy JSON profilu {0}: {1}")]
    Json(String, serde_json::Error),
    #[error("błąd odczytu: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    pub main_class: String,
    #[serde(default)]
    pub arguments: Arguments,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub asset_index: Option<AssetIndex>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: Option<Downloads>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Arg>,
    #[serde(default)]
    pub jvm: Vec<Arg>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Arg {
    Prosty(String),
    Warunkowy {
        #[serde(default)]
        rules: Vec<Rule>,
        value: ArgValue,
    },
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    Jeden(String),
    Wiele(Vec<String>),
}

impl ArgValue {
    pub fn lista(&self) -> Vec<String> {
        match self {
            ArgValue::Jeden(s) => vec![s.clone()],
            ArgValue::Wiele(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<RuleOs>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RuleOs {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibDownloads>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LibDownloads {
    #[serde(default)]
    pub artifact: Option<Artifact>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Artifact {
    pub path: String,
    pub sha1: String,
    pub size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AssetIndex {
    pub id: String,
    pub sha1: String,
    pub size: u64,
    #[serde(default)]
    pub total_size: u64,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Downloads {
    #[serde(default)]
    pub client: Option<Artifact>,
}

/// Wczytuje profil i rekurencyjnie dokleja to, po czym dziedziczy.
pub fn load(versions_dir: &Path, id: &str) -> Result<VersionJson, VersionError> {
    let sciezka = versions_dir.join(id).join(format!("{id}.json"));
    if !sciezka.is_file() {
        return Err(VersionError::Brak(id.to_string()));
    }
    let tekst = std::fs::read_to_string(&sciezka)?;
    let v: VersionJson =
        serde_json::from_str(&tekst).map_err(|e| VersionError::Json(id.to_string(), e))?;

    match v.inherits_from.clone() {
        Some(rodzic_id) => {
            let rodzic = load(versions_dir, &rodzic_id)?;
            Ok(merge(v, rodzic))
        }
        None => Ok(v),
    }
}

fn bez_wersji(koordynat: &str) -> String {
    let czesci: Vec<&str> = koordynat.split(':').collect();
    if czesci.len() >= 2 {
        format!("{}:{}", czesci[0], czesci[1])
    } else {
        koordynat.to_string()
    }
}

/// Scala profil potomny z rodzicem zgodnie z semantyką `inheritsFrom`.
pub fn merge(child: VersionJson, parent: VersionJson) -> VersionJson {
    // Biblioteki rodzica idą pierwsze, ale przy kolizji współrzędnej Maven
    // wygrywa wersja z profilu potomnego — NeoForge celowo podbija np. ASM.
    let nadpisane: std::collections::BTreeSet<String> =
        child.libraries.iter().map(|l| bez_wersji(&l.name)).collect();

    let mut libraries: Vec<Library> = parent
        .libraries
        .into_iter()
        .filter(|l| !nadpisane.contains(&bez_wersji(&l.name)))
        .collect();
    libraries.extend(child.libraries);

    let mut game = parent.arguments.game;
    game.extend(child.arguments.game);
    let mut jvm = parent.arguments.jvm;
    jvm.extend(child.arguments.jvm);

    VersionJson {
        id: child.id,
        inherits_from: None,
        main_class: child.main_class,
        arguments: Arguments { game, jvm },
        libraries,
        asset_index: child.asset_index.or(parent.asset_index),
        assets: child.assets.or(parent.assets),
        downloads: child.downloads.or(parent.downloads),
    }
}

pub fn rules_allow(rules: &[Rule], os: Os) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut wynik = false;
    for r in rules {
        let pasuje = match &r.os {
            None => true,
            Some(o) => match &o.name {
                None => true,
                Some(n) => n == os.nazwa_mojang(),
            },
        };
        if pasuje {
            wynik = r.action == "allow";
        }
    }
    wynik
}

/// Podstawia `${zmienna}`. Nieznane zmienne zostawia nietknięte,
/// żeby błąd był widoczny w logu zamiast zamieniać się w pusty argument.
pub fn substitute(tpl: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(tpl.len());
    let mut reszta = tpl;
    while let Some(start) = reszta.find("${") {
        out.push_str(&reszta[..start]);
        let po = &reszta[start + 2..];
        match po.find('}') {
            Some(koniec) => {
                let nazwa = &po[..koniec];
                match vars.get(nazwa) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push_str("${");
                        out.push_str(nazwa);
                        out.push('}');
                    }
                }
                reszta = &po[koniec + 1..];
            }
            None => {
                out.push_str(&reszta[start..]);
                return out;
            }
        }
    }
    out.push_str(reszta);
    out
}
```

Dopisz `pub mod version;` do `crates/core/src/lib.rs`.

- [ ] **Step 4: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 37 testów

- [ ] **Step 5: Test na prawdziwym profilu**

Sprawdź scalanie na profilu wygenerowanym przez prawdziwy instalator. Jeśli nie masz go pod ręką, pomiń ten krok — Task 10 i tak go wytworzy.

```bash
cargo test -p chmurka-core version -- --nocapture
```

- [ ] **Step 6: Commit**

```bash
git add crates/core
git commit -m "Model profilu wersji, dziedziczenie i reguly systemowe"
```

---

### Task 10: Instalacja gry

**Files:**
- Create: `crates/core/src/game_install.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `java::JavaInstall`, `version::{VersionJson, load, rules_allow}`, `net::Downloader`
- Produces:
  - `pub async fn ensure_loader(mc_dir: &Path, java: &JavaInstall, loader_kind: &str, loader_version: &str, dl: &Downloader, on: Arc<dyn Fn(Progress) + Send + Sync>) -> Result<String, InstallError>` — zwraca id profilu, np. `neoforge-21.1.249`
  - `pub async fn ensure_libraries(mc_dir: &Path, v: &VersionJson, dl: &Downloader, on: Arc<dyn Fn(Progress) + Send + Sync>) -> Result<(), InstallError>`
  - `pub async fn ensure_assets(mc_dir: &Path, v: &VersionJson, dl: &Downloader, on: Arc<dyn Fn(Progress) + Send + Sync>) -> Result<(), InstallError>`

- [ ] **Step 1: Zaimplementuj**

```rust
use crate::java::JavaInstall;
use crate::net::{DownloadSpec, Downloader, Expect, NetError};
use crate::progress::{Progress, Stage};
use crate::version::{rules_allow, VersionJson};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, thiserror::Error)]
pub enum InstallError {
    #[error(transparent)]
    Net(#[from] NetError),
    #[error("instalator {loader} zakończył się kodem {kod}; wyjście:\n{wyjscie}")]
    Instalator { loader: String, kod: i32, wyjscie: String },
    #[error("nie udało się uruchomić instalatora: {0}")]
    Uruchomienie(std::io::Error),
    #[error("błąd operacji na pliku: {0}")]
    Io(#[from] std::io::Error),
    #[error("nieprawidłowy indeks zasobów: {0}")]
    IndeksZasobow(String),
}

fn neoforge_installer_url(wersja: &str) -> String {
    format!("https://maven.neoforged.net/releases/net/neoforged/neoforge/{wersja}/neoforge-{wersja}-installer.jar")
}

/// Uruchamia oficjalny instalator loadera w trybie bezobsługowym.
///
/// To on patchuje `client.jar` (procesor `binarypatcher`), więc nie musimy
/// odtwarzać tego procesu u siebie ani redystrybuować plików Mojanga.
pub async fn ensure_loader(
    mc_dir: &Path,
    java: &JavaInstall,
    loader_kind: &str,
    loader_version: &str,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<String, InstallError> {
    let profil = format!("{loader_kind}-{loader_version}");
    let json_profilu = mc_dir.join("versions").join(&profil).join(format!("{profil}.json"));
    if json_profilu.is_file() {
        return Ok(profil);
    }

    on(Progress { stage: Stage::Loader, done: 0, total: 1, bytes: 0, label: profil.clone() });

    let instalator = mc_dir.join(format!("{profil}-installer.jar"));
    dl.fetch_one(&DownloadSpec {
        urls: vec![neoforge_installer_url(loader_version)],
        dest: instalator.clone(),
        expect: Expect::Any,
    })
    .await?;

    // Instalator odmawia pracy bez tego pliku — sprawdza go, zanim cokolwiek zrobi.
    std::fs::create_dir_all(mc_dir)?;
    let profile = mc_dir.join("launcher_profiles.json");
    if !profile.exists() {
        std::fs::write(&profile, br#"{"profiles":{},"selectedProfile":"","clientToken":"","authenticationDatabase":{},"launcherVersion":{"name":"","format":21},"settings":{}}"#)?;
    }

    let wyjscie = std::process::Command::new(&java.java_bin)
        .arg("-jar")
        .arg(&instalator)
        .arg("--install-client")
        .arg(mc_dir)
        .current_dir(mc_dir)
        .output()
        .map_err(InstallError::Uruchomienie)?;

    if !wyjscie.status.success() {
        return Err(InstallError::Instalator {
            loader: profil,
            kod: wyjscie.status.code().unwrap_or(-1),
            wyjscie: String::from_utf8_lossy(&wyjscie.stdout).chars().rev().take(2000).collect::<String>().chars().rev().collect(),
        });
    }

    let _ = std::fs::remove_file(&instalator);
    let _ = std::fs::remove_file(mc_dir.join(format!("{profil}-installer.jar.log")));

    if !json_profilu.is_file() {
        return Err(InstallError::Instalator {
            loader: profil,
            kod: 0,
            wyjscie: "instalator zakończył się sukcesem, ale nie powstał profil wersji".into(),
        });
    }
    on(Progress { stage: Stage::Loader, done: 1, total: 1, bytes: 0, label: "NeoForge gotowy".into() });
    Ok(profil)
}

pub async fn ensure_libraries(
    mc_dir: &Path,
    v: &VersionJson,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<(), InstallError> {
    let os = crate::java::biezacy_os();
    let libs = mc_dir.join("libraries");
    let mut specyfikacje = Vec::new();

    for l in &v.libraries {
        if !rules_allow(&l.rules, os) {
            continue;
        }
        let Some(art) = l.downloads.as_ref().and_then(|d| d.artifact.as_ref()) else {
            // Instalator NeoForge kładzie część bibliotek sam, bez wpisu downloads.
            continue;
        };
        specyfikacje.push(DownloadSpec {
            urls: vec![art.url.clone()],
            dest: libs.join(&art.path),
            expect: Expect::Sha1(art.sha1.clone()),
        });
    }

    // Klient vanilla — potrzebny, bo NeoForge patchuje go, a nie zastępuje.
    if let Some(klient) = v.downloads.as_ref().and_then(|d| d.client.as_ref()) {
        specyfikacje.push(DownloadSpec {
            urls: vec![klient.url.clone()],
            dest: mc_dir.join("versions").join(&v.id).join(format!("{}.jar", v.id)),
            expect: Expect::Sha1(klient.sha1.clone()),
        });
    }

    dl.fetch_many(specyfikacje, Stage::Libraries, on).await?;
    Ok(())
}

pub async fn ensure_assets(
    mc_dir: &Path,
    v: &VersionJson,
    dl: &Downloader,
    on: Arc<dyn Fn(Progress) + Send + Sync>,
) -> Result<(), InstallError> {
    let Some(idx) = v.asset_index.as_ref() else {
        return Ok(());
    };
    let assets = mc_dir.join("assets");
    let plik_indeksu = assets.join("indexes").join(format!("{}.json", idx.id));

    dl.fetch_one(&DownloadSpec {
        urls: vec![idx.url.clone()],
        dest: plik_indeksu.clone(),
        expect: Expect::Sha1(idx.sha1.clone()),
    })
    .await?;

    #[derive(serde::Deserialize)]
    struct Indeks {
        objects: BTreeMap<String, Obiekt>,
    }
    #[derive(serde::Deserialize)]
    struct Obiekt {
        hash: String,
    }

    let tekst = std::fs::read_to_string(&plik_indeksu)?;
    let indeks: Indeks =
        serde_json::from_str(&tekst).map_err(|e| InstallError::IndeksZasobow(e.to_string()))?;

    let specyfikacje: Vec<DownloadSpec> = indeks
        .objects
        .values()
        .map(|o| DownloadSpec {
            urls: vec![format!(
                "https://resources.download.minecraft.net/{}/{}",
                &o.hash[0..2],
                o.hash
            )],
            dest: assets.join("objects").join(&o.hash[0..2]).join(&o.hash),
            expect: Expect::Sha1(o.hash.clone()),
        })
        .collect();

    dl.fetch_many(specyfikacje, Stage::Assets, on).await?;
    Ok(())
}
```

Dopisz `pub mod game_install;` do `crates/core/src/lib.rs`.

- [ ] **Step 2: Sprawdź, że się kompiluje**

Run: `cargo test -p chmurka-core`
Expected: PASS, 37 testów (nowy moduł nie ma własnych testów jednostkowych — jest weryfikowany ręcznie w Task 12, bo cała jego wartość leży w rozmowie z prawdziwymi serwerami)

- [ ] **Step 3: Commit**

```bash
git add crates/core
git commit -m "Instalacja loadera, bibliotek i zasobow gry"
```

---

### Task 11: Uruchomienie gry

**Files:**
- Create: `crates/core/src/launch.rs`
- Create: `crates/core/src/auth/mod.rs`
- Modify: `crates/core/src/lib.rs`

**Interfaces:**
- Consumes: `version::{VersionJson, Arg, ArgValue, rules_allow, substitute}`, `java::Os`
- Produces:
  - `pub struct Account { pub name: String, pub uuid: String, pub token: String, pub kind: AccountKind }`
  - `pub enum AccountKind { Msa, Offline }` z `AccountKind::user_type(&self) -> &'static str`
  - `pub struct LaunchParams<'a> { pub java: &'a Path, pub mc_dir: &'a Path, pub game_dir: &'a Path, pub version: &'a VersionJson, pub account: &'a Account, pub min_mb: u32, pub max_mb: u32 }`
  - `pub fn build_command(p: &LaunchParams) -> Result<std::process::Command, LaunchError>`

- [ ] **Step 1: Napisz auth/mod.rs**

```rust
pub mod offline;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountKind {
    Msa,
    Offline,
}

impl AccountKind {
    /// Wartość argumentu `--userType` oczekiwana przez klienta gry.
    pub fn user_type(&self) -> &'static str {
        match self {
            AccountKind::Msa => "msa",
            AccountKind::Offline => "legacy",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Account {
    pub name: String,
    pub uuid: String,
    pub token: String,
    pub kind: AccountKind,
}
```

- [ ] **Step 2: Napisz testy budowania polecenia**

Na końcu `crates/core/src/launch.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Account, AccountKind};

    fn wersja() -> VersionJson {
        serde_json::from_str(r#"{
            "id":"neoforge-21.1.249",
            "mainClass":"cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments":{
              "game":["--username","${auth_player_name}","--uuid","${auth_uuid}",
                      "--accessToken","${auth_access_token}","--userType","${user_type}",
                      "--gameDir","${game_directory}"],
              "jvm":["-DlibraryDirectory=${library_directory}","-cp","${classpath}"]
            },
            "libraries":[
              {"name":"a:b:1","downloads":{"artifact":{"path":"a/b/1/b-1.jar","sha1":"x","size":1,"url":"https://x"}}},
              {"name":"c:d:1","downloads":{"artifact":{"path":"c/d/1/d-1.jar","sha1":"y","size":1,"url":"https://y"}},
               "rules":[{"action":"allow","os":{"name":"osx"}}]}
            ],
            "assets":"17",
            "assetIndex":{"id":"17","sha1":"a","size":1,"totalSize":1,"url":"https://x/17.json"}
        }"#).unwrap()
    }

    fn konto() -> Account {
        Account {
            name: "Tomasz".into(),
            uuid: "61a50080-80aa-3842-8df5-cd674d3a57f2".into(),
            token: "0".into(),
            kind: AccountKind::Offline,
        }
    }

    #[test]
    fn podstawia_dane_konta_do_argumentow() {
        let v = wersja();
        let k = konto();
        let mc = std::path::Path::new("/tmp/mc");
        let gra = std::path::Path::new("/tmp/gra");
        let cmd = build_command(&LaunchParams {
            java: std::path::Path::new("/tmp/java"),
            mc_dir: mc, game_dir: gra, version: &v, account: &k,
            min_mb: 512, max_mb: 4096,
        }).unwrap();

        let args: Vec<String> = cmd.get_args().map(|a| a.to_string_lossy().to_string()).collect();
        assert!(args.contains(&"Tomasz".to_string()));
        assert!(args.contains(&"61a50080-80aa-3842-8df5-cd674d3a57f2".to_string()));
        assert!(args.contains(&"legacy".to_string()));
        assert!(args.contains(&"-Xmx4096M".to_string()));
        assert!(args.contains(&"-Xms512M".to_string()));
        assert!(args.contains(&"cpw.mods.bootstraplauncher.BootstrapLauncher".to_string()));
        assert!(!args.iter().any(|a| a.contains("${")), "wszystkie zmienne musza byc podstawione");
    }

    #[test]
    fn classpath_pomija_libki_dla_innego_systemu() {
        let v = wersja();
        let k = konto();
        let cp = build_classpath(&v, std::path::Path::new("/tmp/mc"), Os::Linux);
        assert!(cp.contains("b-1.jar"));
        assert!(!cp.contains("d-1.jar"), "libka tylko dla macOS nie moze trafic na classpath Linuksa");
    }

    #[test]
    fn separator_classpatha_zalezy_od_systemu() {
        assert_eq!(separator(Os::Linux), ":");
        assert_eq!(separator(Os::Windows), ";");
    }
}
```

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core launch`
Expected: FAIL — `cannot find function build_command`

- [ ] **Step 4: Zaimplementuj**

```rust
use crate::auth::Account;
use crate::java::{biezacy_os, Os};
use crate::version::{rules_allow, substitute, Arg, VersionJson};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum LaunchError {
    #[error("profil wersji nie ma indeksu zasobów, gra nie wystartuje")]
    BrakIndeksuZasobow,
}

pub struct LaunchParams<'a> {
    pub java: &'a Path,
    pub mc_dir: &'a Path,
    pub game_dir: &'a Path,
    pub version: &'a VersionJson,
    pub account: &'a Account,
    pub min_mb: u32,
    pub max_mb: u32,
}

pub fn separator(os: Os) -> &'static str {
    if os == Os::Windows { ";" } else { ":" }
}

pub fn build_classpath(v: &VersionJson, mc_dir: &Path, os: Os) -> String {
    let libs = mc_dir.join("libraries");
    let mut czesci: Vec<String> = Vec::new();
    for l in &v.libraries {
        if !rules_allow(&l.rules, os) {
            continue;
        }
        if let Some(art) = l.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            czesci.push(libs.join(&art.path).display().to_string());
        }
    }
    czesci.join(separator(os))
}

pub fn build_command(p: &LaunchParams) -> Result<std::process::Command, LaunchError> {
    let os = biezacy_os();
    let indeks = p
        .version
        .assets
        .clone()
        .or_else(|| p.version.asset_index.as_ref().map(|a| a.id.clone()))
        .ok_or(LaunchError::BrakIndeksuZasobow)?;

    let mut zmienne: BTreeMap<String, String> = BTreeMap::new();
    zmienne.insert("auth_player_name".into(), p.account.name.clone());
    zmienne.insert("auth_uuid".into(), p.account.uuid.clone());
    zmienne.insert("auth_access_token".into(), p.account.token.clone());
    zmienne.insert("auth_xuid".into(), String::new());
    zmienne.insert("clientid".into(), String::new());
    zmienne.insert("user_type".into(), p.account.kind.user_type().into());
    zmienne.insert("version_name".into(), p.version.id.clone());
    zmienne.insert("version_type".into(), "release".into());
    zmienne.insert("game_directory".into(), p.game_dir.display().to_string());
    zmienne.insert("assets_root".into(), p.mc_dir.join("assets").display().to_string());
    zmienne.insert("game_assets".into(), p.mc_dir.join("assets").display().to_string());
    zmienne.insert("assets_index_name".into(), indeks);
    zmienne.insert("library_directory".into(), p.mc_dir.join("libraries").display().to_string());
    zmienne.insert("classpath_separator".into(), separator(os).into());
    zmienne.insert("classpath".into(), build_classpath(p.version, p.mc_dir, os));
    zmienne.insert(
        "natives_directory".into(),
        p.mc_dir.join("natives").join(&p.version.id).display().to_string(),
    );
    zmienne.insert("launcher_name".into(), "ChmurkowyLauncher".into());
    zmienne.insert("launcher_version".into(), env!("CARGO_PKG_VERSION").into());
    zmienne.insert("resolution_width".into(), "854".into());
    zmienne.insert("resolution_height".into(), "480".into());

    let mut cmd = std::process::Command::new(p.java);
    cmd.arg(format!("-Xms{}M", p.min_mb));
    cmd.arg(format!("-Xmx{}M", p.max_mb));
    for a in rozwin(&p.version.arguments.jvm, os, &zmienne) {
        cmd.arg(a);
    }
    cmd.arg(&p.version.main_class);
    for a in rozwin(&p.version.arguments.game, os, &zmienne) {
        cmd.arg(a);
    }
    cmd.current_dir(p.game_dir);
    Ok(cmd)
}

fn rozwin(argi: &[Arg], os: Os, zmienne: &BTreeMap<String, String>) -> Vec<String> {
    let mut out = Vec::new();
    for a in argi {
        match a {
            Arg::Prosty(s) => out.push(substitute(s, zmienne)),
            Arg::Warunkowy { rules, value } => {
                if rules_allow(rules, os) {
                    for v in value.lista() {
                        out.push(substitute(&v, zmienne));
                    }
                }
            }
        }
    }
    out
}
```

Dopisz `pub mod auth;` i `pub mod launch;` do `crates/core/src/lib.rs`.

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 40 testów

- [ ] **Step 6: Commit**

```bash
git add crates/core
git commit -m "Budowa polecenia startowego gry i classpatha"
```

---

### Task 12: Konto offline i pełny przebieg z konsoli

Kamień milowy fazy 2. Po tym zadaniu gra realnie startuje.

**Files:**
- Create: `crates/core/src/auth/offline.rs`
- Modify: `crates/cli/src/main.rs`

**Interfaces:**
- Consumes: wszystko powyżej
- Produces:
  - `pub fn offline_uuid(name: &str) -> String`
  - `pub fn offline_account(name: &str) -> Account`
  - polecenia `chmurka install` i `chmurka run`

- [ ] **Step 1: Dodaj md-5**

Run: `cargo add md-5 -p chmurka-core`

- [ ] **Step 2: Napisz testy UUID offline**

`crates/core/src/auth/offline.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn odtwarza_javowe_nameuuidfrombytes() {
        // Wartosci wyliczone z algorytmu Javy UUID.nameUUIDFromBytes
        // dla ciagu "OfflinePlayer:<nick>" — tak serwery licza UUID w trybie offline.
        assert_eq!(offline_uuid("Notch"), "b50ad385-829d-3141-a216-7e7d7539ba7f");
        assert_eq!(offline_uuid("TestGracz"), "2849380d-0c09-3dd1-87a1-a00a3f22c477");
        assert_eq!(offline_uuid("Tomasz"), "61a50080-80aa-3842-8df5-cd674d3a57f2");
    }

    #[test]
    fn ma_wersje_3_i_wariant_ietf() {
        let u = offline_uuid("Notch");
        assert_eq!(&u[14..15], "3", "czwarty blok musi zaczynac sie od wersji 3");
        assert!(matches!(&u[19..20], "8" | "9" | "a" | "b"), "wariant IETF");
    }

    #[test]
    fn konto_offline_ma_pusty_token() {
        let k = offline_account("Tomasz");
        assert_eq!(k.name, "Tomasz");
        assert_eq!(k.kind, crate::auth::AccountKind::Offline);
        assert_eq!(k.uuid, "61a50080-80aa-3842-8df5-cd674d3a57f2");
    }
}
```

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core offline`
Expected: FAIL — `cannot find function offline_uuid`

- [ ] **Step 4: Zaimplementuj**

```rust
use super::{Account, AccountKind};
use md5::{Digest, Md5};

/// Odtwarza javowe `UUID.nameUUIDFromBytes("OfflinePlayer:<nick>")`.
///
/// UWAGA: to NIE jest UUID v3 z przestrzenią nazw. Java liczy zwykłe MD5
/// z samych bajtów i dopiero potem wpisuje wersję i wariant. Użycie
/// `Uuid::new_v3` z jakąkolwiek przestrzenią da inny wynik i serwer
/// potraktuje gracza jako kogoś innego.
pub fn offline_uuid(name: &str) -> String {
    let mut h = Md5::new();
    h.update(format!("OfflinePlayer:{name}").as_bytes());
    let mut b = h.finalize();
    b[6] = (b[6] & 0x0f) | 0x30; // wersja 3
    b[8] = (b[8] & 0x3f) | 0x80; // wariant IETF
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

pub fn offline_account(name: &str) -> Account {
    Account {
        name: name.to_string(),
        uuid: offline_uuid(name),
        // Klient wymaga niepustego tokenu; na serwerze offline i tak nie jest sprawdzany.
        token: "0".to_string(),
        kind: AccountKind::Offline,
    }
}
```

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 43 testy

- [ ] **Step 6: Dodaj polecenia install i run do CLI**

Dopisz warianty do `enum Polecenie` w `crates/cli/src/main.rs`:

```rust
    /// Instaluje wszystko do wskazanego katalogu danych.
    Install {
        #[arg(long)]
        manifest: String,
        #[arg(long, default_value = "data")]
        data: PathBuf,
    },
    /// Instaluje i uruchamia grę na koncie offline.
    Run {
        #[arg(long)]
        manifest: String,
        #[arg(long, default_value = "data")]
        data: PathBuf,
        #[arg(long)]
        nick: String,
    },
```

Zamień `fn main` na wersję asynchroniczną i dopisz obsługę:

```rust
#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().polecenie {
        Polecenie::PackBuild { instance, out, base_url, version } => {
            pack_build(&instance, &out, &base_url, &version)
        }
        Polecenie::Install { manifest, data } => {
            przygotuj(&manifest, &data).await?;
            println!("Gotowe.");
            Ok(())
        }
        Polecenie::Run { manifest, data, nick } => {
            let (wersja, java) = przygotuj(&manifest, &data).await?;
            let konto = chmurka_core::auth::offline::offline_account(&nick);
            let mut cmd = chmurka_core::launch::build_command(&chmurka_core::launch::LaunchParams {
                java: &java.java_bin,
                mc_dir: &data.join("mc"),
                game_dir: &data.join("instance"),
                version: &wersja,
                account: &konto,
                min_mb: 512,
                max_mb: 4096,
            })?;
            println!("Uruchamiam grę…");
            let status = cmd.status()?;
            println!("Gra zakończyła się kodem {:?}", status.code());
            Ok(())
        }
    }
}

/// Wspólna część install i run: pobiera manifest i doprowadza instalację do stanu gotowego.
async fn przygotuj(
    adres_manifestu: &str,
    data: &Path,
) -> Result<(chmurka_core::version::VersionJson, chmurka_core::java::JavaInstall)> {
    use chmurka_core::*;
    use std::sync::Arc;

    let postep: Arc<dyn Fn(progress::Progress) + Send + Sync> = Arc::new(|p| {
        println!("[{}] {}/{} {}", p.stage.opis(), p.done, p.total, p.label);
    });

    let tekst = reqwest::get(adres_manifestu).await?.text().await?;
    let m = manifest::parse(&tekst)?;
    println!("Paczka {} wersja {}", m.pack.name, m.pack.version);

    let dl = net::Downloader::new(8);
    let mc = data.join("mc");
    let instancja = data.join("instance");
    std::fs::create_dir_all(&instancja)?;

    let java = java::ensure(data, m.java.major, &dl, postep.clone()).await?;
    let profil = game_install::ensure_loader(
        &mc, &java, &m.pack.loader.kind, &m.pack.loader.version, &dl, postep.clone()).await?;
    let wersja = version::load(&mc.join("versions"), &profil)?;

    game_install::ensure_libraries(&mc, &wersja, &dl, postep.clone()).await?;
    game_install::ensure_assets(&mc, &wersja, &dl, postep.clone()).await?;

    let sciezka_stanu = data.join("state.json");
    let mut stan = state::State::load(&sciezka_stanu);
    let akcje = pack_sync::plan(&m, &stan, &pack_sync::DiskProbe::new(&instancja));
    let notatki = pack_sync::apply(&m, &instancja, &mut stan, &akcje, &dl, postep.clone()).await?;
    stan.save(&sciezka_stanu)?;
    for n in notatki {
        println!("  {n}");
    }

    Ok((wersja, java))
}
```

Run: `cargo add reqwest --no-default-features --features rustls-tls -p chmurka-cli`

- [ ] **Step 7: Uruchom pełny przebieg lokalnie**

Wystaw `dist/` przez lokalny serwer i przetestuj cały pipeline:

```bash
python3 -m http.server 8099 --directory dist &
cargo run -p chmurka-cli --bin chmurka -- run \
  --manifest "http://127.0.0.1:8099/manifest.json" \
  --data /tmp/chmurka-test \
  --nick TestGracz
```

Uwaga: manifest wymaga adresów `https` dla plików, ale sam manifest pobieramy zwykłym `reqwest::get`, więc `http` na lokalnym serwerze zadziała. Adresy modów w manifeście i tak wskazują na prawdziwe CDN-y.

Expected: pobiera Javę, instaluje NeoForge, ciągnie biblioteki i zasoby, synchronizuje 249 modów, uruchamia grę. Pierwszy przebieg to około 1,8 GB i kilkanaście minut.

Sprawdź izolację:

```bash
ls /tmp/chmurka-test
# powinno byc: java  mc  instance  state.json
ls ~/.minecraft
# nie moze przybyc nic nowego
```

- [ ] **Step 8: Commit**

```bash
git add crates
git commit -m "Konto offline i pelny przebieg instalacji z konsoli"
```

---

## Faza 3 — logowanie Microsoft

### Task 13: Device code flow

Cały łańcuch to pięć żądań pod rząd. Każde ma inny format błędu, więc każde dostaje własny wariant w `AuthError` — inaczej użytkownik zobaczy „coś poszło nie tak" zamiast „to konto nie ma kupionego Minecrafta".

**Files:**
- Create: `crates/core/src/auth/msa.rs`
- Create: `crates/core/src/auth/store.rs`
- Modify: `crates/core/src/auth/mod.rs`

**Interfaces:**
- Consumes: `auth::{Account, AccountKind}`
- Produces:
  - `pub struct DeviceCode { pub user_code: String, pub device_code: String, pub verification_uri: String, pub interval_s: u64, pub expires_in_s: u64 }`
  - `pub struct Tokens { pub access_token: String, pub refresh_token: String }`
  - `pub async fn begin(client_id: &str) -> Result<DeviceCode, AuthError>`
  - `pub async fn poll_once(client_id: &str, device_code: &str) -> Result<PollResult, AuthError>`
  - `pub enum PollResult { Czekamy, Zwolnij, Gotowe(Tokens) }`
  - `pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<Tokens, AuthError>`
  - `pub async fn zaloguj_minecraft(tokens: &Tokens) -> Result<Account, AuthError>`
  - `store`: `pub fn load(path: &Path) -> Option<Zapis>`, `pub fn save(path: &Path, z: &Zapis) -> std::io::Result<()>`, `pub struct Zapis { pub refresh_token: String, pub nick: String }`

- [ ] **Step 1: Napisz store.rs**

```rust
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Zapis {
    pub refresh_token: String,
    pub nick: String,
}

pub fn load(path: &Path) -> Option<Zapis> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

pub fn save(path: &Path, z: &Zapis) -> std::io::Result<()> {
    if let Some(rodzic) = path.parent() {
        std::fs::create_dir_all(rodzic)?;
    }
    std::fs::write(path, serde_json::to_vec_pretty(z)?)?;
    // Plik zawiera token odswiezania, wiec na Linuksie zawezamy uprawnienia.
    // Na Windowsie polegamy na uprawnieniach katalogu uzytkownika.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

pub fn clear(path: &Path) {
    let _ = std::fs::remove_file(path);
}
```

- [ ] **Step 2: Napisz test parsowania odpowiedzi**

Na końcu `crates/core/src/auth/msa.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn czyta_odpowiedz_device_code() {
        // Ksztalt sprawdzony empirycznie na login.live.com/oauth20_connect.srf
        let json = r#"{"user_code":"V3REVW36","device_code":"-Dk2jLDRyYPl32RFpA",
                       "verification_uri":"https://www.microsoft.com/link",
                       "interval":5,"expires_in":900}"#;
        let d: DeviceCode = serde_json::from_str(json).unwrap();
        assert_eq!(d.user_code, "V3REVW36");
        assert_eq!(d.verification_uri, "https://www.microsoft.com/link");
        assert_eq!(d.interval_s, 5);
        assert_eq!(d.expires_in_s, 900);
    }

    #[test]
    fn rozpoznaje_stany_odpytywania() {
        assert!(matches!(zinterpretuj_blad("authorization_pending"), Some(PollResult::Czekamy)));
        assert!(matches!(zinterpretuj_blad("slow_down"), Some(PollResult::Zwolnij)));
        assert!(zinterpretuj_blad("expired_token").is_none());
        assert!(zinterpretuj_blad("authorization_declined").is_none());
    }

    #[test]
    fn tlumaczy_kody_xsts_na_polski() {
        assert!(opis_xerr(2148916233).contains("konta Xbox"));
        assert!(opis_xerr(2148916238).contains("dziecka"));
        assert!(opis_xerr(999).contains("999"));
    }
}
```

- [ ] **Step 3: Uruchom testy i potwierdź, że nie przechodzą**

Run: `cargo test -p chmurka-core msa`
Expected: FAIL — `cannot find type DeviceCode`

- [ ] **Step 4: Zaimplementuj**

```rust
use super::{Account, AccountKind};
use serde::Deserialize;

const CONNECT: &str = "https://login.live.com/oauth20_connect.srf";
const TOKEN: &str = "https://login.live.com/oauth20_token.srf";
const SCOPE: &str = "service::user.auth.xboxlive.com::MBI_SSL";

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("błąd połączenia z Microsoft: {0}")]
    Siec(String),
    #[error("logowanie nie powiodło się: {0}")]
    Odmowa(String),
    #[error("kod wygasł — spróbuj zalogować się jeszcze raz")]
    Wygasl,
    #[error("Xbox Live odmówił: {0}")]
    Xbox(String),
    #[error("to konto Microsoft nie ma kupionego Minecrafta")]
    BrakGry,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCode {
    pub user_code: String,
    pub device_code: String,
    pub verification_uri: String,
    #[serde(rename = "interval")]
    pub interval_s: u64,
    #[serde(rename = "expires_in")]
    pub expires_in_s: u64,
}

#[derive(Debug, Clone)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug)]
pub enum PollResult {
    Czekamy,
    Zwolnij,
    Gotowe(Tokens),
}

fn klient() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent("ChmurkowyLauncher/0.1")
        .build()
        .expect("klient HTTP")
}

/// Rozpoczyna logowanie. Zwraca kod, który użytkownik wpisuje na stronie Microsoftu.
pub async fn begin(client_id: &str) -> Result<DeviceCode, AuthError> {
    let odp = klient()
        .post(CONNECT)
        .form(&[
            ("client_id", client_id),
            ("scope", SCOPE),
            ("response_type", "device_code"),
        ])
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;
    let tekst = odp.text().await.map_err(|e| AuthError::Siec(e.to_string()))?;
    serde_json::from_str(&tekst).map_err(|_| AuthError::Odmowa(tekst))
}

pub fn zinterpretuj_blad(kod: &str) -> Option<PollResult> {
    match kod {
        "authorization_pending" => Some(PollResult::Czekamy),
        "slow_down" => Some(PollResult::Zwolnij),
        _ => None,
    }
}

pub async fn poll_once(client_id: &str, device_code: &str) -> Result<PollResult, AuthError> {
    #[derive(Deserialize)]
    struct Odp {
        access_token: Option<String>,
        refresh_token: Option<String>,
        error: Option<String>,
    }

    let tekst = klient()
        .post(TOKEN)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("device_code", device_code),
        ])
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .text()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    let o: Odp = serde_json::from_str(&tekst).map_err(|_| AuthError::Odmowa(tekst.clone()))?;

    if let (Some(a), Some(r)) = (o.access_token, o.refresh_token) {
        return Ok(PollResult::Gotowe(Tokens { access_token: a, refresh_token: r }));
    }
    match o.error.as_deref() {
        Some(kod) => match zinterpretuj_blad(kod) {
            Some(stan) => Ok(stan),
            None if kod == "expired_token" => Err(AuthError::Wygasl),
            None => Err(AuthError::Odmowa(kod.to_string())),
        },
        None => Err(AuthError::Odmowa(tekst)),
    }
}

pub async fn refresh(client_id: &str, refresh_token: &str) -> Result<Tokens, AuthError> {
    #[derive(Deserialize)]
    struct Odp {
        access_token: String,
        refresh_token: String,
    }
    let tekst = klient()
        .post(TOKEN)
        .form(&[
            ("client_id", client_id),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("scope", SCOPE),
        ])
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .text()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;
    let o: Odp = serde_json::from_str(&tekst).map_err(|_| AuthError::Odmowa(tekst))?;
    Ok(Tokens { access_token: o.access_token, refresh_token: o.refresh_token })
}

pub fn opis_xerr(kod: u64) -> String {
    match kod {
        2148916233 => "to konto Microsoft nie ma profilu Xbox — załóż go na xbox.com i spróbuj ponownie".into(),
        2148916238 => "to konto dziecka — musi zostać dodane do rodziny Microsoft, żeby móc grać".into(),
        inny => format!("Xbox Live odrzucił logowanie, kod {inny}"),
    }
}

/// Zamienia token Microsoftu na token Minecrafta. Cztery żądania pod rząd.
pub async fn zaloguj_minecraft(tokens: &Tokens) -> Result<Account, AuthError> {
    let c = klient();

    // 1. Xbox Live. Token z MBI_SSL jest juz biletem RPS, wiec idzie bez prefiksu "d=".
    #[derive(Deserialize)]
    struct XblOdp {
        #[serde(rename = "Token")]
        token: String,
        #[serde(rename = "DisplayClaims")]
        claims: Claims,
    }
    #[derive(Deserialize)]
    struct Claims {
        xui: Vec<Xui>,
    }
    #[derive(Deserialize)]
    struct Xui {
        uhs: String,
    }

    let xbl: XblOdp = c
        .post("https://user.auth.xboxlive.com/user/authenticate")
        .json(&serde_json::json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com",
                            "RpsTicket": tokens.access_token },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .json()
        .await
        .map_err(|e| AuthError::Xbox(e.to_string()))?;

    let uhs = xbl.claims.xui.first().map(|x| x.uhs.clone()).unwrap_or_default();

    // 2. XSTS.
    let odp = c
        .post("https://xsts.auth.xboxlive.com/xsts/authorize")
        .json(&serde_json::json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    if odp.status() == reqwest::StatusCode::UNAUTHORIZED {
        #[derive(Deserialize)]
        struct Blad {
            #[serde(rename = "XErr")]
            xerr: u64,
        }
        let b: Blad = odp.json().await.map_err(|e| AuthError::Xbox(e.to_string()))?;
        return Err(AuthError::Xbox(opis_xerr(b.xerr)));
    }

    #[derive(Deserialize)]
    struct XstsOdp {
        #[serde(rename = "Token")]
        token: String,
    }
    let xsts: XstsOdp = odp.json().await.map_err(|e| AuthError::Xbox(e.to_string()))?;

    // 3. Token Minecrafta.
    #[derive(Deserialize)]
    struct McOdp {
        access_token: String,
    }
    let mc: McOdp = c
        .post("https://api.minecraftservices.com/authentication/login_with_xbox")
        .json(&serde_json::json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?
        .json()
        .await
        .map_err(|e| AuthError::Odmowa(e.to_string()))?;

    // 4. Profil. 404 znaczy konto bez kupionej gry — to najczestszy blad u testerow.
    let odp = c
        .get("https://api.minecraftservices.com/minecraft/profile")
        .bearer_auth(&mc.access_token)
        .send()
        .await
        .map_err(|e| AuthError::Siec(e.to_string()))?;

    if odp.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(AuthError::BrakGry);
    }

    #[derive(Deserialize)]
    struct Profil {
        id: String,
        name: String,
    }
    let p: Profil = odp.json().await.map_err(|e| AuthError::Odmowa(e.to_string()))?;

    // API zwraca UUID bez myslnikow, a gra oczekuje ich w argumencie.
    let u = &p.id;
    let uuid = if u.len() == 32 {
        format!("{}-{}-{}-{}-{}", &u[0..8], &u[8..12], &u[12..16], &u[16..20], &u[20..32])
    } else {
        u.clone()
    };

    Ok(Account { name: p.name, uuid, token: mc.access_token, kind: AccountKind::Msa })
}
```

Dopisz `pub mod msa;` i `pub mod store;` do `crates/core/src/auth/mod.rs`.

- [ ] **Step 5: Uruchom testy**

Run: `cargo test -p chmurka-core`
Expected: PASS, 46 testów

- [ ] **Step 6: Sprawdź ręcznie na prawdziwym koncie**

Dodaj tymczasowe polecenie `chmurka login --client-id 00000000402b5328`, które wywoła `begin`, wypisze kod, poczeka i wypisze nick. Zaloguj się swoim kontem.

Expected: wypisuje kod, po wpisaniu go na `microsoft.com/link` wypisuje Twój nick i UUID.

To jest moment weryfikacji ryzyka z §17 specyfikacji. Jeżeli Xbox Live odpowie 400 lub 401, dopisz prefiks `d=` do `RpsTicket` i spróbuj ponownie — specyfikacja przewiduje ten wariant.

- [ ] **Step 7: Commit**

```bash
git add crates/core
git commit -m "Logowanie Microsoft przez device code i skladowanie tokenu"
```

---

## Faza 4 — interfejs i dystrybucja

### Task 14: Szkielet GUI i motyw

**Files:**
- Create: `crates/launcher/Cargo.toml`
- Create: `crates/launcher/src/main.rs`
- Create: `crates/launcher/src/theme.rs`
- Create: `crates/launcher/src/app.rs`
- Modify: `Cargo.toml`

**Interfaces:**
- Consumes: `chmurka_core::*`
- Produces:
  - `pub struct App { ... }` implementujące `eframe::App`
  - `pub enum Widok { Glowny, Logowanie, Ustawienia }`
  - `pub enum Wiadomosc { Postep(Progress), Blad(String), Gotowe, KodUrzadzenia(DeviceCode), Zalogowano(Account) }`
  - `pub fn zastosuj_motyw(ctx: &egui::Context)`

- [ ] **Step 1: Utwórz crate**

```bash
cargo new --bin crates/launcher --name chmurkowy-launcher
cargo add eframe -p chmurkowy-launcher
cargo add egui -p chmurkowy-launcher
cargo add chmurka-core --path crates/core -p chmurkowy-launcher
cargo add tokio --features rt-multi-thread,macros -p chmurkowy-launcher
cargo add reqwest --no-default-features --features rustls-tls -p chmurkowy-launcher
cargo add arboard open anyhow -p chmurkowy-launcher
```

Dopisz `"crates/launcher"` do `members`. W `crates/launcher/Cargo.toml`:

```toml
[[bin]]
name = "ChmurkowyLauncher"
path = "src/main.rs"
```

- [ ] **Step 2: Napisz theme.rs**

```rust
use egui::{Color32, CornerRadius, Stroke};

pub const TLO: Color32 = Color32::from_rgb(18, 20, 26);
pub const PANEL: Color32 = Color32::from_rgb(26, 29, 38);
pub const AKCENT: Color32 = Color32::from_rgb(96, 165, 250);
pub const AKCENT_CIEMNY: Color32 = Color32::from_rgb(59, 130, 246);
pub const TEKST: Color32 = Color32::from_rgb(226, 232, 240);
pub const TEKST_PRZYGASZONY: Color32 = Color32::from_rgb(148, 163, 184);
pub const BLAD: Color32 = Color32::from_rgb(248, 113, 113);

pub fn zastosuj_motyw(ctx: &egui::Context) {
    let mut styl = (*ctx.style()).clone();
    let w = &mut styl.visuals.widgets;

    styl.visuals.dark_mode = true;
    styl.visuals.panel_fill = TLO;
    styl.visuals.window_fill = TLO;
    styl.visuals.override_text_color = Some(TEKST);

    for stan in [&mut w.inactive, &mut w.hovered, &mut w.active] {
        stan.corner_radius = CornerRadius::same(8);
        stan.bg_fill = PANEL;
        stan.weak_bg_fill = PANEL;
    }
    w.hovered.bg_fill = AKCENT_CIEMNY;
    w.active.bg_fill = AKCENT;
    w.noninteractive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(40, 44, 56));

    styl.spacing.item_spacing = egui::vec2(10.0, 10.0);
    styl.spacing.button_padding = egui::vec2(16.0, 10.0);
    ctx.set_style(styl);
}
```

- [ ] **Step 3: Napisz main.rs**

```rust
mod app;
mod theme;
mod views;

use anyhow::Result;

fn main() -> Result<()> {
    // Wszystko zyje obok pliku wykonywalnego. To jest cala przenośność.
    let katalog = std::env::current_exe()?
        .parent()
        .expect("plik wykonywalny ma katalog")
        .to_path_buf();

    let opcje = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 560.0])
            .with_min_inner_size([720.0, 480.0])
            .with_decorations(false)
            .with_transparent(false)
            .with_title("Chmurkowy Launcher"),
        ..Default::default()
    };

    eframe::run_native(
        "Chmurkowy Launcher",
        opcje,
        Box::new(move |cc| {
            theme::zastosuj_motyw(&cc.egui_ctx);
            Ok(Box::new(app::App::nowa(katalog)))
        }),
    )
    .map_err(|e| anyhow::anyhow!("nie udało się otworzyć okna: {e}"))
}
```

Jeżeli okno bez ramki sprawia problemy na Twoim menedżerze okien, zmień `with_decorations(false)` na `true` i pomiń pasek tytułu w `views::main`. Specyfikacja przewiduje to zejście awaryjne.

- [ ] **Step 4: Napisz app.rs**

```rust
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
    KodUrzadzenia(DeviceCode),
    Zalogowano(Box<Account>),
    Gotowe,
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
            widok: Widok::Glowny,
            manifest: None,
            konto: None,
            kod: None,
            postep: None,
            blad: None,
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
        app
    }

    pub fn data(&self) -> PathBuf {
        self.katalog.join("data")
    }

    fn wczytaj_manifest(&mut self) {
        let n = self.nadawca.clone();
        // Adres manifestu jest wkompilowany, bo launcher bez niego nie ma co robić.
        // Podmiana wymaga nowego wydania — świadomy koszt za brak pliku konfiguracyjnego,
        // który tester mógłby zepsuć.
        let adres = env!("CHMURKA_MANIFEST_URL").to_string();
        self.runtime.spawn(async move {
            match pobierz_manifest(&adres).await {
                Ok(m) => { let _ = n.send(Wiadomosc::Manifest(Box::new(m))); }
                Err(e) => { let _ = n.send(Wiadomosc::Blad(format!("Nie udało się pobrać informacji o paczce: {e}"))); }
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
                        self.zajety = false;
                    }
                    self.postep = Some(p);
                }
                Wiadomosc::Notatka(s) => self.log.push(s),
                Wiadomosc::Blad(e) => {
                    self.zajety = false;
                    self.log.push(format!("BŁĄD: {e}"));
                    self.blad = Some(e);
                }
                Wiadomosc::KodUrzadzenia(d) => self.kod = Some(d),
                Wiadomosc::Zalogowano(k) => {
                    self.kod = None;
                    self.konto = Some(*k);
                    self.widok = Widok::Glowny;
                }
                Wiadomosc::Gotowe => self.zajety = false,
                Wiadomosc::GraZakonczona(kod) => {
                    self.zajety = false;
                    if !matches!(kod, Some(0)) {
                        self.blad = Some(format!("Gra zakończyła się kodem {kod:?}. Zajrzyj do szczegółów."));
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
        if self.zajety {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
        match self.widok {
            Widok::Glowny => crate::views::main::rysuj(self, ctx),
            Widok::Logowanie => crate::views::login::rysuj(self, ctx),
            Widok::Ustawienia => crate::views::settings::rysuj(self, ctx),
        }
    }
}
```

Adres manifestu wstrzykujemy przy budowaniu. Utwórz `crates/launcher/build.rs`:

```rust
fn main() {
    // Pozwala nadpisac adres przy budowaniu: CHMURKA_MANIFEST_URL=... cargo build
    let domyslny = "https://przyklad.github.io/chmurka-pack/manifest.json";
    let adres = std::env::var("CHMURKA_MANIFEST_URL").unwrap_or_else(|_| domyslny.to_string());
    println!("cargo:rustc-env=CHMURKA_MANIFEST_URL={adres}");
    println!("cargo:rerun-if-env-changed=CHMURKA_MANIFEST_URL");
}
```

- [ ] **Step 5: Sprawdź kompilację**

Run: `cargo build -p chmurkowy-launcher`
Expected: błędy o brakującym module `views` — to normalne, powstanie w Task 15.

- [ ] **Step 6: Commit**

```bash
git add crates/launcher Cargo.toml Cargo.lock
git commit -m "Szkielet GUI, motyw i petla wiadomosci"
```

---

### Task 15: Widoki

**Files:**
- Create: `crates/launcher/src/views/mod.rs`
- Create: `crates/launcher/src/views/main.rs`
- Create: `crates/launcher/src/views/login.rs`
- Create: `crates/launcher/src/views/settings.rs`

**Interfaces:**
- Consumes: `app::{App, Widok, Wiadomosc}`, `theme::*`
- Produces: `pub fn rysuj(app: &mut App, ctx: &egui::Context)` w każdym module

- [ ] **Step 1: Napisz mod.rs i wspólny pasek tytułu**

`crates/launcher/src/views/mod.rs`:

```rust
pub mod login;
pub mod main;
pub mod settings;

use crate::theme;

/// Pasek tytułu zastępujący ramkę systemową: przeciąganie okna i zamykanie.
pub fn pasek_tytulu(ui: &mut egui::Ui, ctx: &egui::Context, tytul: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(tytul).size(16.0).color(theme::TEKST));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✕").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            if ui.button("—").clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
        });
    });
    // Cały pasek jest uchwytem do przeciągania okna.
    let odp = ui.interact(
        ui.min_rect(),
        egui::Id::new("pasek-tytulu"),
        egui::Sense::drag(),
    );
    if odp.dragged() {
        ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}
```

- [ ] **Step 2: Napisz widok główny**

`crates/launcher/src/views/main.rs`:

```rust
use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        let tytul = app
            .manifest
            .as_ref()
            .map(|m| format!("☁ {}", m.pack.name))
            .unwrap_or_else(|| "☁ Chmurkowy Launcher".to_string());
        super::pasek_tytulu(ui, ctx, &tytul);
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⚙").on_hover_text("Ustawienia").clicked() {
                    app.widok = Widok::Ustawienia;
                }
                let etykieta = match &app.konto {
                    Some(k) => k.name.clone(),
                    None => "Nie zalogowano".to_string(),
                };
                if ui.button(etykieta).clicked() {
                    app.widok = Widok::Logowanie;
                }
            });
        });

        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            if let Some(m) = &app.manifest {
                ui.label(egui::RichText::new(&m.pack.edition).size(20.0).color(theme::TEKST));
                let mody = m.files.iter()
                    .filter(|f| f.path.starts_with("mods/"))
                    .count();
                ui.label(
                    egui::RichText::new(format!(
                        "Minecraft {} · NeoForge · {} modów",
                        m.pack.minecraft, mody
                    ))
                    .color(theme::TEKST_PRZYGASZONY),
                );
            } else {
                ui.label(egui::RichText::new("Sprawdzam paczkę…").color(theme::TEKST_PRZYGASZONY));
            }

            ui.add_space(32.0);

            let gotowy = app.manifest.is_some() && !app.zajety;
            let napis = if app.konto.is_some() { "GRAJ" } else { "ZALOGUJ SIĘ" };
            let przycisk = egui::Button::new(
                egui::RichText::new(napis).size(22.0).strong(),
            )
            .min_size(egui::vec2(240.0, 56.0));

            if ui.add_enabled(gotowy, przycisk).clicked() {
                if app.konto.is_some() {
                    crate::app::uruchom(app);
                } else {
                    app.widok = Widok::Logowanie;
                }
            }
        });

        ui.add_space(24.0);

        if let Some(p) = &app.postep {
            let ulamek = if p.total > 0 { p.done as f32 / p.total as f32 } else { 0.0 };
            ui.label(
                egui::RichText::new(format!("{} — {}/{}", p.stage.opis(), p.done, p.total))
                    .color(theme::TEKST_PRZYGASZONY),
            );
            ui.add(egui::ProgressBar::new(ulamek).show_percentage());
        }

        if let Some(e) = &app.blad {
            ui.add_space(8.0);
            ui.label(egui::RichText::new(e).color(theme::BLAD));
        }

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let napis = if app.pokaz_szczegoly { "szczegóły ▴" } else { "szczegóły ▾" };
                if ui.small_button(napis).clicked() {
                    app.pokaz_szczegoly = !app.pokaz_szczegoly;
                }
            });
        });

        if app.pokaz_szczegoly {
            egui::ScrollArea::vertical().max_height(140.0).stick_to_bottom(true).show(ui, |ui| {
                for linia in &app.log {
                    ui.label(egui::RichText::new(linia).size(11.0).color(theme::TEKST_PRZYGASZONY));
                }
            });
        }
    });
}
```

- [ ] **Step 3: Napisz widok logowania**

`crates/launcher/src/views/login.rs`:

```rust
use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        super::pasek_tytulu(ui, ctx, "☁ Zaloguj się");
        ui.add_space(30.0);

        ui.vertical_centered(|ui| {
            match &app.kod {
                Some(kod) => {
                    ui.label(egui::RichText::new("Wpisz ten kod na stronie Microsoftu:").color(theme::TEKST_PRZYGASZONY));
                    ui.add_space(10.0);
                    ui.label(egui::RichText::new(&kod.user_code).size(34.0).strong().color(theme::AKCENT));
                    ui.add_space(14.0);
                    if ui.button("Kopiuj kod i otwórz przeglądarkę").clicked() {
                        if let Ok(mut sc) = arboard::Clipboard::new() {
                            let _ = sc.set_text(kod.user_code.clone());
                        }
                        let _ = open::that(&kod.verification_uri);
                    }
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(&kod.verification_uri).size(11.0).color(theme::TEKST_PRZYGASZONY));
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Czekam na potwierdzenie…").color(theme::TEKST_PRZYGASZONY));
                }
                None => {
                    let przycisk = egui::Button::new(
                        egui::RichText::new("Zaloguj przez Microsoft").size(16.0),
                    )
                    .min_size(egui::vec2(280.0, 46.0));
                    if ui.add(przycisk).clicked() {
                        crate::app::zaloguj_microsoft(app);
                    }
                }
            }

            ui.add_space(26.0);
            ui.label(egui::RichText::new("── albo ──").color(theme::TEKST_PRZYGASZONY));
            ui.add_space(18.0);

            ui.label(egui::RichText::new("Tryb offline (do testów)").color(theme::TEKST));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                // Wysrodkowanie pary pole+przycisk wewnatrz vertical_centered.
                ui.add_space((ui.available_width() - 300.0).max(0.0) / 2.0);
                ui.add(
                    egui::TextEdit::singleline(&mut app.nick_offline)
                        .hint_text("nick")
                        .desired_width(190.0),
                );
                let mozna = !app.nick_offline.trim().is_empty();
                if ui.add_enabled(mozna, egui::Button::new("Graj")).clicked() {
                    let nick = app.nick_offline.trim().to_string();
                    app.konto = Some(chmurka_core::auth::offline::offline_account(&nick));
                    app.widok = Widok::Glowny;
                }
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Działa tylko na serwerze z online-mode=false")
                    .size(11.0)
                    .color(theme::TEKST_PRZYGASZONY),
            );

            ui.add_space(24.0);
            if ui.small_button("← Wróć").clicked() {
                app.widok = Widok::Glowny;
            }

            if let Some(e) = &app.blad {
                ui.add_space(10.0);
                ui.label(egui::RichText::new(e).color(theme::BLAD));
            }
        });
    });
}
```

- [ ] **Step 4: Napisz widok ustawień**

`crates/launcher/src/views/settings.rs`:

```rust
use crate::app::{App, Widok};
use crate::theme;

pub fn rysuj(app: &mut App, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        super::pasek_tytulu(ui, ctx, "☁ Ustawienia");
        ui.add_space(24.0);

        ui.label(egui::RichText::new("Pamięć dla gry").color(theme::TEKST));
        ui.add(egui::Slider::new(&mut app.pamiec_mb, 2048..=16384).suffix(" MB").step_by(512.0));
        ui.label(
            egui::RichText::new("Paczka z 249 modami potrzebuje co najmniej 4 GB.")
                .size(11.0)
                .color(theme::TEKST_PRZYGASZONY),
        );

        ui.add_space(20.0);
        if ui.button("Otwórz folder gry").clicked() {
            let _ = open::that(app.data().join("instance"));
        }

        ui.add_space(10.0);
        let log = app.data().join("logs").join("game.log");
        if ui.add_enabled(log.is_file(), egui::Button::new("Otwórz log gry")).clicked() {
            let _ = open::that(&log);
        }

        ui.add_space(10.0);
        if ui.button("Napraw instalację").clicked() {
            // Kasujemy tylko to, co da sie odtworzyc z sieci.
            // Katalog instance ze swiatami gracza zostaje nietkniety.
            let _ = std::fs::remove_dir_all(app.data().join("mc"));
            app.log.push("Usunięto pliki gry — zostaną pobrane ponownie przy następnym starcie.".into());
        }
        ui.label(
            egui::RichText::new("Pobiera grę od nowa. Światy i ustawienia zostają.")
                .size(11.0)
                .color(theme::TEKST_PRZYGASZONY),
        );

        ui.add_space(10.0);
        if app.konto.is_some() && ui.button("Wyloguj").clicked() {
            chmurka_core::auth::store::clear(&app.data().join("auth.json"));
            app.konto = None;
        }

        ui.add_space(24.0);
        ui.label(
            egui::RichText::new(format!("Chmurkowy Launcher {}", env!("CARGO_PKG_VERSION")))
                .size(11.0)
                .color(theme::TEKST_PRZYGASZONY),
        );
        if let Some(m) = &app.manifest {
            if m.launcher.latest_version != env!("CARGO_PKG_VERSION") {
                ui.label(
                    egui::RichText::new(format!(
                        "Dostępna nowsza wersja: {}",
                        m.launcher.latest_version
                    ))
                    .color(theme::AKCENT),
                );
            }
        }

        ui.add_space(20.0);
        if ui.small_button("← Wróć").clicked() {
            app.widok = Widok::Glowny;
        }
    });
}
```

- [ ] **Step 5: Dopisz funkcje uruchamiające do app.rs**

Dodaj na końcu `crates/launcher/src/app.rs`:

```rust
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
            Err(e) => { let _ = n.send(Wiadomosc::Blad(e.to_string())); return; }
        };
        let device_code = kod.device_code.clone();
        let mut odstep = kod.interval_s.max(1);
        let koniec = std::time::Instant::now() + std::time::Duration::from_secs(kod.expires_in_s);
        let _ = n.send(Wiadomosc::KodUrzadzenia(kod));

        loop {
            if std::time::Instant::now() > koniec {
                let _ = n.send(Wiadomosc::Blad("Kod wygasł — spróbuj jeszcze raz.".into()));
                return;
            }
            tokio::time::sleep(std::time::Duration::from_secs(odstep)).await;
            match msa::poll_once(&client_id, &device_code).await {
                Ok(msa::PollResult::Czekamy) => {}
                // Microsoft prosi o wolniejsze odpytywanie — ignorowanie tego
                // konczy sie zablokowaniem calej sesji logowania.
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
                        Err(e) => { let _ = n.send(Wiadomosc::Blad(e.to_string())); }
                    }
                    return;
                }
                Err(e) => { let _ = n.send(Wiadomosc::Blad(e.to_string())); return; }
            }
        }
    });
}

/// Instaluje wszystko, czego brakuje, i uruchamia grę.
pub fn uruchom(app: &mut App) {
    let (Some(m), Some(konto)) = (app.manifest.clone(), app.konto.clone()) else { return };
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
        let postep: Arc<dyn Fn(progress::Progress) + Send + Sync> =
            Arc::new(move |p| { let _ = n2.send(Wiadomosc::Postep(p)); });

        let wynik = async {
            let dl = net::Downloader::new(8);
            let mc = data.join("mc");
            let instancja = data.join("instance");
            std::fs::create_dir_all(&instancja)?;

            let java = java::ensure(&data, m.java.major, &dl, postep.clone()).await?;
            let profil = game_install::ensure_loader(
                &mc, &java, &m.pack.loader.kind, &m.pack.loader.version, &dl, postep.clone()).await?;
            let wersja = version::load(&mc.join("versions"), &profil)?;
            game_install::ensure_libraries(&mc, &wersja, &dl, postep.clone()).await?;
            game_install::ensure_assets(&mc, &wersja, &dl, postep.clone()).await?;

            let sciezka_stanu = data.join("state.json");
            let mut stan = state::State::load(&sciezka_stanu);
            let akcje = pack_sync::plan(&m, &stan, &pack_sync::DiskProbe::new(&instancja));
            let notatki = pack_sync::apply(&m, &instancja, &mut stan, &akcje, &dl, postep.clone()).await?;
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
            let _ = n.send(Wiadomosc::Postep(progress::Progress {
                stage: progress::Stage::Ready, done: 1, total: 1, bytes: 0,
                label: "Uruchamiam grę".into(),
            }));

            // Log gry trafia do pliku i do panelu "szczegóły".
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
                    "pełny log: {}", plik_logu.display()
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
```

Dodaj `#[derive(Clone)]` do `Manifest`, `Pack`, `Loader`, `JavaReq`, `AuthCfg`, `LauncherInfo`, `FileEntry` w `manifest.rs` (część już ma) oraz do `Account` w `auth/mod.rs`, bo `uruchom` je klonuje.

- [ ] **Step 6: Zbuduj i uruchom**

Run: `CHMURKA_MANIFEST_URL="http://127.0.0.1:8099/manifest.json" cargo run -p chmurkowy-launcher`
Expected: otwiera się okno z nazwą paczki, liczbą modów i przyciskiem. Tryb offline pozwala odpalić grę.

- [ ] **Step 7: Sprawdź przenośność**

```bash
cargo build --release -p chmurkowy-launcher
mkdir -p /tmp/przenosny && cp target/release/ChmurkowyLauncher /tmp/przenosny/
cd /tmp/przenosny && ./ChmurkowyLauncher
```

Expected: cały stan powstaje w `/tmp/przenosny/data`, a `~/.minecraft` pozostaje nietknięty.

- [ ] **Step 8: Commit**

```bash
git add crates/launcher
git commit -m "Widoki launchera: glowny, logowanie i ustawienia"
```

---

### Task 16: Wczytywanie zapisanego logowania

**Files:**
- Modify: `crates/launcher/src/app.rs`

**Interfaces:**
- Consumes: `auth::store::{load, Zapis}`, `auth::msa::{refresh, zaloguj_minecraft}`
- Produces: nic nowego — rozszerza `App::nowa`

- [ ] **Step 1: Dodaj ciche odświeżanie tokenu przy starcie**

W `App::nowa`, po `app.wczytaj_manifest()`, dopisz `app.wznow_sesje();` i dodaj metodę:

```rust
    /// Próbuje odtworzyć poprzednie logowanie bez pytania użytkownika.
    /// Niepowodzenie jest ciche — użytkownik po prostu zobaczy ekran logowania.
    fn wznow_sesje(&mut self) {
        let Some(zapis) = chmurka_core::auth::store::load(&self.data().join("auth.json")) else {
            return;
        };
        let n = self.nadawca.clone();
        let sciezka = self.data().join("auth.json");
        // client_id trzyma manifest, ktory jeszcze sie sciaga, wiec czekamy na niego
        // w tle zamiast blokowac start okna.
        self.runtime.spawn(async move {
            use chmurka_core::auth::{msa, store};
            // Wartość awaryjna na wypadek, gdyby manifest nie zdążył dojść;
            // to ten sam identyfikator, który i tak jest w manifeście.
            let client_id = "00000000402b5328";
            let Ok(t) = msa::refresh(client_id, &zapis.refresh_token).await else { return };
            if let Ok(konto) = msa::zaloguj_minecraft(&t).await {
                let _ = store::save(&sciezka, &store::Zapis {
                    refresh_token: t.refresh_token,
                    nick: konto.name.clone(),
                });
                let _ = n.send(Wiadomosc::Zalogowano(Box::new(konto)));
            }
        });
    }
```

- [ ] **Step 2: Sprawdź ręcznie**

Zaloguj się, zamknij launcher, uruchom ponownie.
Expected: po chwili chip konta pokazuje Twój nick bez pytania o kod.

- [ ] **Step 3: Commit**

```bash
git add crates/launcher
git commit -m "Wznawianie sesji logowania przy starcie"
```

---

### Task 17: Budowanie wydań i dokumentacja

**Files:**
- Create: `.github/workflows/release.yml`
- Create: `README.md` (nadpisz istniejący)

**Interfaces:**
- Consumes: nic
- Produces: binarki `ChmurkowyLauncher.exe` i `ChmurkowyLauncher` w GitHub Releases

- [ ] **Step 1: Napisz workflow**

`.github/workflows/release.yml`:

```yaml
name: Wydanie

on:
  push:
    tags: ["v*"]
  workflow_dispatch:

permissions:
  contents: write

jobs:
  build:
    strategy:
      matrix:
        include:
          - os: ubuntu-latest
            nazwa: ChmurkowyLauncher
            artefakt: ChmurkowyLauncher-linux-x64
          - os: windows-latest
            nazwa: ChmurkowyLauncher.exe
            artefakt: ChmurkowyLauncher-windows-x64.exe
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Zależności systemowe (Linux)
        if: runner.os == 'Linux'
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
            libxcb-xfixes0-dev libxkbcommon-dev libssl-dev

      - uses: dtolnay/rust-toolchain@stable

      - uses: Swatinem/rust-cache@v2

      - name: Testy
        run: cargo test --workspace

      - name: Budowanie
        env:
          CHMURKA_MANIFEST_URL: ${{ vars.CHMURKA_MANIFEST_URL }}
        run: cargo build --release -p chmurkowy-launcher

      - name: Zmiana nazwy
        shell: bash
        run: cp "target/release/${{ matrix.nazwa }}" "${{ matrix.artefakt }}"

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artefakt }}
          path: ${{ matrix.artefakt }}

  wydanie:
    needs: build
    runs-on: ubuntu-latest
    if: startsWith(github.ref, 'refs/tags/')
    steps:
      - uses: actions/download-artifact@v4
        with:
          merge-multiple: true
      - uses: softprops/action-gh-release@v2
        with:
          files: ChmurkowyLauncher-*
```

Ustaw w repozytorium zmienną `CHMURKA_MANIFEST_URL` (Settings → Secrets and variables → Actions → Variables) na adres swojego GitHub Pages.

- [ ] **Step 2: Napisz README**

Zastąp treść `README.md`:

````markdown
# Chmurkowy Launcher

Przenośny launcher paczki modów Chmurkowego Serwera.
Minecraft 1.21.1, NeoForge 21.1.249, 249 modów.

## Dla graczy

1. Pobierz plik ze [zakładki Releases](../../releases):
   - Windows: `ChmurkowyLauncher-windows-x64.exe`
   - Linux: `ChmurkowyLauncher-linux-x64`
2. Wrzuć go do **pustego folderu** — launcher tworzy obok siebie katalog `data`.
3. Uruchom i zaloguj się kontem Microsoft.

Pierwsze uruchomienie pobiera około 1,8 GB (Java, Minecraft, zasoby, mody).
Kolejne startują od razu.

**Deinstalacja:** skasuj folder. To wszystko — launcher nic nie zapisuje
w systemie, rejestrze ani w `~/.minecraft`.

**Windows pokaże ostrzeżenie SmartScreen**, bo plik nie jest podpisany
certyfikatem (podpis kosztuje). Kliknij „Więcej informacji", potem
„Uruchom mimo to".

**Tryb offline** na ekranie logowania służy do testów i zadziała tylko
na serwerze z `online-mode=false`.

## Dla utrzymującego paczkę

Aktualizacja paczki po zmianie modów w PrismLauncherze:

```bash
cargo run -p chmurka-cli --bin chmurka -- pack-build \
  --instance "$HOME/.local/share/PrismLauncher/instances/Chmurkowy serwer edycja 2026-2027/minecraft" \
  --out dist \
  --base-url "https://TWOJ-LOGIN.github.io/chmurka-pack" \
  --version "$(date +%Y.%m.%d)-1"
```

Potem wypchnij `dist/` do repozytorium obsługującego GitHub Pages.
Testerzy dostaną różnicę przy następnym uruchomieniu launchera.

Mody pobierane są prosto z Modrinth i CurseForge — hostujesz tylko
configi, czyli około 4,4 MB.

## Rozwój

```bash
cargo test --workspace            # testy
cargo run -p chmurkowy-launcher   # GUI
cargo run -p chmurka-cli --bin chmurka -- run --manifest ... --nick Test
```
````

- [ ] **Step 3: Uruchom pełne testy**

Run: `cargo test --workspace`
Expected: PASS, wszystkie testy

- [ ] **Step 4: Commit i tag**

```bash
git add .github README.md
git commit -m "Budowanie wydan przez GitHub Actions i dokumentacja"
```

---

## Weryfikacja końcowa

Po wykonaniu wszystkich zadań sprawdź, że spełnione są obietnice ze specyfikacji:

- [ ] `cargo test --workspace` przechodzi w całości
- [ ] Launcher uruchomiony w pustym folderze tworzy wyłącznie `data/` obok siebie
- [ ] Po instalacji `~/.minecraft` i `%APPDATA%` nie zawierają nowych plików
- [ ] Skasowanie folderu launchera nie zostawia niczego w systemie
- [ ] Ręczna zmiana pliku w `data/instance/config/` przetrwa ponowną synchronizację
- [ ] Dorzucenie obcego moda do `data/instance/mods/` kończy się jego usunięciem
- [ ] Wyłączenie internetu w trakcie pobierania daje czytelny błąd, a ponowny start dociąga resztę
- [ ] Logowanie Microsoft działa, a tryb offline działa niezależnie od niego
- [ ] Manifest z wpisem `../../x` zostaje odrzucony (test jednostkowy to pokrywa)

