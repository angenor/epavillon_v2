#!/usr/bin/env bash
# Lot 96 (phase 2) : galeries d'activité, D7 (96-galeries.sql).
#
#   96-galeries.sh <clone>   sur un clone local de la production (après la migration
#                            2026-10-29-galerie-activite) ; prépare
#                            legacy.fichiers_a_deposer pour 60-fichiers.sh, sans rien déposer
#   96-galeries.sh flux      écrit le flux SQL sur la sortie, à faire lire par le
#                            psql de la production (BASCULE.md, « Après »)
#
# Les photos se lisent dans la v1 du banc (BANC, v1.media_gallery et
# v1_storage.objects), leurs fichiers dans la copie locale des buckets
# (FICHIERS_V1) : le flux porte la ligne v1 (activité, légende, auteur, date),
# le chemin, la taille, le type et l'empreinte, jamais le fichier.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RACINE=$(cd "$ICI/../../.." && pwd)
FICHIERS=${FICHIERS_V1:-$RACINE/sauvegardes/reprise-v1/fichiers}
CONTENEUR=${CONTENEUR:-epavillon-postgres}
BANC=${BANC:-epavillon_repetition}

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac

TRAVAIL=$(mktemp -d)
trap 'rm -rf "$TRAVAIL"' EXIT

# Le chemin en tête de ligne, pour retrouver le fichier ; le reste déjà au format
# texte de COPY (une légende peut porter tabulation ou barre oblique inverse).
docker exec -i "$CONTENEUR" psql -U postgres -d "$BANC" -X -q -At -v ON_ERROR_STOP=1 -c "COPY (
    SELECT o.bucket_id || '/' || o.name, g.id, g.context_type, g.media_type,
           g.thumbnail_url IS NULL OR g.thumbnail_url = g.media_url,
           g.context_id, g.title, g.uploaded_by, g.created_at, o.id
    FROM v1.media_gallery g
    LEFT JOIN v1_storage.objects o
           ON o.bucket_id || '/' || o.name = regexp_replace(g.media_url, '^.*/storage/v1/object/public/', '')
    ORDER BY g.id
) TO STDOUT" > "$TRAVAIL/photos"
[ -s "$TRAVAIL/photos" ] || { echo "aucune photo de galerie dans $BANC" >&2; exit 1; }

while IFS= read -r ligne; do
    f=${ligne%%$'\t'*}
    test -f "$FICHIERS/$f" || { echo "Photo absente : $FICHIERS/$f" >&2; exit 1; }
done < "$TRAVAIL/photos"

flux() {
    printf '%s\n' 'CREATE TEMP TABLE lot96_photo (fichier text, id uuid PRIMARY KEY, contexte text, genre text, vignette_identique boolean, activite_v1 uuid, titre text, auteur_v1 uuid, cree_le timestamptz, objet_v1 uuid, taille bigint, mime text, sha256 text);'
    printf '%s\n' 'COPY lot96_photo FROM STDIN WITH (FORMAT text);'
    while IFS= read -r ligne; do
        f=${ligne%%$'\t'*}
        printf '%s\t%s\t%s\t%s\n' "$ligne" "$(wc -c < "$FICHIERS/$f" | tr -d ' ')" \
            "$(file -b --mime-type "$FICHIERS/$f")" "$(shasum -a 256 "$FICHIERS/$f" | cut -d' ' -f1)"
    done < "$TRAVAIL/photos"
    printf '%s\n' '\.'
    cat "$ICI/96-galeries.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 -v depot=1 2>&1 \
    | sed 's/^NOTICE:  //'
