-- CdP30 et CdP31 (02/11/2026), à la demande du commanditaire : tout dossier
-- encore en évaluation ou déposé devient « non retenu ». Les déposants de la
-- CdP31 ont reçu le courriel « pas de créneau à ce stade » le même jour.
-- Le graphe n'autorise pas submitted -> rejected : passage par under_review,
-- comme au back-office. Aucun courriel ne part à la décision.
-- Les six séances liées (CdP31, planifiées, jamais publiées, sans inscrit)
-- restent en l'état : invisibles du site public.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp30-cdp31-non-retenus', true);

DO $$
DECLARE
    v_editions uuid[] := ARRAY['01a17da1-9b45-78c6-a281-ec35bfeeae2a',   -- CdP30
                               '01a17da1-9b46-75b2-895a-d9e226226be2']::uuid[]; -- CdP31
    v_n int;
BEGIN
    UPDATE programme.proposals SET status = 'under_review'
     WHERE event_id = ANY (v_editions) AND deleted_at IS NULL AND status = 'submitted';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 8 THEN RAISE EXCEPTION 'Garde : 8 dossiers déposés attendus, % touchés', v_n; END IF;

    UPDATE programme.proposals
       SET status = 'rejected', decision_reason = 'Non retenu le 02/11 : faute de créneau.'
     WHERE event_id = ANY (v_editions) AND deleted_at IS NULL AND status = 'under_review';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 109 THEN RAISE EXCEPTION 'Garde : 109 dossiers à refuser attendus, % touchés', v_n; END IF;
END;
$$;
COMMIT;
