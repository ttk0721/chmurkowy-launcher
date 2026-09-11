# Manifest

Manifest to jeden plik JSON, w którym zapisane jest wszystko, co launcher musi
wiedzieć o paczce: wersja Minecrafta, wersja NeoForge, wymagana Java, lista
plików razem z sumami kontrolnymi i adresami do pobrania. Sam launcher nie ma
tej wiedzy w środku — pobiera manifest przy każdym uruchomieniu i dopiero
z niego dowiaduje się, co ma zainstalować. Dzięki temu dodanie moda nie wymaga
wydania nowej wersji programu.

Adres manifestu wpisuje się w plik programu przy budowaniu
(`crates/launcher/build.rs`):

```bash
CHMURKA_MANIFEST_URL=https://ttk0721.github.io/chmurkowy-launcher/manifest.json \
    cargo build --release
```

Bez tej zmiennej w binarce ląduje wartość zapasowa
`https://przyklad.github.io/chmurka-pack/manifest.json`, pod którą nic nie leży.

Wczytywanie i sprawdzanie manifestu siedzi w `crates/core/src/manifest.rs`,
funkcja `parse`. Manifest bieżącej paczki leży w repozytorium jako
`docs/manifest.json` i stamtąd trafia na GitHub Pages.

## Pola najwyższego poziomu

| Pole | Wymagane | Co zawiera |
| --- | --- | --- |
| `schema` | tak | Liczba całkowita. Launcher przyjmuje wyłącznie `1` (stała `SCHEMA`). |
| `pack` | tak | Nazwa paczki, jej wersja, wersja Minecrafta i loadera. |
| `java` | tak | Wymagane wydanie Javy. |
| `memory` | tak | Widełki pamięci. |
| `auth` | tak | Identyfikator aplikacji do logowania kontem Microsoft. |
| `launcher` | tak | Najnowsza wersja launchera i adresy jego plików. |
| `mirror_dirs` | nie | Katalogi sprzątane z obcych plików. Brak pola to pusta lista. |
| `narzedzia` | nie | Zewnętrzne programy wymagane przez mody. Brak pola to pusta lista. |
| `files` | tak | Lista wszystkich plików paczki. |

Kolejność pól w pliku nie ma znaczenia. W `docs/manifest.json` stoją
alfabetycznie, bo tak zapisuje obiekty `serde_json`.

### pack

| Pole | Typ | Znaczenie |
| --- | --- | --- |
| `name` | tekst | Nazwa paczki. Launcher pokazuje ją na ekranie głównym i podstawia do zmiennej `INST_NAME` w komendach gracza. |
| `edition` | tekst | Podpis edycji, np. `edycja 2026/2027`. Pole nieobowiązkowe, brak oznacza pusty tekst. |
| `version` | tekst | Wersja paczki, np. `2026.09.10-21`. |
| `minecraft` | tekst | Wersja gry, tu `1.21.1`. |
| `loader.kind` | tekst | Rodzaj loadera, tu `neoforge`. |
| `loader.version` | tekst | Wersja loadera, tu `21.1.249`. |

Z `loader.kind` i `loader.version` powstaje nazwa profilu wersji:
`neoforge-21.1.249`. Pod tą nazwą launcher szuka na dysku pliku
`mc/versions/neoforge-21.1.249/neoforge-21.1.249.json` i po jego obecności
poznaje, że NeoForge jest już zainstalowane.

### java

| Pole | Typ | Znaczenie |
| --- | --- | --- |
| `major` | liczba | Numer wydania Javy, tu `21`. Trafia do ścieżki `data/java/21` i do adresu pobrania z Adoptium. |
| `distribution` | tekst | Nieobowiązkowe, domyślnie `temurin`. |

!!! uwaga

    `distribution` jest wczytywane, ale nic z niego nie korzysta. Adres
    pobrania Javy jest w `crates/core/src/java.rs` wpisany na sztywno
    i kończy się segmentami `hotspot/normal/eclipse`, czyli zawsze prowadzi do
    Temurina. Wpisanie tu innej nazwy niczego nie zmieni.

### memory

| Pole | Typ | Znaczenie |
| --- | --- | --- |
| `min_mb` | liczba | Dolna granica pamięci. |
| `max_mb` | liczba | Górna granica pamięci. |

!!! uwaga

    To pole też jest tylko wczytywane. Przy uruchamianiu gry launcher bierze
    `-Xms` i `-Xmx` z ustawień gracza (`crates/launcher/src/app.rs`), a nie
    z manifestu. Pola mimo to nie da się z pliku wyrzucić: struktura `Memory`
    nie ma wartości domyślnych, więc manifest bez `memory` nie wczyta się
    u nikogo.

### auth

| Pole | Typ | Znaczenie |
| --- | --- | --- |
| `msa_client_id` | tekst | Identyfikator aplikacji używany przy logowaniu kontem Microsoft. |

W bieżącej paczce jest to `00000000402b5328`, czyli identyfikator oficjalnego
launchera Minecrafta — ten sam, który `chmurka login` ma wpisany jako wartość
domyślną.

### launcher

| Pole | Typ | Znaczenie |
| --- | --- | --- |
| `latest_version` | tekst | Numer najnowszego wydania launchera. |
| `urls` | obiekt | Adresy plików programu, po jednym na system. Pole nieobowiązkowe, brak oznacza pustą mapę. |

Klucze w `urls` to `windows-x64` i `linux-x64`. Launcher wybiera swój klucz
funkcją `klucz_systemu` z `crates/core/src/aktualizacja.rs`, która zwraca
`windows-x64` na Windowsie, a `linux-x64` wszędzie indziej.

Adresy wskazują konkretne wydanie na GitHubie, a nie `latest`. Powód jest
w komentarzu przy generowaniu manifestu: launcher podmienia sam siebie, więc
musi dostać dokładnie tę wersję, którą manifest ogłasza. Przy `latest` klient
sięgający po plik, zanim CI skończy budowanie, pobrałby po cichu poprzednią
binarkę i uznał, że aktualizacja nie wskoczyła.

### mirror_dirs

Lista katalogów (względem katalogu instancji), w których nie wolno być niczemu
spoza manifestu. W bieżącej paczce to jeden wpis: `mods`. Każdy plik znaleziony
w takim katalogu, a nieopisany w `files`, zostaje skasowany.

Powód jest twardy: obcy mod w `mods/` wywraca całą paczkę przy starcie gry.
Poza katalogami z tej listy launcher niczego nie kasuje — dlatego `config`,
`saves` czy `resourcepacks` nie są tu wymienione.

### narzedzia

Zewnętrzne programy, których wymagają niektóre mody. W bieżącej paczce są to
`yt-dlp` i `ffmpeg`, potrzebne modowi Create: Harmonics do odtwarzania dźwięku.
Mod potrafi pobrać je sam, ale pyta gracza o zgodę w trakcie gry — launcher
przygotowuje je wcześniej, z tych samych oficjalnych źródeł.

| Pole | Typ | Znaczenie |
| --- | --- | --- |
| `nazwa` | tekst | Nazwa pokazywana w pasku postępu. |
| `katalog` | tekst | Katalog w instancji, do którego trafia zawartość, np. `audio_providers/yt-dlp`. |
| `pliki` | obiekt | Docelowa nazwa pliku dla każdego systemu. Puste oznacza, że pod adresem leży archiwum. |
| `zrodla` | obiekt | Adresy pobrania dla poszczególnych systemów. |

`pliki` istnieje, bo ten sam program nazywa się inaczej na każdym systemie:
`yt-dlp` kontra `yt-dlp.exe`. `ffmpeg` tego pola nie ma, bo przychodzi jako
archiwum — format rozpoznawany jest z końcówki adresu (`.tar.xz` albo `.zip`),
żeby nie trzymać go osobno i nie doprowadzić do rozjazdu.

!!! uwaga

    Wpisy w `narzedzia` nie mają sum kontrolnych, bo ich adresy prowadzą do
    wydań `latest`, które zmieniają się w czasie — nie da się przypiąć hasha do
    pliku, który jutro będzie inny. Zamiast tego launcher ufa HTTPS
    i oficjalnemu źródłu, tak samo jak przy pobieraniu Javy z Adoptium.
    Brak adresu dla danego systemu nie jest błędem: launcher pomija narzędzie,
    a mod poradzi sobie sam.

### files

Lista wszystkich plików paczki. W bieżącym manifeście ma 710 wpisów: 264 mody
i 445 plików w `config/`, plus `options.txt`.

| Pole | Typ | Znaczenie |
| --- | --- | --- |
| `path` | tekst | Ścieżka względna wewnątrz katalogu instancji, zawsze z ukośnikiem zwykłym. |
| `size` | liczba | Rozmiar pliku w bajtach. |
| `sha512` | tekst | Suma SHA-512 zawartości, zapisana szesnastkowo. |
| `policy` | tekst | `mirror`, `smart` albo `seed` — patrz niżej. |
| `urls` | lista | Adresy, spod których można pobrać ten plik. Muszą być co najmniej jeden i każdy musi zaczynać się od `https://`. |

!!! uwaga

    `size` nie bierze udziału w niczym — o tym, czy plik jest właściwy,
    decyduje wyłącznie `sha512`. Rozmiar jest w manifeście informacją dla
    człowieka i dla narzędzi zewnętrznych.

## Co znaczy `policy`

Polityka odpowiada na jedno pytanie: czy wolno nadpisać plik, który już leży na
dysku gracza. Rozstrzygają to `Policy` w `crates/core/src/manifest.rs`
i funkcja `plan` w `crates/core/src/pack_sync.rs`.

| Polityka | Zastosowanie w paczce | Zachowanie |
| --- | --- | --- |
| `mirror` | mody | Plik musi zgadzać się co do bajtu. Gdy hash na dysku jest inny albo pliku nie ma — pobranie. Obce pliki w katalogach z `mirror_dirs` są kasowane. |
| `smart` | pliki w `config/` | Nadpisujemy tylko wtedy, gdy gracz pliku nie ruszał. |
| `seed` | `options.txt` | Wgrywamy raz, gdy pliku nie ma. Potem nigdy więcej go nie dotykamy, nawet jeśli manifest ma inną treść. |

Mody dostają `mirror`, bo jeden obcy albo niezgodny plik w `mods/` wywraca
uruchomienie całej paczki. Configi dostają `smart`, bo gracz ma prawo je
zmieniać — ale nowy mod potrzebuje swojego configu, więc pliku nietkniętego
trzeba móc zaktualizować. `options.txt` dostaje `seed`, bo to sterowanie,
głośność i ustawienia grafiki: raz podrzucone sensowne wartości startowe,
a potem to już sprawa gracza.

Rozróżnienie „gracz zmienił" od „paczka się zaktualizowała" bierze się z pliku
`state.json`, w którym launcher zapisuje hash każdego pliku, jaki sam wgrał.
Szczegóły: [Gdzie leżą dane](dane.md).

Dla polityki `smart` rozpatrywanych jest pięć przypadków:

| Na dysku | W `state.json` | Co robi launcher |
| --- | --- | --- |
| brak pliku | nieważne | pobiera |
| hash zgodny z zapisanym | jest wpis | pobiera, jeśli manifest podaje inny skrót |
| hash inny niż zapisany | jest wpis | zostawia wersję gracza i pisze o tym w logu |
| hash zgodny z manifestem | brak wpisu | nie pobiera, tylko dopisuje wpis do `state.json` |
| hash inny niż w manifeście | brak wpisu | nie rusza pliku — nie wiadomo, skąd pochodzi |

Pełny przebieg synchronizacji opisuje
[Synchronizacja paczki](synchronizacja.md).

## Weryfikacja SHA-512

Pobieraniem zajmuje się `crates/core/src/net.rs`, liczeniem sum —
`crates/core/src/hash.rs`. Dla każdego pliku z manifestu przebieg jest taki:

1. Zanim cokolwiek poleci przez sieć, launcher sprawdza plik, który już leży na
   miejscu. Gdy jego SHA-512 zgadza się z manifestem, pobranie jest pomijane.
2. Pobieranie idzie do pliku roboczego o nazwie docelowej z dopiskiem `.part`,
   na przykład `sodium.jar.part`.
3. Po ściągnięciu liczona jest suma pliku roboczego. Dopiero gdy się zgadza,
   plik jest przenoszony na miejsce docelowe. Niezgodny plik roboczy jest
   kasowany i próba liczy się jako nieudana.
4. Każdy adres z `urls` dostaje trzy próby (stała `PROBY`), z przerwą rosnącą
   **między** próbami: 400 ms i 800 ms. Po ostatniej próbie danego adresu
   przerwy już nie ma — launcher od razu przechodzi do następnego adresu.
5. Gdy skończą się wszystkie adresy, pobieranie kończy się błędem
   `nie udało się pobrać …: wyczerpano próby dla adresów …`, z ostatnim błędem
   w treści.

Sumy porównywane są bez rozróżniania wielkości liter (`eq_ignore_ascii_case`),
więc zapis `AB12…` i `ab12…` znaczy to samo.

Plik roboczy nazywa się `<pełna nazwa>.part`, a nie `<trzon>.part`. To wygląda
na drobiazg, ale wcześniej było inaczej i wywracało pobieranie: dwa pliki
o wspólnym trzonie nazwy, jak `chloride-client.toml_backup1`
i `chloride-client.toml_backup2`, dostawały wspólny plik roboczy
`chloride-client.part`. Pobierały się równolegle, więc jedno zadanie
przenosiło plik na miejsce, a drugie chwilę później próbowało przenieść coś,
czego już nie było — gracz dostawał błąd PLIK-03.

Sumy liczone są strumieniowo, kawałkami po 64 kB, żeby 93-megabajtowy mod nie
wjeżdżał w całości do pamięci.

!!! wskazówka

    SHA-512 dotyczy wyłącznie plików z manifestu. Biblioteki i zasoby samej
    gry sprawdzane są po SHA-1, bo takie sumy podaje Mojang w swoich plikach
    wersji. Java z Adoptium nie jest sprawdzana hashem w ogóle — pod adresem
    leży wydanie `latest`, którego suma zmienia się z każdą aktualizacją,
    więc dowodem powodzenia jest znalezienie działającej binarki po
    rozpakowaniu.

## Co launcher odrzuca przy wczytywaniu

Manifest przychodzi z sieci, więc `parse` traktuje go jak dane niezaufane.
Każdy z poniższych przypadków przerywa wczytywanie — launcher nie próbuje
naprawiać manifestu ani pomijać złych wpisów.

| Sytuacja | Komunikat |
| --- | --- |
| JSON nie daje się sparsować | `nieprawidłowy JSON manifestu: …` |
| `schema` inne niż 1 | `nieobsługiwana wersja schematu manifestu: … (launcher rozumie 1)` |
| Zła ścieżka we wpisie | `wpis <path>: …` |
| Wpis bez adresów | `wpis <path>: brak jakiegokolwiek adresu do pobrania` |
| Adres nie na `https://` | `wpis <path>: adres musi być https, jest: …` |

Ścieżki sprawdza `validate_rel` z `crates/core/src/paths.rs`. Bez tej funkcji
wpis `../../.bashrc` pozwoliłby nadpisać dowolny plik na dysku gracza.
Odrzucane są:

| Ścieżka | Powód |
| --- | --- |
| pusta | nie ma czego zapisać |
| `/etc/passwd` | ścieżka absolutna |
| `C:/Windows/system32` | litera dysku to też ścieżka absolutna |
| `mods/../../secret` | segment `..` wyprowadza poza katalog instancji |
| `mods\sodium.jar` | ukośnik odwrotny; manifest zawsze używa zwykłego |
| `mods//sodium.jar`, `mods/./sodium.jar` | pusty segment albo `.` |

Ukośnik odwrotny jest odrzucany osobno, zanim ścieżka zostanie rozbita na
segmenty. Na Windowsie bywa separatorem, więc wpis `mods\..\..\secret`
przeszedłby przez kontrolę segmentów, gdyby dzielić tekst tylko po `/`.

Po wczytaniu manifestu ścieżki są sklejane z katalogiem instancji funkcją
`join_within`, która powtarza tę samą walidację — kontrola jest w dwóch
miejscach, bo `plan` i `apply` mogą dostać dane z innego źródła niż `parse`.

## Skąd biorą się adresy

Manifest buduje `chmurka pack-build` (`crates/cli/src/main.rs`) na podstawie
instancji PrismLaunchera. Dla każdego moda decyduje o adresie jedno pytanie:
czy plik na dysku utrzymującego to naprawdę ten sam plik, który leży pod
adresem źródłowym.

1. Narzędzie czyta metadane modów z katalogu `mods/.index` (pliki `.toml`
   packwiza). Są w nich adres pobrania albo identyfikator pliku na CurseForge
   oraz hash — czasem SHA-512, czasem SHA-1.
2. Niezależnie od tego liczy własne SHA-512 z pliku leżącego na dysku. To jest
   jedyne źródło prawdy o tym, w co się faktycznie gra.
3. Gdy oba hashe się zgadzają, do manifestu trafia adres źródłowy: CDN
   Modrinth albo CurseForge. Nie kopiujemy cudzych plików bez potrzeby.
4. Gdy się różnią — albo gdy metadane w ogóle nie mają hasha — plik jest
   kopiowany do własnego magazynu i adres wskazuje na nasz serwis. Inaczej
   testerzy dostaliby inną paczkę niż ta, którą utrzymujący sprawdził.
   Narzędzie wypisuje wtedy ostrzeżenie z listą takich modów.

Configi i `options.txt` zawsze idą do własnego magazynu — nie mają żadnego
adresu źródłowego, bo to pliki utrzymującego.

Magazyn jest adresowany treścią: plik ląduje pod
`files/<dwa pierwsze znaki hasha>/<pełny hash>`, a adres powstaje przez
doklejenie tej ścieżki do `--base-url`. Ten sam plik w kolejnych wersjach
paczki zajmuje miejsce tylko raz, a adres nigdy nie wskaże innej treści, niż
wskazywał wczoraj.

W bieżącym manifeście występują trzy hosty:

| Host | Co stamtąd idzie |
| --- | --- |
| `cdn.modrinth.com` | mody pobrane z Modrinth, niezmienione |
| `mediafilez.forgecdn.net` | mody z CurseForge, niezmienione |
| `ttk0721.github.io` | nasz magazyn: configi, `options.txt` i mody różniące się od źródła |

Cały przebieg przebudowy opisuje
[Przebudowa manifestu](../paczka/przebudowa.md).

## Przykład

Poniżej prawdziwy fragment `docs/manifest.json`, przycięty do trzech wpisów
w `files` i jednego narzędzia. Oryginał ma 710 wpisów i ponad 300 kB.

```json
{
  "schema": 1,
  "pack": {
    "name": "Chmurkowy Serwer",
    "edition": "edycja 2026/2027",
    "version": "2026.09.10-21",
    "minecraft": "1.21.1",
    "loader": { "kind": "neoforge", "version": "21.1.249" }
  },
  "java": { "major": 21, "distribution": "temurin" },
  "memory": { "max_mb": 4096, "min_mb": 512 },
  "auth": { "msa_client_id": "00000000402b5328" },
  "launcher": {
    "latest_version": "0.4.24",
    "urls": {
      "linux-x64": "https://github.com/ttk0721/chmurkowy-launcher/releases/download/v0.4.24/ChmurkowyLauncher-linux-x64",
      "windows-x64": "https://github.com/ttk0721/chmurkowy-launcher/releases/download/v0.4.24/ChmurkowyLauncher-windows-x64.exe"
    }
  },
  "mirror_dirs": ["mods"],
  "narzedzia": [
    {
      "nazwa": "yt-dlp",
      "katalog": "audio_providers/yt-dlp",
      "pliki": { "linux-x64": "yt-dlp", "windows-x64": "yt-dlp.exe" },
      "zrodla": {
        "linux-x64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux",
        "windows-x64": "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
      }
    }
  ],
  "files": [
    {
      "path": "mods/AI-Improvements-1.21-0.5.3.jar",
      "size": 28984,
      "sha512": "ded870e90953ea915d24ac4c81799b95b321dda39936cd4cd620302b48882141fa3b9f1af658bdc6c25dc48de003223b91f3669b69efcc92c8666a3983f0fccc",
      "policy": "mirror",
      "urls": [
        "https://cdn.modrinth.com/data/DSVgwcji/versions/dGNP90t0/AI-Improvements-1.21-0.5.3.jar"
      ]
    },
    {
      "path": "config/fml.toml",
      "size": 1523,
      "sha512": "dda737dbfafc46c162935d0e2b17f4cb87bd6c4afda324f253e59adbdd3a296b19866634394ada91c68939ef0abd40202a62d6cf47563b24ac53cc61a7b330d4",
      "policy": "smart",
      "urls": [
        "https://ttk0721.github.io/chmurkowy-launcher/files/dd/dda737dbfafc46c162935d0e2b17f4cb87bd6c4afda324f253e59adbdd3a296b19866634394ada91c68939ef0abd40202a62d6cf47563b24ac53cc61a7b330d4"
      ]
    },
    {
      "path": "options.txt",
      "size": 13141,
      "sha512": "973e3db2a0e0037186e03551c0e9b37f057eff7ec260147fc80804cb6acca403f92b33adb2cab8c0dbf8859b1a24f2084e17d90056ad447948698c7253400585",
      "policy": "seed",
      "urls": [
        "https://ttk0721.github.io/chmurkowy-launcher/files/97/973e3db2a0e0037186e03551c0e9b37f057eff7ec260147fc80804cb6acca403f92b33adb2cab8c0dbf8859b1a24f2084e17d90056ad447948698c7253400585"
      ]
    }
  ]
}
```

Widać tu obie drogi adresu: mod pobrany z Modrinth wskazuje prosto na CDN
Modrinth, a `config/fml.toml` i `options.txt` leżą w naszym magazynie pod
nazwą równą własnej sumie SHA-512.

## Sprawdzenie manifestu

`chmurka pack-build` na koniec przepuszcza własny wynik przez tę samą funkcję
`manifest::parse`, której używa launcher, i przerywa budowanie, gdy plik jej
nie przejdzie. Zepsuty manifest ma się wywrócić u utrzymującego, a nie
u dziecka przed ekranem.
