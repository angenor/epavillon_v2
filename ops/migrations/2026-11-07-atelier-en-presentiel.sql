-- Une journée peut se tenir en présentiel seulement (07/11/2026), à la demande
-- du commanditaire : l'atelier préparatoire de la CdP31, le 08/11, ne sera pas
-- diffusé en ligne. Modèle : 060 § « DIRECT DE LA JOURNÉE ».
--   1. event.programme_tracks.is_streamed (vrai par défaut : rien ne change
--      pour les autres journées) ;
--   2. l'atelier l'éteint, ses activités aussi (leur page ne promet plus de
--      direct) ;
--   3. sa réunion Zoom est annulée ici même, comme le fait
--      live::repo::meetings::annuler_reunion_journee : le travail
--      live.cancel_meeting, déjà connu du worker, la retire chez Zoom.
-- Les inscriptions à la journée restent : on leur écrit ensuite.
-- Rejouable. À jouer AVANT l'API, qui lit la colonne ; le rejouer après la mise
-- en ligne annule une réunion qu'une inscription aurait recréée entre-temps.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-07-atelier-en-presentiel', true);

ALTER TABLE event.programme_tracks
    ADD COLUMN IF NOT EXISTS is_streamed boolean NOT NULL DEFAULT true;

COMMENT ON COLUMN event.programme_tracks.is_streamed IS
    'La journée se suit en ligne (un direct, une inscription). Faux : présentiel seulement, ni inscription ni réunion.';

DO $$
DECLARE
    v_atelier uuid;
    v_reunion record;
    v_n int;
BEGIN
    SELECT id INTO v_atelier FROM event.programme_tracks
     WHERE event_id = '01a17da1-9b46-75b2-895a-d9e226226be2' AND slug = 'atelier-preparatoire';
    IF v_atelier IS NULL THEN
        RAISE EXCEPTION 'Garde : atelier préparatoire de la CdP31 introuvable';
    END IF;

    UPDATE event.programme_tracks SET is_streamed = false WHERE id = v_atelier;

    UPDATE programme.sessions s SET is_streamed = false
      FROM programme.session_tracks st
     WHERE st.session_id = s.id AND st.track_id = v_atelier AND s.is_streamed;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    RAISE NOTICE 'Activités de l''atelier passées sans diffusion : %', v_n;

    FOR v_reunion IN
        UPDATE live.meetings SET status = 'cancelled', sync_status = 'not_required'
         WHERE track_id = v_atelier AND status NOT IN ('cancelled', 'failed')
        RETURNING id, external_id
    LOOP
        RAISE NOTICE 'Réunion de la journée annulée : % (Zoom : %)', v_reunion.id,
            COALESCE(v_reunion.external_id, 'pas encore créée');
        IF v_reunion.external_id IS NOT NULL THEN
            INSERT INTO platform.jobs (queue, task, payload, idempotency_key)
            VALUES ('live', 'live.cancel_meeting', jsonb_build_object('meeting_id', v_reunion.id),
                    v_reunion.id || ':annulation')
            ON CONFLICT DO NOTHING;
        END IF;
    END LOOP;

    SELECT count(*) INTO v_n FROM programme.track_registrations
     WHERE track_id = v_atelier AND status <> 'cancelled';
    RAISE NOTICE 'Inscriptions à l''atelier gardées : %', v_n;
END;
$$;

COMMIT;
