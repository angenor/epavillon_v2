#!/usr/bin/env bash
# Contenu (30/10) : pages des trois fils de la CdP31 (2026-10-30-journees-cdp31.sql).
#
#   2026-10-30-journees-cdp31.sh <copie>   sur une copie locale de la production, migration
#                                          2026-10-30-journees-speciales jouée
#   2026-10-30-journees-cdp31.sh flux      écrit le flux SQL sur la sortie, à faire lire par le
#                                          psql de la production
#
# Aucun fichier joint : le flux est le .sql seul. Rejouable, une seconde passe ne change rien.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CONTENEUR=${CONTENEUR:-epavillon-postgres}

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <copie locale>|flux" >&2; exit 2 ;;
esac

if [ "$cible" = flux ]; then
    cat "$ICI/2026-10-30-journees-cdp31.sql"
    exit 0
fi
docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 \
    < "$ICI/2026-10-30-journees-cdp31.sql" 2>&1 | sed 's/^NOTICE:  //'
