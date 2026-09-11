# Od kliknięcia GRAJ do gry

Ta strona opisuje jedną ścieżkę w kodzie: od naciśnięcia przycisku GRAJ do
procesu Minecrafta, który działa i którego launcher pilnuje aż do zamknięcia.
Wszystko, co tu opisane, da się przeczytać w trzech funkcjach w
`crates/launcher/src/app.rs` — `uruchom`, `odpal_z_samonaprawa`
i `przygotuj_i_odpal` — oraz w modułach `crates/core`, które one wołają.

Podział jest taki: launcher decyduje **kiedy** i **w jakiej kolejności**,
a `chmurka-core` wie **jak**. W `app.rs` nie ma pobierania plików ani składania
argumentów Javy, tylko wywołania po kolei i obsługa tego, co poszło nie tak.

## Skrót: kolejność wywołań

| Krok | Funkcja | Plik | Co powstaje na dysku |
| --- | --- | --- | --- |
| 1 | `java::ensure` albo `wlasna_java` | `core/src/java.rs`, `launcher/src/app.rs` | `data/java/<major>/…` |
| 2 | `game_install::ensure_loader` | `core/src/game_install.rs` | `data/mc/versions/<profil>/<profil>.json` |
| 3 | `version::load` | `core/src/version.rs` | nic — czyta z dysku |
| 4 | `game_install::ensure_libraries` | `core/src/game_install.rs` | `data/mc/libraries/…`, `data/mc/versions/<id>/<id>.jar` |
| 5 | `game_install::ensure_assets` | `core/src/game_install.rs` | `data/mc/assets/indexes/…`, `data/mc/assets/objects/…` |
| 6 | `pack_sync::plan` i `pack_sync::apply` | `core/src/pack_sync.rs` | `data/instance/…`, `data/state.json` |
| 7 | `narzedzia::zapewnij` | `core/src/narzedzia.rs` | katalogi narzędzi w `data/instance/` |
| 8 | `komendy::uruchom` (etap `Przed`) | `core/src/komendy.rs` | zależy od gracza |
| 9 | `launch::build_command` | `core/src/launch.rs` | nic — składa `std::process::Command` |
| 10 | `spawn` i oczekiwanie na proces | `launcher/src/app.rs` | `data/logs/game.log` |

Ścieżki są podane względem katalogu danych. Gdzie on leży na poszczególnych
systemach, opisuje [Gdzie leżą dane](dane.md).

## Kliknięcie GRAJ

`uruchom` w `app.rs` nie robi nic samo z siebie, dopóki nie ma kompletu:

```rust
let (Some(m), Some(konto)) = (app.manifest.clone(), app.konto.clone()) else {
    return;
};
```

Bez manifestu nie wiadomo, co instalować, bez konta nie ma czego podstawić
w argumenty gry. Dalej funkcja kasuje poprzedni błąd, ustawia `zajety = true`
(to wygasza przycisk), czyści log i przekazuje robotę do zadania w tle.

Postęp wraca do wątku rysującego kanałem `Wiadomosc::Postep`. Funkcje z `core`
nie wiedzą nic o egui — dostają domknięcie `Arc<dyn Fn(Progress)>` i wołają je,
gdy mają coś do powiedzenia. Etapy pochodzą z `Stage` w `core/src/progress.rs`:

| Wariant `Stage` | Tekst w linii stanu |
| --- | --- |
| `Manifest` | Sprawdzam paczkę |
| `Java` | Pobieram Javę |
| `Loader` | Instaluję NeoForge |
| `Libraries` | Pobieram biblioteki |
| `Assets` | Pobieram zasoby gry |
| `Pack` | Pobieram mody |
| `Narzedzia` | Pobieram biblioteki dźwięku |
| `Ready` | Gotowe |

Meldunki z etapu `Ready` trafiają dodatkowo do listy `app.log`, bo to pod nimi
kryją się zdania w rodzaju „Wykonuję Twoją komendę przed startem…".

## Katalogi i pobieracz

`przygotuj_i_odpal` zaczyna od trzech rzeczy:

```rust
let dl = net::Downloader::new(8);
let mc = data.join("mc");
let instancja = data.join("instance");
std::fs::create_dir_all(&instancja)?;
```

Rozdział katalogów jest celowy: w `mc/` leży sam Minecraft (wersje, biblioteki,
zasoby), w `instance/` — paczka modów, konfiguracje i światy gracza. Dzięki temu
samonaprawa może skasować `mc/` i pobrać grę od nowa, nie ruszając niczego, co
należy do gracza.

Ósemka w `Downloader::new(8)` to semafor: tyle pobrań idzie naraz.
`fetch_many` trzyma w locie do szesnastu zadań (`buffer_unordered(16)`), ale
każde i tak czeka na semafor, więc realny limit to osiem połączeń. Każdy adres
z listy dostaje pełne trzy próby (`PROBY` w `core/src/net.rs`) z rosnącą
przerwą, zanim pobieracz przejdzie do adresu zapasowego. Plik ląduje najpierw
jako `<pełna nazwa>.part` i dopiero po sprawdzeniu sumy kontrolnej zmienia nazwę
na docelową.

## 1. Java

Sprawdzenie Javy jest pierwsze, przed jakimkolwiek pobieraniem. Powód jest
w komentarzu w `app.rs`: zła ścieżka ma się wyjaśnić od razu, a nie po
kwadransie instalowania.

### Java pobrana przez launcher

`java::ensure(root, major, …)` szuka binarki w `data/java/<major>`. Gdy jej nie
ma, pobiera JRE z Adoptium pod adresem budowanym przez `adoptium_url`:

```
https://api.adoptium.net/v3/binary/latest/21/ga/linux/x64/jre/hotspot/normal/eclipse
```

Pobranie idzie z `Expect::Any`, czyli bez sprawdzania sumy kontrolnej — Adoptium
nie podaje jej w przekierowaniu. Zamiast tego wynik weryfikuje się inaczej:
po rozpakowaniu archiwum musi się w nim znaleźć działający plik wykonywalny,
a jeśli się nie znajdzie, `ensure` zwraca `JavaError::BrakBinarki`.

Archiwum (`.zip` na Windowsie, `.tar.gz` gdzie indziej) ląduje w
`data/java-<major>.<rozszerzenie>` i znika po rozpakowaniu. Samo rozpakowanie
około 180 MB trwa kilka sekund, których nie da się zmierzyć z zewnątrz, więc
przed nim leci `Progress::trwa(Stage::Java, "Rozpakowuję Javę…")` — pasek stoi,
ale przynajmniej wiadomo dlaczego.

Binarki szuka `znajdz_binarke`: najpierw `<katalog>/bin/<nazwa>`, a potem
o poziom głębiej, bo archiwa Adoptium mają na wierzchu jeden katalog
(np. `jdk-21.0.12+7-jre`). Nazwa zależy od systemu — na Windowsie to
`javaw.exe`, bo `java.exe` otwierałaby obok gry okno konsoli.

### Java wskazana ręcznie

Gdy w Ustawieniach jest wpisana własna ścieżka (`ustawienia.java.wlasna()`),
launcher woła `wlasna_java` zamiast `java::ensure`. Ta funkcja pyta wskazany
plik o wersję przez `javy::sprawdz`, z limitem `limity::SPRAWDZENIE_JAVY`
(15 sekund), i odrzuca dwa przypadki:

- numer główny inny niż `manifest.java.major` — paczka nie ruszy na innej Javie;
- 32-bitową JVM — nie zaadresuje sterty, której wymaga paczka, a gracz
  zobaczyłby tylko „Could not reserve enough space for object heap".

Oba sprawdzenia można wyłączyć przełącznikiem `pomin_sprawdzanie`. Do samego
uruchomienia gry ścieżka przechodzi jeszcze przez `javy::do_uruchamiania`, które
na Windowsie podmienia wskazane `java.exe` na leżące obok `javaw.exe` — inaczej
przy każdym uruchomieniu gry otwierałaby się druga, czarna konsola.

!!! uwaga

    Błąd własnej Javy (`USTAW-01`) nie podlega samonaprawie. Launcher nie ma
    prawa poprawiać ani kasować tego, co gracz sam wpisał w Ustawieniach.

## 2. Instalacja NeoForge

`ensure_loader` uruchamia oficjalny instalator NeoForge w trybie bezobsługowym.
To on łata `client.jar` procesorem `binarypatcher`, więc launcher nie musi
odtwarzać tego procesu u siebie ani rozpowszechniać plików Mojanga.

Nazwa profilu powstaje z manifestu: `<kind>-<version>`, czyli na przykład
`neoforge-21.1.249`. Instalator pobierany jest z Mavena NeoForged:

```
https://maven.neoforged.net/releases/net/neoforged/neoforge/21.1.249/neoforge-21.1.249-installer.jar
```

Jeśli nie ma jeszcze `data/mc/launcher_profiles.json`, powstaje on tutaj —
z pustą listą profili i pustą bazą logowania. Instalator sprawdza ten plik,
zanim cokolwiek zrobi, i bez niego odmawia pracy.

### Kiedy instalacja jest kompletna: oba pliki json

Sprawdzenie „czy już jest zainstalowane" wymaga **dwóch** plików:

```rust
fn instalacja_kompletna(json_profilu: &Path, json_vanilla: &Path) -> bool {
    json_profilu.is_file() && json_vanilla.is_file()
}
```

Sam profil nie wystarcza, bo instalator zapisuje go, **zanim** ściągnie
biblioteki. Po przerwanej instalacji na dysku zostaje więc
`versions/neoforge-<wersja>/neoforge-<wersja>.json`, a katalog `libraries`
w ogóle nie powstaje. Wcześniejsza wersja kodu brała ten plik za dowód gotowej
instalacji, pomijała instalator i przewracała się dopiero przy wczytywaniu
wersji, po której profil dziedziczy. Gracz dostawał wtedy `INST-03` i żadna
liczba prób tego nie zmieniała — bo przy każdej kolejnej launcher znowu widział
profil i znowu pomijał instalator.

Drugi plik, `versions/<wersja Minecrafta>/<wersja Minecrafta>.json`, jest
wskaźnikiem, że instalator naprawdę doszedł do końca. Dlatego po nieudanej
próbie kod kasuje niedokończony profil:

```rust
// Niedokonczony profil musi zniknac, inaczej przy kolejnym starcie
// wygladalby na gotowa instalacje i instalator juz by nie ruszyl.
let _ = std::fs::remove_file(&json_profilu);
```

Ten warunek jest zabezpieczony testem `sam_profil_bez_czystej_wersji_to_nie_gotowa_instalacja`
w `game_install.rs`.

### Wstępne pobranie plików vanilla

Zanim ruszy instalator, `zasiej_czysta_wersje` kładzie na dysku dwa pliki czystej
wersji: `versions/<mc>/<mc>.json` i `versions/<mc>/<mc>.jar`.

Instalator umie pobrać je sam — i to jest właśnie problem. Robi to własnym
połączeniem z limitem pięciu sekund wpisanym na sztywno
(`DownloadUtils.getConnection`). Na wolnej maszynie albo przy niesprawnym IPv6
ten limit mija, zanim cokolwiek przyjdzie, i instalacja pada timeoutem — mimo że
przeglądarka na tym samym komputerze otwiera strony normalnie. Gdy oba pliki już
leżą na miejscu, instalator sprawdza `versions/<wersja>/<wersja>.jar`, widzi że
jest, i pomija całą tę ścieżkę. Pobieraniem zajmuje się wtedy `Downloader`, który
ma powtórki, adresy zapasowe i limit liczony od ciszy, a nie od całości.

Sama funkcja idzie po kolei:

1. kasuje `data/mc/version_manifest_v2.json`, jeśli został z poprzedniego razu,
   i pobiera spis wersji Mojanga z jednego z dwóch adresów (`SPIS_WERSJI`);
2. znajduje w spisie wpis o `id` równym wersji z manifestu paczki;
3. pobiera profil wersji, sprawdzając `Expect::Sha1` z podanym w spisie hashem;
4. czyta z profilu `downloads.client` i pobiera `client.jar`, znów po SHA-1;
5. kasuje spis wersji.

!!! uwaga

    Kasowanie spisu przed pobraniem nie jest ozdobnikiem. Spis nie ma znanego
    z góry hasha, więc leci z `Expect::Any` — a pobieracz przy `Expect::Any`
    uznaje każdy istniejący plik za dobry i nie pobiera go ponownie. Bez
    skasowania launcher czytałby w kółko starą listę wersji.

### Trzy podejścia i przełączenie na IPv4

Resztę plików instalator dociąga sam, tym samym krótkim limitem połączenia.
Dlatego uruchamia się go w pętli:

```rust
const PROB: u32 = 3;
for proba in 1..=PROB {
    …
    if proba > 1 {
        polecenie.arg("-Djava.net.preferIPv4Stack=true");
    }
```

Od drugiego podejścia Java dostaje polecenie trzymania się IPv4. Zepsute IPv6 to
najczęstszy powód, dla którego przeglądarka działa, a Java stoi na timeoucie —
czeka na adres, którego nie da się osiągnąć.

Całe wywołanie wygląda tak:

```
<java> [-Djava.net.preferIPv4Stack=true] -jar <profil>-installer.jar --install-client <data/mc>
```

Proces dostaje `kill_on_drop(true)` oraz limit czasu `limity::INSTALATOR`, czyli
15 minut. Ten limit też ma swoją historię: wcześniej kod czekał tu bez żadnej
górnej granicy. Instalator ma limit *połączenia*, ale nie *odczytu*, więc mirror,
który przyjmował połączenie i milkł w połowie, zatrzymywał go na zawsze,
a razem z nim cały launcher. Trzy podejścia i przełączenie na IPv4 nie ruszały
wtedy ani razu, bo pierwsze podejście się po prostu nie kończyło.

Gdy instalator zwróci błąd, jego wyjście przechodzi przez dwa filtry:

| Funkcja | Do czego służy |
| --- | --- |
| `wyglada_na_siec` | klasyfikuje **pełne** wyjście po ośmiu tropach (`SocketTimeoutException`, `timedFinishConnect`, `DownloadUtils`, …) |
| `bez_ramek_stosu` | usuwa linie zaczynające się od `at ` i `... ` z tekstu pokazywanego graczowi |
| `ogon` | przycina wynik do ostatnich 1200 znaków |

Kolejność jest istotna i obroniona testem `po_obcieciu_ramek_slad_sieci_znika`:
cały ślad sieci potrafi siedzieć wyłącznie w ramkach stosu, które z okna błędu
wylatują. Gdyby klasyfikować po tekście już obciętym, awaria sieciowa
(`SIE-04`) wyglądałaby na zwykłą awarię instalatora (`INST-02`) — a to dwa różne
komunikaty i dwa różne lekarstwa.

Po udanej instalacji znikają `<profil>-installer.jar` i `<profil>-installer.jar.log`.
Jeśli mimo sukcesu nie powstał profil, `ensure_loader` i tak zwraca błąd
z komunikatem „instalator zakończył się sukcesem, ale nie powstał profil wersji".

## 3. Wczytanie profilu wersji

`version::load(&mc.join("versions"), &profil)` czyta `neoforge-<wersja>.json`,
widzi w nim `inheritsFrom` i rekurencyjnie dokleja profil rodzica, czyli czystą
wersję Minecrafta. Scalaniem zajmuje się `version::merge`:

- `mainClass` i `id` biorą się z profilu potomnego;
- argumenty rodzica idą pierwsze, potem argumenty dziecka;
- biblioteki rodzica przechodzą tylko wtedy, gdy dziecko nie ma biblioteki o tej
  samej współrzędnej Maven bez wersji (`org.ow2.asm:asm`). NeoForge celowo
  podbija część bibliotek i ta sama biblioteka w dwóch wersjach na classpathie
  byłaby błędem;
- `assetIndex`, `assets` i `downloads` biorą się z dziecka, a gdy ich tam nie ma
  — z rodzica.

Błąd na tym etapie to `INST-03`, „Pliki gry są niekompletne".

## 4. Biblioteki

`ensure_libraries` przechodzi po `v.libraries` i buduje listę pobrań:

- wpisy odrzucone przez `rules_allow` dla bieżącego systemu są pomijane
  (biblioteka tylko dla macOS nie ma czego szukać na Linuksie);
- wpisy bez `downloads.artifact` też są pomijane — część bibliotek kładzie sam
  instalator NeoForge i nie ma dla nich adresu do pobrania;
- na koniec dokładany jest klient vanilla z `downloads.client`, pod ścieżkę
  `versions/<id>/<id>.jar`, gdzie `id` pochodzi z wczytanego profilu.

Klient vanilla jest potrzebny, bo NeoForge go **łata, a nie zastępuje**.

Każde pobranie ma `Expect::Sha1` z hashem z profilu wersji, więc plik uszkodzony
w połowie nie przejdzie i pobierze się jeszcze raz.

## 5. Zasoby gry

`ensure_assets` pobiera najpierw indeks zasobów do
`assets/indexes/<id>.json` (po SHA-1 z profilu), czyta z niego mapę obiektów
i zamienia ją na listę pobrań:

```
https://resources.download.minecraft.net/<dwa pierwsze znaki hasha>/<hash>
  ->  data/mc/assets/objects/<dwa pierwsze znaki hasha>/<hash>
```

Hash jest tu jednocześnie nazwą pliku i oczekiwaną sumą kontrolną. Profil bez
`assetIndex` nie jest błędem — funkcja kończy się wtedy od razu, bo taki profil
odziedziczy indeks z rodzica albo nie potrzebuje żadnego.

## 6. Synchronizacja paczki

To jedyny etap, który dotyka katalogu gracza:

```rust
let sciezka_stanu = data.join("state.json");
let mut stan = state::State::load(&sciezka_stanu);
let akcje = pack_sync::plan(m, &stan, &pack_sync::DiskProbe::new(&instancja));
let notatki = pack_sync::apply(m, &instancja, &mut stan, &akcje, &dl, postep.clone()).await?;
stan.save(&sciezka_stanu)?;
```

Rozdział na `plan` i `apply` jest po to, żeby decyzje dały się sprawdzić testem
bez dotykania dysku: `plan` widzi świat wyłącznie przez cechę `FileProbe`.
Wynikiem jest lista akcji (`Download`, `Delete`, `Record`, `SkipModified`,
`SkipUnknown`), a każdy plik dostaje `Expect::Sha512` z manifestu. Reguły, które
o tym decydują, opisuje osobno [Synchronizacja paczki](synchronizacja.md); to,
skąd biorą się wpisy, opisuje [Manifest](manifest.md).

Notatki zwrócone przez `apply` (na przykład „pomijam X — plik został zmieniony
ręcznie") trafiają do launchera jako `Wiadomosc::Notatka` i lądują w logu, który
gracz widzi na ekranie konsoli.

Stan zapisuje się **po** udanym pobraniu wszystkiego. Gdyby zapisywał się
wcześniej, przerwana synchronizacja zostawiłaby w `state.json` wpisy o plikach,
których na dysku nie ma.

## 7. Biblioteki dźwięku

```rust
if ustawienia.biblioteki_dzwieku && !m.narzedzia.is_empty() {
    if let Err(e) = narzedzia::zapewnij(&instancja, &m.narzedzia, &dl, postep.clone()).await {
        // …notatka, i lecimy dalej
    }
}
```

To jedyny etap przygotowania, którego niepowodzenie **nie** przerywa
uruchamiania. `narzedzia::zapewnij` pobiera `yt-dlp` i `ffmpeg` z tych samych
oficjalnych źródeł, z których wziąłby je mod Create: Harmonics — launcher robi
to wcześniej tylko po to, żeby mod nie pytał dziecka o zgodę w trakcie gry.
Skoro mod poradzi sobie sam, brak tych plików nie jest powodem, żeby nie
uruchomić gry. Gracz dostaje notatkę i tyle.

Dwa szczegóły z `narzedzia.rs`, obydwa wynikające z praktyki:

- obecność pliku `.md` w katalogu narzędzia nie liczy się jako dowód instalacji
  (`juz_jest` szuka czegokolwiek poza `.md`), bo mod sam zostawia tam plik
  z instrukcją i launcher nigdy nie pobrałby narzędzia;
- chodzenie po rozpakowanym drzewie używa `file_type` z `read_dir`, które **nie**
  podąża za dowiązaniami. Archiwum z dowiązaniem wskazującym na własnego
  przodka zapętliłoby przechodzenie drzewa, a gracz stałby na „Rozpakowuję…"
  bez końca.

## 8. Komenda przed startem

Jeśli w Ustawieniach → Zaawansowane jest wpisana komenda przed startem, wykonuje
się ona tutaj — po skompletowaniu plików, a przed zbudowaniem komendy gry.

Komenda idzie przez powłokę systemu (`/bin/sh -c` albo `cmd /C`), bo ludzie
wpisują tam `cp swiat swiat.bak && echo gotowe`, a nie listę argumentów. Katalog
roboczy to `data/instance`. Środowisko dostaje zmienne o nazwach takich samych
jak w Prism Launcherze, żeby gotowe skrypty przenosiły się bez przeróbek:

| Zmienna | Wartość |
| --- | --- |
| `INST_NAME` | `manifest.pack.name` |
| `INST_ID` | zawsze `chmurka` |
| `INST_DIR` | `data/instance` |
| `INST_MC_DIR` | `data/mc` |
| `INST_JAVA` | ścieżka do pliku wykonywalnego Javy |
| `INST_JAVA_ARGS` | `-Xms…M -Xmx…M` plus dodatkowe parametry gracza |

Na wierzch dokładane są własne zmienne środowiskowe gracza z tej samej zakładki.

Niepowodzenie tej komendy **przerywa** uruchamianie (`?` w `przygotuj_i_odpal`).
Tak ma być: wpisuje się tu rzeczy w rodzaju „zrób kopię świata", a granie na
świecie, którego kopia się nie udała, jest dokładnie tym, przed czym ta komenda
miała chronić. Komenda dostaje limit `limity::KOMENDA_GRACZA` — dwie minuty.
Bez niego literówka w skrypcie albo program czekający na wciśnięcie klawisza
zawieszałby start gry na zawsze, a launcher nie ma jak pokazać takiemu procesowi
konsoli. Przekroczenie limitu to `USTAW-03`, zwrócony błąd to `USTAW-02`.

## 9. Zbudowanie komendy uruchomienia

`launch::build_command` dostaje `LaunchParams` i zwraca gotowe
`std::process::Command`. Kolejność składania jest taka:

1. **Program.** Gdy gracz podał komendę opakowującą (`gamemoderun`, `prime-run`),
   to ona jest programem, a ścieżka do Javy jej pierwszym argumentem. Opakowanie
   celowo **nie** idzie przez powłokę: komenda gry ma kilkaset argumentów,
   w tym cały classpath i ścieżki ze spacjami, a sklejanie ich z powrotem
   w jeden łańcuch oznaczałoby cytowanie każdego z osobna.
2. **Zmienne środowiskowe** gracza.
3. **Sterta.** `-Xms<minimum>M` i `-Xmx<maksimum>M`, ale tylko wtedy, gdy wśród
   dodatkowych parametrów nie ma własnego `-Xmx`. Dwa `-Xmx` w jednej komendzie
   są legalne (wygrywa ostatni), ale mylą przy diagnozowaniu. Minimum przechodzi
   wcześniej przez `minimum_sterty_mb`, które przycina je do przedziału
   od 256 MB do wartości suwaka — `-Xms` większe od `-Xmx` to nie ostrzeżenie,
   tylko odmowa startu JVM.
4. **Dodatkowe parametry gracza**, nietknięte.
5. **Argumenty JVM z profilu wersji**, rozwinięte przez `rozwin`.
6. **Klasa główna** (`mainClass`).
7. **Argumenty gry z profilu wersji.**
8. **`--fullscreen`**, jeśli gracz tak ustawił. Ta flaga nie występuje w profilu
   wersji — jest bezargumentową flagą przyjmowaną przez samą grę, więc dokłada
   się ją na końcu, gdzie nie rozdzieli żadnej pary argument-wartość.
9. **Katalog roboczy** ustawiony na `data/instance`.

Zmienne w argumentach (`${auth_player_name}`, `${classpath}`,
`${natives_directory}` i pozostałe) podstawia `version::substitute`. Nieznana
zmienna zostaje w tekście nietknięta, zamiast zamienić się w pusty argument —
lepiej zobaczyć `${cos}` w logu niż szukać, dlaczego gra dostała pustą wartość.

### Reguły `features`, czyli czego w komendzie nie ma

Profil wersji opisuje część argumentów warunkowo: „doklej `--width` i `--height`,
**jeśli** launcher obsługuje własną rozdzielczość". Przez długi czas launcher
sprawdzał w regułach tylko pole `os`, więc wszystkie te warunki przechodziły —
i do komendy trafiały `--quickPlaySingleplayer ${quickPlaySingleplayer}`
z nierozwiniętą zmienną. Gracz był witany ekranem
„Failed to Quick Play — Could not find world".

Teraz odpowiada za to `Mozliwosci` w `version.rs`. Domyślnie wszystko jest
wyłączone, a jedyna możliwość, którą launcher potrafi włączyć, to
`has_custom_resolution` — i tylko wtedy, gdy gracz naprawdę narzucił rozmiar
okna. Nieznane nazwy są fałszywe celowo: nowa wersja Minecrafta może dołożyć
możliwość, o której nic nie wiadomo, a zgadywanie „chyba umiemy" kończy się
argumentem ze zmienną, której nie da się podstawić.

### Pamięć: bez własnych nastaw odśmiecacza i metaspace

W komendzie nie ma `-XX:MaxMetaspaceSize` ani żadnych nastaw odśmiecacza. To nie
przeoczenie, tylko wniosek z awarii, i pilnują tego dwa testy w `launch.rs`:
`nie_ograniczamy_metaspace` i `nie_narzucamy_wlasnych_nastaw_odsmiecacza`.

`-XX:MaxMetaspaceSize=512M` miało powstrzymać obszar, który przy tylu modach
rośnie i rośnie. Skutek był taki, że przy 349 plikach modów gra dobijała do
limitu **w trakcie rozgrywki** — klasy wczytują się leniwie, więc pierwszy nowy
potwór albo nowa struktura przy generowaniu terenu kończyły się
`OutOfMemoryError: Metaspace`. Gra nie padała od razu, tylko wchodziła
w korkociąg pełnych zbiórek: kolejne ticki serwera trwały 40, 80 i 120 sekund,
obraz stawał na jednej klatce i zostawało tylko ubicie procesu. Metaspace ma
rosnąć tyle, ile trzeba — ogranicza go pamięć systemu.

Cel pauzy odśmiecacza zostaje Javie z tego samego powodu: domyślne ustawienia
działały tu bez zarzutu, a własne były zgadywaniem pod maszynę, której problem
leżał zupełnie gdzie indziej.

!!! wskazówka

    Gracz, który wie, co robi, nadal może wpisać swoje parametry w Ustawieniach.
    Trafią do komendy nietknięte — łącznie z `-XX:MaxMetaspaceSize`, jeśli
    naprawdę tego chce.

## 10. Uruchomienie i nadzór nad procesem

Log gry leci prosto do pliku, a nie do pamięci launchera:

```rust
let plik_logu = katalog_logow.join("game.log");
let uchwyt = std::fs::File::create(&plik_logu)?;
let uchwyt2 = uchwyt.try_clone()?;
```

Dwa uchwyty do tego samego pliku to `stdout` i `stderr` gry. Dzięki temu, że
zapis idzie do pliku, launcher może schować się do zasobnika, a log i tak
powstaje w całości. Skutkiem ubocznym było to, że gracz nie miał **żadnego**
sposobu sprawdzić, co się dzieje — dlatego powstał moduł `core/src/konsola.rs`,
który czyta ogon pliku i odpowiada jednym zdaniem: log rośnie, log stoi od
90 sekund, albo pliku jeszcze nie ma.

Zaraz po `spawn` dzieje się rzecz najważniejsza dla samonaprawy:

```rust
ruszyla.store(true, Ordering::SeqCst);
let _ = n.send(Wiadomosc::GraWystartowala);
```

Od tego momentu nie wolno niczego ponawiać ani kasować, bo proces gry działa,
a drugie okno Minecrafta byłoby gorsze od każdego błędu. `GraWystartowala`
przełącza widok na konsolę (między kliknięciem GRAJ a pojawieniem się okna gry
mijają dwie minuty, w których na ekranie nie ma nic) i chowa okno launchera,
jeśli gracz tak ustawił.

Oczekiwanie na koniec gry idzie przez `tokio::task::spawn_blocking`, bo
`Child::wait` blokuje wątek. Po zakończeniu gry:

1. wykonuje się komenda po zakończeniu — w przeciwieństwie do tej przed startem
   jej niepowodzenie **niczego nie przerywa**. Gra już się skończyła, nie ma
   czego chronić, a okno błędu po udanej rozgrywce tylko by przestraszyło;
   gracz dostaje notatkę;
2. przy kodzie wyjścia oznaczającym sukces leci `Wiadomosc::GraZakonczona`
   i funkcja kończy się sukcesem;
3. przy niepowodzeniu z pliku logu czytanych jest ostatnie 60 linii i powstaje
   `BladLaunchera::GraPadla`.

`GraPadla` niesie ze sobą więcej niż sam kod wyjścia, bo bez tego nie da się
odróżnić przyczyn:

| Pole | Po co |
| --- | --- |
| `ogon_logu` | po nim `bledy.rs` rozpoznaje `OutOfMemoryError`, awarię moda i odrzucone parametry Javy |
| `wlasne_argumenty` | bez tego nie wolno obwiniać parametrów, których gracz nie wpisał |
| `zabita_przez_system` | SIGKILL od jądra albo `systemd-oomd`; w logu nie ma po tym śladu, bo urywa się w pół zdania |
| `sterta_mb` | odróżnia „podnieś suwak" (`GRA-02`) od „ten komputer dał już wszystko" (`GRA-06`) |

Rozpoznanie zabicia przez system robi `zabita_przez_system` w `app.rs`: sygnał
numer 9 na systemach uniksowych albo kod wyjścia 137, bo powłoki i menedżery
procesów raportują zabicie sygnałem jako 128 plus numer.

## Samonaprawa

`odpal_z_samonaprawa` opakowuje cały powyższy przebieg w pętlę trzech podejść.
Powód jest w komentarzu przy `enum Samonaprawa`: rady w katalogu błędów są
poprawne, ale „wejdź w Ustawienia i kliknij Napraw instalację" jest dla
dziesięciolatka ścianą tekstu, której nikt nie wykona. Więc launcher wykonuje tę
radę sam i pokazuje błąd dopiero wtedy, gdy to nie pomogło.

Lekarstwo dobierane jest **po kodzie błędu**, a nie po typie wyjątku — kod jest
tym, co widzi gracz i co trafia do administracji, więc reguła i komunikat nie
rozjadą się przy kolejnej zmianie w środku.

| Wariant | Co launcher robi | Przy jakich kodach |
| --- | --- | --- |
| `Ponow` | czeka 2 sekundy i próbuje jeszcze raz, nic nie kasując | `SIE-01`, `SIE-02`, `SIE-03`, `SIE-04`, `PLIK-03` |
| `JavaOdNowa` | kasuje `data/java` i pobiera Javę od zera | `INST-01` |
| `GraOdNowa` | kasuje `data/mc` i instaluje grę od zera | `INST-02`, `INST-03`, `INST-04` |
| `Nic` | nic; błąd idzie prosto na ekran | wszystkie pozostałe |

Kasowanie `data/mc` jest bezpieczne właśnie dlatego, że światy, ustawienia gry
i paczka modów leżą w `data/instance`.

Do `Nic` trafia wszystko, czego launcher sam nie ruszy: pełny dysk (`PLIK-01`),
brak uprawnień (`PLIK-02`), logowanie, padnięta gra (`GRA-*`), zła paczka
(`PACZKA-*`) i wszystko, co gracz wpisał w Ustawieniach (`USTAW-*`). Przy
własnej komendzie gracza ponowienie byłoby wręcz szkodliwe — uruchomiłoby ją
trzy razy pod rząd.

Pętla przerywa się i pokazuje błąd w trzech przypadkach:

```rust
if ruszyla.load(Ordering::SeqCst) || lek == Samonaprawa::Nic || ostatnie {
    let _ = n.send(Wiadomosc::BladZKodem(Box::new(b)));
    return;
}
```

Pierwszy warunek jest najważniejszy: `ruszyla` to `AtomicBool` zakładany na nowo
przy każdym podejściu i ustawiany w `przygotuj_i_odpal` zaraz po `spawn`. Gdy
gra wystartowała i dopiero potem padła, żadna samonaprawa się nie uruchomi —
inaczej launcher odpalałby Minecrafta drugi raz.

Przed każdą kolejną próbą gracz dostaje notatkę w rodzaju:

```
Coś się nie udało. Pobieram pliki gry od nowa… (kod INST-03, podejście 2 z 3)
```

To samo zdanie idzie na pasek postępu jako `Stage::Loader`. Teksty pochodzą
z `opis_samonaprawy` i mają uspokajać, a nie tłumaczyć. Pełną listę kodów wraz
z krokami dla gracza opisuje [Kody błędów](../gracz/kody-bledow.md).
