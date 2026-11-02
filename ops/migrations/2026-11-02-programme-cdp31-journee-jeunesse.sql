-- Journée jeunesse de la CdP31, 14/11 (02/11/2026), à la demande du commanditaire.
--   1. Nouvelle transition cancelled -> rejected, motif exigé (docs/database/070 § 1).
--   2. CDP31-00134 (Youth Center for Impact), annulé, devient « non retenu » ;
--      sa séance annulée quitte le site public.
--   3. CDP31-00240 « Intelligence artificielle et environnement… » (IFDD),
--      retirée du calendrier ce jour, revient : retenue, 09 h 50 à 10 h 20
--      (Europe/Istanbul), publiée, diffusée, dans la partie « S'inspirer » de
--      la journée jeunesse.
-- Aucun inscrit sur ces deux séances ; aucun courriel n'est lié à ces statuts.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp31-journee-jeunesse', true);

INSERT INTO programme.proposal_transitions_allowed
    (from_status, to_status, required_permission, allowed_for_owner, requires_reason)
VALUES ('cancelled', 'rejected', 'programme.proposal.decide', false, true)
ON CONFLICT DO NOTHING;

DO $$
DECLARE
    v_track   uuid := '01a18dc9-5da9-7b2a-a15a-0b6917f228ec';
    v_inspirer uuid := '01a18dc9-5da9-7100-94f8-3583ed99a30b';
    v_ia      uuid := '01a17da1-9bea-7da2-af56-7c4e6a731f1c';
    v_yci     uuid := '01a17da1-9bea-736e-8afe-514fd510dffb';
    v_canal   uuid := '01a06241-e7f7-7fd8-ae70-7e5b4565f458';
    v_n int;
BEGIN
    -- 2. Youth Center for Impact
    UPDATE programme.proposals
       SET status = 'rejected',
           decision_reason = 'Annulé par erreur le 02/11 : non retenu, faute de créneau.'
     WHERE reference_code = 'CDP31-00134' AND status = 'cancelled';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : CDP31-00134 annulé attendu, % touché', v_n; END IF;

    UPDATE programme.sessions
       SET published_at = NULL, is_streamed = false
     WHERE id = v_yci AND status = 'cancelled'
       AND NOT EXISTS (SELECT 1 FROM programme.registrations r
                        WHERE r.session_id = v_yci AND r.status <> 'cancelled');
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance annulée sans inscrit attendue, % touchée', v_n; END IF;

    -- 3. Atelier IA de l'IFDD
    UPDATE programme.proposals
       SET status = 'accepted', decision_reason = NULL
     WHERE reference_code = 'CDP31-00240' AND status = 'under_review';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : CDP31-00240 en évaluation attendu, % touché', v_n; END IF;

    UPDATE programme.sessions
       SET starts_at = timestamp '2026-11-14 09:50' AT TIME ZONE 'Europe/Istanbul',
           ends_at   = timestamp '2026-11-14 10:20' AT TIME ZONE 'Europe/Istanbul',
           status = 'scheduled', published_at = now(),
           is_streamed = true, broadcast_channel_id = v_canal
     WHERE id = v_ia AND status = 'planned' AND room_id IS NOT NULL;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance IA planifiée avec salle attendue, % touchée', v_n; END IF;

    UPDATE programme.session_tracks
       SET part_id = v_inspirer, is_highlight = false
     WHERE session_id = v_ia AND track_id = v_track;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : séance IA rattachée à la journée jeunesse attendue, % touchée', v_n; END IF;
END;
$$;
COMMIT;
