-- Adhésions (02/11) : deux personnes reprises de la v1 comme membres de l'IFDD
-- déposent pour une autre organisation et voyaient l'espace de l'IFDD et ses
-- dossiers. Accord du commanditaire : l'adhésion IFDD est révoquée, une adhésion
-- active est posée dans la bonne organisation, aucun avis ne part.
--   Wafiiqah Youssouf Mbechezi : WECT, référente (seule personne de WECT).
--   Aline Dequidt : FDC (Fresque du Climat), membre.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-02-adhesions-ifdd-corrigees', true);

DO $$
DECLARE
    v_ifdd uuid := '01a06241-e7fa-78de-af7c-101f2273be63';
    v_cas record;
    v_personne uuid;
    v_fonction text;
BEGIN
    FOR v_cas IN
        SELECT * FROM (VALUES
            ('wafiiqahyoussouf@gmail.com', '01a17da1-98f6-759c-8baf-54034af1781f'::uuid, 'manager'::org.membership_role),
            ('deqaline@gmail.com',         '01a17da1-98f5-7712-a340-af5e75013741'::uuid, 'member'::org.membership_role)
        ) AS t(courriel, organisation, role)
    LOOP
        SELECT p.id INTO v_personne FROM identity.people p WHERE p.primary_email = v_cas.courriel;
        SELECT m.job_title INTO v_fonction FROM org.memberships m
         WHERE m.person_id = v_personne AND m.organization_id = v_ifdd AND m.status = 'active';
        IF v_personne IS NULL OR NOT FOUND THEN
            RAISE EXCEPTION 'Garde : adhésion IFDD active introuvable pour %', v_cas.courriel;
        END IF;

        UPDATE org.memberships
           SET status = 'revoked', is_primary = false, revoked_at = now()
         WHERE person_id = v_personne AND organization_id = v_ifdd;

        INSERT INTO org.memberships (organization_id, person_id, role, status, is_primary, job_title, approved_by, approved_at)
        VALUES (v_cas.organisation, v_personne, v_cas.role, 'active', true, COALESCE(v_fonction, 'Non renseignée'),
                '01a0627c-f3be-7185-89c8-9a4015ad158e', now())
        ON CONFLICT (organization_id, person_id) DO UPDATE
           SET status = 'active', role = EXCLUDED.role, is_primary = true, revoked_at = NULL,
               job_title = EXCLUDED.job_title, approved_by = EXCLUDED.approved_by, approved_at = now();
    END LOOP;
END;
$$;
COMMIT;
