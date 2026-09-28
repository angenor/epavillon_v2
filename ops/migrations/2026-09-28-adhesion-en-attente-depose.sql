-- Rejoindre une organisation ne bloque plus le dépôt : une demande en attente donne
-- `org_applicant`, qui ne porte que le dépôt. Modèle : docs/database/030_identity.sql,
-- docs/database/040_organizations.sql.
BEGIN;
INSERT INTO identity.roles (code, label, description, allowed_scopes, is_system) VALUES
    ('org_applicant', '{"fr":"Membre en attente de validation","en":"Member awaiting approval"}', '{"fr":"Dépose et suit ses propres dossiers en attendant l''accord d''un référent","en":"Submits and tracks their own proposals until a manager approves"}', '{organization}', false)
ON CONFLICT (code) DO NOTHING;
INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('org_applicant', 'programme.proposal.submit')
ON CONFLICT DO NOTHING;
CREATE OR REPLACE FUNCTION org.tg_sync_membership_role()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_role text;
BEGIN
    v_role := CASE
        WHEN NEW.status = 'active' AND NEW.role = 'manager' THEN 'org_manager'
        WHEN NEW.status = 'active' THEN 'org_member'
        WHEN NEW.status = 'pending' AND NEW.invited_at IS NULL THEN 'org_applicant'
    END;

    -- Un changement d'état ou de rôle retire l'ancien : l'attribution est
    -- révoquée, jamais supprimée — elle dit à quel titre la personne a siégé.
    UPDATE identity.role_assignments
       SET revoked_at = now(),
           revoked_reason = 'Adhésion à l''organisation close ou modifiée'
     WHERE person_id = NEW.person_id
       AND scope_type = 'organization'
       AND scope_id = NEW.organization_id
       AND role_code IN ('org_manager', 'org_member', 'org_applicant')
       AND revoked_at IS NULL
       AND role_code IS DISTINCT FROM v_role;

    IF v_role IS NOT NULL THEN
        INSERT INTO identity.role_assignments
            (person_id, role_code, scope_type, scope_id, granted_by, note)
        VALUES (NEW.person_id, v_role, 'organization', NEW.organization_id,
                COALESCE(NEW.approved_by, NEW.invited_by),
                'Attribué avec l''adhésion à l''organisation')
        ON CONFLICT DO NOTHING;
    END IF;

    RETURN NULL;
END;
$$;
-- Les demandes déjà en attente reçoivent le rôle, par le déclencheur.
UPDATE org.memberships SET status = status WHERE status = 'pending' AND invited_at IS NULL;
COMMIT;
