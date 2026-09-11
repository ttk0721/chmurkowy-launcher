# Chmurkowy Launcher

Launcher paczki modów Chmurkowego Serwera — Minecraft 1.21.1 z NeoForge.

Instalujesz raz. Potem launcher sam sprawdza przy każdym starcie, czy jest
nowsza wersja, i sam się aktualizuje — nie trzeba niczego pobierać ręcznie.
Po uruchomieniu pobiera Javę, instaluje Minecrafta, synchronizuje paczkę modów
i odpala grę. Nie dotyka `~/.minecraft` ani innych instalacji gry.

## Instalacja

[Releases](../../releases)

**Windows** — `ChmurkowyLauncher-setup.exe`

Instaluje się dla Twojego konta, bez pytania o hasło administratora, i dodaje
skrót w menu Start. Odinstalowanie jak każdego innego programu, przez
„Aplikacje i funkcje".

**Linux** — `ChmurkowyLauncher-linux-x64.tar.gz`

```bash
tar xzf ChmurkowyLauncher-linux-x64.tar.gz
cd ChmurkowyLauncher-linux
./install.sh
```

Instaluje do katalogu domowego, bez `sudo`, i dodaje wpis w menu aplikacji.
Odinstalowanie: `./uninstall.sh` (światy i ustawienia zostają) albo
`./uninstall.sh --wszystko`.

Wymaga glibc 2.35 lub nowszego — Ubuntu 22.04+, Debian 12+, Mint 21+,
Fedora 36+, Arch. Poza tym tylko biblioteki graficzne i dźwiękowe, które są
w każdym środowisku graficznym.

Pierwsze uruchomienie pobiera około 1,8 GB, kolejne startują od razu.

### Gdzie trafiają pliki

Gra, paczka modów, światy i ustawienia leżą osobno od samego programu:

| System | Katalog |
|---|---|
| Windows | `%LOCALAPPDATA%\ChmurkowyLauncher\data` |
| Linux | `~/.local/share/chmurkowy-launcher/data` |

Dzięki temu aktualizacja launchera nie rusza Twoich światów, a odinstalowanie
programu ich nie kasuje. Kto miał starszą wersję z folderem `data` obok pliku,
nie musi nic robić — launcher przeniesie dane sam przy pierwszym uruchomieniu.

W wydaniach są też gołe pliki wykonywalne (`ChmurkowyLauncher-linux-x64`,
`ChmurkowyLauncher-windows-x64.exe`). To po nie sięga samoaktualizacja; do
zwykłego użytku weź instalator.

## Co jest w środku

| Crate | Odpowiedzialność |
|---|---|
| `crates/core` | Cała logika: manifest, pobieranie, Java, instalacja gry, synchronizacja paczki, logowanie, uruchamianie |
| `crates/launcher` | Interfejs (egui) — cienka warstwa nad `core` |
| `crates/cli` | Narzędzie `chmurka`: buduje manifest paczki i pozwala przetestować cały przebieg bez okna |

```bash
cargo test --workspace
cargo run -p chmurkowy-launcher
```

Mody pobierane są prosto z Modrinth i CurseForge, na podstawie manifestu
opisującego każdy plik adresem i skrótem SHA-512. Launcher weryfikuje
każdy pobrany plik i dociąga wyłącznie to, co się zmieniło.

## Jak pracujemy

Na `main` nie wchodzi się wprost — każda zmiana idzie przez pull requesta.
Przed scaleniem muszą przejść cztery kontrole (`testy (linux)`, `testy (windows)`,
`clippy`, `dokumentacja`) oraz skanowanie CodeQL, a gałąź musi być aktualna
względem `main`.

Scalamy wyłącznie przez **Squash and merge**. Pozostałe sposoby odpadają przez
same reguły: „Create a merge commit" kłóci się z wymogiem liniowej historii,
a „Rebase and merge" z wymogiem podpisów — GitHub nie podpisuje commitów
odtwarzanych przy rebase.

Commity muszą być podpisane, a adres autora musi występować w kluczu GPG.
Inaczej GitHub oznacza podpis jako `bad_email`, nie uznaje go za zweryfikowany
i odmawia przyjęcia zmiany.
