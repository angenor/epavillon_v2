#!/usr/bin/env bash
# Lot 100 (phase 2) : rediffusions de la CdP30 attendues (100-rediffusions.sql).
#
#   100-rediffusions.sh <clone>   sur un clone local de la production
#   100-rediffusions.sh flux      écrit le flux SQL sur la sortie, à faire lire par
#                                 le psql de la production (BASCULE.md, « Après »)
#
# Lit la v1 au banc (BANC, défaut epavillon_repetition) : les seuls identifiants
# des activités qui avaient une vidéo YouTube. La vidéo elle-même ne voyage pas :
# la chaîne a été piratée, les vidéos reviendront après la CdP31.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CONTENEUR=${CONTENEUR:-epavillon-postgres}
BANC=${BANC:-epavillon_repetition}

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac

banc() { docker exec -i "$CONTENEUR" psql -U postgres -d "$BANC" -X -q -At -v ON_ERROR_STOP=1 "$@"; }

flux() {
    printf '%s\n' 'CREATE TEMP TABLE lot100_video (activite uuid PRIMARY KEY);'
    printf '%s\n' 'COPY lot100_video FROM STDIN WITH (FORMAT text);'
    banc -c "COPY (SELECT id FROM v1.activities WHERE nullif(btrim(youtube_link), '') IS NOT NULL) TO STDOUT"
    printf '%s\n' '\.'
    cat "$ICI/100-rediffusions.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 2>&1 \
    | sed 's/^NOTICE:  //'
