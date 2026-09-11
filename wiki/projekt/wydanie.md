# Wydanie nowej wersji

Wydanie to pięć kroków: podbicie wersji, scalenie tej zmiany przez pull
requesta, tag adnotowany, wypchnięcie tagu i — na samym końcu — podbicie
manifestu. Cztery pierwsze kroki robi człowiek. Dopiero wypchnięcie tagu
uruchamia workflow `.github/workflows/release.yml`, który buduje pliki
i publikuje wydanie — czyli wszystko między krokiem czwartym a piątym.

Ostatni krok jest osobno i nie robi go automat. Dopóki manifest podaje starą
wersję, launcher gracza nie wie, że coś wyszło — pliki w wydaniu leżą, ale
nikt po nie nie sięga.

## 1. Podbicie wersji w Cargo.toml

Wersja launchera stoi w jednym miejscu — w `crates/launcher/Cargo.toml`:

```toml
[package]
name = "chmurkowy-launcher"
version = "0.4.24"
```

Ta liczba jest wkompilowana w binarkę. Launcher czyta ją przez
`env!("CARGO_PKG_VERSION")` i porównuje z `launcher.latest_version`
z manifestu (`crates/launcher/src/app.rs`). Jeśli zapomnisz ją podbić,
wydanie powstanie, ale program w środku nadal będzie się przedstawiał starym
numerem — po podbiciu manifestu każdy launcher uzna, że jest nieaktualny,
pobierze plik, zobaczy tę samą wersję i zrobi to jeszcze raz.

Podbij `version` i przebuduj, żeby `Cargo.lock` złapał nowy numer:

```bash
cargo check --workspace
```

Do commita idą dwa pliki: `crates/launcher/Cargo.toml` i `Cargo.lock`.

!!! uwaga

    Numer musi mieć kształt `X.Y.Z` z trzech liczb. Funkcja `numery`
    w `crates/core/src/aktualizacja.rs` rozbija go po kropkach i porównuje
    liczbowo — dzięki temu 0.4.10 wygrywa z 0.4.9, co przy porównaniu
    tekstowym wypadłoby odwrotnie. Wersja o innym kształcie jest porównywana
    tekstowo i wtedy „nowsza” znaczy tylko „inna”.

Numery wolno przeskakiwać. Wersja 0.4.23 nigdy nie została opublikowana, więc
po 0.4.22 przyszło 0.4.24 — to nie jest problem, bo porównanie sprawdza tylko,
czy manifest podaje coś wyższego.

## 2. Commit i pull request

Na `main` nie wchodzi się wprost, więc podbicie wersji też idzie przez pull
requesta — razem ze zmianami, które mają być w tym wydaniu, albo osobno.
Szczegóły reguł: [Kontrole i ochrona gałęzi](kontrole.md).

Po scaleniu przez **Squash and merge** na `main` powstaje **nowy** commit,
inny niż ten z gałęzi. Tag zakłada się na nim, więc najpierw trzeba ściągnąć
aktualny `main`:

```bash
git switch main
git pull
```

## 3. Tag adnotowany

Opis wydania bierze się z treści tagu. Nie ma osobnego pliku z listą zmian —
changelog powstaje razem z wersją, więc nie da się go zapomnieć ani rozjechać
z tym, co faktycznie weszło.

```bash
git tag -a v0.4.25
```

Git otworzy edytor. To, co tam napiszesz, trafi w całości do opisu wydania na
GitHubie i będzie widoczne dla każdego, kto otworzy stronę wydania. Opis jest
renderowany jako Markdown — w dotychczasowych wydaniach to lista punktów,
każdy zaczynający się od pogrubionego zdania o tym, co się zmieniło, a niżej
akapit, dlaczego.

!!! uwaga

    Tag musi być adnotowany, czyli zakładany z `-a` (albo `-s`). Tag lekki,
    czyli `git tag v0.4.25` bez żadnej flagi, nie ma własnej treści.
    Workflow czyta ją poleceniem `git tag -l --format='%(contents)'`,
    a dla tagu lekkiego to polecenie zwraca treść commita — w opisie wydania
    wylądowałby wtedy komunikat commita zamiast listy zmian.

## 4. Wypchnięcie tagu

```bash
git push origin v0.4.25
```

To wypchnięcie uruchamia workflow wydania. Sam `git push` bez tagu go nie
uruchomi — wyzwalaczem jest `push` z filtrem `tags: ["v*"]`. Uruchomi za to
Kontrolę i CodeQL, bo te chodzą na `push` do `main` i na `pull_request`.

## Co robi workflow wydania

### Matryca dwóch systemów

Zadanie `build` idzie równolegle na dwóch obrazach:

| Obraz | Plik z `target/release` | Nazwa artefaktu |
|---|---|---|
| `ubuntu-22.04` | `ChmurkowyLauncher` | `ChmurkowyLauncher-linux-x64` |
| `windows-latest` | `ChmurkowyLauncher.exe` | `ChmurkowyLauncher-windows-x64.exe` |

Matryca ma `fail-fast: false`, więc wywrotka na jednym systemie nie przerywa
budowania na drugim. Na obu systemach lecą po kolei: instalacja zależności
systemowych (tylko Linux — `eframe` linkuje się tam z bibliotekami okien
i dźwięku), `cargo test --workspace`, a potem
`cargo build --release -p chmurkowy-launcher`. Nieprzechodzący test zatrzymuje
wydanie przed zbudowaniem czegokolwiek.

!!! uwaga

    Krok budowania podstawia zmienną `CHMURKA_MANIFEST_URL` z ustawień
    repozytorium (Settings → Secrets and variables → Actions → Variables).
    Adres manifestu jest wkompilowany w binarkę — plik konfiguracyjny obok
    programu dałoby się zgubić albo zepsuć. Gdy zmienna jest pusta,
    `crates/launcher/build.rs` wstawia adres zastępczy
    `https://przyklad.github.io/chmurka-pack/manifest.json`, a taki launcher
    nie pobierze niczego u nikogo. Budowanie się nie wywraca, więc to trzeba
    sprawdzić samemu.

### Kontrola zgodności glibc

Po zbudowaniu, tylko na Linuksie, workflow sprawdza binarkę:

```bash
MAX=$(objdump -T target/release/ChmurkowyLauncher \
  | grep -oE 'GLIBC_[0-9]+\.[0-9]+' | sort -uV | tail -1)
```

Jeśli najwyższa wymagana wersja przekracza `GLIBC_2.35`, krok kończy się
błędem i wydanie nie powstaje.

Powód jest historyczny. Binarka wymaga glibc w wersji z systemu, na którym
powstała. Budowanie na `ubuntu-latest` (24.04) dawało `GLIBC_2.39` i taki plik
nie startował na Ubuntu 22.04, Debianie 12, Mincie 21 ani Fedorze 39 — czyli
u sporej części odbiorców. Dlatego obraz jest przypięty do `ubuntu-22.04`
(glibc 2.35, pokrywa wszystko od 2022 roku wzwyż), a kontrola pilnuje, żeby
podbicie obrazu w przyszłości nie odcięło tych systemów po cichu. Bez niej
zmiana jednej linijki w workflow psułaby grę ludziom bez żadnego sygnału.

### Pakowanie

Na Linuksie powstaje archiwum `ChmurkowyLauncher-linux-x64.tar.gz`
z katalogiem `ChmurkowyLauncher-linux`, w którym leżą: binarka, `install.sh`,
`uninstall.sh`, wpis `chmurkowy-launcher.desktop` i `ikona.png`. Zaraz po
spakowaniu workflow wypisuje zawartość przez `tar tzf` — w logu widać wtedy,
czy w archiwum naprawdę jest wszystko.

Na Windowsie powstaje instalator. Workflow szuka kompilatora Inno Setup pod
`C:\Program Files (x86)\Inno Setup 6\ISCC.exe`, a gdy go nie ma — dokłada go
przez `choco install innosetup`. Numer wersji bierze z nazwy tagu, ucinając
początkowe `v`, i podaje kompilatorowi:

```powershell
& $iscc "/DWersja=$wersja" packaging\windows\chmurkowy.iss
```

Skrypt `packaging/windows/chmurkowy.iss` ma na wypadek pominięcia tej
definicji wartość zapasową `0.0.0` — instalator zbudowany ręcznie bez
`/DWersja` przedstawi się właśnie tak.

Trzy rzeczy w tym skrypcie są nieoczywiste i celowe:

- `PrivilegesRequired=lowest` i katalog `{autopf}` — instalacja idzie dla
  użytkownika, nie dla całego komputera. Launcher aktualizuje się sam,
  podmieniając własny plik, więc musi leżeć tam, gdzie ma prawo zapisu bez
  hasła administratora. W `Program Files` każda poprawka wymagałaby ponownej
  instalacji z podniesionymi uprawnieniami.
- `AppId` to stały identyfikator `{{7B3C1E42-9D5A-4F18-AC77-2E6B0D9F4A31}`.
  Po nim Windows poznaje, że kolejna instalacja dotyczy tej samej aplikacji.
  Zmiana tej wartości zrobiłaby z aktualizacji drugi, równoległy wpis
  w „Aplikacjach i funkcjach”.
- Sekcja `[Files]` wymienia wyłącznie `ChmurkowyLauncher.exe`. Katalog z grą,
  paczką i światami leży w `%LOCALAPPDATA%\ChmurkowyLauncher` i nie jest tu
  wymieniony — odinstalowanie ma usunąć program, a nie światy gracza.

Wyniki obu systemów idą do artefaktów kroku `upload-artifact` z ustawieniem
`if-no-files-found: ignore`. Lista ścieżek jest wspólna dla obu obrazów,
a każdy z nich wytwarza tylko część tych plików — bez tego ustawienia krok
zgłaszałby brak plików, które z założenia nie miały tam powstać.

### Publikacja od razu po Linuksie

Linux kompiluje się około 6 minut, Windows około 11. Workflow nie czeka na
oba: zaraz po zakończeniu części linuksowej publikuje wydanie z plikami
Linuksa i dokłada do opisu adnotację, że wersja na Windowsa jeszcze się
kompiluje i pojawi się za kilka minut. Gotowa binarka nie ma po co leżeć
w szufladzie te dodatkowe minuty.

Opis powstaje tak:

```bash
git fetch --force --tags origin
git tag -l --format='%(contents)' "$GITHUB_REF_NAME" > notatki.md
```

Pobranie tagów jest konieczne mimo `fetch-depth: 0` przy `checkout`. Bez
jawnego pobrania tagów adnotowanych git widzi tag jako lekki i `%(contents)`
zwraca treść commita zamiast opisu wydania.

Gdy zadanie `dolacz-windowsa` skończy pracę, pobiera artefakt Windowsa,
odtwarza opis z tagu **bez** adnotacji i dokłada do wydania dwa pliki. Czyli
adnotacja o czekaniu znika sama, bo opis jest budowany od zera z treści tagu.

Stan końcowy wydania:

| Plik | Z którego zadania | Do czego służy |
|---|---|---|
| `ChmurkowyLauncher-linux-x64` | `build` (Linux) | Goła binarka — po nią sięga samoaktualizacja |
| `ChmurkowyLauncher-linux-x64.tar.gz` | `build` (Linux) | Archiwum instalacyjne dla gracza |
| `ChmurkowyLauncher-windows-x64.exe` | `dolacz-windowsa` | Goła binarka — po nią sięga samoaktualizacja |
| `ChmurkowyLauncher-setup.exe` | `dolacz-windowsa` | Instalator dla gracza |

Gołe binarki leżą w wydaniu obok instalatorów właśnie dlatego, że
[samoaktualizacja](../budowa/samoaktualizacja.md) podmienia pojedynczy plik
launchera, a nie przechodzi przez instalator.

### Gdy Windows nie wyjdzie

Trzecie zadanie, `sprostuj-windowsa`, uruchamia się warunkiem
`if: failure()`. Gdy budowanie na Windowsie padnie już po tym, jak Linux
opublikował wydanie, w opisie zostałaby obietnica pliku, który nigdy nie
przyjdzie — ktoś odświeżałby stronę bez końca.

Zadanie najpierw sprawdza, czy wydanie w ogóle istnieje
(`gh release view`); brak wydania znaczy, że Linux też nie wyszedł i nie ma
czego prostować. Jeśli istnieje, podmienia opis na treść tagu z adnotacją, że
budowanie wersji na Windowsa się nie powiodło i na razie jest tam tylko wersja
dla Linuksa.

!!! wskazówka

    Workflow ma też wyzwalacz `workflow_dispatch`, ale wszystkie kroki
    publikujące są obwarowane warunkiem
    `startsWith(github.ref, 'refs/tags/')`. Ręczne uruchomienie z gałęzi
    zbuduje więc pliki i zostawi je jako artefakty biegu, ale żadnego wydania
    nie utworzy ani nie zmieni. To bezpieczny sposób na sprawdzenie, czy
    budowanie i instalator w ogóle przechodzą.

## 5. Podbicie manifestu

Ten krok decyduje o tym, czy gracze cokolwiek dostaną. Manifest
(`docs/manifest.json`) ma sekcję:

```json
"launcher": {
  "latest_version": "0.4.24",
  "urls": {
    "linux-x64": "https://github.com/ttk0721/chmurkowy-launcher/releases/download/v0.4.24/ChmurkowyLauncher-linux-x64",
    "windows-x64": "https://github.com/ttk0721/chmurkowy-launcher/releases/download/v0.4.24/ChmurkowyLauncher-windows-x64.exe"
  }
}
```

Trzy wartości: numer i dwa adresy. Manifest buduje narzędzie `chmurka`, które
składa tę sekcję z wartości podanej flagą:

```bash
cargo run -p chmurka-cli -- pack-build \
  --instance <katalog minecraft instancji PrismLaunchera> \
  --out dist \
  --base-url <adres bazowy plików paczki> \
  --version 2026.09.10-21 \
  --launcher-version 0.4.25
```

Gdy mody się nie zmieniły, wersja paczki (`--version`) zostaje ta sama —
zmienia się wyłącznie numer launchera. Całą procedurę przebudowy opisuje
[Przebudowa manifestu](../paczka/przebudowa.md).

Adresy celowo wskazują konkretne wydanie, a nie `latest`. Launcher podmienia
się sam, więc musi dostać dokładnie tę wersję, którą manifest ogłasza. Przy
adresie z `latest` klient sięgający po plik, zanim workflow skończy budowanie,
pobrałby po cichu poprzednią binarkę i uznał, że aktualizacja nie wskoczyła.

!!! uwaga

    Manifest podbijaj dopiero wtedy, gdy pliki wydania są już na miejscu.
    Adres wskazuje konkretne wydanie, więc wcześniej nie ma czego pobrać.
    Nieudana próba nie powtarza się sama: launcher zapamiętuje wersję, przy
    której się wywrócił, i przy kolejnym starcie zostaje przy swojej
    (`Decyzja::JuzProbowano` w `crates/core/src/aktualizacja.rs`). Gracz,
    który trafi na moment między jednym a drugim, zostanie na starej wersji
    do czasu, aż sam kliknie sprawdzenie.

Commit z podbiciem manifestu idzie przez pull requesta tak samo jak każdy
inny. Po scaleniu GitHub Pages przebudowuje serwis, a launchery graczy
zobaczą nową wersję przy najbliższym sprawdzeniu — przy starcie albo
w ciągu dziesięciu minut, jeśli ktoś ma launcher otwarty.
