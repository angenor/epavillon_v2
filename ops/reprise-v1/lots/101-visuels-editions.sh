#!/usr/bin/env bash
# Lot 101 (phase 2) : bandeau et logo de la CdP30, logo de la CdP31 (101-visuels-editions.sql).
#
#   101-visuels-editions.sh <clone>   sur un clone local de la production (après la migration
#                                     2026-10-29-logo-edition-et-gabarits) ; prépare
#                                     legacy.fichiers_a_deposer pour 60-fichiers.sh, sans rien déposer
#   101-visuels-editions.sh flux      écrit le flux SQL sur la sortie, à faire lire par le
#                                     psql de la production
#
# Fichiers, hors du dépôt git, sous FICHIERS_V1 :
#   ancien-serveur/event_banniere_par_defaut_16_9_reduit.jpg et
#   ancien-serveur/COP30_Official_Logo_reduit.png, relevés sur l'ancien serveur le 28/10 ;
#   le logo de la CdP31, chemin lu dans la v1 du banc (BANC, v1.events.logo_url et
#   v1_storage.objects). Le flux porte chemin, taille, type et empreinte, jamais le fichier.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RACINE=$(cd "$ICI/../../.." && pwd)
FICHIERS=${FICHIERS_V1:-$RACINE/sauvegardes/reprise-v1/fichiers}
CONTENEUR=${CONTENEUR:-epavillon-postgres}
BANC=${BANC:-epavillon_repetition}
CDP31_V1=d03adb7b-b51c-4e78-8fcd-7f956272c30d

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac

TRAVAIL=$(mktemp -d)
trap 'rm -rf "$TRAVAIL"' EXIT

docker exec -i "$CONTENEUR" psql -U postgres -d "$BANC" -X -q -At -F $'\t' -v ON_ERROR_STOP=1 -c "
    SELECT o.bucket_id || '/' || o.name, o.id
    FROM v1.events e
    JOIN v1_storage.objects o
      ON o.bucket_id || '/' || o.name = regexp_replace(e.logo_url, '^.*/storage/v1/object/public/', '')
    WHERE e.id = '$CDP31_V1'" > "$TRAVAIL/cdp31"
[ "$(wc -l < "$TRAVAIL/cdp31" | tr -d ' ')" = 1 ] || { echo "logo v1 de la CdP31 introuvable dans $BANC" >&2; exit 1; }

{
    printf 'cdp30_banniere\tancien-serveur/event_banniere_par_defaut_16_9_reduit.jpg\t\\N\n'
    printf 'cdp30_logo\tancien-serveur/COP30_Official_Logo_reduit.png\t\\N\n'
    printf 'cdp31_logo\t%s\n' "$(cat "$TRAVAIL/cdp31")"
} > "$TRAVAIL/fichiers"

while IFS=$'\t' read -r _ f _; do
    test -f "$FICHIERS/$f" || { echo "Fichier absent : $FICHIERS/$f" >&2; exit 1; }
done < "$TRAVAIL/fichiers"

flux() {
    printf '%s\n' 'CREATE TEMP TABLE lot101_fichier (cle text PRIMARY KEY, fichier text, objet_v1 uuid, taille bigint, mime text, sha256 text);'
    printf '%s\n' 'COPY lot101_fichier FROM STDIN WITH (FORMAT text);'
    while IFS=$'\t' read -r cle f objet; do
        printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$cle" "$f" "$objet" "$(wc -c < "$FICHIERS/$f" | tr -d ' ')" \
            "$(file -b --mime-type "$FICHIERS/$f")" "$(shasum -a 256 "$FICHIERS/$f" | cut -d' ' -f1)"
    done < "$TRAVAIL/fichiers"
    printf '%s\n' '\.'
    cat "$ICI/101-visuels-editions.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 -v depot=1 2>&1 \
    | sed 's/^NOTICE:  //'
