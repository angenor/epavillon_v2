-- Audit des adhésions : l'équipe accepte une demande et change un rôle depuis le
-- Modèle : docs/database/040_organizations.sql (org.memberships).
-- back-office (31/10). Rejouable.
DROP TRIGGER IF EXISTS tg_memberships_audit ON org.memberships;
CREATE TRIGGER tg_memberships_audit
    AFTER INSERT OR UPDATE OR DELETE ON org.memberships
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
