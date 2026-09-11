# Kontrole i ochrona gałęzi

Na `main` nie wchodzi się wprost. Każda zmiana idzie przez pull requesta,
a przed scaleniem muszą przejść cztery kontrole i skanowanie CodeQL. Gałąź musi
być przy tym aktualna względem `main`.

## Co się uruchamia na pull requeście

| Nazwa kontroli | Workflow | Obraz | Co uruchamia |
|---|---|---|---|
| `testy (linux)` | `kontrola.yml` | `ubuntu-22.04` | `cargo test --workspace` |
| `testy (windows)` | `kontrola.yml` | `windows-latest` | `cargo test --workspace` |
| `clippy` | `kontrola.yml` | `ubuntu-22.04` | `cargo clippy --workspace --all-targets -- -D warnings` |
| `dokumentacja` | `kontrola.yml` | `ubuntu-22.04` | `mkdocs build` i sprawdzenie, czy `docs/wiki` jest aktualne |
| `codeql (rust)` | `codeql.yml` | `ubuntu-22.04` | `github/codeql-action` na kodzie Rusta |
| `codeql (actions)` | `codeql.yml` | `ubuntu-22.04` | `github/codeql-action` na plikach workflow |

Oba workflow chodzą na `pull_request`, na `push` do `main` i dają się
uruchomić ręcznie (`workflow_dispatch`).

### Dlaczego kontrole są w osobnym workflow

`release.yml` też uruchamia `cargo test --workspace`, ale rusza wyłącznie
z tagu `v*`. Jego zadania nie nadają się na wymagane kontrole gałęzi:
na pull requeście nigdy by nie powstały, więc każdy pull request czekałby
w nieskończoność na coś, co się nie wydarzy. Ochrona gałęzi potrzebuje
kontroli uruchamianych na pull requeście — i po to jest `kontrola.yml`.

### Przerywanie poprzednich biegów

```yaml
concurrency:
  group: kontrola-${{ github.ref }}
  cancel-in-progress: true
```

Nowe wejście na tę samą gałąź przerywa poprzedni bieg. Bez tego poprawka
wypchnięta minutę po poprzedniej czekałaby w kolejce za biegiem, którego wynik
i tak nikogo już nie obchodzi.

### Zależności systemowe

Zadania kompilujące Rusta na Linuksie — `testy (linux)` i `clippy` —
doinstalowują `libgtk-3-dev`, `libxcb-render0-dev`,
`libxcb-shape0-dev`, `libxcb-xfixes0-dev`, `libxkbcommon-dev`, `libssl-dev`
i `libasound2-dev`. `cargo test --workspace` buduje także `crates/launcher`,
a `eframe` linkuje się na Linuksie z bibliotekami okien i dźwięku — bez nich
nie ruszy nawet kompilacja testów.

Testy chodzą na tym samym obrazie, co wydanie (`ubuntu-22.04`), a nie na
najnowszym. Binarka dla graczy powstaje właśnie tam ze względu na wersję
glibc, więc testy lecą na tym, na czym powstaje plik dla graczy.

### Clippy z `-D warnings`

`-D warnings` znaczy: każde ostrzeżenie przewraca kontrolę. Repozytorium jest
dziś pod tym warunkiem czyste i ma takie zostać. Ostrzeżenie, które wolno
zignorować, po miesiącu jest ostrzeżeniem, którego nikt już nie czyta.

## Dlaczego nazwy kontroli nie zawierają wersji obrazu

Zadanie testów ma nazwę złożoną z etykiety, a nie z nazwy obrazu:

```yaml
name: testy (${{ matrix.etykieta }})
strategy:
  matrix:
    include:
      - os: ubuntu-22.04
        etykieta: linux
      - os: windows-latest
        etykieta: windows
```

To ta nazwa trafia do ustawień ochrony gałęzi jako wymagana kontrola. Gdyby
brzmiała `testy (ubuntu-22.04)`, podbicie obrazu na 24.04 zmieniłoby jej
nazwę — a wymagana kontrola o starej nazwie przestałaby powstawać.

Najgorsze jest to, co dzieje się potem. GitHub nie zgłasza takiej sytuacji
jako błędu. Po prostu przepuszcza scalanie, bo „wymagana kontrola” nie ma jak
się nie udać, skoro w ogóle jej nie ma. Brama zostaje otwarta i nikt tego nie
widzi, dopóki ktoś nie scali czegoś zepsutego.

Etykieta `linux` przetrwa każde podbicie obrazu, bo nie mówi o obrazie nic.
Wersję obrazu zmienia się w jednym miejscu — w polu `os` — i nazwa kontroli
zostaje ta sama.

## CodeQL we własnym workflow

GitHub potrafi włączyć skanowanie kodu „konfiguracją domyślną”, jednym
przełącznikiem w ustawieniach repozytorium. Tu tego nie robimy, bo ta droga
nie przyjmuje Rusta. Jej API wymienia dopuszczalne języki i są to:
`actions`, `c-cpp`, `csharp`, `go`, `java-kotlin`, `javascript-typescript`,
`python`, `ruby` i `swift`.

Włączenie jej dałoby zieloną bramkę skanowania, która nie przeczyta ani
linijki z 571 kB kodu Rusta — czyli z całego programu. Bramka, która nic nie
sprawdza, jest gorsza od jej braku, bo usypia: widać zielony znaczek
i przestaje się o tym myśleć.

Własny workflow `codeql.yml` skanuje dwa języki naraz — `rust` i `actions`
(ten drugi to same pliki workflow). Trzy rzeczy w nim warto znać:

- **`build-mode: none`.** CodeQL czyta źródła wprost, bez kompilowania.
  Budowanie całego workspace pod analizę podwoiłoby czas każdego pull
  requesta, a testy i tak kompilują go obok.
- **Uprawnienie `security-events: write`.** Bez niego wyniki nie mają gdzie
  trafić i bramka skanowania nigdy nie dostanie odpowiedzi.
- **Harmonogram `cron: "31 4 * * 1"`** (poniedziałki). Kod nie zmienia się
  w każdy poniedziałek, ale baza reguł CodeQL owszem. Bez tego biegu o nowo
  rozpoznanej klasie błędów dowiedzielibyśmy się dopiero przy najbliższej
  zmianie w kodzie.

## Zasady ochrony gałęzi

### Wyłącznie przez pull requesta

Na `main` nie da się wypchnąć wprost. To nie jest kwestia dyscypliny, tylko
ustawienia: bez pull requesta kontrole nie mają gdzie się uruchomić przed
scaleniem, a po scaleniu jest już za późno — gracze pobierają to, co leży
na `main`.

Wymóg „gałąź aktualna względem `main`” dokłada do tego drugą rzecz: kontrole
mają przejść na tym, co naprawdę powstanie po scaleniu, a nie na starszym
stanie, w którym cudza zmiana jeszcze nie istniała.

### Wyłącznie Squash and merge

Scalamy wyłącznie przez **Squash and merge**. Pozostałe dwa sposoby odpadają
same, przez zderzenie z innymi regułami:

| Sposób | Dlaczego odpada |
|---|---|
| Create a merge commit | Kłóci się z wymogiem liniowej historii — commit scalający ma dwóch rodziców |
| Rebase and merge | Kłóci się z wymogiem podpisów — GitHub nie podpisuje commitów odtwarzanych przy rebase |

Drugi wiersz jest nieoczywisty i łatwo się na nim przejechać. Commity na
gałęzi mogą być poprawnie podpisane, a mimo to przycisk „Rebase and merge”
wytworzy na `main` nowe, nieopodpisane commity — bo rebase tworzy nowe obiekty
i to GitHub jest tym, który je zapisuje. Przy „Squash and merge” GitHub
również tworzy commit, ale ten jeden podpisuje własnym kluczem.

Skutek uboczny warto znać przy wydawaniu: po scaleniu na `main` stoi **inny**
commit niż ten z gałęzi. Tag wydania zakłada się na nim, więc najpierw
`git pull`. Opisuje to [Wydanie nowej wersji](wydanie.md).

O samym wymogu podpisu i o tym, co robić, gdy GitHub go nie uznaje, mówi
[Podpisywanie commitów](podpisy.md).
