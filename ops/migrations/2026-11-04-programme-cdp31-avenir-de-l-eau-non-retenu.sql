-- Programme de la CdP31 (04/11/2026), à la demande du commanditaire :
-- « L'avenir de l'Eau : Alliances entre peuples autochtones et communautés
-- locales… » (CDP31-00194), retenue et publiée le 20/11 de 13 h à 14 h au Stand,
-- n'est plus retenue. La séance quitte le site public (planifiée, non publiée),
-- son direct est coupé et sa salle libérée ; le dossier passe par « en
-- évaluation » (aucune transition directe accepted -> rejected), puis « non
-- retenu ». Ni inscrit, ni réunion Zoom, ni rappel : aucun courriel.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-04-programme-cdp31-avenir-de-l-eau-non-retenu', true);

DO $$
DECLARE
    v_seance uuid := '01a17da1-9bea-74d0-aa68-40e66288c337';
    v_n int;
BEGIN
    SELECT count(*) INTO v_n
      FROM programme.sessions s
      JOIN programme.proposals p ON p.id = s.proposal_id
     WHERE s.id = v_seance AND p.reference_code = 'CDP31-00194' AND p.status = 'accepted'
       AND NOT EXISTS (SELECT 1 FROM programme.registrations r WHERE r.session_id = s.id AND r.status <> 'cancelled')
       AND NOT EXISTS (SELECT 1 FROM live.meetings m WHERE m.session_id = s.id);
    IF v_n <> 1 THEN
        RAISE EXCEPTION 'Garde : séance CDP31-00194 retenue, sans inscrit ni réunion, attendue ; % trouvée', v_n;
    END IF;

    UPDATE programme.sessions
       SET status = 'planned', published_at = NULL, is_streamed = false, room_id = NULL
     WHERE id = v_seance;

    UPDATE programme.proposals
       SET status = 'under_review',
           decision_reason = 'Retirée du programme par l''IFDD.',
           decided_at = NULL, decided_by = NULL
     WHERE reference_code = 'CDP31-00194' AND status = 'accepted';

    UPDATE programme.proposals
       SET status = 'rejected',
           decision_reason = 'Non retenue par l''IFDD le 04/11.'
     WHERE reference_code = 'CDP31-00194' AND status = 'under_review';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 1 THEN RAISE EXCEPTION 'Garde : CDP31-00194 en évaluation attendu, % touché', v_n; END IF;
END;
$$;
COMMIT;
