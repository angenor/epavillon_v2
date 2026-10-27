#!/usr/bin/env bash
# Lot 93 (phase 2) : photos des intervenants sans adresse (93-photos-intervenants.sql).
#
#   93-photos-intervenants.sh <clone>   sur un clone local de la production (après le lot 92) ;
#                                       prépare legacy.fichiers_a_deposer pour 60-fichiers.sh
#   93-photos-intervenants.sh flux      écrit le flux SQL sur la sortie, à faire lire par
#                                       le psql de la production (BASCULE.md, « Après »)
#
# Lit les trois fichiers dans la copie locale des buckets v1 (FICHIERS_V1) ; le
# flux n'en porte que le chemin, la taille, le type et l'empreinte.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RACINE=$(cd "$ICI/../../.." && pwd)
FICHIERS=${FICHIERS_V1:-$RACINE/sauvegardes/reprise-v1/fichiers}
CONTENEUR=${CONTENEUR:-epavillon-postgres}
PHOTOS=(
    epavillonp/intervenants/6d7b03f8-c6cf-409f-8a75-46f35cfde35a_hq_1759256116608_img_0951.jpg
    epavillonp/intervenants/27446304-c6ba-4f6c-9719-3c9f2fd01cb2_hq_1761828564132_photo_js_20240913_002_.jpg
    epavillonp/intervenants/5bbd54f3-22ba-47e9-9533-38adfb052f82_hq_1791878963490_48083.jpg
)

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac
for f in "${PHOTOS[@]}"; do
    test -f "$FICHIERS/$f" || { echo "Photo absente : $FICHIERS/$f" >&2; exit 1; }
done

flux() {
    printf '%s\n' 'CREATE TEMP TABLE lot93_photo (fichier text PRIMARY KEY, taille bigint, mime text, sha256 text);'
    printf '%s\n' 'COPY lot93_photo FROM STDIN WITH (FORMAT text);'
    for f in "${PHOTOS[@]}"; do
        printf '%s\t%s\t%s\t%s\n' "$f" "$(wc -c < "$FICHIERS/$f" | tr -d ' ')" \
            "$(file -b --mime-type "$FICHIERS/$f")" "$(shasum -a 256 "$FICHIERS/$f" | cut -d' ' -f1)"
    done
    printf '%s\n' '\.'
    cat "$ICI/93-photos-intervenants.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 -v depot=1 2>&1 \
    | sed 's/^NOTICE:  //'
