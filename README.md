# Chmurkowy Launcher

Przenośny launcher paczki modów Chmurkowego Serwera.
Minecraft 1.21.1, NeoForge 21.1.249, 249 modów.

Jeden plik wykonywalny. Pobiera Javę, instaluje grę, synchronizuje paczkę
i uruchamia Minecrafta — wszystko w swoim folderze.

## Dla graczy

1. Pobierz plik z zakładki [Releases](../../releases):
   - Windows: `ChmurkowyLauncher-windows-x64.exe`
   - Linux: `ChmurkowyLauncher-linux-x64`
2. Wrzuć go do **pustego folderu** — launcher tworzy obok siebie katalog `data`.
3. Uruchom i zaloguj się kontem Microsoft.

Pierwsze uruchomienie pobiera około 1,8 GB (Java, Minecraft, zasoby, mody).
Kolejne startują od razu — sprawdzana jest tylko różnica.

**Deinstalacja: skasuj folder.** To wszystko. Launcher nie zapisuje niczego
w systemie, rejestrze ani w `~/.minecraft`.

**Windows pokaże ostrzeżenie SmartScreen**, bo plik nie jest podpisany
certyfikatem (podpis kosztuje). Kliknij „Więcej informacji", potem
„Uruchom mimo to".

**Tryb offline** na ekranie logowania służy do testów i zadziała tylko
na serwerze z `online-mode=false`.

## Dla utrzymującego paczkę

Paczka jest serwowana z katalogu `docs/` przez GitHub Pages pod adresem
<https://ttk0721.github.io/chmurkowy-launcher/>.

Po zmianie modów w PrismLauncherze:

```bash
cargo run --release -p chmurka-cli --bin chmurka -- pack-build \
  --instance "$HOME/.local/share/PrismLauncher/instances/Chmurkowy serwer edycja 2026-2027/minecraft" \
  --out docs \
  --base-url "https://ttk0721.github.io/chmurkowy-launcher" \
  --version "$(date +%Y.%m.%d)-1"

git add docs && git commit -m "Paczka $(date +%Y.%m.%d)" && git push
```

I tyle. Testerzy dostaną samą różnicę przy następnym uruchomieniu launchera —
nie muszą pobierać launchera na nowo.

Stare pliki zostają w `docs/files/`, bo są adresowane hashem. Dzięki temu
poprzednie wersje paczki pozostają pobieralne, a nowa wersja dokłada tylko
to, co się faktycznie zmieniło.

### Co właściwie hostujesz

Prawie nic. 246 z 249 modów pobieranych jest prosto z Modrinth i CurseForge.
Configi to około 3 MB.

Wyjątkiem są mody, których plik na dysku **różni się** od tego pod adresem
źródłowym (bo zostały lokalnie załatane). Packer wykrywa to, porównując hash
z metadanych PrismLaunchera, i takie mody kopiuje do `docs/`, wypisując
ostrzeżenie. Dzięki temu testerzy zawsze dostają dokładnie tę paczkę,
którą masz u siebie. Jeśli któraś rozbieżność jest niezamierzona, pobierz
ten mod na nowo w PrismLauncherze i przebuduj manifest.

### Wskazanie launchera na Twój manifest

Adres manifestu jest wkompilowany. Ustaw go przy budowaniu:

```bash
CHMURKA_MANIFEST_URL="https://ttk0721.github.io/chmurkowy-launcher/manifest.json" \
  cargo build --release -p chmurkowy-launcher
```

W GitHub Actions jest już ustawiona zmienna repozytorium `CHMURKA_MANIFEST_URL`
(Settings → Secrets and variables → Actions → Variables). Nowe wydanie:

```bash
git tag -a v0.2.0 -m "opis" && git push origin v0.2.0
```

Workflow zbuduje binarki dla Windowsa i Linuksa i wrzuci je do Releases.

## Jak to działa

| Element | Skąd | Rozmiar |
|---|---|---|
| Java 21 | Adoptium | 45 MB |
| Minecraft + NeoForge | instalator NeoForge (bezobsługowo) | 121 MB |
| Biblioteki i zasoby | CDN Mojanga | ~875 MB |
| Mody | Modrinth i CurseForge | ~670 MB |
| Configi i mody załatane lokalnie | Twój GitHub Pages | ~65 MB |

Układ folderów u gracza:

```
ChmurkowyLauncher/
├── ChmurkowyLauncher.exe
└── data/
    ├── java/21/     JRE
    ├── mc/          versions/, libraries/, assets/
    ├── instance/    gameDir: mods/, config/, saves/, screenshots/
    ├── state.json   co launcher sam wgrał
    └── auth.json    token logowania
```

`mc/` da się odtworzyć z sieci, `instance/` zawiera dane gracza — dlatego
„Napraw instalację" kasuje tylko to pierwsze i nie rusza światów.

Mody synchronizowane są lustrzanie: obcy plik w `mods/` jest kasowany, bo
wywala całą paczkę. Configi nadpisywane są tylko wtedy, gdy gracz ich nie
zmieniał — inaczej zostają nietknięte, z wpisem w logu.

## Rozwój

```bash
cargo test --workspace            # testy
cargo run -p chmurkowy-launcher   # GUI
cargo run -p chmurka-cli --bin chmurka -- login   # sprawdzenie logowania
cargo run -p chmurka-cli --bin chmurka -- run --manifest <adres> --nick Test
```

Dokumentacja projektowa: [`docs/superpowers/specs/`](docs/superpowers/specs/),
plan implementacji: [`docs/superpowers/plans/`](docs/superpowers/plans/).
