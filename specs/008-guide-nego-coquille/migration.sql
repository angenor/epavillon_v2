-- =============================================================================
-- ePavillon v2 — migration 0a : la coquille (Guide Négo)
--
-- Du schéma en service AVANT Guide Négo vers celui de docs/database/ après 0a.
--
-- POURQUOI CE FICHIER EXISTE
--   0a ne change aucun objet du schéma : il sème une ligne, le drapeau
--   `guide_nego.enabled`, dans `900_seed.sql`. Ce fichier n'est chargé qu'à la
--   création du volume ; une base en service ne reçoit donc jamais la ligne.
--   En local, elle a été posée à la main (quickstart de 0a). En production, elle
--   manque : `GET /platform/feature-flags` ne la rend pas, et la bascule du § 14
--   de docs/DEPLOIEMENT.md y répondrait « UPDATE 0 » — l'application resterait
--   fermée sans que rien ne le dise.
--
--   Le contrôle du § 13 ne l'aurait pas vu : il compare des schémas, pas des
--   lignes semées.
--
-- REJOUABLE SANS DÉGÂT
--   `ON CONFLICT DO NOTHING` : sur une base qui porte déjà la ligne, allumée ou
--   non, rien ne change. Un drapeau ouvert n'est jamais refermé par ce script.
--
-- ORDRE
--   Avant 009 (0b) et 010 (0c). Aucune dépendance technique, mais c'est l'ordre
--   des étapes, et celui du § 15 de docs/DEPLOIEMENT.md.
-- =============================================================================

BEGIN;

-- Éteint, comme dans le semis : on l'ouvre par la bascule du § 14, jamais ici.
INSERT INTO platform.feature_flags (key, description, is_enabled, rollout_percent) VALUES
    ('guide_nego.enabled',
     'Guide Négo : l''application mobile entière. Elle s''ouvre sans compte : allumer = is_enabled ET rollout_percent = 100.',
     false, 0)
ON CONFLICT (key) DO NOTHING;

COMMIT;


-- =============================================================================
-- APRÈS : la ligne existe, une seule fois.
--
--   SELECT key, is_enabled, rollout_percent
--     FROM platform.feature_flags WHERE key = 'guide_nego.enabled';
--
-- Attendu : une ligne. `false, 0` sur une base neuve de Guide Négo ; l'état
-- déjà posé sinon.
-- =============================================================================
