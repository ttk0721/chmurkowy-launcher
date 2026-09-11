# Mody hostowane u nas

Większość paczki gracz pobiera prosto z Modrinth i CurseForge — w dzisiejszym
manifeście 250 modów z `cdn.modrinth.com` i 11 z `mediafilez.forgecdn.net`.
Trzy mody idą z naszego adresu na GitHub Pages. Nie jest to wybór podjęty
ręcznie: decyduje o tym `chmurka pack-build` przy każdym budowaniu.

## Kiedy mod ląduje u nas

Dla każdego jara narzędzie liczy SHA-512 z pliku leżącego w instancji,
a potem sprawdza, czy to naprawdę ten sam plik, który gracz pobierze
spod adresu źródłowego:

| Co mówią metadane PrismLaunchera | Co robi budowanie |
|---|---|
| `hash-format = 'sha512'` i hasz zgadza się z plikiem | wpis dostaje adres źródłowy |
| `hash-format = 'sha1'` i policzony SHA-1 zgadza się z plikiem | wpis dostaje adres źródłowy |
| hasz się nie zgadza | plik trafia do naszego magazynu |
| metadane nie mają hasza albo format jest inny | plik trafia do naszego magazynu |

Powód jest taki, że manifest ma opisywać dokładnie tę paczkę, którą
utrzymujący sprawdził u siebie. Gdy plik na dysku różni się od tego spod
adresu źródłowego — bo mod został lokalnie załatany, pobieranie się urwało
albo autor podmienił plik pod tym samym adresem — wskazanie CDN-u dałoby
graczom inną paczkę niż ta, na której ktokolwiek grał. Pobranie i tak by się
nie udało, bo launcher sprawdza SHA-512 każdego pobranego pliku i odrzuca
niezgodny.

Budowanie wypisuje wtedy na wyjście błędów listę takich modów:

```
UWAGA: 3 mod(ów) różni się od pliku pod adresem źródłowym — hostuję je u siebie,
żeby testerzy dostali dokładnie to, co masz w instancji:
  - create-1.21.1-6.0.10.jar
  …
Jeśli to niezamierzone, pobierz te mody na nowo w PrismLauncherze.
```

!!! uwaga

    To ostrzeżenie jest jedynym miejscem, w którym widać, że mod przeszedł
    pod nasz adres. W samym manifeście nie ma śladu po powodzie — jest tylko
    inny adres. Czytaj tę listę przy każdej przebudowie: mod, który wskoczył
    na nią po urwanym pobieraniu, będzie u nas siedział tak długo, aż ktoś
    pobierze go na nowo.

## Co jest dziś hostowane u nas

| Plik | Rozmiar |
|---|---|
| `mods/create-1.21.1-6.0.10.jar` | 19,1 MB |
| `mods/create-aeronautics-bundled-1.21.1-1.3.0.jar` | 33,1 MB |
| `mods/sable-neoforge-1.21.1-2.0.1.jar` | 12,9 MB |

Poza modami u nas leżą zawsze wszystkie pliki konfiguracji (445 wpisów)
i `options.txt` — one nie mają żadnego adresu źródłowego, bo są nasze.
Razem `docs/files/` waży dziś około 70 MB.

## Magazyn adresowany treścią

Plik hostowany u nas ląduje pod ścieżką złożoną z jego własnego hasza:

```
docs/files/<dwa pierwsze znaki hasza>/<pełny hasz SHA-512>
```

a do manifestu trafia adres złożony z `--base-url` i tej samej ścieżki:

```
https://ttk0721.github.io/chmurkowy-launcher/files/c3/c39055b1a284e7a6…
```

Kopiowanie jest pomijane, gdy plik o takiej nazwie już w magazynie leży,
więc ten sam config w dziesięciu kolejnych wersjach paczki zajmuje miejsce
raz. Dwie rzeczy wynikają z tego wprost:

- Adres zależy od treści, więc **nie da się podmienić pliku w miejscu**.
  Nowa wersja moda to nowy hasz, nowy plik w magazynie i nowy wpis
  w manifeście. Wgranie nowego jara do `docs/files/` bez przebudowy
  manifestu nie zmienia niczego.
- `pack-build` nigdy niczego z magazynu nie kasuje. Po zmianie moda stary
  plik zostaje, choć manifest już go nie wymienia.

## `mirror_dirs`, czyli katalog lustrzany

W manifeście są dwie osobne rzeczy o tej samej nazwie i łatwo je pomylić:

- `"policy": "mirror"` przy pojedynczym pliku — plik musi zgadzać się
  co do hasza, inaczej jest pobierany od nowa. Dotyczy tego jednego wpisu.
- `"mirror_dirs": ["mods"]` — lista katalogów, w których launcher kasuje
  wszystko, czego manifest nie wymienia. Dotyczy całego katalogu.

Działa to tak: po zaplanowaniu każdego wpisu launcher przechodzi rekurencyjnie
katalogi z `mirror_dirs` i dla każdego znalezionego pliku spoza manifestu
dodaje skasowanie. Plik jest usuwany, a jego wpis znika ze `state.json`,
z notatką `usunięto obcy plik: mods/nazwa.jar`. Pliki i katalogi o nazwie
zaczynającej się od kropki są pomijane — `mods/.index` to metadane
PrismLaunchera, nie nasza sprawa.

`mirror_dirs` zawiera dziś wyłącznie `mods` i jest wpisane na sztywno
w `crates/cli/src/main.rs`. Katalog `config/` nie jest lustrzany, bo jego
pliki mają politykę `smart`, która z założenia chroni to, co gracz sam
poprawił. Kasowanie tam wszystkiego nieznanego przeczyłoby tej polityce
i wyrzucało pliki modów, o których paczka nie wie.

## Czym to grozi przy aktualizacji takiego moda

**Repozytorium rośnie o pełny rozmiar każdej wersji.** Podbicie Create
o jedną wersję dokłada kolejne 19 MB do `docs/files/`, a stary plik zostaje.
Skasowanie go ręcznie zwalnia miejsce w katalogu roboczym, ale nie w historii
gita — obiekt zostaje w repozytorium na zawsze. Przy modach tej wielkości
kilka aktualizacji z rzędu zauważalnie powiększa każdy klon.

**Skasowanie pliku, który manifest wciąż wymienia, zatrzymuje graczy.**
Wpis hostowany u nas ma dokładnie jeden adres — nie ma zapasowego źródła,
na które launcher mógłby się przełączyć. Po trzech nieudanych próbach
pobieranie kończy się błędem `nie udało się pobrać …: wyczerpano próby dla
adresów […]`. Kasuj z magazynu dopiero wtedy, gdy żaden opublikowany manifest
już tego pliku nie wymienia.

**Mod może wrócić na adres źródłowy sam.** Jeśli pobierzesz go w
PrismLauncherze na nowo i plik zgodzi się z haszem z metadanych, kolejna
przebudowa wstawi adres Modrinth albo CurseForge. Nasza kopia przestanie być
komukolwiek potrzebna, ale nadal będzie leżeć w `docs/files/`.

**Ruch idzie z naszego serwera.** Każdy gracz przy pierwszej instalacji
pobiera te megabajty z GitHub Pages zamiast z CDN-u autora moda. To jedyne
pliki paczki, za których dostarczenie odpowiadamy sami.

!!! wskazówka

    Po podbiciu wersji moda hostowanego u nas warto sprawdzić, czy przy
    następnej przebudowie nadal jest na liście ostrzeżenia. Jeśli zniknął,
    mod wrócił na adres źródłowy i nie ma powodu, żeby dalej zajmować sobą
    repozytorium.

Jak to wygląda od strony gracza, opisuje
[Synchronizacja paczki](../budowa/synchronizacja.md); budowę samego pliku —
[Manifest](../budowa/manifest.md).
