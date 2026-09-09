fn main() {
    // Adres manifestu jest wkompilowany, bo launcher bez niego nie ma co robić,
    // a plik konfiguracyjny obok binarki tester mógłby zepsuć albo zgubić.
    // Podmiana przy budowaniu: CHMURKA_MANIFEST_URL=... cargo build
    let domyslny = "https://przyklad.github.io/chmurka-pack/manifest.json";
    let adres = std::env::var("CHMURKA_MANIFEST_URL").unwrap_or_else(|_| domyslny.to_string());
    println!("cargo:rustc-env=CHMURKA_MANIFEST_URL={adres}");
    println!("cargo:rerun-if-env-changed=CHMURKA_MANIFEST_URL");
}
