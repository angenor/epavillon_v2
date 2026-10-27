#!/usr/bin/env bash
# Lot 70 (phase 2) : témoignages de l'accueil v1 et leurs photos (70-temoignages.sql).
#
#   70-temoignages.sh <clone>   sur un clone local de la production ; pose aussi
#                               legacy.fichiers_a_deposer, d'où le dépôt des photos :
#                               FICHIERS_V1=<PHOTOS> 60-fichiers.sh deposer <clone>
#   70-temoignages.sh flux      écrit le flux SQL sur la sortie, à faire lire par
#                               le psql de la production (BASCULE.md, « Après »)
#
# Site ouvert, la production se joue directement : la v1 n'y est pas reposée,
# ces témoignages vivent dans le code de la v1, pas dans Supabase.
#
# Variables :
#   PHOTOS     défaut : public/images/temoignages du dépôt v1, voisin de celui-ci
#   CONTENEUR  conteneur Postgres local (défaut : epavillon-postgres)
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RACINE=$(cd "$ICI/../../.." && pwd)
PHOTOS=${PHOTOS:-$RACINE/../epavillonvue/public/images/temoignages}
CONTENEUR=${CONTENEUR:-epavillon-postgres}

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac
test -d "$PHOTOS" || { echo "Photos absentes : $PHOTOS" >&2; exit 1; }

flux() {
    printf '%s\n' 'CREATE TEMP TABLE lot70_photo (fichier text PRIMARY KEY, taille bigint, mime text, sha256 text);'
    printf '%s\n' 'COPY lot70_photo FROM STDIN WITH (FORMAT text);'
    for f in "$PHOTOS"/*; do
        printf '%s\t%s\t%s\t%s\n' "$(basename "$f")" "$(wc -c < "$f" | tr -d ' ')" \
            "$(file -b --mime-type "$f")" "$(shasum -a 256 "$f" | cut -d' ' -f1)"
    done
    printf '%s\n' '\.'
    cat "$ICI/70-temoignages.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 -v depot=1 2>&1 \
    | sed 's/^NOTICE:  //'
