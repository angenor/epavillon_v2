-- Régie de la CdP31 (02/11) : remise à zéro des essais du commanditaire avant
-- l'ouverture au public. Trois activités démarrées puis terminées reviennent
-- « programmées » sans heures de direct ; le journal de la régie et les trois
-- messages d'essai sont effacés. Les inscriptions du compte d'essai et les
-- réunions Zoom restent (une réunion effacée ici resterait orpheline chez Zoom).
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-regie-remise-a-zero', true);

DO $$
DECLARE
    v_seances uuid[] := ARRAY['01a19970-342c-73a9-a89c-43a5d4712e7c',
                              '01a17da1-9bea-767f-92bc-b063c6ad6146',
                              '01a198d3-12c9-7050-b9b9-299ff4b827fd']::uuid[];
    v_messages uuid[] := ARRAY['01a19a31-7564-71c1-bc37-d464477e1b6e',
                               '01a19a89-4c37-71b3-a0d8-a8de59ff8dc9',
                               '01a19a89-8a3f-721e-8b48-c5db03f426f9']::uuid[];
    v_n int;
BEGIN
    SELECT count(*) INTO v_n FROM programme.sessions
     WHERE id = ANY (v_seances) AND status = 'completed' AND live_started_at IS NOT NULL;
    IF v_n <> 3 THEN
        RAISE EXCEPTION 'Garde : 3 activités terminées par la régie attendues, % trouvées', v_n;
    END IF;

    UPDATE programme.sessions SET status = 'scheduled' WHERE id = ANY (v_seances);
    UPDATE programme.sessions SET live_started_at = NULL, live_ended_at = NULL WHERE id = ANY (v_seances);

    DELETE FROM programme.control_room_actions WHERE session_id = ANY (v_seances);
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 6 THEN
        RAISE EXCEPTION 'Garde : 6 gestes de régie attendus, % effacés', v_n;
    END IF;

    DELETE FROM live.incidents WHERE id = ANY (v_messages);
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 3 THEN
        RAISE EXCEPTION 'Garde : 3 messages d''essai attendus, % effacés', v_n;
    END IF;
END;
$$;
COMMIT;
