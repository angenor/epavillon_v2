-- Destinataires d'un message du comité et lu / non lu par message (27/10). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql § 6, 110_engagement.sql
-- (catalogue des avis). Demande du commanditaire : la v1 permettait de choisir
-- les destinataires et marquait chaque message lu ; la v2 le reprend, en mieux.
--
-- Déployer l'API avec : elle écrit et lit ces tables, et annonce les messages
-- adressés (`programme.comment.addressed`).
BEGIN;

-- Destinataires d'un fil du comité (demande du commanditaire, 27/10 ; la v1 les
-- choisissait aussi). Sans ligne, un message `committee` est lu de tout le
-- comité ; avec, de son auteur et des personnes nommées seulement,
-- administrateurs compris : choisir ses destinataires n'a de sens que si le
-- choix est tenu. Posés sur le message qui ouvre le fil : les réponses restent
-- dans le cercle, et y ajouter quelqu'un lui ouvre tout le fil.
CREATE TABLE IF NOT EXISTS programme.proposal_comment_recipients (
    comment_id   uuid        NOT NULL REFERENCES programme.proposal_comments(id) ON DELETE CASCADE,
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_proposal_comment_recipients_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    added_by     uuid        CONSTRAINT xmod_fk_proposal_comment_recipients_adder
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    added_at     timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (comment_id, person_id)
);

CREATE INDEX IF NOT EXISTS ix_proposal_comment_recipients_person ON programme.proposal_comment_recipients (person_id);

CREATE OR REPLACE FUNCTION programme.tg_check_comment_recipient()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM programme.proposal_comments c
                   WHERE c.id = NEW.comment_id AND c.parent_id IS NULL AND c.visibility = 'committee') THEN
        RAISE EXCEPTION 'Seul un message du comité qui ouvre un fil a des destinataires.'
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_proposal_comment_recipients_root';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_proposal_comment_recipients_root ON programme.proposal_comment_recipients;
CREATE TRIGGER tg_proposal_comment_recipients_root
    BEFORE INSERT OR UPDATE ON programme.proposal_comment_recipients
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_comment_recipient();

-- Lu / non lu, message par message (demande du commanditaire, 27/10). Une ligne
-- par lecteur : « lu par 2 sur 3 » se compte, « non lu » est l'absence de
-- ligne, « marquer comme non lu » la retire. L'auteur n'en a pas : il a lu ce
-- qu'il a écrit.
CREATE TABLE IF NOT EXISTS programme.proposal_comment_reads (
    comment_id   uuid        NOT NULL REFERENCES programme.proposal_comments(id) ON DELETE CASCADE,
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_proposal_comment_reads_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    read_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (comment_id, person_id)
);

CREATE INDEX IF NOT EXISTS ix_proposal_comment_reads_person ON programme.proposal_comment_reads (person_id);

-- Qui, du côté du comité, peut lire ce message : la règle en un seul endroit.
-- Une note personnelle n'est qu'à son auteur ; un message partagé avec
-- l'organisation est lu de tout le comité ; un message du comité, du cercle de
-- son fil (tout le comité sans destinataires). Le fil se remonte jusqu'à sa
-- racine, `parent_id` pouvant désigner une réponse.
CREATE OR REPLACE FUNCTION programme.comment_in_committee_circle(p_comment_id uuid, p_person_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    WITH RECURSIVE fil AS (
        SELECT id, parent_id FROM programme.proposal_comments WHERE id = p_comment_id
        UNION ALL
        SELECT c.id, c.parent_id FROM programme.proposal_comments c JOIN fil f ON c.id = f.parent_id
    ),
    racine AS (
        SELECT r.id, r.author_id FROM fil JOIN programme.proposal_comments r ON r.id = fil.id
        WHERE fil.parent_id IS NULL
    )
    SELECT CASE m.visibility
               WHEN 'private' THEN m.author_id = p_person_id
               WHEN 'submitter' THEN true
               ELSE NOT EXISTS (SELECT 1 FROM programme.proposal_comment_recipients d
                                WHERE d.comment_id = (SELECT id FROM racine))
                    OR m.author_id = p_person_id
                    OR (SELECT author_id FROM racine) = p_person_id
                    OR EXISTS (SELECT 1 FROM programme.proposal_comment_recipients d
                               WHERE d.comment_id = (SELECT id FROM racine) AND d.person_id = p_person_id)
           END
    FROM programme.proposal_comments m
    WHERE m.id = p_comment_id;
$$;

INSERT INTO engagement.notification_types (code, module_code, label, default_channels, criticality, expected_variables) VALUES
('programme.comment.addressed',     'programme',   '{"fr":"Message du comité qui vous est adressé","en":"Committee message addressed to you"}', '{in_app,email}', 'normal', '{titre_activite,auteur}')
ON CONFLICT (code) DO NOTHING;

COMMIT;
