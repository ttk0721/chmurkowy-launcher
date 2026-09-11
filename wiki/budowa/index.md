# Jak to działa

Ten dział jest dla osoby, która zagląda do kodu: chce coś w launcherze
zmienić, przejąć projekt albo zrozumieć, dlaczego coś zostało zrobione tak,
a nie inaczej. Zakłada znajomość Rusta i wiersza poleceń.

Jeśli szukasz opisu dla gracza — jak zainstalować launcher, co kliknąć
i co zrobić, gdy gra nie startuje — jest osobno, w dziale
[Dla gracza](../gracz/index.md).

Launcher to jeden plik wykonywalny napisany w Rust. Sam pobiera Javę,
instaluje Minecrafta 1.21.1 z NeoForge, synchronizuje paczkę modów
i uruchamia grę. Nie korzysta z `~/.minecraft` ani z żadnej istniejącej
instalacji gry — wszystko, czego potrzebuje, trzyma we własnym katalogu
danych.

## Co jest w tym dziale

| Strona | O czym mówi |
|---|---|
| [Architektura](architektura.md) | Podział na trzy crate'y, moduły `crates/core`, rozmowa interfejsu z logiką, użyte biblioteki, komendy do budowania i testów |
| [Od kliknięcia GRAJ do gry](przebieg-uruchomienia.md) | Kolejność kroków od naciśnięcia przycisku do procesu gry i co się dzieje, gdy któryś z nich zawiedzie |
| [Manifest](manifest.md) | Plik `manifest.json` — jedyne źródło informacji o tym, co launcher ma pobrać i uruchomić |
| [Synchronizacja paczki](synchronizacja.md) | Jak launcher decyduje, który plik pobrać, który skasować, a którego nie ruszać, bo zmienił go gracz |
| [Samoaktualizacja](samoaktualizacja.md) | Jak launcher podmienia sam siebie i dlaczego robi to bez pytania gracza o zgodę przy starcie |
| [Gdzie leżą dane](dane.md) | Katalogi gry, paczki, ustawień i logów na Windowsie i na Linuksie |

Budowanie samej paczki modów — dodawanie modów, przebudowa manifestu,
wypychanie plików — opisuje dział [Paczka modów](../paczka/index.md).
Wydania, kontrole na pull requestach i podpisywanie commitów:
[Praca nad projektem](../projekt/index.md).

!!! wskazówka

    Kod tego projektu ma komentarze tłumaczące przyczyny, a nie opisujące
    działanie linijka po linijce. Zanim zmienisz coś, co wygląda na
    niepotrzebnie zawiłe, sprawdź komentarz nad tym miejscem — zwykle stoi
    za tym konkretny błąd, który już kogoś kosztował wieczór.
