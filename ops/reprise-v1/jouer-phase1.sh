#!/usr/bin/env bash
# Phase 1 de la reprise sur un clone du banc, telle que la bascule la joue
# (BASCULE.md, geste 4). Le banc doit être monté : ops/reprise-v1/repeter.sh.
#
#   jouer-phase1.sh [base]      défaut : epavillon_finale
#   jouer-phase1.sh [base] envoi
#       puis, une fois les fichiers déposés dans Garage (geste 5) : retire de la
#       base ce qui ne doit pas partir en production et écrit l'archive à envoyer.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
RACINE=$(cd "$ICI/../.." && pwd)
CONTENEUR=${CONTENEUR:-epavillon-postgres}
BASE=${1:-epavillon_finale}
case "$BASE" in epavillon_finale|epavillon_finale_*) ;; *) echo "Base refusée : $BASE" >&2; exit 2 ;; esac

psql_base() { docker exec -i "$CONTENEUR" psql -U postgres -d "$BASE" -X -q -v ON_ERROR_STOP=1 "$@"; }
etape() { printf '\n== %s\n' "$*"; }

if [ "${2:-}" = envoi ]; then
    etape "Retrait de la v1 brute (empreintes bcrypt) et des tables de travail du dépôt"
    psql_base <<'SQL'
DROP SCHEMA v1, v1_auth, v1_storage CASCADE;
DROP TABLE legacy.garage_a_purger, legacy.fichiers_a_deposer;
SQL
    sortie="$RACINE/sauvegardes/reprise-v1/$BASE-$(date +%Y%m%d-%H%M).sql.gz"
    etape "Archive pour la production : $sortie"
    docker exec "$CONTENEUR" pg_dump -U postgres --clean --if-exists "$BASE" | gzip > "$sortie"
    ls -lh "$sortie"
    exit 0
fi

etape "Clone du banc : $BASE"
docker exec -i "$CONTENEUR" psql -U postgres -d postgres -X -q -v ON_ERROR_STOP=1 \
    -c "SET client_min_messages = warning" \
    -c "DROP DATABASE IF EXISTS $BASE WITH (FORCE)" \
    -c "CREATE DATABASE $BASE TEMPLATE epavillon_repetition"

etape "Migration D1 (celle que reçoit la production)"
psql_base < "$RACINE/ops/migrations/2026-10-27-compte-sans-mot-de-passe.sql"

for lot in 10-referentiels 20-personnes-comptes-roles 30-organisations 40-evenements 50-activites; do
    etape "Lot $lot"
    psql_base < "$ICI/lots/$lot.sql"
done

etape "Lot 60 (déclaration des fichiers)"
"$ICI/lots/60-fichiers.sh" sql "$BASE"

etape "Bilan"
psql_base -At -F ' | ' <<'SQL'
SELECT 'id_map ' || disposition, count(*) FROM legacy.id_map GROUP BY 1 ORDER BY 1;
SELECT 'personnes', count(*) FROM identity.people;
SELECT 'comptes à nouveau mot de passe', count(*) FROM identity.accounts WHERE password_reset_required_at IS NOT NULL;
SELECT 'organisations', count(*) FROM org.organizations;
SELECT 'propositions ' || status, count(*) FROM programme.proposals GROUP BY status ORDER BY 1;
SELECT 'séances ' || status, count(*) FROM programme.sessions GROUP BY status ORDER BY 1;
SELECT 'médias', count(*) FROM media.assets;
SELECT 'outbox non publiée ' || event_type, count(*) FROM platform.outbox_events WHERE published_at IS NULL GROUP BY 1 ORDER BY 1;
SELECT 'travaux ' || task || ' ' || status, count(*) FROM platform.jobs GROUP BY 1 ORDER BY 1;
SQL
