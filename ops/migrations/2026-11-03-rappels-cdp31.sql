-- Rappels de la CdP31 (03/11/2026), à la demande du commanditaire : aucune
-- règle de rappels n'existait pour l'édition, donc aucun rappel n'était
-- programmé (activités : engagement.schedule_session_reminders rend 0 sans
-- règle ; journées : decalages_de_journee lit la même règle).
--   1. Règle de l'édition, réglages par défaut du modèle : 2 jours, 1 jour,
--      1 heure, 30 minutes, par courriel.
--   2. Rappels des inscrits actuels aux activités à venir (la fonction du
--      modèle, qui pose aussi les travaux).
--   3. Rappels de journée (2 jours, 1 jour) des inscrits actuels, posés comme
--      `engagement::jobs::journees::poser_rappels` : même charge, même clé.
-- Les inscriptions suivantes passent par le chemin ordinaire. Rejouable.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-03-rappels-cdp31', true);

DO $$
DECLARE
    v_edition uuid := '01a17da1-9b46-75b2-895a-d9e226226be2';
    v_seance record;
    v_crees int := 0;
    v_n int;
BEGIN
    INSERT INTO engagement.reminder_rules (event_id, created_by)
    SELECT v_edition, '01a0627c-f3be-7185-89c8-9a4015ad158e'
     WHERE NOT EXISTS (SELECT 1 FROM engagement.reminder_rules WHERE event_id = v_edition);

    FOR v_seance IN
        SELECT DISTINCT s.id
          FROM programme.sessions s
          JOIN programme.registrations r ON r.session_id = s.id
         WHERE s.event_id = v_edition AND s.starts_at > now()
           AND s.status::text NOT IN ('cancelled', 'postponed')
           AND r.status::text IN ('registered', 'attended')
    LOOP
        v_n := engagement.schedule_session_reminders(v_seance.id);
        v_crees := v_crees + v_n;
    END LOOP;
    RAISE NOTICE 'rappels d''activité créés : %', v_crees;

    WITH debut AS (
        SELECT t.id AS track_id,
               (SELECT min(x.d) FROM (
                    SELECT s.starts_at AS d
                      FROM programme.session_tracks st
                      JOIN programme.sessions s ON s.id = st.session_id
                     WHERE st.track_id = t.id AND s.status <> 'cancelled'
                    UNION ALL
                    SELECT m.starts_at FROM event.programme_track_moments m WHERE m.track_id = t.id) x) AS d
          FROM event.programme_tracks t
         WHERE t.event_id = v_edition
    ), poses AS (
        INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
        SELECT 'email', 'engagement.send_day_reminder',
               jsonb_build_object('track_registration_id', tr.id,
                                  'offset_minutes', o.minutes,
                                  'starts_at', floor(extract(epoch FROM debut.d))::bigint),
               tr.id || ':' || o.minutes || ':' || floor(extract(epoch FROM debut.d))::bigint,
               debut.d - make_interval(mins => o.minutes::int)
          FROM programme.track_registrations tr
          JOIN debut ON debut.track_id = tr.track_id
          CROSS JOIN (VALUES (2880::bigint), (1440::bigint)) AS o(minutes)
         WHERE tr.status::text IN ('registered', 'attended')
           AND debut.d IS NOT NULL
           AND debut.d - make_interval(mins => o.minutes::int) > now()
        ON CONFLICT DO NOTHING
        RETURNING 1
    )
    SELECT count(*) INTO v_n FROM poses;
    RAISE NOTICE 'rappels de journée posés : %', v_n;
END;
$$;
COMMIT;
