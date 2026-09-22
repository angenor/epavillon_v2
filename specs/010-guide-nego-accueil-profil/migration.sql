-- =============================================================================
-- ePavillon v2 — migration 0c : thématiques suivies (Guide Négo)
--
-- Du schéma en service après 0b vers celui de docs/database/ après 0c.
--
-- POURQUOI CE FICHIER EXISTE
--   `docs/database/` n'est chargé qu'à la création du volume. On ne recharge
--   pas une base qui porte des comptes : on la migre, selon le § 13 de
--   docs/DEPLOIEMENT.md. Il vaut AUSSI en local — la base de développement ne
--   se détruit pas, on la sauvegarde, on joue ce script, et SQLx compile
--   contre elle. **`down -v` n'est jamais la procédure.**
--
-- REJOUABLE SANS DÉGÂT
--   Chaque objet est créé sous condition. Le rejouer ne duplique rien, ne perd
--   rien, et ne fait échouer aucune étape déjà passée.
--
-- ORDRE
--   1. reference : le vocabulaire des thématiques de négociation et ses dix termes
--   2. negotiation : la table des suivis, ses index, ses triggers
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/. Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. reference — les thématiques de négociation sont un vocabulaire à elles
--
-- Ce qu'une personne SUIT en salle, pas ce que le Pavillon programme :
-- `activity_theme` classe les activités du site, `negotiation_theme` dit ce
-- qu'on montre en premier à une négociatrice. Cinq codes coexistent avec un
-- homonyme sous `activity_theme` — la clé est (taxonomy_code, code), et c'est
-- voulu. is_system : aucun écran ne le modifie.
-- -----------------------------------------------------------------------------

INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system)
VALUES ('negotiation_theme',
        '{"fr":"Thématiques de négociation","en":"Negotiation themes"}'::jsonb,
        '{"fr":"Filières suivies par une négociatrice ou un négociateur : adaptation, finance, genre…","en":"Tracks followed by a negotiator: adaptation, finance, gender…"}'::jsonb,
        true, false, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('negotiation_theme', 'adaptation',      '{"fr":"Adaptation","en":"Adaptation"}', 10),
    ('negotiation_theme', 'mitigation',      '{"fr":"Atténuation","en":"Mitigation"}', 20),
    ('negotiation_theme', 'finance',         '{"fr":"Finance","en":"Finance"}', 30),
    ('negotiation_theme', 'loss_and_damage', '{"fr":"Pertes et préjudices","en":"Loss and damage"}', 40),
    ('negotiation_theme', 'article_6',       '{"fr":"Article 6","en":"Article 6"}', 50),
    ('negotiation_theme', 'transparency',    '{"fr":"Transparence","en":"Transparency"}', 60),
    ('negotiation_theme', 'gender',          '{"fr":"Genre","en":"Gender"}', 70),
    ('negotiation_theme', 'just_transition', '{"fr":"Transition juste","en":"Just transition"}', 80),
    ('negotiation_theme', 'agriculture',     '{"fr":"Agriculture","en":"Agriculture"}', 90),
    ('negotiation_theme', 'technology',      '{"fr":"Technologie","en":"Technology"}', 100)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- LE BLOC QUI SUIT EST LE MÊME TEXTE que le § 2 bis.3 bis de
-- docs/database/100_negotiations.sql, à l'idempotence près : IF NOT EXISTS et
-- DROP TRIGGER IF EXISTS. C'est ce qui rend la comparaison de schémas du § 13
-- exploitable — un mot qui diffère dans un COMMENT ressortirait comme un écart.

-- -----------------------------------------------------------------------------
-- 2. negotiation — les thématiques suivies
--
-- Une préférence de lecture, choisie et défaite librement depuis l'application :
-- elle n'ouvre aucun droit, elle commande ce qu'on montre en premier et ce dont
-- on avertit. Ce n'est PAS une compétence attestée — celle-là vit dans
-- identity.negotiator_profiles, renseignée par un administrateur.
--
-- Même patron que network_memberships : un suivi se ferme (left_at), il ne se
-- supprime pas, et l'index unique ne porte que sur le suivi vivant.
-- -----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS negotiation.theme_subscriptions (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_theme_subscriptions_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    theme_term_id   uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    followed_at     timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    CONSTRAINT ck_theme_subscriptions_period CHECK (left_at IS NULL OR left_at >= followed_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_theme_subscriptions_active
    ON negotiation.theme_subscriptions (person_id, theme_term_id) WHERE left_at IS NULL;
CREATE INDEX IF NOT EXISTS ix_theme_subscriptions_theme
    ON negotiation.theme_subscriptions (theme_term_id, followed_at DESC) WHERE left_at IS NULL;

DROP TRIGGER IF EXISTS tg_theme_subscriptions_audit ON negotiation.theme_subscriptions;
CREATE TRIGGER tg_theme_subscriptions_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.theme_subscriptions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
DROP TRIGGER IF EXISTS tg_theme_subscriptions_check_theme ON negotiation.theme_subscriptions;
CREATE TRIGGER tg_theme_subscriptions_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.theme_subscriptions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

COMMENT ON TABLE negotiation.theme_subscriptions IS
    'Thématiques de négociation qu''une personne choisit de suivre. N''ouvre aucun droit : commande ce qu''on lui montre en premier et ce dont on l''avertit. Choix révocable, jamais une compétence attestée — celle-là vit dans identity.negotiator_profiles.';
COMMENT ON COLUMN negotiation.theme_subscriptions.left_at IS
    'Un suivi se ferme, il ne se supprime pas : ce qu''une personne a suivi pendant une COP reste lisible, et l''index unique ne porte que sur le suivi vivant.';

COMMIT;

-- -----------------------------------------------------------------------------
-- La réponse de rotation perdue (reprise 2, 22/09) — ADR-020
--
-- L'ancien jeton présenté dans la minute qui suit sa rotation, tant que sa
-- remplaçante n'a jamais été renouvelée, n'est pas un vol : c'est une réponse
-- perdue au retour. Pour retrouver la remplaçante sans deviner, la ligne
-- remplacée la nomme.
-- -----------------------------------------------------------------------------

ALTER TABLE identity.sessions
    ADD COLUMN IF NOT EXISTS replaced_by uuid REFERENCES identity.sessions(id) ON DELETE SET NULL;

COMMENT ON COLUMN identity.sessions.replaced_by IS
    'La session qui a remplacé celle-ci à sa rotation — la remplaçante VIVANTE, tenue à jour. C''est ce qui distingue une réponse de rotation perdue d''un vol : l''ancien jeton, présenté dans la minute qui suit sa rotation (AUTH_REFRESH_GRACE) et tant que la remplaçante n''a jamais été renouvelée, révoque celle-ci (motif « response_lost ») et en ouvre une neuve, une seule vivante. Hors de ces bornes, tout est coupé (« reuse_detected »). ADR-020 de Guide Négo, qui nuance R3 de specs/001.';
