#!/usr/bin/env bash
# Lot 90 (phase 2) : journées spéciales de la CdP30 (90-journees-speciales.sql).
#
#   90-journees-speciales.sh <clone>   sur un clone local de la production
#   90-journees-speciales.sh flux      écrit le flux SQL sur la sortie, à faire lire par
#                                      le psql de la production (BASCULE.md, « Après »)
#
# Ne lit pas la v1 : le relevé des journées (textes publics des pages v1) et les
# correspondances écrites dans le lot suffisent, la production portant déjà les
# propositions et legacy.id_map.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RELEVE=$ICI/../journees-speciales
CONTENEUR=${CONTENEUR:-epavillon-postgres}
JOURNEES=(cdp30-journee-jeunesse-climat cdp30-journee-finance-durable)

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac

flux() {
    printf '%s\n' 'CREATE TEMP TABLE lot90_releve (code text PRIMARY KEY, doc jsonb NOT NULL);'
    for j in "${JOURNEES[@]}"; do
        # Le relevé passe entre dollars : sa balise ne doit pas y figurer.
        if grep -q 'releve\$' "$RELEVE/$j.json"; then
            echo "$j.json contient la balise \$releve\$" >&2; exit 1
        fi
        printf "INSERT INTO lot90_releve SELECT d->'programme_track'->>'code', d FROM (SELECT \$releve\$"
        cat "$RELEVE/$j.json"
        printf '%s\n' "\$releve\$::jsonb AS d) x;"
    done
    cat "$ICI/90-journees-speciales.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 2>&1 \
    | sed 's/^NOTICE:  //'
