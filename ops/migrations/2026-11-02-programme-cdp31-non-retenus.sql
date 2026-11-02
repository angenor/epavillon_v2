-- Programme de la CdP31 (02/11) : cinq activités retirées du Stand par le
-- commanditaire ne sont pas retenues. Retirer de la grille n'ôte que la salle :
-- la séance gardait son heure, son dossier « retenu » et son direct, d'où deux
-- directs simultanés qui bloquaient la publication.
--   1. Nouvelle transition accepted -> under_review (docs/database/070 § 1),
--      motif exigé comme pour rejected -> under_review.
--   2. Les cinq dossiers repassent « en évaluation » (défait, pour eux,
--      2026-11-01-programme-cdp31-retenus). Aucun abonné à programme.proposal.* :
--      aucun avis ne part.
-- Le direct de ces séances et de « test 2 » a été coupé avant ce script
-- (request_id commanditaire:2026-11-02-sans-direct-hors-stand).
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp31-non-retenus', true);

INSERT INTO programme.proposal_transitions_allowed
    (from_status, to_status, required_permission, allowed_for_owner, requires_reason)
VALUES ('accepted', 'under_review', 'programme.proposal.decide', false, true)
ON CONFLICT DO NOTHING;

DO $$
DECLARE
    v_codes text[] := ARRAY['CDP31-00166','CDP31-00144','CDP31-00183','CDP31-00204','CDP31-00185'];
    v_n int;
BEGIN
    SELECT count(*) INTO v_n
      FROM programme.proposals p
      JOIN programme.sessions s ON s.proposal_id = p.id
     WHERE p.reference_code = ANY (v_codes) AND p.status = 'accepted' AND p.deleted_at IS NULL
       AND s.room_id IS NULL AND NOT s.is_streamed;
    IF v_n <> 5 THEN
        RAISE EXCEPTION 'Garde : 5 dossiers retenus, séance sans salle ni direct, attendus ; % trouvés', v_n;
    END IF;

    UPDATE programme.proposals
       SET status = 'under_review',
           decision_reason = 'Retirée du Stand par l''IFDD avant l''annonce du programme.',
           decided_at = NULL, decided_by = NULL
     WHERE reference_code = ANY (v_codes) AND status = 'accepted' AND deleted_at IS NULL;
END;
$$;
COMMIT;
