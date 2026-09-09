# Chmurkowy Launcher — projekt

Data: 2026-09-09
Status: zatwierdzony

## 1. Cel

Przenośny launcher, który pozwala przetestować paczkę modów Chmurkowego Serwera na
kilku komputerach bez instalowania czegokolwiek w systemie. Użytkownik pobiera jeden
plik, uruchamia go, loguje się i gra. Skasowanie folderu launchera usuwa launcher,
paczkę, Javę i wszystkie dane gry — nie zostaje nic.

Paczka: Minecraft 1.21.1, NeoForge 21.1.249, 249 modów, Java 21.

## 2. Zasady projektowe

1. **Przenośność bezwzględna.** Wszystko żyje w folderze obok pliku wykonywalnego.
   Żadnych wpisów w rejestrze, `%APPDATA%`, `~/.minecraft` ani zmiennych środowiskowych.
2. **Zero redystrybucji.** Launcher nie hostuje ani jednego moda ani pliku Mojanga.
   Każdy plik pobiera z oficjalnego źródła. Hostujemy wyłącznie configi i manifest.
3. **Aktualizacje różnicowe.** Zmiana jednego moda oznacza pobranie jednego moda.
4. **Awaria jest głośna.** Każdy błąd kończy się czytelnym komunikatem wskazującym plik
   i źródło. Nigdy nie kontynuujemy po cichu — niekompletna paczka to crash w grze,
   który trudno zdiagnozować.
5. **Bezpiecznik offline.** Tryb offline działa zawsze i nie zależy od żadnej usługi
   zewnętrznej. To gwarancja, że da się testować paczkę nawet gdy logowanie padnie.

## 3. Fakty ustalone eksperymentalnie

Wszystko poniżej zostało sprawdzone 2026-09-09, nie założone.

| Fakt | Dowód |
|---|---|
| Instalator NeoForge działa bezobsługowo | `java -jar neoforge-21.1.249-installer.jar --install-client <dir>` → exit 0, „Successfully installed client into launcher" |
| Instalator sam patchuje `client.jar` | Uruchamia procesor `binarypatcher`, produkuje `neoforge-21.1.249-client.jar` |
| Wynik instalacji | `versions/1.21.1/{json,jar}`, `versions/neoforge-21.1.249/json`, 73 libki, 121 MB |
| Profil startowy | `mainClass = cpw.mods.bootstraplauncher.BootstrapLauncher`, `inheritsFrom = 1.21.1`, 47 libek NeoForge + 97 vanilla |
| Wymagana Java | `javaVersion: { component: java-runtime-delta, majorVersion: 21 }` |
| Rozmiar assetów 1.21.1 | `assetIndex.totalSize = 824 904 414` bajtów (825 MB), indeks `17` |
| Adoptium wydaje JRE 21 | HTTP 307 na `api.adoptium.net/v3/binary/latest/21/ga/{windows,linux}/x64/jre/hotspot/normal/eclipse` |
| Prism trzyma metadane packwiz | `mods/.index/*.pw.toml`, 249 plików na 249 jarów, mapowanie 1:1 |
| 239 modów ma URL do Modrinth | `mode = 'url'`, host `cdn.modrinth.com`, hash `sha512` |
| 10 modów z CurseForge ma działające linki | `mediafilez.forgecdn.net/files/{id/1000}/{id%1000}/{plik}` → HTTP 200 dla wszystkich 10 |
| Logowanie nie wymaga rejestracji w Azure | `POST login.live.com/oauth20_connect.srf`, `client_id=00000000402b5328`, `scope=service::user.auth.xboxlive.com::MBI_SSL` → zwraca `user_code`, `device_code`, `verification_uri` |
| Nowy endpoint Azure odrzuca to ID | `AADSTS700016` — to aplikacja spoza katalogu AAD, stąd ominięcie rejestracji |

### Bilans transferu

| Źródło | Rozmiar | Kto hostuje |
|---|---|---|
| JRE 21 | 45 MB | Adoptium |
| Vanilla + NeoForge | 121 MB | Mojang + NeoForge |
| Libki i natives | ~50 MB | Mojang |
| Assety | 825 MB | Mojang |
| 239 modów | 670 MB | Modrinth |
| 10 modów | 8 MB | CurseForge |
| **Configi + manifest** | **4,4 MB** | **my** |

Pierwsza instalacja: ~1,8 GB, z czego z naszego hostingu 4,4 MB.

## 4. Układ folderów u użytkownika

```
ChmurkowyLauncher/
├── ChmurkowyLauncher.exe
└── data/
    ├── settings.json          RAM, ostatni nick, stan okna
    ├── auth.json              token odświeżania Microsoft
    ├── state.json             hashe plików, które launcher sam wgrał
    ├── logs/launcher.log
    ├── java/21/               JRE z Adoptium
    ├── mc/                    versions/, libraries/, assets/
    └── instance/              gameDir: mods/, config/, saves/, screenshots/, logs/
```

Gra dostaje `--gameDir <...>/data/instance`, więc świat, screeny i logi gry zostają w środku.

Uzasadnienie rozdziału `mc/` i `instance/`: `mc/` to treść odtwarzalna z sieci,
`instance/` zawiera dane gracza. „Napraw instalację" kasuje `mc/`, nie ruszając światów.

## 5. Format manifestu

Jeden plik `manifest.json` na GitHub Pages. Launcher pobiera go przy każdym starcie.

```json
{
  "schema": 1,
  "pack": {
    "name": "Chmurkowy Serwer",
    "edition": "edycja 2026/2027",
    "version": "2026.09.09-1",
    "minecraft": "1.21.1",
    "loader": { "kind": "neoforge", "version": "21.1.249" }
  },
  "java": { "major": 21, "distribution": "temurin" },
  "memory": { "min_mb": 512, "max_mb": 4096 },
  "auth": { "msa_client_id": "00000000402b5328" },
  "launcher": {
    "latest_version": "1.0.0",
    "urls": {
      "windows-x64": "https://github.com/<user>/<repo>/releases/download/v1.0.0/ChmurkowyLauncher.exe",
      "linux-x64":   "https://github.com/<user>/<repo>/releases/download/v1.0.0/ChmurkowyLauncher"
    }
  },
  "mirror_dirs": ["mods"],
  "files": [
    {
      "path": "mods/sodium-neoforge-0.8.13+mc1.21.1.jar",
      "size": 1234567,
      "sha512": "4f537696af95...",
      "policy": "mirror",
      "urls": ["https://cdn.modrinth.com/data/AANobbMI/versions/uMOpc5uV/sodium-neoforge-0.8.13%2Bmc1.21.1.jar"]
    },
    {
      "path": "config/create-client.toml",
      "size": 512,
      "sha512": "a1b2c3...",
      "policy": "smart",
      "urls": ["https://<user>.github.io/<repo>/files/a1/a1b2c3..."]
    },
    {
      "path": "options.txt",
      "size": 8819,
      "sha512": "d4e5f6...",
      "policy": "seed",
      "urls": ["https://<user>.github.io/<repo>/files/d4/d4e5f6..."]
    }
  ]
}
```

Uwagi projektowe:

- **`urls` to lista, nie pojedynczy adres.** Jeśli mod zniknie z Modrinth, dopisujesz
  mirror i nikt nie musi pobierać nowego launchera. Launcher próbuje po kolei.
- **Jeden algorytm hashowania — SHA-512.** Modrinth podaje sha512 w metadanych packwiz,
  więc dla modów przepisujemy hash bez pobierania pliku. Dla naszych plików liczymy sam.
  (Mojang używa SHA-1 dla assetów i libek, ale to inna ścieżka kodu — patrz §6.)
- **Nasze pliki są adresowane treścią** (`files/<2 znaki>/<pełny hash>`), więc wypchnięcie
  nowej wersji paczki dodaje tylko nowe blob-y, a stare wersje pozostają pobieralne.
- `mirror_dirs` mówi, w których katalogach kasować pliki nieobecne w manifeście.

## 6. Pipeline instalacji

Maszyna stanów, każdy krok wznawialny i weryfikowany hashem:

1. **Manifest** — pobierz `manifest.json`, zwaliduj schemat i ścieżki (§10).
2. **Java** — jeśli brak `data/java/21`, pobierz z Adoptium (zip na Windows, tar.gz na
   Linuksie), rozpakuj, sprawdź `java -version`.
3. **Gra** — jeśli brak `data/mc/versions/neoforge-21.1.249`, pobierz instalator NeoForge
   i uruchom `<nasza java> -jar installer.jar --install-client data/mc`. Wcześniej stwórz
   atrapę `data/mc/launcher_profiles.json`, której instalator wymaga.
4. **Metadane wersji** — wczytaj `neoforge-*.json`, rozwiąż `inheritsFrom` scalając z
   `1.21.1.json`: `libraries` łączone (NeoForge nadpisuje vanilla przy kolizji
   `groupId:artifactId`), `arguments.jvm` i `arguments.game` konkatenowane, `mainClass`
   z NeoForge.
5. **Libki i natives** — dociągnij z CDN Mojanga te, których instalator nie pobrał,
   weryfikując SHA-1 z JSON-a. Zastosuj reguły `rules` (filtr po systemie).
6. **Assety** — pobierz `assetIndex`, potem obiekty do `data/mc/assets/objects/<2>/<hash>`.
   To ~4000 małych plików; równoległość ograniczona do 8.
7. **Paczka** — synchronizuj według §7.
8. **Start** — złóż classpath i argumenty (§9), odpal proces.

Kroki 2–7 raportują postęp jednolitym zdarzeniem `Progress { etap, zrobione, wszystkie, bajty }`.

## 7. Synchronizacja paczki

Launcher trzyma `data/state.json`: mapa `ścieżka → hash pliku, który sam wgrał`.
Bez tego nie da się odróżnić „gracz zmienił config" od „paczka się zaktualizowała".

### Tryb `mirror` (katalog `mods/`)

| Na dysku | W manifeście | Akcja |
|---|---|---|
| brak | tak | pobierz |
| hash zgodny | tak | nic |
| hash niezgodny | tak | pobierz ponownie |
| jest | nie | **usuń** |

Kasowanie jest konieczne: obcy mod w `mods/` wywala całą paczkę przy starcie.

### Tryb `smart` (katalog `config/`)

`H_zapisany` to hash z `state.json`, `H_manifest` to hash oczekiwany.

| Na dysku | `H_zapisany` | Warunek | Akcja |
|---|---|---|---|
| brak | dowolny | jest w manifeście | pobierz |
| `== H_zapisany` | jest | `H_manifest != H_zapisany` | nadpisz — gracz nie ruszał |
| `== H_zapisany` | jest | `H_manifest == H_zapisany` | nic |
| `!= H_zapisany` | jest | jest w manifeście | **zostaw**, zapisz w logu „gracz zmienił, pomijam" |
| `== H_manifest` | brak wpisu | jest w manifeście | dopisz wpis do `state.json`, nic nie pobieraj — plik jest już właściwy |
| `!= H_manifest` | brak wpisu | jest w manifeście | zostaw, zapisz w logu (nie wiemy, czyj to plik) |
| `== H_zapisany` | jest | brak w manifeście | usuń — był nasz, już zbędny |
| `!= H_zapisany` | jest | brak w manifeście | zostaw |

### Tryb `seed` (`options.txt`)

Wgrywany tylko gdy pliku nie ma. Nigdy nie nadpisywany — to ustawienia klawiszy i grafiki.

## 8. Uwierzytelnianie

### Microsoft (domyślne)

Device code flow przez stary endpoint, bez rejestracji aplikacji:

1. `POST https://login.live.com/oauth20_connect.srf`
   z `client_id` (z manifestu), `scope=service::user.auth.xboxlive.com::MBI_SSL`,
   `response_type=device_code` → `user_code`, `device_code`, `interval`, `expires_in`.
2. UI pokazuje kod i przycisk „kopiuj i otwórz", który kopiuje kod do schowka i otwiera
   `https://www.microsoft.com/link`.
3. Odpytywanie `POST https://login.live.com/oauth20_token.srf` co `interval` sekund
   z `grant_type=urn:ietf:params:oauth:grant-type:device_code`.
   Obsłużyć `authorization_pending`, `slow_down`, `expired_token`, `authorization_declined`.
4. Xbox Live: `POST https://user.auth.xboxlive.com/user/authenticate`.
5. XSTS: `POST https://xsts.auth.xboxlive.com/xsts/authorize`. Obsłużyć `XErr`:
   `2148916233` (brak konta Xbox), `2148916238` (konto dziecka) — czytelne komunikaty.
6. Minecraft: `POST https://api.minecraftservices.com/authentication/login_with_xbox`.
7. Profil: `GET https://api.minecraftservices.com/minecraft/profile` → UUID i nick.
   HTTP 404 oznacza konto bez kupionego Minecrafta — trzeba to jasno napisać.

`refresh_token` trafia do `data/auth.json`. Odświeżanie przy starcie, cicho.

W kroku 4 `RpsTicket` przekazujemy **bez prefiksu `d=`**. Prefiks `d=` dotyczy tokenów z
nowego flow AAD; token ze `service::user.auth.xboxlive.com::MBI_SSL` jest już biletem RPS
i prefiksu nie przyjmuje. Jeżeli Xbox Live odpowie na to HTTP 400 lub 401, ponawiamy raz
z prefiksem — to jedyny warunek, przy którym wariant alternatywny w ogóle się uruchamia.

### Offline

Nick → UUID wersji 3 z `MD5("OfflinePlayer:" + nick)`, zgodnie z konwencją serwerów.
Token pusty. Działa tylko na serwerze z `online-mode=false` — UI musi to napisać wprost,
żeby nikt nie zgłaszał tego jako błędu.

### Zabezpieczenie przed odcięciem `client_id`

`msa_client_id` jest polem manifestu. Gdyby Microsoft zablokował identyfikator oficjalnego
launchera, wystarczy zarejestrować własną aplikację w Azure i podmienić wartość w manifeście.
Żaden tester nie musi wtedy pobierać nowej wersji launchera.

## 9. Uruchomienie gry

Classpath: wszystkie `libraries` po scaleniu (§6, krok 4) z pominięciem wpisów z
`arguments.jvm` `-DignoreList`. Podstawienia w szablonach argumentów:

`${auth_player_name}`, `${auth_uuid}`, `${auth_access_token}`, `${user_type}` (`msa`
albo `legacy` dla offline), `${version_name}`, `${game_directory}`, `${assets_root}`,
`${assets_index_name}`, `${library_directory}`, `${classpath}`, `${classpath_separator}`
(`;` na Windows, `:` na Linuksie), `${natives_directory}`, `${launcher_name}`,
`${launcher_version}`, `${version_type}`.

Pamięć z manifestu, nadpisywalna w ustawieniach: `-Xms`, `-Xmx`.

Proces potomny: `stdout`/`stderr` przechwytywane do `data/logs/game.log` i do panelu
„szczegóły". Kod wyjścia różny od zera pokazuje ostatnie 40 linii logu z przyciskiem
„otwórz pełny log" — testowanie paczki polega głównie na czytaniu crashy, więc to musi
być wygodne.

## 10. Bezpieczeństwo

- **Walidacja ścieżek z manifestu.** Odrzucamy ścieżki absolutne, zawierające `..`,
  litery dysków, ukośniki odwrotne i puste segmenty. Po złożeniu ścieżka jest
  kanonizowana i musi leżeć wewnątrz `data/instance`. Bez tego manifest mógłby nadpisać
  dowolny plik na dysku ofiary.
- **Weryfikacja hashem przed podmianą.** Pobieranie do `<plik>.part`, sprawdzenie
  SHA-512, dopiero potem zmiana nazwy. Przerwane pobieranie nigdy nie zostawia
  uszkodzonego moda udającego poprawny.
- **Tylko HTTPS.** Adresy `http://` w manifeście są odrzucane.
- **Token w pliku.** `data/auth.json` leży w folderze launchera, bo pęk kluczy systemowy
  kłóci się z przenośnością. Plik dostaje uprawnienia 0600 na Linuksie.
  Ograniczenie jest świadome i zostanie opisane w README: folder launchera należy
  traktować jak dane logowania.

## 11. Struktura kodu

Workspace Rust, trzy crate'y:

```
crates/
├── core/        logika bez UI, w pełni testowalna
│   ├── manifest.rs      schemat, parsowanie, walidacja ścieżek
│   ├── net.rs           pobieranie: retry, wznawianie, limit równoległości, postęp
│   ├── java.rs          Adoptium: rozwiązanie wersji, pobranie, rozpakowanie
│   ├── game_install.rs  instalator NeoForge, scalanie JSON-ów, libki, assety
│   ├── pack_sync.rs     planowanie i wykonanie synchronizacji (§7)
│   ├── auth/            msa.rs, offline.rs, store.rs
│   ├── launch.rs        classpath, szablony argumentów, proces
│   └── state.rs         maszyna stanów i zdarzenia postępu
├── launcher/    GUI w egui, cienka warstwa nad core
└── packer/      CLI: instancja Prisma → manifest.json + blob-y do wypchnięcia
```

Rozdział na `core` i `launcher` jest celowy: całą logikę da się przetestować bez okna,
a GUI sprowadza się do rysowania stanu i wysyłania poleceń.

### `packer`

`chmurka-pack build --instance <ścieżka>/minecraft --out dist/`

1. Czyta `mods/.index/*.pw.toml`. Dla `mode = 'url'` przepisuje URL i hash bez pobierania.
   Dla `mode = 'metadata:curseforge'` składa adres z `project-id` i `file-id`.
2. Pomija `*.bak`, `*.disabled` i sam katalog `.index/` (62 MB śmieci w obecnej paczce).
3. Weryfikuje mapowanie 1:1 między metadanymi a jarami — rozjazd przerywa budowanie,
   bo oznacza mod dodany ręcznie bez metadanych.
4. Kopiuje `config/` i `options.txt` do `dist/files/<2>/<hash>`, licząc SHA-512.
5. Zapisuje `dist/manifest.json`.

Publikacja: `git -C dist push` → GitHub Pages. U testerów następny start dociąga różnicę.

## 12. Interfejs

Okno bez ramki systemowej, ~900×560, trzy widoki.

**Główny** — nazwa i edycja paczki, liczba modów, wersja MC i loadera, chip konta w prawym
górnym rogu, duży przycisk GRAJ, pod nim linia stanu i pasek postępu, rozwijane „szczegóły"
z logiem. Ikona koła zębatego.

**Logowanie** — przycisk „Zaloguj przez Microsoft", pod nim separator „albo" i sekcja
trybu offline: pole na nick, przycisk „Graj" i zdanie wyjaśniające, że działa tylko na
serwerze z `online-mode=false`. Tryb offline jest widoczny od razu, bez chowania w
ustawieniach — to bezpiecznik do testowania paczki.

Po kliknięciu logowania Microsoft: duży kod, przycisk „kopiuj i otwórz przeglądarkę",
odliczanie ważności kodu.

**Ustawienia** — suwak RAM, „napraw instalację" (kasuje `data/mc/`, zostawia światy),
„otwórz folder gry", wersja launchera z informacją o dostępnej nowszej.

Brak ramki systemowej jest jedynym elementem UI o niepewnym koszcie. Jeśli okaże się
kłopotliwy, wracamy do zwykłego okna — reszta wyglądu od tego nie zależy.

## 13. Obsługa błędów

Każda operacja sieciowa: 3 próby z narastającym odstępem. Niezgodność hasha powoduje
jedną ponowną próbę z kolejnego URL-a z listy. Wyczerpanie prób kończy się komunikatem
zawierającym nazwę pliku, próbowane adresy i przyczynę.

Nie ma cichych zaniechań. Jeżeli mod się nie pobrał, launcher odmawia startu gry —
uruchomienie niekompletnej paczki daje crash, którego przyczyny nikt nie odgadnie.

## 14. Testy

**Jednostkowe:** parsowanie manifestu; odrzucanie ścieżek wrogich (`../`, absolutnych,
z literą dysku, z ukośnikiem odwrotnym); pełna tablica decyzyjna z §7 dla obu trybów;
UUID offline na znanym wektorze; scalanie `inheritsFrom`; podstawianie szablonów
argumentów; budowa classpatha z regułami systemowymi.

**Integracyjne:** lokalny serwer HTTP z atrapą paczki; synchronizacja do katalogu
tymczasowego i porównanie drzewa; ręcznie zmodyfikowany config zostaje nietknięty;
przerwane pobieranie wznawia się i nie zostawia pliku `.part`; plik o złym hashu jest
odrzucany, nie instalowany.

**Ręczne:** pełna instalacja na czystym Linuksie, potem na czystym Windowsie.

## 15. Dystrybucja

GitHub Actions, macierz `ubuntu-latest` i `windows-latest`. Tag `v*` buduje obie binarki
i wrzuca do Releases. To omija brak `rustup` i `mingw-w64` na maszynie deweloperskiej
i daje natywne buildy zamiast cross-kompilacji.

Aktualizacja launchera: manifest podaje najnowszą wersję i adresy. Launcher porównuje ją
ze swoją i pokazuje informację z przyciskiem pobierania. Podmiany pliku wykonywalnego
w locie nie robimy — na Windowsie działający plik jest zablokowany i wymagałoby to
procesu pomocniczego, co nie jest warte złożoności przy kilku testerach.

Niepodpisany plik wykonywalny wywoła ostrzeżenie SmartScreen na Windowsie. Podpisywanie
kodu kosztuje i wykracza poza zakres; README opisze, jak przejść przez ostrzeżenie.

## 16. Poza zakresem

Świadomie pominięte: obsługa wielu paczek i profili, przeglądanie i zarządzanie modami
z poziomu launchera, podmiana pliku wykonywalnego w locie, macOS, architektury ARM,
zarządzanie shaderami i resourcepackami, podgląd skórki, statystyki gry, automatyczne
łączenie z serwerem.

## 17. Ryzyka

| Ryzyko | Waga | Odpowiedź |
|---|---|---|
| Microsoft odcina `client_id` oficjalnego launchera | wysoka | `msa_client_id` w manifeście + tryb offline; podmiana bez nowego wydania launchera |
| Mod znika z Modrinth albo CurseForge | średnia | `urls` jest listą; dopisanie mirrora to zmiana manifestu |
| Pierwsze pobranie 1,8 GB zniechęca testera | średnia | wyraźny postęp, wznawialność, jasny komunikat, że to jednorazowo |
| CDN ogranicza tempo przy 249 równoległych żądaniach | niska | limit równoległości 8, retry z narastającym odstępem |
| Instalator NeoForge zmienia zachowanie w przyszłej wersji | niska | wersja przypięta w manifeście; test integracyjny na sztucznym profilu |
| Ramka niestandardowa okna sprawia problemy na jakimś WM | niska | zejście do zwykłego okna nie rusza reszty wyglądu |
