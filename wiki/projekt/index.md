# Praca nad projektem

Ten dział opisuje, jak zmiany trafiają z gałęzi roboczej do launchera
stojącego na komputerze gracza. Trzy strony, w kolejności, w jakiej się z nimi
stykasz:

- [Wydanie nowej wersji](wydanie.md) — podbicie wersji, tag adnotowany,
  co dzieje się w workflow wydania i jak gracze dowiadują się o nowej wersji.
- [Kontrole i ochrona gałęzi](kontrole.md) — co musi przejść, zanim pull
  request da się scalić, i dlaczego reguły są ustawione właśnie tak.
- [Podpisywanie commitów](podpisy.md) — wymóg podpisu, status `bad_email`
  i naprawa niezgodnego adresu.

## Układ repozytorium

| Katalog | Zawartość |
|---|---|
| `crates/core` | Cała logika: manifest, pobieranie, Java, instalacja gry, synchronizacja paczki, uruchamianie |
| `crates/launcher` | Interfejs (egui) — warstwa nad `core`, binarka `ChmurkowyLauncher` |
| `crates/cli` | Narzędzie `chmurka`: buduje manifest paczki i pozwala przejść cały przebieg bez okna |
| `.github/workflows` | `release.yml` (wydanie), `kontrola.yml` (testy, clippy, dokumentacja), `codeql.yml` (skanowanie) |
| `packaging/windows` | Skrypt instalatora Inno Setup (`chmurkowy.iss`) i ikona |
| `packaging/linux` | `install.sh`, `uninstall.sh`, wpis `.desktop` |
| `wiki` | Źródła tej dokumentacji (pliki `.md`) |
| `docs` | Korzeń GitHub Pages: `manifest.json`, pliki paczki i zbudowana dokumentacja w `docs/wiki` |

Dwa ostatnie wiersze mylą nazwami. `docs/` nie jest katalogiem dokumentacji,
tylko tym, co GitHub Pages publikuje pod adresem serwisu — leży tam manifest,
z którego korzysta każdy uruchomiony launcher. Dokumentacja jest wynikiem
budowania i ląduje w podkatalogu `docs/wiki`, żeby nie zadeptać manifestu.

## Krótki obieg zmiany

1. Gałąź robocza, commity podpisane kluczem GPG.
2. Pull request — na `main` nie wchodzi się wprost.
3. Trzy kontrole (`testy (linux)`, `testy (windows)`, `clippy`) i skanowanie
   CodeQL muszą przejść, a gałąź musi być aktualna względem `main`.
4. Scalenie przez **Squash and merge**.
5. Przy wydaniu: podbicie wersji, tag adnotowany, a na końcu podbicie
   manifestu — dopiero ono sprawia, że launchery graczy widzą nową wersję.

Dwie strony poza tym działem opisują sąsiednie tematy:
[Samoaktualizacja](../budowa/samoaktualizacja.md) tłumaczy, co robi launcher
gracza po podbiciu manifestu, a [Przebudowa manifestu](../paczka/przebudowa.md)
— jak manifest powstaje.
