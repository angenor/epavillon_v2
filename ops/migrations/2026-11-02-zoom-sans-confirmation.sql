-- APRÈS la mise en ligne du worker qui coupe la confirmation de Zoom (02/11/2026).
-- Les réunions d'activités à venir déjà créées gardent la confirmation de Zoom
-- tant qu'elles ne sont pas relues : les repasser à « pending » pose un travail
-- live.sync_meeting par réunion (déclencheur tg_meetings_enqueue_sync), qui la
-- coupe chez Zoom. Les réunions de journée l'ont déjà coupée.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-zoom-sans-confirmation', true);

UPDATE live.meetings
   SET sync_status = 'pending'
 WHERE track_id IS NULL AND sync_status = 'synced'
   AND external_id IS NOT NULL AND scheduled_start_at > now();
COMMIT;
