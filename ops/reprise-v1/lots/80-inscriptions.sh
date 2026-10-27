#!/usr/bin/env bash
# Lot 80 (phase 2) : inscriptions aux activités (80-inscriptions.sql).
#
#   80-inscriptions.sh <clone>   sur un clone local de la production
#   80-inscriptions.sh flux      écrit le flux SQL sur la sortie, à faire lire par
#                                le psql de la production (BASCULE.md, « Après »)
#
# Lit la v1 au banc (BANC, défaut epavillon_repetition) et n'en fait voyager que
# des identifiants et des dates : invités rapprochés de leur compte ici, par adresse.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CONTENEUR=${CONTENEUR:-epavillon-postgres}
BANC=${BANC:-epavillon_repetition}

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac

banc() { docker exec -i "$CONTENEUR" psql -U postgres -d "$BANC" -X -q -At -v ON_ERROR_STOP=1 "$@"; }

flux() {
    printf "SELECT set_config('lot80.attendues', '%s', false) \\\\g /dev/null\n" \
        "$(banc -c 'SELECT count(*) FROM v1.activity_registrations')"
    printf '%s\n' 'CREATE TEMP TABLE lot80_inscription (id uuid PRIMARY KEY, activity_id uuid, personne_v1 uuid, organisation_v1 uuid, registration_date timestamptz, invite boolean);'
    printf '%s\n' 'COPY lot80_inscription FROM STDIN WITH (FORMAT text);'
    banc -c "COPY (
        SELECT r.id, r.activity_id, COALESCE(r.user_id, c.id), u.organization_id,
               r.registration_date, r.user_id IS NULL
        FROM v1.activity_registrations r
        JOIN v1.events e ON e.id = (SELECT a.event_id FROM v1.activities a WHERE a.id = r.activity_id)
        LEFT JOIN v1_auth.users c ON r.user_id IS NULL AND lower(c.email) = lower(r.guest_email)
        LEFT JOIN v1.users u ON u.id = COALESCE(r.user_id, c.id)
        WHERE e.title NOT ILIKE 'PACO%'
    ) TO STDOUT"
    printf '%s\n' '\.'
    printf '%s\n' 'CREATE TEMP TABLE lot80_paco (id text PRIMARY KEY);'
    printf '%s\n' 'COPY lot80_paco FROM STDIN WITH (FORMAT text);'
    banc -c "COPY (
        SELECT r.id FROM v1.activity_registrations r
        JOIN v1.events e ON e.id = (SELECT a.event_id FROM v1.activities a WHERE a.id = r.activity_id)
        WHERE e.title ILIKE 'PACO%'
    ) TO STDOUT"
    printf '%s\n' '\.'
    cat "$ICI/80-inscriptions.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 2>&1 \
    | sed 's/^NOTICE:  //'
