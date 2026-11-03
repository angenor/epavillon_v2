-- Carte « En ce moment » du tableau de bord (03/11/2026). Rejouable.
-- Modèle : docs/database/030_identity.sql § permissions.
--
-- À JOUER AVANT DE DÉPLOYER L'API : `GET /admin/presence` exige cette
-- permission. Sans la ligne, la route refuse tout le monde (403), super
-- administrateurs compris, et la carte ne s'affiche à personne.
--
-- Additive, aucune donnée touchée :
--   - permission analytics.presence.read (portée globale), à aucun rôle ;
--   - le déclencheur tg_permissions_grant_super_admin la donne au rôle
--     super_admin à l'insertion ; la seconde instruction la rattache aussi si
--     la permission existait déjà sans ce lien. Les personnes qui ont le rôle
--     super_admin l'obtiennent par leur rôle, sans écriture par personne.
BEGIN;

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('analytics.presence.read',
     '{"fr":"Voir combien de personnes sont sur la plateforme en ce moment","en":"See how many people are on the platform right now"}',
     'analytics')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code)
VALUES ('super_admin', 'analytics.presence.read')
ON CONFLICT DO NOTHING;

COMMIT;
