#!/usr/bin/env bash
# Lot 60, temps 2 et 3 : déclarer les fichiers en base, puis les déposer dans Garage.
#
#   60-fichiers.sh sql <base>       60-fichiers.sql en une transaction, sur le
#                                   manifeste de 60-telecharger.sh
#   60-fichiers.sh deposer <base>   dépose dans Garage les objets de
#                                   legacy.fichiers_a_deposer (rejouable : un objet
#                                   déjà présent à la bonne taille est laissé)
#
# Les deux temps sont séparés parce que la bascule les sépare (BASCULE.md,
# gestes 4 et 5). Le worker reste arrêté entre eux : il traiterait un objet
# déclaré mais pas encore déposé.
#
# Variables :
#   SEAU         bucket cible, écrit dans media.assets (temps sql). Par défaut
#                le réglage media.default_bucket de la base.
#   ENV_GARAGE   fichier d'où lire S3_ENDPOINT, S3_REGION, S3_ACCESS_KEY_ID,
#                S3_SECRET_ACCESS_KEY (défaut : .env, la Garage locale). À la
#                bascule : .env.prod, S3_ENDPOINT pointant sur le tunnel.
#   FICHIERS_V1  copie locale des buckets (défaut : sauvegardes/reprise-v1/fichiers).
#   CONTENEUR    conteneur Postgres (défaut : epavillon-postgres).
set -euo pipefail

RACINE=$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
FICHIERS=${FICHIERS_V1:-$RACINE/sauvegardes/reprise-v1/fichiers}
CONTENEUR=${CONTENEUR:-epavillon-postgres}

temps=${1:-}; base=${2:-}
case "$temps" in sql|deposer) ;; *) echo "Usage : $0 sql|deposer <base>" >&2; exit 2 ;; esac
case "$base" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Base refusée : '$base' (un clone du banc, ou la base de la bascule)" >&2; exit 2 ;;
esac

psql_base() { docker exec -i "$CONTENEUR" psql -U postgres -d "$base" -X -q -v ON_ERROR_STOP=1 "$@"; }

if [ "$temps" = sql ]; then
    manifeste=$FICHIERS/manifeste.tsv
    test -f "$manifeste" || { echo "Manifeste absent : lancer 60-telecharger.sh" >&2; exit 1; }
    seau=${SEAU:-$(psql_base -At -c "SELECT value #>> '{}' FROM platform.settings WHERE key = 'media.default_bucket'")}
    echo "== Lot 60 sur $base, bucket cible « $seau »"
    {
        printf '%s\n' "SELECT set_config('lot60.seau', '$seau', false) \\g /dev/null"
        printf '%s\n' 'CREATE TEMP TABLE lot60_manifeste (bucket text, chemin text, taille bigint, type text, sha256 text, depose_le timestamptz);'
        printf '%s\n' "COPY lot60_manifeste FROM STDIN WITH (FORMAT text);"
        tail -n +2 "$manifeste"
        printf '%s\n' '\.'
        cat "$ICI/60-fichiers.sql"
    } | psql_base 2>&1 | sed 's/^NOTICE:  //'
    exit 0
fi

# --- deposer -----------------------------------------------------------------

lire() {
    local valeur
    valeur=$(grep -E "^$1=" "$ENV_FICHIER" | tail -1 | cut -d= -f2-)
    valeur=${valeur%\"}; valeur=${valeur#\"}
    printf '%s' "$valeur"
}
ENV_FICHIER=${ENV_GARAGE:-$RACINE/.env}
test -f "$ENV_FICHIER" || { echo "Fichier de clés absent : $ENV_FICHIER" >&2; exit 1; }
ENDPOINT=${S3_ENDPOINT:-$(lire S3_ENDPOINT)}; ENDPOINT=${ENDPOINT%/}
REGION=${S3_REGION:-$(lire S3_REGION)}

TRAVAIL=$(mktemp -d)
trap 'rm -rf "$TRAVAIL"' EXIT
chmod 700 "$TRAVAIL"
(
    umask 077
    printf 'user = "%s:%s"\n' "$(lire S3_ACCESS_KEY_ID)" "$(lire S3_SECRET_ACCESS_KEY)" > "$TRAVAIL/curl.conf"
)
s3() { curl -sS --aws-sigv4 "aws:amz:${REGION:-garage}:s3" -K "$TRAVAIL/curl.conf" "$@"; }

psql_base -At -F $'\037' -c "SELECT bucket, object_key, source, sha256, byte_size, mime_type FROM legacy.fichiers_a_deposer ORDER BY 1, 2" \
    > "$TRAVAIL/liste"
echo "== Dépôt de $(wc -l < "$TRAVAIL/liste" | tr -d ' ') objets vers $ENDPOINT"

deposes=0 presents=0 echecs=0
while IFS=$'\037' read -r bucket cle source sha taille mime; do
    local_=$FICHIERS/$source
    if [ ! -f "$local_" ] || [ "$(shasum -a 256 "$local_" | cut -d' ' -f1)" != "$sha" ]; then
        echo "ÉCHEC copie locale absente ou altérée : $source" >&2; echecs=$((echecs + 1)); continue
    fi
    longueur=$(s3 -I -o /dev/null -w '%{http_code} %header{content-length}' "$ENDPOINT/$bucket/$cle" < /dev/null || true)
    if [ "$longueur" = "200 $taille" ]; then
        presents=$((presents + 1)); continue
    fi
    code=$(s3 -o /dev/null -w '%{http_code}' -X PUT -T "$local_" \
              -H "Content-Type: $mime" -H "x-amz-content-sha256: $sha" "$ENDPOINT/$bucket/$cle" < /dev/null || true)
    if [ "$code" = 200 ]; then
        deposes=$((deposes + 1))
    else
        echo "ÉCHEC dépôt ($code) : $bucket/$cle" >&2; echecs=$((echecs + 1))
    fi
done < "$TRAVAIL/liste"

printf '%s déposés, %s déjà présents, %s échecs\n' "$deposes" "$presents" "$echecs"
[ "$echecs" -eq 0 ]
