-- Les échanges d'un dossier deviennent des sujets (01/11). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql (programme.proposal_comments).
--
-- Titre et état « résolu » du sujet sur le message racine ; une réponse le
-- rouvre. Les fils existants gardent un titre nul : l'écran les nomme par leur
-- nature. Aucune reprise.

ALTER TABLE programme.proposal_comments
    ADD COLUMN IF NOT EXISTS subject   text,
    ADD COLUMN IF NOT EXISTS closed_at timestamptz,
    ADD COLUMN IF NOT EXISTS closed_by uuid;

ALTER TABLE programme.proposal_comments
    DROP CONSTRAINT IF EXISTS xmod_fk_proposal_comments_closer,
    DROP CONSTRAINT IF EXISTS ck_proposal_comments_subject_on_root,
    DROP CONSTRAINT IF EXISTS ck_proposal_comments_subject_length;

ALTER TABLE programme.proposal_comments
    ADD CONSTRAINT xmod_fk_proposal_comments_closer
        FOREIGN KEY (closed_by) REFERENCES identity.people(id) ON DELETE SET NULL,
    ADD CONSTRAINT ck_proposal_comments_subject_on_root
        CHECK (parent_id IS NULL OR (subject IS NULL AND closed_at IS NULL AND closed_by IS NULL)),
    ADD CONSTRAINT ck_proposal_comments_subject_length
        CHECK (subject IS NULL OR length(btrim(subject)) BETWEEN 2 AND 120);

CREATE OR REPLACE FUNCTION programme.tg_reopen_proposal_comment_subject()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE programme.proposal_comments r
       SET closed_at = NULL, closed_by = NULL
     WHERE r.closed_at IS NOT NULL
       AND r.id = (
           WITH RECURSIVE fil AS (
               SELECT id, parent_id FROM programme.proposal_comments WHERE id = NEW.parent_id
               UNION ALL
               SELECT c.id, c.parent_id FROM programme.proposal_comments c JOIN fil f ON c.id = f.parent_id
           )
           SELECT id FROM fil WHERE parent_id IS NULL);
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_proposal_comments_reopen_subject ON programme.proposal_comments;
CREATE TRIGGER tg_proposal_comments_reopen_subject
    AFTER INSERT ON programme.proposal_comments
    FOR EACH ROW WHEN (NEW.parent_id IS NOT NULL)
    EXECUTE FUNCTION programme.tg_reopen_proposal_comment_subject();
