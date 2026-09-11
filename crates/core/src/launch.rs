use crate::auth::Account;
use crate::java::{biezacy_os, Os};
use crate::version::{rules_allow, rules_allow_z, substitute, Arg, Mozliwosci, VersionJson};
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
    /// Dodatkowe parametry Javy wpisane przez gracza w ustawieniach.
    pub dodatkowe: &'a [String],
    /// Rozmiar okna gry, jeśli gracz go narzucił. `None` zostawia decyzję
    /// Minecraftowi, który pamięta ją w swoim `options.txt`.
    pub okno: Option<(u32, u32)>,
    /// Start na pełnym ekranie.
    pub pelny_ekran: bool,
    /// Zmienne środowiskowe dokładane do procesu gry.
    pub srodowisko: &'a [(String, String)],
    /// Program opakowujący (`gamemoderun`, `prime-run`) wraz z argumentami.
    /// Wstawia się przed ścieżką do Javy.
    pub opakowanie: &'a [String],
}

impl<'a> LaunchParams<'a> {
    /// Parametry bez żadnego z ustawień zaawansowanych.
    ///
    /// Jest po to, żeby dołożenie kolejnego pola nie wymagało ruszania
    /// każdego miejsca, które buduje komendę — a przy okazji mówi wprost,
    /// jak wygląda uruchomienie „jak dawniej".
    pub fn zwykle(
        java: &'a Path,
        mc_dir: &'a Path,
        game_dir: &'a Path,
        version: &'a VersionJson,
        account: &'a Account,
        min_mb: u32,
        max_mb: u32,
    ) -> Self {
        Self {
            java,
            mc_dir,
            game_dir,
            version,
            account,
            min_mb,
            max_mb,
            dodatkowe: &[],
            okno: None,
            pelny_ekran: false,
            srodowisko: &[],
            opakowanie: &[],
        }
    }
}

pub fn separator(os: Os) -> &'static str {
    if os == Os::Windows {
        ";"
    } else {
        ":"
    }
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

    // Profil wersji pyta warunkowo: „doklej --width i --height, jeśli launcher
    // obsługuje własną rozdzielczość". Odpowiadamy zgodnie z tym, czy gracz
    // rzeczywiście narzucił rozmiar okna.
    let mozliwosci = Mozliwosci {
        wlasna_rozdzielczosc: p.okno.is_some(),
    };

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
    zmienne.insert(
        "assets_root".into(),
        p.mc_dir.join("assets").display().to_string(),
    );
    zmienne.insert(
        "game_assets".into(),
        p.mc_dir.join("assets").display().to_string(),
    );
    zmienne.insert("assets_index_name".into(), indeks);
    zmienne.insert(
        "library_directory".into(),
        p.mc_dir.join("libraries").display().to_string(),
    );
    zmienne.insert("classpath_separator".into(), separator(os).into());
    zmienne.insert("classpath".into(), build_classpath(p.version, p.mc_dir, os));
    zmienne.insert(
        "natives_directory".into(),
        p.mc_dir
            .join("natives")
            .join(&p.version.id)
            .display()
            .to_string(),
    );
    zmienne.insert("launcher_name".into(), "ChmurkowyLauncher".into());
    zmienne.insert("launcher_version".into(), env!("CARGO_PKG_VERSION").into());
    let (szer, wys) = p.okno.unwrap_or((854, 480));
    zmienne.insert("resolution_width".into(), szer.to_string());
    zmienne.insert("resolution_height".into(), wys.to_string());

    // Opakowanie wchodzi na sam początek komendy, a Java staje się jego
    // argumentem: `gamemoderun /ścieżka/java -Xmx… net.minecraft…`.
    //
    // Celowo NIE przez powłokę. Komenda gry ma kilkaset argumentów, w tym
    // cały classpath i ścieżki ze spacjami — sklejenie ich z powrotem w jeden
    // łańcuch dla powłoki znaczyłoby cytowanie każdego z osobna i pierwsza
    // pomyłka kończy się grą, która nie startuje.
    let mut cmd = match p.opakowanie.split_first() {
        Some((program, reszta)) => {
            let mut c = std::process::Command::new(program);
            c.args(reszta);
            c.arg(p.java);
            c
        }
        None => std::process::Command::new(p.java),
    };

    for (nazwa, wartosc) in p.srodowisko {
        cmd.env(nazwa, wartosc);
    }

    // Gdy gracz sam podał -Xmx, nasz suwak ustępuje. Dwa -Xmx w jednej komendzie
    // są legalne (wygrywa ostatni), ale mylące przy diagnozowaniu problemów.
    if !p.dodatkowe.iter().any(|a| a.starts_with("-Xmx")) {
        cmd.arg(format!("-Xms{}M", p.min_mb));
        cmd.arg(format!("-Xmx{}M", p.max_mb));
    }

    // Celowo NIE dokładamy tu własnych nastaw odśmiecacza ani limitu
    // metaspace.
    //
    // Próbowaliśmy: `-XX:MaxMetaspaceSize=512M` miało powstrzymać obszar,
    // który przy tylu modach rośnie i rośnie. Skutek był taki, że przy 349
    // plikach modów gra dobijała do limitu **w trakcie rozgrywki** — klasy
    // wczytują się leniwie, więc pierwszy nowy potwór albo nowa struktura
    // przy generowaniu terenu kończyły się `OutOfMemoryError: Metaspace`.
    // Gra nie padała od razu, tylko wchodziła w korkociąg pełnych zbiórek:
    // kolejne ticki serwera trwały 40, 80 i 120 sekund, obraz stawał na
    // jednej klatce i zostawało tylko ubicie procesu.
    //
    // Metaspace ma rosnąć tyle, ile trzeba — ogranicza go pamięć systemu,
    // a nie my. Cel pauzy odśmiecacza też zostawiamy Javie: domyślne
    // ustawienia działały tu bez zarzutu, a nasze zgadywanie tylko zaszkodziło.
    //
    // Gracz, który wie, co robi, nadal może podać swoje parametry
    // w Ustawieniach — trafią do komendy nietknięte.
    for a in p.dodatkowe {
        cmd.arg(a);
    }

    for a in rozwin(&p.version.arguments.jvm, os, &zmienne, &mozliwosci) {
        cmd.arg(a);
    }
    cmd.arg(&p.version.main_class);
    for a in rozwin(&p.version.arguments.game, os, &zmienne, &mozliwosci) {
        cmd.arg(a);
    }
    // `--fullscreen` to bezargumentowa flaga przyjmowana przez samą grę,
    // a nie argument z profilu wersji — profil w ogóle o niej nie wspomina,
    // więc dokładamy ją tutaj.
    if p.pelny_ekran {
        cmd.arg("--fullscreen");
    }
    cmd.current_dir(p.game_dir);
    Ok(cmd)
}

fn rozwin(
    argi: &[Arg],
    os: Os,
    zmienne: &BTreeMap<String, String>,
    moz: &Mozliwosci,
) -> Vec<String> {
    let mut out = Vec::new();
    for a in argi {
        match a {
            Arg::Prosty(s) => out.push(substitute(s, zmienne)),
            Arg::Warunkowy { rules, value } => {
                if rules_allow_z(rules, os, moz) {
                    for v in value.lista() {
                        out.push(substitute(&v, zmienne));
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Account, AccountKind};

    fn wersja() -> VersionJson {
        serde_json::from_str(
            r#"{
            "id":"neoforge-21.1.249",
            "mainClass":"cpw.mods.bootstraplauncher.BootstrapLauncher",
            "arguments":{
              "game":["--username","${auth_player_name}","--uuid","${auth_uuid}",
                      "--accessToken","${auth_access_token}","--userType","${user_type}",
                      "--gameDir","${game_directory}",
                      {"rules":[{"action":"allow","features":{"is_demo_user":true}}],"value":"--demo"},
                      {"rules":[{"action":"allow","features":{"has_custom_resolution":true}}],
                       "value":["--width","${resolution_width}","--height","${resolution_height}"]},
                      {"rules":[{"action":"allow","features":{"is_quick_play_singleplayer":true}}],
                       "value":["--quickPlaySingleplayer","${quickPlaySingleplayer}"]},
                      {"rules":[{"action":"allow","features":{"is_quick_play_multiplayer":true}}],
                       "value":["--quickPlayMultiplayer","${quickPlayMultiplayer}"]}],
              "jvm":["-DlibraryDirectory=${library_directory}","-cp","${classpath}"]
            },
            "libraries":[
              {"name":"a:b:1","downloads":{"artifact":{"path":"a/b/1/b-1.jar","sha1":"x","size":1,"url":"https://x"}}},
              {"name":"c:d:1","downloads":{"artifact":{"path":"c/d/1/d-1.jar","sha1":"y","size":1,"url":"https://y"}},
               "rules":[{"action":"allow","os":{"name":"osx"}}]}
            ],
            "assets":"17",
            "assetIndex":{"id":"17","sha1":"a","size":1,"totalSize":1,"url":"https://x/17.json"}
        }"#,
        )
        .unwrap()
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
        let cmd = build_command(&LaunchParams::zwykle(
            std::path::Path::new("/tmp/java"),
            mc,
            gra,
            &v,
            &k,
            512,
            4096,
        ))
        .unwrap();

        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert!(args.contains(&"Tomasz".to_string()));
        assert!(args.contains(&"61a50080-80aa-3842-8df5-cd674d3a57f2".to_string()));
        assert!(args.contains(&"legacy".to_string()));
        assert!(args.contains(&"-Xmx4096M".to_string()));
        assert!(args.contains(&"-Xms512M".to_string()));
        assert!(args.contains(&"cpw.mods.bootstraplauncher.BootstrapLauncher".to_string()));
        assert!(
            !args.iter().any(|a| a.contains("${")),
            "wszystkie zmienne musza byc podstawione"
        );
    }

    fn argumenty(dodatkowe: &[String]) -> Vec<String> {
        let v = wersja();
        let k = konto();
        let cmd = build_command(&LaunchParams {
            dodatkowe,
            ..podstawa(&v, &k)
        })
        .unwrap();
        cmd.get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect()
    }

    /// Parametry, od ktorych zaczyna kazdy test — zmieniamy w nich tylko to,
    /// co dany test sprawdza.
    fn podstawa<'a>(v: &'a VersionJson, k: &'a Account) -> LaunchParams<'a> {
        LaunchParams::zwykle(
            std::path::Path::new("/tmp/java"),
            std::path::Path::new("/tmp/mc"),
            std::path::Path::new("/tmp/gra"),
            v,
            k,
            512,
            4096,
        )
    }

    /// Domyslnie nie narzucamy grze niczego — okno zostaje takie, jakie
    /// Minecraft zapamietal w swoim options.txt.
    #[test]
    fn bez_wlasnego_rozmiaru_gra_nie_dostaje_width_ani_height() {
        let a = argumenty(&[]);
        for zakazany in ["--width", "--height", "--fullscreen"] {
            assert!(
                !a.iter().any(|x| x == zakazany),
                "{zakazany} nie ma prawa trafic do komendy bez ustawienia: {a:?}"
            );
        }
    }

    /// Wlasna rozdzielczosc byla w profilu wersji od zawsze, tylko
    /// odpowiadalismy „nie umiem" na pytanie o has_custom_resolution.
    #[test]
    fn wlasny_rozmiar_okna_trafia_do_komendy() {
        let v = wersja();
        let k = konto();
        let cmd = build_command(&LaunchParams {
            okno: Some((1920, 1080)),
            ..podstawa(&v, &k)
        })
        .unwrap();
        let a: Vec<String> = cmd
            .get_args()
            .map(|x| x.to_string_lossy().to_string())
            .collect();

        let i = a.iter().position(|x| x == "--width").expect("brak --width");
        assert_eq!(a[i + 1], "1920");
        let j = a
            .iter()
            .position(|x| x == "--height")
            .expect("brak --height");
        assert_eq!(a[j + 1], "1080");
        assert!(
            !a.iter().any(|x| x.contains("${")),
            "zmienne rozdzielczosci musza byc podstawione: {a:?}"
        );
    }

    /// Najwazniejszy strazak przy wlaczaniu `features`: odblokowanie wlasnej
    /// rozdzielczosci nie moze przepuscic Quick Play ani trybu demo. To one
    /// witaly gracza ekranem „Failed to Quick Play — Could not find world".
    #[test]
    fn wlasna_rozdzielczosc_nie_odblokowuje_quick_play_ani_demo() {
        let v = wersja();
        let k = konto();
        let cmd = build_command(&LaunchParams {
            okno: Some((1280, 720)),
            ..podstawa(&v, &k)
        })
        .unwrap();
        let a: Vec<String> = cmd
            .get_args()
            .map(|x| x.to_string_lossy().to_string())
            .collect();
        for zakazany in [
            "--demo",
            "--quickPlaySingleplayer",
            "--quickPlayMultiplayer",
            "--quickPlayPath",
            "--quickPlayRealms",
        ] {
            assert!(
                !a.iter().any(|x| x == zakazany),
                "{zakazany} nie ma prawa trafic do komendy: {a:?}"
            );
        }
    }

    /// `--fullscreen` to prawdziwa, bezargumentowa flaga przyjmowana przez
    /// `net.minecraft.client.main.Main` (sprawdzone na bajtkodzie 1.21.1).
    /// Profil wersji o niej nie wspomina, wiec dokladamy ja sami.
    #[test]
    fn pelny_ekran_dokleja_bezargumentowa_flage() {
        let v = wersja();
        let k = konto();
        let cmd = build_command(&LaunchParams {
            pelny_ekran: true,
            ..podstawa(&v, &k)
        })
        .unwrap();
        let a: Vec<String> = cmd
            .get_args()
            .map(|x| x.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            a.iter().filter(|x| *x == "--fullscreen").count(),
            1,
            "dokladnie raz: {a:?}"
        );
        assert_eq!(
            a.last().map(String::as_str),
            Some("--fullscreen"),
            "flaga bezargumentowa nie moze rozdzielic pary argument-wartosc"
        );
    }

    /// Opakowanie staje sie programem, a Java jego pierwszym argumentem.
    #[test]
    fn opakowanie_stoi_przed_java() {
        let v = wersja();
        let k = konto();
        let opakowanie = vec!["gamemoderun".to_string()];
        let cmd = build_command(&LaunchParams {
            opakowanie: &opakowanie,
            ..podstawa(&v, &k)
        })
        .unwrap();

        assert_eq!(cmd.get_program().to_string_lossy(), "gamemoderun");
        let a: Vec<String> = cmd
            .get_args()
            .map(|x| x.to_string_lossy().to_string())
            .collect();
        assert_eq!(a.first().map(String::as_str), Some("/tmp/java"));
        // Reszta komendy zostaje nietknieta.
        assert!(a.contains(&"-Xmx4096M".to_string()));
    }

    #[test]
    fn opakowanie_z_wlasnymi_argumentami() {
        let v = wersja();
        let k = konto();
        let opakowanie = vec!["env".to_string(), "DRI_PRIME=1".to_string()];
        let cmd = build_command(&LaunchParams {
            opakowanie: &opakowanie,
            ..podstawa(&v, &k)
        })
        .unwrap();
        assert_eq!(cmd.get_program().to_string_lossy(), "env");
        let a: Vec<String> = cmd
            .get_args()
            .map(|x| x.to_string_lossy().to_string())
            .collect();
        assert_eq!(a[0], "DRI_PRIME=1");
        assert_eq!(a[1], "/tmp/java");
    }

    /// Bez pustego opakowania nadal uruchamiamy Jave wprost — puste pole
    /// w Ustawieniach nie moze zmienic niczego.
    #[test]
    fn puste_opakowanie_niczego_nie_zmienia() {
        let v = wersja();
        let k = konto();
        let cmd = build_command(&LaunchParams {
            opakowanie: &[],
            ..podstawa(&v, &k)
        })
        .unwrap();
        assert_eq!(cmd.get_program().to_string_lossy(), "/tmp/java");
    }

    #[test]
    fn zmienne_srodowiskowe_trafiaja_do_procesu_gry() {
        let v = wersja();
        let k = konto();
        let srodowisko = vec![("MESA_GL_VERSION_OVERRIDE".to_string(), "4.6".to_string())];
        let cmd = build_command(&LaunchParams {
            srodowisko: &srodowisko,
            ..podstawa(&v, &k)
        })
        .unwrap();
        let znalezione: Vec<(String, Option<String>)> = cmd
            .get_envs()
            .map(|(n, w)| {
                (
                    n.to_string_lossy().to_string(),
                    w.map(|x| x.to_string_lossy().to_string()),
                )
            })
            .collect();
        assert_eq!(
            znalezione,
            vec![(
                "MESA_GL_VERSION_OVERRIDE".to_string(),
                Some("4.6".to_string())
            )]
        );
    }

    #[test]
    fn wlasne_parametry_trafiaja_do_komendy() {
        let a = argumenty(&["-XX:+UseG1GC".to_string(), "-Dfoo=bar".to_string()]);
        assert!(a.contains(&"-XX:+UseG1GC".to_string()));
        assert!(a.contains(&"-Dfoo=bar".to_string()));
        // Bez wlasnego -Xmx suwak nadal dziala.
        assert!(a.contains(&"-Xmx4096M".to_string()));
    }

    /// Limit metaspace wywrocil rozgrywke na maszynie testera: przy 349
    /// plikach modow gra dobijala do 512 MB w trakcie gry, przy generowaniu
    /// terenu, i wchodzila w korkociag pelnych zbiorek — ticki po 40, 80
    /// i 120 sekund, jedna klatka na sekunde, koniec przez ubicie procesu.
    /// Metaspace ma rosnac tyle, ile trzeba.
    #[test]
    fn nie_ograniczamy_metaspace() {
        let a = argumenty(&[]);
        assert!(
            !a.iter().any(|x| x.starts_with("-XX:MaxMetaspaceSize")),
            "limit metaspace zawiesza gre przy tej liczbie modow: {a:?}"
        );
    }

    /// Domyslne nastawy odsmiecacza dzialaly tu bez zarzutu. Nasze wlasne
    /// byly zgadywaniem pod maszyne, ktorej problem lezal zupelnie gdzie
    /// indziej — i tylko zaszkodzily.
    #[test]
    fn nie_narzucamy_wlasnych_nastaw_odsmiecacza() {
        let a = argumenty(&[]);
        for zakazana in [
            "-XX:MaxGCPauseMillis",
            "-XX:InitiatingHeapOccupancyPercent",
            "-XX:G1HeapRegionSize",
        ] {
            assert!(
                !a.iter().any(|x| x.starts_with(zakazana)),
                "{zakazana} nie ma prawa trafic do komendy: {a:?}"
            );
        }
    }

    /// Wlasne parametry gracza nadal maja przechodzic nietkniete.
    #[test]
    fn wlasne_nastawy_gracza_przechodza() {
        let a = argumenty(&["-XX:MaxMetaspaceSize=1G".to_string()]);
        assert!(a.contains(&"-XX:MaxMetaspaceSize=1G".to_string()));
    }

    #[test]
    fn wlasny_xmx_wypiera_suwak() {
        let a = argumenty(&["-Xmx8G".to_string()]);
        assert!(a.contains(&"-Xmx8G".to_string()));
        assert!(
            !a.iter().any(|x| x == "-Xmx4096M"),
            "nie moze byc dwoch -Xmx naraz"
        );
        assert!(
            !a.iter().any(|x| x.starts_with("-Xms")),
            "gdy gracz zarzadza sterta, nie dokladamy wlasnego -Xms"
        );
    }

    #[test]
    fn nie_przekazujemy_quick_play_ani_trybu_demo() {
        // Argumenty warunkowe z profilu vanilla byly doklejane do komendy, bo
        // reguly sprawdzalismy tylko po systemie. Gra witala wtedy gracza
        // ekranem „Failed to Quick Play — Could not find world”.
        let a = argumenty(&[]);
        for zakazany in [
            "--demo",
            "--quickPlaySingleplayer",
            "--quickPlayMultiplayer",
            "--quickPlayPath",
            "--quickPlayRealms",
        ] {
            assert!(
                !a.iter().any(|x| x == zakazany),
                "{zakazany} nie ma prawa trafic do komendy: {a:?}"
            );
        }
        // Nierozwiniete zmienne tez nie moga przejsc — to one powodowaly blad.
        assert!(
            !a.iter().any(|x| x.contains("${")),
            "w komendzie zostala nierozwinieta zmienna: {a:?}"
        );
    }

    #[test]
    fn classpath_pomija_libki_dla_innego_systemu() {
        let v = wersja();
        let cp = build_classpath(&v, std::path::Path::new("/tmp/mc"), Os::Linux);
        assert!(cp.contains("b-1.jar"));
        assert!(
            !cp.contains("d-1.jar"),
            "libka tylko dla macOS nie moze trafic na classpath Linuksa"
        );
    }

    #[test]
    fn separator_classpatha_zalezy_od_systemu() {
        assert_eq!(separator(Os::Linux), ":");
        assert_eq!(separator(Os::Windows), ";");
    }
}
