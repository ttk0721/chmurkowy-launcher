#!/usr/bin/env bash
# Buduje serwis dokumentacji do `docs/wiki`.
#
# Jedyny właściwy sposób jej budowania — i przez człowieka, i przez kontrolę
# w CI. Powód jest konkretny: mkdocs wpisuje do `sitemap.xml` **datę budowania**,
# więc ten sam, niezmieniony serwis zbudowany innego dnia daje inny plik.
#
# Kontrola porównuje świeże budowanie z tym, co leży w repozytorium, żeby
# źródło i opublikowana strona nie rozjechały się po cichu. Bez ustalonej daty
# zapalała się na czerwono przy KAŻDYM commicie zrobionym innego dnia niż
# ostatnie przebudowanie dokumentacji — nawet gdy nikt nie tknął ani jednej
# strony. Fałszywy alarm, który powtarza się codziennie, uczy ignorowania
# kontroli, a wtedy przestaje ona cokolwiek chronić.
#
# `SOURCE_DATE_EPOCH` to uzgodniony standard powtarzalnych budowań; mkdocs go
# rozumie. Wartość jest stała i celowo nie ma znaczenia: `lastmod` w mapie
# strony i tak niósł datę budowania, a nie datę zmiany treści, więc nigdy
# niczego sensownego nie mówił.
set -euo pipefail

export SOURCE_DATE_EPOCH=1600000000

cd "$(dirname "$0")"

if [ -x ./.venv-docs/bin/mkdocs ]; then
  MKDOCS=./.venv-docs/bin/mkdocs
elif command -v mkdocs >/dev/null 2>&1; then
  MKDOCS=mkdocs
else
  echo "Nie znalazłem mkdocs. Utwórz środowisko:" >&2
  echo "  python3 -m venv .venv-docs" >&2
  echo "  ./.venv-docs/bin/pip install -r wiki-requirements.txt" >&2
  exit 1
fi

exec "$MKDOCS" build "$@"
