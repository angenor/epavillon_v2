-- Programme de la CdP31 (02/11) : les deux activités créées au nom de l'OIF le
-- 01/11 (CDP31-00259 « Atelier de fixation des objectifs… », CDP31-00260
-- « Concertation francophone… ») sont portées par l'IFDD. Le déposant reste
-- angenor99@gmail.com, déjà référent de l'IFDD. Les déclencheurs de
-- synchronisation du porteur recopient le changement dans
-- proposal_organizations et session_organizations.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-programme-cdp31-oif-vers-ifdd', true);

DO $$
DECLARE
    v_oif  uuid := '01a18dc9-5da9-7bd9-9a7c-46022b3e602c';
    v_ifdd uuid := '01a06241-e7fa-78de-af7c-101f2273be63';
    v_n int;
BEGIN
    UPDATE programme.proposals
       SET organization_id = v_ifdd
     WHERE reference_code IN ('CDP31-00259', 'CDP31-00260') AND organization_id = v_oif
       AND submitted_by = '01a0627c-f3be-7185-89c8-9a4015ad158e';
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 2 THEN
        RAISE EXCEPTION 'Garde : 2 dossiers OIF attendus, % touchés', v_n;
    END IF;

    UPDATE programme.sessions s
       SET organization_id = v_ifdd
      FROM programme.proposals p
     WHERE p.id = s.proposal_id AND p.reference_code IN ('CDP31-00259', 'CDP31-00260')
       AND s.organization_id = v_oif;
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 2 THEN
        RAISE EXCEPTION 'Garde : 2 séances OIF attendues, % touchées', v_n;
    END IF;
END;
$$;
COMMIT;
