-- Nature d'un échange sur un dossier (30/10). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql (programme.proposal_comments,
-- taxonomie `proposal_comment_topic`, tg_check_proposal_comment_topic).
--
-- Additive, aucune donnée touchée : les fils existants restent sans nature.
-- À jouer en production avant de démarrer l'API qui lit `topic_term_id`.
BEGIN;

ALTER TABLE programme.proposal_comments
    ADD COLUMN IF NOT EXISTS topic_term_id uuid REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    ADD COLUMN IF NOT EXISTS topic_other text;

ALTER TABLE programme.proposal_comments DROP CONSTRAINT IF EXISTS ck_proposal_comments_topic_on_root;
ALTER TABLE programme.proposal_comments ADD CONSTRAINT ck_proposal_comments_topic_on_root
    CHECK (parent_id IS NULL OR (topic_term_id IS NULL AND topic_other IS NULL));
ALTER TABLE programme.proposal_comments DROP CONSTRAINT IF EXISTS ck_proposal_comments_topic_other;
ALTER TABLE programme.proposal_comments ADD CONSTRAINT ck_proposal_comments_topic_other
    CHECK (topic_other IS NULL OR (topic_term_id IS NOT NULL AND length(btrim(topic_other)) BETWEEN 1 AND 80));

-- Nature d'un échange : vocabulaire ouvert, complété au back-office. Le code
-- `other` est le seul que le service lit : il appelle un intitulé libre.
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('proposal_comment_topic', '{"fr":"Natures d''un échange sur un dossier","en":"Proposal exchange topics"}',
     '{"fr":"Ce dont parle un fil ouvert dans les échanges d''un dossier","en":"What a thread in a proposal''s exchanges is about"}',
     false, false, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('proposal_comment_topic', 'question',        '{"fr":"Question","en":"Question"}', 10),
    ('proposal_comment_topic', 'technical_issue', '{"fr":"Problème technique","en":"Technical issue"}', 20),
    ('proposal_comment_topic', 'documents',       '{"fr":"Documents et pièces","en":"Documents"}', 30),
    ('proposal_comment_topic', 'speakers',        '{"fr":"Intervenants","en":"Speakers"}', 40),
    ('proposal_comment_topic', 'schedule',        '{"fr":"Créneau et horaire","en":"Slot and schedule"}', 50),
    ('proposal_comment_topic', 'comment',         '{"fr":"Commentaire","en":"Comment"}', 60),
    ('proposal_comment_topic', 'complaint',       '{"fr":"Réclamation","en":"Complaint"}', 70),
    ('proposal_comment_topic', 'other',           '{"fr":"Autre","en":"Other"}', 1000)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- Le terme relève de SA taxonomie, et « Autre » porte son intitulé : un CHECK
-- ne lit pas une autre table.
CREATE OR REPLACE FUNCTION programme.tg_check_proposal_comment_topic()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_code text;
BEGIN
    IF NEW.topic_term_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT t.code INTO v_code FROM reference.taxonomy_terms t
     WHERE t.id = NEW.topic_term_id AND t.taxonomy_code = 'proposal_comment_topic';
    IF v_code IS NULL THEN
        RAISE EXCEPTION 'Nature refusée : elle ne relève pas des natures d''échange.'
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_proposal_comments_topic_term';
    END IF;
    IF (v_code = 'other') <> (NEW.topic_other IS NOT NULL) THEN
        RAISE EXCEPTION 'Précisez la nature de l''échange quand vous choisissez « Autre », et seulement alors.'
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_proposal_comments_topic_other';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_proposal_comments_check_topic ON programme.proposal_comments;
CREATE TRIGGER tg_proposal_comments_check_topic
    BEFORE INSERT OR UPDATE OF topic_term_id, topic_other ON programme.proposal_comments
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_proposal_comment_topic();

COMMIT;
