# Paczka modów

Ten dział opisuje utrzymanie paczki: dodawanie i usuwanie modów, przebudowę
manifestu narzędziem `chmurka` i mody, które serwujemy z własnego repozytorium.

Źródłem prawdy o zawartości paczki nie jest żaden plik w tym repozytorium,
tylko instancja PrismLaunchera na komputerze utrzymującego. Mody, konfiguracje
i `options.txt` żyją tam, a `chmurka pack-build` przepisuje ich stan do
`manifest.json`. Dopiero ten plik widzą launchery graczy.

## Co jest w dzisiejszym manifeście

Liczby z `docs/manifest.json` w tym repozytorium:

| Rodzaj wpisu | Ile | `policy` | Skąd pobierane |
|---|---|---|---|
| Mody (`mods/*.jar`) | 264 | `mirror` | 250 z Modrinth, 11 z CurseForge, 3 od nas |
| Pliki konfiguracji (`config/**`) | 445 | `smart` | zawsze od nas |
| `options.txt` | 1 | `seed` | zawsze od nas |

Paczka to Minecraft 1.21.1 z NeoForge 21.1.249 na Javie 21 — te wartości są
wpisane na sztywno w `crates/cli/src/main.rs` i zmienia się je tam, a nie
w manifeście.

## Strony działu

- [Dodanie i usunięcie moda](zmiana-modow.md) — skąd wziąć plik, gdzie go
  położyć i co zobaczą gracze po przebudowie.
- [Przebudowa manifestu](przebudowa.md) — polecenia `chmurka`, numerowanie
  wersji paczki i launchera oraz lista plików, które celowo nie trafiają
  do paczki.
- [Mody hostowane u nas](hostowane.md) — dlaczego trzy mody idą z naszego
  adresu zamiast z CurseForge i Modrinth, jak działa `mirror_dirs` i co się
  psuje przy aktualizacji takiego moda.

Budowę samego pliku opisuje [Manifest](../budowa/manifest.md), a to, jak
launcher doprowadza instancję gracza do zgodności z manifestem —
[Synchronizacja paczki](../budowa/synchronizacja.md).

!!! uwaga

    `docs/manifest.json` i `docs/files/` to wynik działania `chmurka
    pack-build`. Ręczna zmiana czegokolwiek w tych plikach zostanie nadpisana
    przy najbliższej przebudowie, a w międzyczasie może rozjechać hasze,
    po których launcher rozpoznaje, czy plik u gracza jest właściwy.
