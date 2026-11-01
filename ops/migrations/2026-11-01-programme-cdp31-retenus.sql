-- Programme de la CdP31 (01/11) : les activités programmées dont le dossier
-- était resté « en évaluation » passent à « retenu », comme la décision du
-- back-office. Leur séance existe déjà : rien n'est créé. Aucun avis ne part.
-- « test 2 » (CDP31-00120, IFDD) reste en évaluation, à la demande du commanditaire.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-01-programme-cdp31-retenus', true);

DO $$
DECLARE
    v_codes text[] := ARRAY['CDP31-00166','CDP31-00144','CDP31-00160','CDP31-00182','CDP31-00240',
                            'CDP31-00183','CDP31-00204','CDP31-00185','CDP31-00194'];
    v_n int;
BEGIN
    SELECT count(*) INTO v_n
      FROM programme.proposals p
     WHERE p.reference_code = ANY (v_codes) AND p.status = 'under_review' AND p.deleted_at IS NULL
       AND EXISTS (SELECT 1 FROM programme.sessions s WHERE s.proposal_id = p.id);
    IF v_n <> 9 THEN
        RAISE EXCEPTION 'Garde : 9 dossiers attendus en évaluation avec séance, % trouvés', v_n;
    END IF;

    UPDATE programme.proposals
       SET status = 'accepted', decision_reason = NULL
     WHERE reference_code = ANY (v_codes) AND status = 'under_review' AND deleted_at IS NULL;
END;
$$;
COMMIT;
