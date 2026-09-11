# Przebudowa manifestu

Manifest buduje narzędzie wiersza polecenia `chmurka` z crate'a
`crates/cli`. Binarka nazywa się `chmurka` (`[[bin]]` w
`crates/cli/Cargo.toml`), a w drzewie źródeł uruchamia się ją przez cargo:

```bash
cargo run -p chmurka-cli -- pack-build --help
```

Nazwy poleceń pochodzą z wariantów `enum Polecenie` i clap zapisuje je
z myślnikiem, więc polecenie nazywa się `pack-build`, nie `pack build`.

## Polecenia

| Polecenie | Do czego służy |
|---|---|
| `pack-build` | buduje `manifest.json` i katalog plików do wypchnięcia na GitHub Pages |
| `install` | pobiera manifest i doprowadza wskazany katalog danych do stanu gotowego |
| `run` | to co `install`, a potem uruchamia grę na koncie offline |
| `login` | sprawdza logowanie kontem Microsoft i wypisuje nick oraz UUID |

`install` i `run` służą do sprawdzenia paczki bez uruchamiania launchera.
`run` celowo nie czyta ustawień gracza — uruchamia grę tak, jak wyglądałoby
to bez żadnej konfiguracji, z pamięcią 512–4096 MB podaną wprost w kodzie.

## `pack-build` — argumenty

| Argument | Wymagany | Domyślnie | Znaczenie |
|---|---|---|---|
| `--instance <ŚCIEŻKA>` | tak | — | katalog `minecraft` instancji PrismLaunchera |
| `--out <ŚCIEŻKA>` | nie | `dist` | katalog wynikowy: powstaje w nim `manifest.json` i `files/` |
| `--base-url <ADRES>` | tak | — | bazowy adres, pod którym będą leżeć nasze pliki |
| `--version <TEKST>` | tak | — | wersja paczki, trafia do `pack.version` |
| `--launcher-version <TEKST>` | nie | `0.4.20` | wersja ogłaszana graczom jako najnowszy launcher |

Przebudowa, która od razu ląduje tam, skąd czytają gracze:

```bash
cargo run -p chmurka-cli -- pack-build \
  --instance /ścieżka/do/instancji/minecraft \
  --out docs \
  --base-url https://ttk0721.github.io/chmurkowy-launcher \
  --version 2026.09.10-21 \
  --launcher-version 0.4.24
```

`docs/` jest korzeniem GitHub Pages — leżą tam `manifest.json` i pliki paczki,
a gotowa dokumentacja idzie do podkatalogu `docs/wiki/` właśnie po to, żeby
ich nie zadeptać (komentarz na górze `mkdocs.yml`). Domyślne `dist` jest
w `.gitignore`, więc budowanie bez `--out` nie zostawia niczego, co dałoby się
przez przypadek zatwierdzić — i niczego, co zobaczy gracz.

!!! uwaga

    `--base-url` musi wskazywać na to samo miejsce, w którym naprawdę wyląduje
    katalog `files/`. Z tego adresu składane są wpisy dla wszystkich plików
    hostowanych u nas, a walidacja manifestu przepuszcza wyłącznie adresy
    `https://`. Zły adres nie przerwie budowania — zobaczą go dopiero gracze,
    jako nieudane pobieranie.

## Co robi `pack-build`, krok po kroku

1. Sprawdza, czy istnieje `mods/.index`. Gdy go nie ma, przerywa:
   `brak … — czy to na pewno instancja PrismLaunchera?`.
2. Czyta wszystkie pliki `*.toml` z `mods/.index` i wyciąga z nich nazwę
   pliku, adres, hasz i identyfikatory CurseForge.
3. Wypisuje pliki `*.jar` z `mods/` i sortuje je alfabetycznie, żeby lista
   modów w manifeście nie zależała od kolejności, w jakiej system zwrócił
   zawartość katalogu.
4. Jeśli któryś jar nie ma metadanych, przerywa budowanie — nie ma skąd
   go pobrać.
5. Dla każdego moda liczy SHA-512 z pliku na dysku. To jedyne źródło prawdy
   o tym, co gramy; hasz z metadanych bywa SHA-1 (CurseForge) i służy tylko
   do porównania ze źródłem.
6. Porównuje plik z tym, co leży pod adresem źródłowym. Zgodny — do manifestu
   idzie adres Modrinth albo CurseForge. Niezgodny — plik ląduje w naszym
   magazynie. Opisuje to [Mody hostowane u nas](hostowane.md).
7. Zbiera pliki z `config/` (rekurencyjnie) i `options.txt`, kopiuje je
   do magazynu i wystawia na nasz adres.
8. Skleja manifest, sprawdza własny wynik funkcją
   `chmurka_core::manifest::parse` i dopiero wtedy zapisuje
   `<out>/manifest.json`. Gdy walidacja nie przejdzie, kończy się błędem
   `wygenerowany manifest nie przechodzi walidacji` i nie zapisuje nic.
9. Wypisuje `Zapisano N wpisów do <out>`.

Krok ósmy jest tam dlatego, że manifest jest danymi pobieranymi z sieci i
launcher odrzuca go w całości, gdy coś się nie zgadza: obca wersja schematu,
ścieżka wychodząca poza katalog instancji, adres inny niż `https://` albo wpis
bez ani jednego adresu. Lepiej, żeby taki plik nie powstał, niż żeby trafił
na serwer i zatrzymał wszystkich graczy naraz.

## Co trafia do manifestu

| Co | Polityka | Adres | Znaczenie polityki |
|---|---|---|---|
| `mods/*.jar` | `mirror` | źródłowy albo nasz | plik musi zgadzać się co do hasza, inaczej jest pobierany od nowa |
| `config/**` | `smart` | zawsze nasz | nadpisujemy tylko wtedy, gdy gracz pliku nie ruszał |
| `options.txt` | `seed` | zawsze nasz | wgrywamy raz i nigdy więcej nie dotykamy |

Pliki hostowane u nas lądują w magazynie adresowanym treścią:
`files/<dwa pierwsze znaki hasza>/<pełny hasz>`. Ten sam plik w kolejnych
wersjach paczki zajmuje miejsce tylko raz, bo kopiowanie jest pomijane, gdy
plik o takiej nazwie już tam leży.

Reszta manifestu jest wpisana w `crates/cli/src/main.rs` na sztywno i nie da
się jej zmienić argumentem: nazwa paczki, edycja, wersja Minecrafta i
NeoForge, Java 21 Temurin, pamięć 512–4096 MB, identyfikator aplikacji
Microsoft, `mirror_dirs` oraz lista narzędzi zewnętrznych (`yt-dlp`
i `ffmpeg` dla moda Create: Harmonics, pobierane z ich własnych adresów).

## Wersja paczki

`--version` trafia do manifestu wprost, jako `pack.version`. Kod nie sprawdza
kształtu tego tekstu — dzisiejszy `2026.09.10-21` to umowa „data i numer
w ciągu dnia", nie wymóg.

!!! uwaga

    Podbicie wersji paczki samo w sobie niczego u graczy nie uruchamia.
    Launcher porównuje pliki wpis po wpisie, po SHA-512, i nie patrzy na
    `pack.version` — w oknie launchera widać nazwę paczki i edycję, a samą
    wersję wypisuje tylko `chmurka install` i `chmurka run`, w linii
    `Paczka Chmurkowy Serwer wersja …`. Numer służy więc do rozmowy
    i do zgłoszeń, a nie do sterowania aktualizacją.

## Wersja launchera

`--launcher-version` trafia w dwa miejsca naraz: do `launcher.latest_version`
i do dwóch adresów plików wydania, składanych jako

```
https://github.com/ttk0721/chmurkowy-launcher/releases/download/v<wersja>/ChmurkowyLauncher-linux-x64
https://github.com/ttk0721/chmurkowy-launcher/releases/download/v<wersja>/ChmurkowyLauncher-windows-x64.exe
```

Adresy celowo wskazują konkretne wydanie, a nie `latest`. Launcher podmienia
się sam, więc musi dostać dokładnie tę wersję, którą manifest ogłasza. Przy
`latest` klient sięgający po plik, zanim budowanie wydania się skończy,
pobrałby po cichu poprzednią binarkę i uznał, że aktualizacja nie wskoczyła.
Pilnuje tego test `adres_wskazuje_konkretne_wydanie`.

!!! uwaga

    Domyślna wartość `--launcher-version` to `0.4.20` i nie podąża za wersją
    crate'a — `crates/launcher/Cargo.toml` jest dziś na `0.4.24`. Budowanie
    bez tego argumentu ogłasza więc starą wersję. Launcher gracza porównuje
    numery liczbowo, człon po członie, więc ktoś z `0.4.24` nie zrobi nic,
    ale ktoś z `0.4.18` zostanie ściągnięty do `0.4.20` zamiast do najnowszej.
    Podawaj tę wartość zawsze i sprawdź, czy zgadza się z `Cargo.toml`.

Wersję wolno ogłosić dopiero wtedy, gdy wydanie o tagu `v<wersja>` ma już
wgrane pliki. Ogłoszona wcześniej kończy się nieudanym pobraniem u graczy;
launcher zapamiętuje nieudaną próbę w pliku `aktualizacja.txt` w katalogu
danych, żeby nie podmieniać się w kółko zamiast wpuścić gracza do gry.
Szczegóły: [Samoaktualizacja](../budowa/samoaktualizacja.md) i
[Wydanie nowej wersji](../projekt/wydanie.md).

## Co nie trafia do paczki

Ograniczenie dla modów jest jedno: do manifestu wchodzą wyłącznie pliki
o nazwie kończącej się na `.jar`. Wyłączony mod (`*.jar.disabled`) i kopia
(`*.jar.bak`) wypadają z paczki bez słowa.

Filtry z funkcji `smiec_roboczy`, `katalog_smieci` i `katalog_na_swiat`
dotyczą wyłącznie zawartości `config/`. Wszystkie powstały po tym, jak
wymienione pliki naprawdę pojechały do testerów — i mają testy, które
pilnują, żeby pojechać nie mogły ponownie.

| Odpada | Co to jest | Dlaczego |
|---|---|---|
| nazwa zaczyna się od kropki | pliki ukryte | to nie jest konfiguracja paczki |
| `*.bak`, `*.part`, `*~` | pliki robocze | powstały u utrzymującego i nie mają czego szukać u graczy |
| nazwa zawiera `.toml_backup` | kopie configów zostawiane przez NeoForge | zajmowały miejsce w manifeście, a dwie kopie o wspólnym trzonie nazwy wywracały pobieranie u testerów |
| nazwa zawiera `.json_backup` | to samo dla plików JSON | jak wyżej |
| `*.log` | dziennik moda z komputera utrzymującego | graczom niepotrzebny, a rośnie z każdą rozgrywką |
| `sodium-fingerprint.json` | odcisk karty graficznej tego komputera | u każdego gracza jest inny i tak zostanie nadpisany przy pierwszym uruchomieniu, a po drodze mówi wszystkim, jaką kartę ma utrzymujący paczkę |
| `early_window_reference.properties` | rozmiar i położenie okna wczesnego ładowania | sprawa jednego ekranu, nie paczki |
| katalog `log` albo `logs`, dowolną wielkością liter | katalog, który mod zakłada na własne potrzeby | u gracza powstanie sam, a u nas zbiera zapiski z rozgrywek |
| podkatalogi `config/inventoryprofilesnext/` poza `integrationHints` | zakładki handlu z wieśniakami, trzymane osobno dla każdego świata | katalogi nazywają się jak światy gracza; u utrzymującego siedzą tam światy testowe („Test 2", „Test3", „Świat do testów chmurki") i jechały do wszystkich graczy razem ze swoimi nazwami |

Ostatnia reguła dotyczy tylko tego jednego moda: `integrationHints` to jego
jedyny podkatalog wspólny dla wszystkich, a inne mody mają swoje podkatalogi
spisywane normalnie. Pilnują tego testy `regula_nie_wykracza_poza_ten_mod`
i `wspolne_ustawienia_tego_moda_zostaja`.

!!! wskazówka

    Gdy któryś mod zacznie zostawiać w `config/` kolejny plik roboczy, dopisz
    warunek do `smiec_roboczy` w `crates/cli/src/main.rs` razem z testem
    i komentarzem mówiącym, co to za plik. Sam warunek po pół roku nikomu
    nic nie powie, a od tego zależy, czy następna osoba go nie skasuje.
