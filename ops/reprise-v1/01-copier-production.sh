#!/usr/bin/env bash
# Recrée epavillon_repetition depuis la copie de la production v2.
set -euo pipefail
source "$(dirname "$0")/commun.sh"

test -f "$COPIE_PROD" || { echo "Copie de production introuvable : $COPIE_PROD" >&2; exit 1; }

etape "Copie de la production : $(basename "$COPIE_PROD") -> $BASE_BANC"
recreer_base "$BASE_BANC"
gunzip -c "$COPIE_PROD" | psql_dans "$BASE_BANC" >/dev/null
psql_dans "$BASE_BANC" -At -c "SELECT 'personnes : ' || count(*) FROM identity.people"
