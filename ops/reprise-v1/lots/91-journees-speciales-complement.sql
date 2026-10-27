-- Lot 91 (phase 2) : complément du lot 90 (journées spéciales de la CdP30).
--
-- Le lot 90 a créé 14 séances sans ce qu'une séance reçoit de sa proposition en
-- naissant (lots 50 et 60) : les intervenants et le visuel de couverture. Relevé
-- en production par le commanditaire : page d'activité sans panéliste, alors que
-- le formulaire v1 en exigeait un. La séance déplacée par le lot 90, née au lot
-- 50, les a déjà : la garde l'écarte (elle a ses intervenants).
--
-- Joué en production sans flux : `ssh <serveur> "… psql …" < lots/91-journees-speciales-complement.sql`.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-91', true) \g /dev/null

CREATE TEMP TABLE l91 ON COMMIT DROP AS
SELECT s.id AS session_id, s.proposal_id
FROM legacy.id_map m
JOIN programme.sessions s ON s.id = m.target_id
WHERE m.legacy_table = 'code.journees-speciales' AND m.target_table = 'sessions'
  AND NOT EXISTS (SELECT 1 FROM programme.session_speakers ss WHERE ss.session_id = s.id);

DO $$
BEGIN
    IF (SELECT count(*) FROM l91) <> 14 OR EXISTS (SELECT 1 FROM l91 WHERE proposal_id IS NULL) THEN
        RAISE EXCEPTION 'lot 91 : 14 séances du lot 90 sans intervenant attendues, % trouvées',
            (SELECT count(*) FROM l91);
    END IF;
    IF EXISTS (SELECT 1 FROM l91 x WHERE NOT EXISTS (
                   SELECT 1 FROM programme.proposal_speakers ps WHERE ps.proposal_id = x.proposal_id)) THEN
        RAISE EXCEPTION 'lot 91 : proposition sans intervenant';
    END IF;
    IF EXISTS (SELECT 1 FROM l91 x JOIN media.attachments a
               ON a.owner_schema = 'programme' AND a.owner_table = 'sessions' AND a.owner_id = x.session_id) THEN
        RAISE EXCEPTION 'lot 91 : séance déjà dotée d''un média';
    END IF;
END $$;

INSERT INTO programme.session_speakers (
    session_id, person_id, role, job_title_snapshot, organization_snapshot, sort_order, created_at)
SELECT x.session_id, ps.person_id, ps.role, ps.job_title_snapshot, ps.organization_snapshot,
       ps.sort_order, ps.created_at
FROM l91 x
JOIN programme.proposal_speakers ps ON ps.proposal_id = x.proposal_id;

INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role, sort_order,
                               alt_text_override, created_by, created_at)
SELECT 'programme', 'sessions', x.session_id, a.asset_id, a.role, a.sort_order,
       a.alt_text_override, a.created_by, a.created_at
FROM l91 x
JOIN media.attachments a
  ON a.owner_schema = 'programme' AND a.owner_table = 'proposals' AND a.owner_id = x.proposal_id
 AND a.role = 'cover';

-- Outbox et travaux : rien ne doit naître de ces copies.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-91') THEN
        RAISE EXCEPTION 'lot 91 : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-91');
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'lot 91 : travail créé par la transaction';
    END IF;
END $$;

DO $$
BEGIN
    RAISE NOTICE 'lot 91 : % séances complétées, % intervenants, % visuels de couverture',
        (SELECT count(*) FROM l91),
        (SELECT count(*) FROM programme.session_speakers ss JOIN l91 x USING (session_id)),
        (SELECT count(*) FROM media.attachments a JOIN l91 x ON a.owner_id = x.session_id
         WHERE a.owner_schema = 'programme' AND a.owner_table = 'sessions');
    IF EXISTS (SELECT 1 FROM l91 x WHERE NOT EXISTS (
                   SELECT 1 FROM programme.session_speakers ss WHERE ss.session_id = x.session_id)) THEN
        RAISE EXCEPTION 'lot 91 : séance encore sans intervenant';
    END IF;
END $$;

COMMIT;
