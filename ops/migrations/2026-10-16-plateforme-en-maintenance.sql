-- La maintenance de toute la plateforme : le drapeau, éteint, et la permission de le
-- basculer depuis le back-office (admin en portée globale ; super_admin la reçoit par
-- le déclencheur de identity.permissions). Rejouable.
-- Modèle : docs/database/900_seed.sql § 2 et docs/database/030_identity.sql.
--
-- Sans back-office, en dernier recours :
--   fermer :  UPDATE platform.feature_flags SET is_enabled = true,  rollout_percent = 100 WHERE key = 'platform.maintenance';
--   rouvrir : UPDATE platform.feature_flags SET is_enabled = false, rollout_percent = 0   WHERE key = 'platform.maintenance';
BEGIN;
INSERT INTO platform.feature_flags (key, description, is_enabled, rollout_percent) VALUES
    ('platform.maintenance', 'Plateforme entière en maintenance : seuls les administrateurs y accèdent. Allumer = is_enabled ET rollout_percent = 100.', false, 0)
ON CONFLICT (key) DO NOTHING;
INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('identity.platform.maintenance', '{"fr":"Mettre la plateforme en maintenance","en":"Put the platform under maintenance"}', 'identity')
ON CONFLICT (code) DO NOTHING;
INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin', 'identity.platform.maintenance')
ON CONFLICT DO NOTHING;
COMMIT;
