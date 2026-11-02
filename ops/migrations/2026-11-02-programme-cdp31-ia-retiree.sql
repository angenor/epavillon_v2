-- Programme de la CdP31 (02/11) : « Intelligence artificielle et environnement,
-- de la promesse à la mise en œuvre » (CDP31-00240) retirée du calendrier par le
-- commanditaire, qui n'est pas sûr qu'elle soit validée. Retirer du calendrier
-- n'ôtait que la salle : la séance restait publiée, sans lieu, et bloquait la
-- publication. Elle quitte le site public (non publiée, planifiée), son direct
-- est coupé, son dossier repasse « en évaluation ». Sans inscrit : aucun avis.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp31-ia-retiree', true);

DO $$
DECLARE
    v_seance uuid := '01a17da1-9bea-7da2-af56-7c4e6a731f1c';
    v_n int;
BEGIN
    SELECT count(*) INTO v_n
      FROM programme.sessions s
      JOIN programme.proposals p ON p.id = s.proposal_id
     WHERE s.id = v_seance AND s.room_id IS NULL AND s.location_note IS NULL
       AND p.reference_code = 'CDP31-00240' AND p.status = 'accepted'
       AND NOT EXISTS (SELECT 1 FROM programme.registrations r WHERE r.session_id = s.id AND r.status <> 'cancelled');
    IF v_n <> 1 THEN
        RAISE EXCEPTION 'Garde : séance CDP31-00240 retenue, sans salle ni inscrit, attendue ; % trouvée', v_n;
    END IF;

    UPDATE programme.sessions
       SET status = 'planned', published_at = NULL, is_streamed = false
     WHERE id = v_seance;

    UPDATE programme.proposals
       SET status = 'under_review',
           decision_reason = 'Retirée du calendrier par l''IFDD, validation à confirmer.',
           decided_at = NULL, decided_by = NULL
     WHERE reference_code = 'CDP31-00240' AND status = 'accepted';
END;
$$;
COMMIT;
