# Chmurkowy Launcher

Przenośny launcher paczki modów Chmurkowego Serwera — Minecraft 1.21.1 z NeoForge.

Jeden plik wykonywalny. Po uruchomieniu pobiera Javę, instaluje Minecrafta,
synchronizuje 249 modów i odpala grę — wszystko w katalogu obok siebie.
Deinstalacja polega na skasowaniu folderu: launcher nie zapisuje niczego
w systemie, rejestrze ani w `~/.minecraft`.

## Pobieranie

[Releases](../../releases) — `ChmurkowyLauncher-windows-x64.exe` lub
`ChmurkowyLauncher-linux-x64`.

Wrzuć plik do pustego folderu i uruchom. Pierwsze uruchomienie pobiera
około 1,8 GB, kolejne startują od razu.

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
