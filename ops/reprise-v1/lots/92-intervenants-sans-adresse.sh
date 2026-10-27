#!/usr/bin/env bash
# Lot 92 (phase 2) : intervenants écartés pour D16 (92-intervenants-sans-adresse.sql).
#
#   92-intervenants-sans-adresse.sh <clone>   sur un clone local de la production
#   92-intervenants-sans-adresse.sh flux      écrit le flux SQL sur la sortie, à faire lire
#                                             par le psql de la production (BASCULE.md, « Après »)
#
# Les écartés se lisent dans legacy.id_map de la base de référence (REFERENCE,
# défaut epavillon_finale_retour, copie de la production après la phase 1) ; leurs
# fiches dans la v1 du banc (BANC). Le flux porte tous les intervenants v1 des
# activités concernées, pour l'ordre, mais l'identité (civilité, nom, fonction,
# organisation) des seuls écartés ; aucune adresse.
set -euo pipefail
ICI=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
CONTENEUR=${CONTENEUR:-epavillon-postgres}
BANC=${BANC:-epavillon_repetition}
REFERENCE=${REFERENCE:-epavillon_finale_retour}

cible=${1:-}
case "$cible" in
    ''|epavillon|epavillon_repetition|postgres|template*)
        echo "Usage : $0 <clone local>|flux" >&2; exit 2 ;;
esac

psql_de() { local base=$1; shift; docker exec -i "$CONTENEUR" psql -U postgres -d "$base" -X -q -At -v ON_ERROR_STOP=1 "$@"; }

ecartes=$(psql_de "$REFERENCE" -c "SELECT string_agg(quote_literal(legacy_id), ',') FROM legacy.id_map
    WHERE legacy_table = 'public.activity_speakers' AND disposition = 'skipped' AND note LIKE 'D16%'")
[ -n "$ecartes" ] || { echo "aucun intervenant écarté pour D16 dans $REFERENCE" >&2; exit 1; }

flux() {
    printf "SELECT set_config('lot92.attendus', '%s', false) \\\\g /dev/null\n" \
        "$(psql_de "$BANC" -c "SELECT count(*) FROM v1.activity_speakers WHERE id::text IN ($ecartes)")"
    printf '%s\n' 'CREATE TEMP TABLE lot92_int (id uuid PRIMARY KEY, activity_id uuid NOT NULL, rang int NOT NULL, civilite text, prenom text, nom text, fonction text, organisation text, disponible boolean, created_at timestamptz);'
    printf '%s\n' 'COPY lot92_int FROM STDIN WITH (FORMAT text);'
    psql_de "$BANC" -c "COPY (
        WITH s AS (
            SELECT s.*, s.id::text IN ($ecartes) AS ecarte,
                   row_number() OVER (PARTITION BY s.activity_id ORDER BY s.created_at, s.id) AS rang
            FROM v1.activity_speakers s
            WHERE s.activity_id IN (SELECT activity_id FROM v1.activity_speakers WHERE id::text IN ($ecartes))
        )
        SELECT id, activity_id, rang,
               CASE WHEN ecarte THEN CASE lower(btrim(civility))
                   WHEN 'mr' THEN 'm' WHEN 'm.' THEN 'm' WHEN 'm' THEN 'm' WHEN 'm. v2' THEN 'm'
                   WHEN 'mrs' THEN 'mme' WHEN 'mme' THEN 'mme' WHEN 'ms' THEN 'mme'
                   WHEN 'dr' THEN 'dr' WHEN 'dr.' THEN 'dr'
                   WHEN 'prof' THEN 'pr' END END,
               CASE WHEN ecarte THEN btrim(first_name) END,
               CASE WHEN ecarte THEN btrim(last_name) END,
               CASE WHEN ecarte THEN NULLIF(btrim(position), '') END,
               CASE WHEN ecarte THEN NULLIF(btrim(organization), '') END,
               CASE WHEN ecarte THEN COALESCE(is_available_for_questions, true) END,
               CASE WHEN ecarte THEN created_at END
        FROM s
    ) TO STDOUT"
    printf '%s\n' '\.'
    cat "$ICI/92-intervenants-sans-adresse.sql"
}

if [ "$cible" = flux ]; then
    flux
    exit 0
fi
flux | docker exec -i "$CONTENEUR" psql -U postgres -d "$cible" -X -q -v ON_ERROR_STOP=1 2>&1 \
    | sed 's/^NOTICE:  //'
