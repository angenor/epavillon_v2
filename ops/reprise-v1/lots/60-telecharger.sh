#!/usr/bin/env bash
# Lot 60, temps 1 : copie locale de tous les objets des buckets Supabase.
#
# Lecture seule côté Supabase. Destination : sauvegardes/reprise-v1/fichiers/
# (ignoré par Git), un fichier par objet sous <bucket>/<chemin>, plus
# manifeste.tsv : bucket, chemin, taille, type, sha256, date de dépôt.
# Rejouable : un objet déjà présent, de même taille que dans Supabase et à
# l'empreinte du manifeste précédent, n'est pas retéléchargé.
#
# La clé secrète est lue dans .env et ne passe que par un fichier de
# configuration curl temporaire (mode 600) : ni argument visible, ni journal.
set -euo pipefail

RACINE=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
DEST=${FICHIERS_V1:-$RACINE/sauvegardes/reprise-v1/fichiers}
MANIFESTE=$DEST/manifeste.tsv

lire_env() {
    local valeur
    valeur=$(grep -E "^$1=" "$RACINE/.env" | tail -1 | cut -d= -f2-)
    valeur=${valeur%\"}; valeur=${valeur#\"}; valeur=${valeur%\'}; valeur=${valeur#\'}
    printf '%s' "$valeur"
}
URL=$(lire_env V1_SUPABASE_URL)
URL=${URL%/}
test -n "$URL" || { echo "V1_SUPABASE_URL absente de .env" >&2; exit 1; }

TRAVAIL=$(mktemp -d)
trap 'rm -rf "$TRAVAIL"' EXIT
chmod 700 "$TRAVAIL"
CONF=$TRAVAIL/curl.conf
(
    umask 077
    cle=$(lire_env V1_SUPABASE_SECRET_KEY)
    test -n "$cle" || { echo "V1_SUPABASE_SECRET_KEY absente de .env" >&2; exit 1; }
    printf 'header = "apikey: %s"\nheader = "Authorization: Bearer %s"\n' "$cle" "$cle" > "$CONF"
)

api() { curl -sS --fail --retry 3 --retry-delay 2 -K "$CONF" "$@"; }

# Liste récursive d'un bucket : l'API rend un niveau à la fois, un dossier a
# un `id` nul.
lister() {
    local bucket=$1 prefixe=$2 offset=0 page
    while :; do
        page=$(api -X POST "$URL/storage/v1/object/list/$bucket" \
            -H 'Content-Type: application/json' \
            --data "$(jq -cn --arg p "$prefixe" --argjson o "$offset" \
                '{prefix: $p, limit: 1000, offset: $o, sortBy: {column: "name", order: "asc"}}')")
        jq -r --arg b "$bucket" --arg p "$prefixe" '.[] | select(.id != null)
            | [$b, (if $p == "" then .name else $p + "/" + .name end),
               (.metadata.size // 0), (.metadata.mimetype // ""), (.created_at // "")]
            | map(tostring) | join("\u001f")' <<<"$page"
        while IFS= read -r dossier; do
            lister "$bucket" "${prefixe:+$prefixe/}$dossier"
        done < <(jq -r '.[] | select(.id == null) | .name' <<<"$page")
        [ "$(jq length <<<"$page")" -lt 1000 ] && break
        offset=$((offset + 1000))
    done
}

mkdir -p "$DEST"
echo "== Inventaire des buckets"
api "$URL/storage/v1/bucket" | jq -r '.[].id' | sort > "$TRAVAIL/buckets"
: > "$TRAVAIL/objets.tsv"
while IFS= read -r bucket; do
    lister "$bucket" "" >> "$TRAVAIL/objets.tsv"
    printf '  %-16s %s objets\n' "$bucket" "$(awk -F'\037' -v b="$bucket" '$1 == b' "$TRAVAIL/objets.tsv" | wc -l | tr -d ' ')"
done < "$TRAVAIL/buckets"

# Empreinte du passage précédent (bash 3.2 de macOS : pas de tableau associatif).
empreinte_connue() {
    [ -f "$MANIFESTE" ] || return 0
    awk -F'\t' -v b="$1" -v c="$2" '$1 == b && $2 == c {print $5; exit}' "$MANIFESTE"
}

echo "== Téléchargement"
nouveau=$TRAVAIL/manifeste.tsv
printf 'bucket\tchemin\ttaille\ttype\tsha256\tdepose_le\n' > "$nouveau"
telecharges=0 gardes=0 echecs=0
while IFS=$'\037' read -r bucket chemin taille type depose; do
    local_=$DEST/$bucket/$chemin
    connue=$(empreinte_connue "$bucket" "$chemin")
    if [ -f "$local_" ] && [ "$(stat -f %z "$local_")" = "$taille" ] && [ -n "$connue" ] \
       && [ "$(shasum -a 256 "$local_" | cut -d' ' -f1)" = "$connue" ]; then
        gardes=$((gardes + 1))
    else
        mkdir -p "$(dirname "$local_")"
        chemin_url=$(jq -rn --arg c "$chemin" '$c | split("/") | map(@uri) | join("/")')
        if api -o "$local_.part" "$URL/storage/v1/object/authenticated/$bucket/$chemin_url" \
           && [ "$(stat -f %z "$local_.part")" = "$taille" ]; then
            mv "$local_.part" "$local_"
            telecharges=$((telecharges + 1))
        else
            rm -f "$local_.part"
            echecs=$((echecs + 1))
            printf 'ÉCHEC %s/%s\n' "$bucket" "$chemin" >&2
            continue
        fi
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\n' "$bucket" "$chemin" "$taille" "$type" \
        "$(shasum -a 256 "$local_" | cut -d' ' -f1)" "$depose" >> "$nouveau"
done < "$TRAVAIL/objets.tsv"
mv "$nouveau" "$MANIFESTE"

total=$(wc -l < "$TRAVAIL/objets.tsv" | tr -d ' ')
volume=$(awk -F'\t' 'NR > 1 {s += $3} END {printf "%.1f", s / 1048576}' "$MANIFESTE")
printf '%s objets (%s Mo) : %s téléchargés, %s déjà présents, %s échecs\n' \
    "$total" "$volume" "$telecharges" "$gardes" "$echecs"
printf 'Manifeste : %s\n' "${MANIFESTE#"$RACINE"/}"
[ "$echecs" -eq 0 ]
