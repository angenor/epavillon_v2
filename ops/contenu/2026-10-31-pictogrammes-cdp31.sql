-- Contenu (31/10) : les pictogrammes des trois fils de la CdP31.
--
--   atelier_preparatoire     -> book    (le Guide des négociations)
--   journee_finance_durable  -> coins
--   journee_jeunesse_climat  -> sprout
--
-- Jouer APRÈS ops/migrations/2026-10-31-pictogramme-fil.sql et
-- ops/contenu/2026-10-30-journees-cdp31.sql (qui crée les fils).
--
-- REJOUABLE : seul un fil SANS pictogramme en reçoit un. Une seconde passe ne
-- change rien, et un choix fait entre-temps au back-office est gardé.
-- Un fil absent n'arrête rien : le compte rendu dit combien ont reçu le leur.
--
-- Production :
--   ssh <serveur> "cd /opt/epavillon && $COMPOSE exec -T postgres psql -U postgres -d epavillon -X -q -v ON_ERROR_STOP=1" < ops/contenu/2026-10-31-pictogrammes-cdp31.sql

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'contenu-2026-10-31-pictogrammes-cdp31', true) \g /dev/null

WITH poses AS (
    UPDATE event.programme_tracks t
       SET icon_code = x.icon
      FROM (VALUES ('atelier_preparatoire',    'book'),
                   ('journee_finance_durable', 'coins'),
                   ('journee_jeunesse_climat', 'sprout')) AS x(code, icon),
           event.events e
     WHERE e.id = t.event_id
       AND e.slug = 'cdp31-antalya-2026'
       AND t.code = x.code
       AND t.icon_code IS NULL
    RETURNING t.code, t.icon_code
)
SELECT count(*) AS pictogrammes_poses, string_agg(code || ' = ' || icon_code, ', ' ORDER BY code) AS detail
  FROM poses;

COMMIT;
