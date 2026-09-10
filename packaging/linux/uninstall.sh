#!/bin/sh
# Odinstalowanie Chmurkowego Launchera.
#
# Domyślnie usuwa program i skrót, a zostawia katalog `data` — leżą w nim
# światy z singleplayera, ustawienia i cała paczka modów. Skasowanie tego
# przy zwykłym „odinstaluj" byłoby przykrą niespodzianką.
#
#   ./uninstall.sh            program i skrót, dane zostają
#   ./uninstall.sh --wszystko dodatkowo światy, ustawienia i paczka

set -eu

KATALOG_DANYCH="${XDG_DATA_HOME:-$HOME/.local/share}"
DOCELOWY="$KATALOG_DANYCH/chmurkowy-launcher"
SKROTY="$KATALOG_DANYCH/applications"
IKONY="$KATALOG_DANYCH/icons/hicolor/256x256/apps"

WSZYSTKO=0
if [ "${1:-}" = "--wszystko" ]; then
    WSZYSTKO=1
fi

rm -f "$DOCELOWY/ChmurkowyLauncher"
rm -f "$SKROTY/chmurkowy-launcher.desktop"
rm -f "$IKONY/chmurkowy-launcher.png"

if [ "$WSZYSTKO" -eq 1 ]; then
    rm -rf "$DOCELOWY"
    echo "Usunięto launcher razem z danymi gry."
else
    # Katalog znika tylko wtedy, gdy naprawdę nic w nim nie zostało.
    rmdir "$DOCELOWY" 2>/dev/null || true
    if [ -d "$DOCELOWY/data" ]; then
        echo "Usunięto launcher. Dane gry zostały w:"
        echo "  $DOCELOWY/data"
        echo "Aby skasować także światy i ustawienia: ./uninstall.sh --wszystko"
    else
        echo "Usunięto launcher."
    fi
fi

if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$SKROTY" >/dev/null 2>&1 || true
fi
