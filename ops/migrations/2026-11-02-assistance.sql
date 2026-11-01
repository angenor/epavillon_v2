-- Assistance sur toute la plateforme (01/11). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql (programme.proposal_comments)
-- et docs/database/030_identity.sql (programme.support.answer).
--
-- Un sujet peut ne concerner aucun dossier : question sur la plateforme ou
-- suggestion, `proposal_id` nul, partagé entre son auteur et l'équipe. Aucune
-- reprise : les messages existants ont tous un dossier.

ALTER TABLE programme.proposal_comments ALTER COLUMN proposal_id DROP NOT NULL;

ALTER TABLE programme.proposal_comments DROP CONSTRAINT IF EXISTS ck_proposal_comments_platform;
ALTER TABLE programme.proposal_comments
    ADD CONSTRAINT ck_proposal_comments_platform
        CHECK (proposal_id IS NOT NULL OR (visibility = 'submitter' AND NOT is_change_request));

CREATE INDEX IF NOT EXISTS ix_proposal_comments_platform
    ON programme.proposal_comments (created_at DESC)
    WHERE proposal_id IS NULL AND deleted_at IS NULL;

CREATE OR REPLACE FUNCTION programme.tg_check_proposal_comment_parent()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM programme.proposal_comments p
                   WHERE p.id = NEW.parent_id AND p.proposal_id IS NOT DISTINCT FROM NEW.proposal_id) THEN
        RAISE EXCEPTION 'Une réponse se pose dans le fil de son message, sur le même dossier.'
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_proposal_comments_parent';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_proposal_comments_check_parent ON programme.proposal_comments;
CREATE TRIGGER tg_proposal_comments_check_parent
    BEFORE INSERT OR UPDATE OF parent_id, proposal_id ON programme.proposal_comments
    FOR EACH ROW WHEN (NEW.parent_id IS NOT NULL)
    EXECUTE FUNCTION programme.tg_check_proposal_comment_parent();

INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('platform_support_topic', '{"fr":"Natures d''une demande d''assistance","en":"Support request topics"}',
     '{"fr":"Ce dont parle un sujet d''assistance qui ne concerne aucun dossier","en":"What a support thread about no proposal is about"}',
     false, false, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('platform_support_topic', 'question',        '{"fr":"Question","en":"Question"}', 10),
    ('platform_support_topic', 'technical_issue', '{"fr":"Problème technique","en":"Technical issue"}', 20),
    ('platform_support_topic', 'account',         '{"fr":"Mon compte ou mon organisation","en":"My account or organisation"}', 30),
    ('platform_support_topic', 'suggestion',      '{"fr":"Suggestion","en":"Suggestion"}', 40)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

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
     WHERE t.id = NEW.topic_term_id
       AND t.taxonomy_code = CASE WHEN NEW.proposal_id IS NULL THEN 'platform_support_topic'
                                  ELSE 'proposal_comment_topic' END;
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

-- CE QUE L'ASSISTANCE MONTRE À UNE PERSONNE (01/11) : la règle en un seul
-- endroit, lue par la liste des sujets, son compteur de non-lus et la lecture.
-- Côté équipe (`p_team`) : les messages partagés des dossiers de son périmètre
-- (`p_events`, ou tous si `p_global`) et, en portée globale seulement, les
-- sujets sans dossier. Côté organisation : les messages partagés des dossiers
-- qu'elle peut suivre (adhésion active, ou demande en attente de la déposante,
-- comme le service `acces_au_dossier`) et ses propres sujets sans dossier.
-- `platform_author` : l'auteur du sujet sans dossier, nul sur un dossier ; il
-- dit qui, dans ce fil, parle pour l'équipe.
CREATE OR REPLACE FUNCTION programme.support_visible_comments(
    p_person uuid, p_team boolean, p_global boolean, p_events uuid[])
RETURNS TABLE (comment_id uuid, platform_author uuid)
LANGUAGE sql
STABLE
AS $$
    WITH RECURSIVE plateforme AS (
        SELECT c.id, c.author_id AS root_author FROM programme.proposal_comments c
         WHERE c.proposal_id IS NULL AND c.parent_id IS NULL AND c.deleted_at IS NULL
        UNION ALL
        SELECT c.id, p.root_author FROM programme.proposal_comments c
          JOIN plateforme p ON c.parent_id = p.id
         WHERE c.deleted_at IS NULL
    )
    SELECT c.id, NULL::uuid
      FROM programme.proposal_comments c
      JOIN programme.proposals d ON d.id = c.proposal_id AND d.deleted_at IS NULL
     WHERE c.deleted_at IS NULL AND c.visibility = 'submitter'
       AND CASE WHEN p_team THEN p_global OR d.event_id = ANY(p_events)
                ELSE EXISTS (SELECT 1 FROM org.memberships m
                              WHERE m.organization_id = d.organization_id AND m.person_id = p_person
                                AND (m.status = 'active'
                                     OR (d.submitted_by = p_person AND m.status = 'pending'
                                         AND m.invited_at IS NULL)))
           END
    UNION ALL
    SELECT p.id, p.root_author FROM plateforme p
     WHERE CASE WHEN p_team THEN p_global ELSE p.root_author = p_person END;
$$;

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('programme.support.answer', '{"fr":"Répondre à l''assistance","en":"Answer support requests"}', 'programme')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin', 'programme.support.answer')
ON CONFLICT DO NOTHING;
