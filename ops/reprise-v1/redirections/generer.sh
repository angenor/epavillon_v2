#!/usr/bin/env bash
# Écrit la table des redirections v1 → v2 (D11) lue par le site :
# frontend/server/utils/redirections-v1.json.
#
#   ops/reprise-v1/redirections/generer.sh <base>
#
# <base> : une base du conteneur portant `legacy.id_map`, par exemple une
# restauration jetable de la dernière sauvegarde de production. Lecture seule.
#
# Ne sort que des identifiants v1 et des adresses v2 publiques : une séance
# compte si `programme.v_public_schedule` la montre, une édition si
# `event.v_public_editions` la montre. Le reste retombe, côté site, sur
# l'équivalent le plus proche.
set -euo pipefail

BASE=${1:?usage : generer.sh <base du conteneur>}
CONTENEUR=${CONTENEUR:-epavillon-postgres}
RACINE=$(cd "$(dirname "$0")/../../.." && pwd)
SORTIE="$RACINE/frontend/server/utils/redirections-v1.json"

docker exec -i "$CONTENEUR" psql -U postgres -d "$BASE" -X -q -A -t -v ON_ERROR_STOP=1 > "$SORTIE.tmp" <<'SQL'
BEGIN READ ONLY;

WITH editions AS (
    SELECT m.legacy_id,
           '/evenements/' || e.slug AS page,
           CASE WHEN e.programme_published_at IS NOT NULL
                THEN '/programmations?edition=' || e.slug
                ELSE '/evenements/' || e.slug END AS programmation,
           e.id AS event_id
      FROM legacy.id_map m
      JOIN event.v_public_editions e ON e.id = m.target_id
     WHERE m.legacy_table = 'public.events'
),
activites AS (
    SELECT m.legacy_id,
           COALESCE('/programmations/' || e.slug || '/' || s.slug, ed.programmation) AS cible
      FROM legacy.id_map m
      JOIN programme.proposals p ON p.id = m.target_id
      JOIN event.v_public_editions e ON e.id = p.event_id
      JOIN editions ed ON ed.event_id = e.id
      LEFT JOIN programme.v_public_schedule s ON s.proposal_id = p.id
     WHERE m.legacy_table = 'public.activities'
),
-- Les pages statiques de la v1 portaient le nom de la journée dans l'adresse ;
-- la v2 montre une journée spéciale dans la liste du jour de la programmation (D14).
journees AS (
    SELECT CASE m.legacy_id
               WHEN 'journee_jeunesse_climat' THEN 'journee-jeunesse'
               WHEN 'journee_finance_durable' THEN 'journee-finance'
           END AS page_v1,
           '/programmations?edition=' || e.slug
               || COALESCE('&vue=liste&jour=' || t.starts_on, '') AS cible
      FROM legacy.id_map m
      JOIN event.programme_tracks t ON t.id = m.target_id AND t.published_at IS NOT NULL
      JOIN event.v_public_editions e ON e.id = t.event_id AND e.programme_published_at IS NOT NULL
     WHERE m.legacy_table = 'code.journees-speciales'
       AND m.target_table = 'programme_tracks'
)
SELECT jsonb_pretty(jsonb_build_object(
    'activites', (SELECT COALESCE(jsonb_object_agg(legacy_id, cible), '{}') FROM activites),
    'editions', (SELECT COALESCE(jsonb_object_agg(legacy_id,
                     jsonb_build_object('page', page, 'programmation', programmation)), '{}') FROM editions),
    'journees', (SELECT COALESCE(jsonb_object_agg(page_v1, cible), '{}') FROM journees WHERE page_v1 IS NOT NULL)
));

COMMIT;
SQL

mv "$SORTIE.tmp" "$SORTIE"
python3 - "$SORTIE" <<'PY'
import json, sys
t = json.load(open(sys.argv[1]))
print(f"{len(t['activites'])} activités, {len(t['editions'])} éditions, {len(t['journees'])} journées : {sys.argv[1]}")
PY
