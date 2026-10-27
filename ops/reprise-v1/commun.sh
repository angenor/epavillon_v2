# Réglages partagés du banc de répétition, chargés par les scripts numérotés.
# Les bases nommées ici sont les seules que le banc crée ou efface.

RACINE=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
ICI="$RACINE/ops/reprise-v1"

CONTENEUR=${CONTENEUR:-epavillon-postgres}
BASE_BANC=epavillon_repetition
BASE_PASSAGE=epavillon_passage_v1
BASE_NEUVE=epavillon_neuve

COPIE_PROD=${COPIE_PROD:-$RACINE/sauvegardes/epavillon-20261027-114451.sql.gz}
EXTRACTION_V1=${EXTRACTION_V1:-$RACINE/sauvegardes/reprise-v1/supabase-v1-20261027-1126-avant-rls.dump}

# pg_restore du poste (18) : l'archive Supabase vient d'un pg_dump 18, que le
# pg_restore 17 du conteneur refuse. Il écrit du SQL, joué par le psql du conteneur.
PG_RESTORE=${PG_RESTORE:-/opt/homebrew/opt/libpq/bin/pg_restore}

psql_dans() {
    local base=$1; shift
    case "$base" in
        postgres|"$BASE_BANC"|"$BASE_PASSAGE"|"$BASE_NEUVE") ;;
        *) echo "Base refusée : $base (le banc ne touche qu'à ses propres bases)" >&2; return 1 ;;
    esac
    docker exec -i "$CONTENEUR" psql -U postgres -d "$base" -X -q -v ON_ERROR_STOP=1 "$@"
}

recreer_base() {
    case "$1" in
        "$BASE_BANC"|"$BASE_PASSAGE"|"$BASE_NEUVE") ;;
        *) echo "Base refusée : $1" >&2; return 1 ;;
    esac
    psql_dans postgres -c "SET client_min_messages = warning" -c "DROP DATABASE IF EXISTS $1 WITH (FORCE)" -c "CREATE DATABASE $1 TEMPLATE template0"
}

etape() { printf '\n== %s\n' "$*"; }
