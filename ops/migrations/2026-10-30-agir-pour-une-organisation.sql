-- Le super-administrateur agit dans l'espace d'un dossier d'organisation (30/10). Rejouable.
-- Modèle : docs/database/030_identity.sql (identity.permissions).
--
-- Additive : une permission, donnée à super_admin par tg_permissions_grant_super_admin.
-- À jouer en production avant de démarrer l'API qui la lit.
BEGIN;

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('programme.proposal.act_for_organization', '{"fr":"Agir dans l''espace d''une organisation","en":"Act in an organisation''s space"}', 'programme')
ON CONFLICT (code) DO NOTHING;

-- Le déclencheur ne joue qu'à l'insertion : rejouée, la migration rattrape une base où il manquerait.
INSERT INTO identity.role_permissions (role_code, permission_code)
VALUES ('super_admin', 'programme.proposal.act_for_organization')
ON CONFLICT DO NOTHING;

COMMIT;
