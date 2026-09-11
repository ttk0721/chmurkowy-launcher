# Dodanie i usunięcie moda

Mody dodaje się i usuwa w PrismLauncherze, w instancji utrzymującego paczkę.
Narzędzie `chmurka` niczego nie pobiera ani nie kasuje — tylko spisuje to,
co już leży w instancji, i zamienia na manifest.

## Dwa pliki na jeden mod

`chmurka pack-build` czyta dwa miejsca w katalogu `minecraft` instancji:

| Ścieżka | Co tam jest |
|---|---|
| `mods/nazwa.jar` | sam mod — z niego liczymy rozmiar i SHA-512 |
| `mods/.index/nazwa.pw.toml` | metadane: `filename`, `mode`, `url`, `hash`, `hash-format`, `project-id`, `file-id` |

Metadane zakłada PrismLauncher, gdy mod pobiera się jego własnym okienkiem
pobierania modów. Z nich bierze się adres, spod którego gracz pobierze plik:
dla Modrinth jest to `url` wprost, dla CurseForge adres składany
z `file-id` (`crates/cli/src/pw.rs`, funkcja `curseforge_url`).

Dlatego jar skopiowany do `mods/` ręcznie nie nadaje się do paczki: nie ma
metadanych, więc nie wiadomo, skąd gracz miałby go wziąć. Budowanie kończy
się wtedy błędem i nie zapisuje nic:

```
mody bez metadanych, nie wiadomo skąd je pobrać: ["nazwa.jar"]
```

To celowe. Cicha zgoda oznaczałaby manifest, którego nie da się w całości
pobrać, a błąd wyszedłby dopiero u gracza.

## Dodanie moda

1. W PrismLauncherze otwórz instancję, wybierz listę modów i pobierz mod
   przez wbudowane wyszukiwanie (Modrinth albo CurseForge). PrismLauncher
   zapisze `mods/nazwa.jar` i `mods/.index/nazwa.pw.toml`.
2. Uruchom grę z PrismLaunchera i sprawdź, czy wchodzi i czy mod nie kłóci
   się z resztą paczki. Manifest nie ma żadnej kontroli zgodności modów —
   sprawdza tylko to, czy każdy jar ma metadane.
3. Jeśli mod dokłada swoje pliki do `config/`, zostaw je tam. Przy
   przebudowie zostaną spisane razem z resztą konfiguracji i pojadą
   do graczy jako pliki z polityką `smart`.
4. Przebuduj manifest i opublikuj go — [Przebudowa manifestu](przebudowa.md).

## Usunięcie moda

1. W PrismLauncherze usuń mod z listy. Kasuje to i jar, i jego plik
   w `mods/.index/`.
2. Przebuduj manifest.

Osierocony plik `.pw.toml` bez jara nie przeszkadza: budowanie idzie po
plikach `*.jar` leżących w `mods/` i tylko dla nich szuka metadanych.
Metadane bez pliku są po prostu pomijane. Odwrotna sytuacja — jar bez
metadanych — przerywa budowanie.

!!! uwaga

    Wyłączenie moda w PrismLauncherze zmienia nazwę pliku na
    `nazwa.jar.disabled`. Do manifestu trafiają wyłącznie pliki o nazwie
    kończącej się na `.jar`, więc wyłączony mod po prostu wypada z paczki —
    bez ostrzeżenia i bez wpisu w wyniku budowania. Tak samo znikają kopie
    `nazwa.jar.bak`. Jeśli mod ma zostać w paczce, nie wyłączaj go, tylko
    zostaw włączony.

## Co się stanie u graczy

Launcher przy każdym starcie pobiera manifest i układa plan dla każdego
wpisu osobno (`crates/core/src/pack_sync.rs`, funkcja `plan`). Mody mają
politykę `mirror`, więc liczy się wyłącznie zgodność SHA-512 pliku na dysku
z haszem z manifestu.

| Co zrobiłeś w instancji | Co robi launcher gracza |
|---|---|
| Dodałeś mod | pobiera nowy plik, sprawdza SHA-512 i zapisuje go w `mods/` |
| Podbiłeś wersję moda | pobiera nowy plik, a stary znika jako plik spoza manifestu |
| Usunąłeś mod | kasuje plik u gracza i notuje `usunięto obcy plik: mods/nazwa.jar` |
| Nic nie zmieniłeś w danym modzie | nie rusza pliku i nic nie pobiera |

Pobierane jest tylko to, co się nie zgadza. Przy 264 modach zmiana jednego
z nich oznacza dla gracza jedno pobranie, nie całą paczkę od nowa.

Kasowanie działa dlatego, że manifest wymienia `mods` w `mirror_dirs`.
Launcher przechodzi wtedy ten katalog rekurencyjnie i usuwa każdy plik,
którego manifest nie zna. Szczegóły i skutki uboczne opisuje
[Mody hostowane u nas](hostowane.md#mirror_dirs-czyli-katalog-lustrzany).

!!! uwaga

    Mod, który gracz sam wrzucił do `mods/`, zostanie skasowany przy
    najbliższym uruchomieniu. To nie jest awaria, tylko sens polityki
    `mirror`: wszyscy mają grać na tej samej paczce, bo inaczej nie da się
    powiedzieć, co ktoś właściwie uruchomił, gdy gra przestanie działać.

## Konfiguracje zmieniają się inaczej

Pliki z `config/` mają politykę `smart`, a `options.txt` politykę `seed`:

- `smart` — launcher nadpisuje plik tylko wtedy, gdy sam go wcześniej wgrał
  i gracz od tego czasu go nie ruszał. Porównanie idzie po SHA-512
  zapisanym w `state.json`.
- `seed` — plik jest wgrywany raz, przy pustej instancji, i nigdy więcej
  nie jest dotykany. Dlatego zmiana `options.txt` w instancji utrzymującego
  nie dotrze do nikogo, kto już raz uruchomił grę.

Znaczy to tyle, że zmiana ustawień moda w paczce dojedzie tylko do graczy,
którzy sami tego pliku nie poprawiali. Pomijany plik launcher wypisuje
w dzienniku jako `pomijam config/... — plik został zmieniony ręcznie`.
