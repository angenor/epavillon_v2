-- Rattachements des déposants à l'organisation de leur activité (04/11/2026),
-- à la demande du commanditaire, après le courriel d'Anne-Sophie Lapointe : son
-- activité CdP31 n'apparaissait pas dans son espace. La reprise v1 a importé ces
-- trois dossiers retenus avec leur organisation, sans rattacher le déposant à
-- celle-ci (dans la v1, changer le nom de l'organisateur créait une autre
-- organisation) ; l'espace organisation ne montre que les activités des
-- organisations de la personne.
--   CDP31-00235 Anne-Sophie Lapointe -> Gouvernement du Québec (gestionnaire : aucun membre)
--   CDP31-00172 Saliou Diallo        -> Irokko (gestionnaire : aucun membre)
--   CDP31-00255 Borbála Bretus       -> Commission de l'Union Africaine (membre : un
--                                       gestionnaire existe déjà)
-- Fonction « Non renseignée », comme la reprise. Adhésions actives approuvées
-- par l'IFDD ; le rôle suit (tg_memberships_sync_role). Aucun courriel.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-04-rattachements-deposants-cdp31', true);

DO $$
DECLARE
    v_n int;
BEGIN
    INSERT INTO org.memberships (organization_id, person_id, role, status, job_title, approved_by, approved_at)
    SELECT p.organization_id, p.submitted_by, r.role::org.membership_role, 'active', 'Non renseignée',
           '01a0627c-f3be-7185-89c8-9a4015ad158e', now()
      FROM (VALUES ('CDP31-00235', 'manager'), ('CDP31-00172', 'manager'), ('CDP31-00255', 'member')) AS r(code, role)
      JOIN programme.proposals p ON p.reference_code = r.code AND p.status = 'accepted'
     WHERE NOT EXISTS (SELECT 1 FROM org.memberships m
                        WHERE m.organization_id = p.organization_id AND m.person_id = p.submitted_by);
    GET DIAGNOSTICS v_n = ROW_COUNT;
    IF v_n <> 3 THEN RAISE EXCEPTION 'Garde : 3 rattachements attendus, % créés', v_n; END IF;
END;
$$;
COMMIT;
