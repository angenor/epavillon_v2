-- Journée jeunesse de la CdP31 (02/11/2026), à la demande du commanditaire :
-- CDP31-00138 (Youth Voices Cameroon), annulé ce jour, devient « non retenu »
-- et sa séance annulée quitte le site public. Aucun inscrit, aucun courriel.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp31-yvc-non-retenu', true);

DO $$
DECLARE
    v_seance uuid := '01a17da1-9bea-7368-91c4-f7f5434ef814';
    v_n int;
BEGIN
    UPDATE programme.proposals
       SET status = 'rejected',
           decision_reason = 'Annulé le 02/11 : non retenu, faute de créneau.'
     WHERE reference_code = 'CDP31-00138' AND status = 'cancelled';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : CDP31-00138 annulé attendu, % touché', v_n; END IF;

    UPDATE programme.sessions
       SET published_at = NULL, is_streamed = false
     WHERE id = v_seance AND status = 'cancelled'
       AND NOT EXISTS (SELECT 1 FROM programme.registrations r
                        WHERE r.session_id = v_seance AND r.status <> 'cancelled');
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance annulée sans inscrit attendue, % touchée', v_n; END IF;
END;
$$;
COMMIT;
