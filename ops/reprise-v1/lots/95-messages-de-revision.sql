-- Lot 95 (phase 2) : messages de révision, leurs destinataires et leurs lectures.
--
-- `lot95_message` (public.revision_comments, avec `shared_with_revisionists`) et
-- `lot95_lecture` (public.revision_comment_reads), écrits par
-- 95-messages-de-revision.sh. Joué après la migration
-- 2026-10-27-destinataires-et-lectures-des-messages et l'API qui la lit.
--
-- Choix :
--   - visibilité : `submitter` si le message était partagé avec le soumissionnaire
--     ou si son auteur n'avait aucun rôle (un soumissionnaire qui répond),
--     `committee` sinon ; la v1 n'a pas de fil, chaque message en ouvre un ;
--   - destinataires v1 (`shared_with_revisionists`) d'un message du comité : ses
--     destinataires v2, l'auteur exclu ; liste vide, tout le comité. Sur un message
--     partagé avec le soumissionnaire, la v2 n'en porte pas : liste écartée (note) ;
--   - lecture v1 : lecture v2 du message à sa date ; celle de l'auteur est
--     implicite en v2 (il a lu ce qu'il a écrit), écartée.
-- Rien ne part : ces insertions n'émettent rien (vérifié), les avis aux
-- destinataires ne naissent que du service.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-95', true) \g /dev/null

CREATE TEMP TABLE l95_message ON COMMIT DROP AS
SELECT m.*, platform.uuid_v7() AS comment_id,
       legacy.map('public.activities', m.activity_id::text) AS proposal_id,
       legacy.map('auth.users', m.auteur::text) AS author_id,
       CASE WHEN m.partage_soumissionnaire OR NOT m.auteur_equipe THEN 'submitter' ELSE 'committee' END AS visibilite
FROM lot95_message m;

CREATE TEMP TABLE l95_destinataire ON COMMIT DROP AS
SELECT DISTINCT m.comment_id, legacy.map('auth.users', d::text) AS person_id, m.created_at
FROM l95_message m CROSS JOIN LATERAL unnest(m.destinataires) AS d
WHERE m.visibilite = 'committee';

CREATE TEMP TABLE l95_lecture ON COMMIT DROP AS
SELECT l.id, l.comment_id AS message_v1, l.lecteur, l.read_at,
       m.comment_id, m.author_id, legacy.map('auth.users', l.lecteur::text) AS person_id
FROM lot95_lecture l
LEFT JOIN l95_message m ON m.id = l.comment_id;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM l95_message WHERE proposal_id IS NULL OR author_id IS NULL) THEN
        RAISE EXCEPTION 'lot 95 : message dont l''activité ou l''auteur manque à legacy.id_map';
    END IF;
    IF EXISTS (SELECT 1 FROM l95_destinataire WHERE person_id IS NULL)
       OR EXISTS (SELECT 1 FROM l95_lecture WHERE person_id IS NULL OR comment_id IS NULL) THEN
        RAISE EXCEPTION 'lot 95 : destinataire, lecteur ou message lu absent';
    END IF;
    IF EXISTS (SELECT 1 FROM l95_message WHERE btrim(COALESCE(texte, '')) = '') THEN
        RAISE EXCEPTION 'lot 95 : message vide';
    END IF;
    IF EXISTS (SELECT 1 FROM legacy.id_map WHERE legacy_table IN
                   ('public.revision_comments', 'public.revision_comment_reads'))
       OR to_regclass('programme.proposal_comment_reads') IS NULL THEN
        RAISE EXCEPTION 'lot 95 : déjà joué, ou migration des destinataires et lectures absente';
    END IF;
END $$;

INSERT INTO programme.proposal_comments (id, proposal_id, author_id, visibility, body, edited_at, created_at)
SELECT comment_id, proposal_id, author_id, visibilite::programme.comment_visibility, texte,
       CASE WHEN updated_at > created_at + interval '1 second' THEN updated_at END,
       created_at
FROM l95_message;

INSERT INTO programme.proposal_comment_recipients (comment_id, person_id, added_by, added_at)
SELECT d.comment_id, d.person_id, m.author_id, d.created_at
FROM l95_destinataire d JOIN l95_message m USING (comment_id)
WHERE d.person_id <> m.author_id;

INSERT INTO programme.proposal_comment_reads (comment_id, person_id, read_at)
SELECT comment_id, person_id, min(read_at)
FROM l95_lecture
WHERE person_id <> author_id
GROUP BY comment_id, person_id;

-- Correspondance ----------------------------------------------------------------------

SELECT legacy.remember('public.revision_comments', id::text, 'programme', 'proposal_comments', comment_id,
                       'migrated', concat_ws(' ; ',
                           CASE visibilite WHEN 'submitter' THEN 'visible du soumissionnaire' ELSE 'comité' END,
                           CASE WHEN visibilite = 'committee' AND cardinality(destinataires) > 0
                                THEN 'destinataires repris' END,
                           CASE WHEN visibilite = 'submitter' AND cardinality(destinataires) > 0
                                THEN 'liste de destinataires écartée : un message partagé n''en porte pas' END))
FROM l95_message \g /dev/null

SELECT legacy.remember('public.revision_comment_reads', id::text, 'programme', 'proposal_comment_reads',
                       CASE WHEN person_id <> author_id THEN comment_id END,
                       CASE WHEN person_id <> author_id THEN 'migrated' ELSE 'skipped' END,
                       CASE WHEN person_id <> author_id THEN 'ligne (comment_id, person_id) ; cible : le message'
                            ELSE 'lecture de l''auteur, implicite en v2' END)
FROM l95_lecture \g /dev/null

-- Outbox et travaux : rien ne doit naître de ces lignes.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-95') THEN
        RAISE EXCEPTION 'lot 95 : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-95');
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'lot 95 : travail créé par la transaction';
    END IF;
END $$;

-- Compte rendu ---------------------------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    FOR r IN
        SELECT s.t, s.extraites,
               count(m.*) FILTER (WHERE m.disposition = 'migrated') AS migrated,
               count(m.*) FILTER (WHERE m.disposition = 'skipped') AS skipped
        FROM (VALUES ('public.revision_comments', (SELECT count(*) FROM lot95_message)),
                     ('public.revision_comment_reads', (SELECT count(*) FROM lot95_lecture))) AS s(t, extraites)
        LEFT JOIN legacy.id_map m ON m.legacy_table = s.t
        GROUP BY s.t, s.extraites ORDER BY s.t
    LOOP
        RAISE NOTICE 'lot 95 : % : extraites %, migrated %, skipped %', r.t, r.extraites, r.migrated, r.skipped;
        IF r.extraites <> r.migrated + r.skipped THEN
            RAISE EXCEPTION 'lot 95 : % : % extraites, % consignées', r.t, r.extraites, r.migrated + r.skipped;
        END IF;
    END LOOP;
    RAISE NOTICE 'lot 95 : messages % (comité %, dont restreints %, partagés %), destinataires %, lectures %',
        (SELECT count(*) FROM l95_message),
        (SELECT count(*) FROM l95_message WHERE visibilite = 'committee'),
        (SELECT count(DISTINCT comment_id) FROM programme.proposal_comment_recipients
          WHERE comment_id IN (SELECT comment_id FROM l95_message)),
        (SELECT count(*) FROM l95_message WHERE visibilite = 'submitter'),
        (SELECT count(*) FROM programme.proposal_comment_recipients
          WHERE comment_id IN (SELECT comment_id FROM l95_message)),
        (SELECT count(*) FROM programme.proposal_comment_reads
          WHERE comment_id IN (SELECT comment_id FROM l95_message));
END $$;

COMMIT;
