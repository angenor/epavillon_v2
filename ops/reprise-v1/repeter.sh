#!/usr/bin/env bash
# Monte le banc de répétition de zéro : copie de la production, migrations de
# schéma de la reprise, purge des essais, v1 brute, lot 00. epavillon_repetition
# sert ensuite de modèle aux lots (lots/CONTRAT.md) : aucune connexion n'y reste.
set -euo pipefail
source "$(dirname "$0")/commun.sh"

# Migrations de production que la reprise exige, dans l'ordre où la bascule les jouera.
MIGRATIONS=(
    "$RACINE/ops/migrations/2026-10-27-reprise-v1-correspondance.sql"
)

"$ICI/01-copier-production.sh"

for m in "${MIGRATIONS[@]}"; do
    etape "Migration $(basename "$m")"
    psql_dans "$BASE_BANC" -c "SET client_min_messages = warning" -f - < "$m"
done

etape "Purge des essais (D2)"
psql_dans "$BASE_BANC" -At < "$ICI/03-purger-essais.sql"

"$ICI/02-poser-v1-brute.sh"

etape "Lot 00 : socle"
psql_dans "$BASE_BANC" < "$ICI/lots/00-socle.sql"

restantes=$(psql_dans postgres -At -c "SELECT count(*) FROM pg_stat_activity WHERE datname = '$BASE_BANC'")
if [ "$restantes" != 0 ]; then
    echo "ÉCHEC : $restantes connexion(s) encore ouvertes sur $BASE_BANC" >&2
    exit 1
fi
etape "Banc prêt : $BASE_BANC (modèle des lots, aucune connexion ouverte)"
