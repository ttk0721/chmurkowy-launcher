#!/bin/sh
# Instalacja Chmurkowego Launchera dla bieżącego użytkownika.
#
# Celowo bez sudo i bez /usr. Launcher aktualizuje się sam, podmieniając
# własny plik, więc musi leżeć tam, gdzie ma prawo zapisu. Instalacja
# systemowa odebrałaby mu tę możliwość i każda poprawka wymagałaby
# ponownego pobierania paczki przez administratora.
#
# Działa tak samo na Ubuntu, Mincie, Fedorze i Archu — nie zależy od
# menedżera pakietów.

set -eu

KATALOG_DANYCH="${XDG_DATA_HOME:-$HOME/.local/share}"
DOCELOWY="$KATALOG_DANYCH/chmurkowy-launcher"
SKROTY="$KATALOG_DANYCH/applications"
IKONY="$KATALOG_DANYCH/icons/hicolor/256x256/apps"
ZRODLO="$(cd "$(dirname "$0")" && pwd)"

if [ ! -f "$ZRODLO/ChmurkowyLauncher" ]; then
    echo "Nie znalazłem pliku ChmurkowyLauncher obok tego skryptu." >&2
    echo "Rozpakuj całe archiwum i uruchom install.sh z środka." >&2
    exit 1
fi

mkdir -p "$DOCELOWY" "$SKROTY" "$IKONY"

# Podmiana działającego pliku kończy się błędem „Text file busy", więc
# najpierw usuwamy stary. Katalog data leży obok i zostaje nietknięty.
rm -f "$DOCELOWY/ChmurkowyLauncher"
cp "$ZRODLO/ChmurkowyLauncher" "$DOCELOWY/ChmurkowyLauncher"
chmod 755 "$DOCELOWY/ChmurkowyLauncher"

cp "$ZRODLO/ikona.png" "$IKONY/chmurkowy-launcher.png"

# Ścieżkę wstawiamy dopiero teraz — w pliku w archiwum jej nie ma,
# bo katalog domowy każdy ma inny.
sed "s|@EXEC@|$DOCELOWY/ChmurkowyLauncher|" \
    "$ZRODLO/chmurkowy-launcher.desktop" > "$SKROTY/chmurkowy-launcher.desktop"
chmod 644 "$SKROTY/chmurkowy-launcher.desktop"

# Bez tego skrót bywa widoczny dopiero po wylogowaniu. Nie każde środowisko
# ma to narzędzie, więc brak nie jest błędem.
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$SKROTY" >/dev/null 2>&1 || true
fi

echo "Gotowe."
echo "  Program:  $DOCELOWY/ChmurkowyLauncher"
echo "  Dane gry: $DOCELOWY/data"
echo
echo "Chmurkowy Launcher jest w menu aplikacji. Możesz go też uruchomić tak:"
echo "  $DOCELOWY/ChmurkowyLauncher"
echo
echo "Odinstalowanie: ./uninstall.sh (świat i ustawienia zostają)."
