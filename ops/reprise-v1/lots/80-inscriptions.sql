-- Lot 80 (phase 2) : inscriptions aux activités (public.activity_registrations).
--
-- Joué en production sans y reposer la v1 : 80-inscriptions.sh extrait du banc,
-- en identifiants et dates seulement, `lot80_inscription` (inscription, activité,
-- personne v1, organisation v1, date, invité) et `lot80_paco` (inscriptions PACO,
-- écartées, D9), et fixe `lot80.attendues` (lignes de la table v1).
--
-- Correspondances :
--   - séance : celle de la proposition issue de l'activité ; une activité rejetée
--     dans la v1 n'a pas de séance, son inscription est écartée ;
--   - personne : le compte v1 ; un invité est rattaché au compte de même adresse,
--     rapprochement fait au banc (les trois invités de la CdP30 en ont un) ;
--   - statut `registered` : la v1 n'a jamais relevé la présence (attended
--     toujours faux, ni durée ni heure d'entrée) ; organisation : celle du compte ;
--   - identifiants Zoom de la v1 laissés : réunions expirées (D3).
-- Rien ne doit partir : une inscription émet `programme.registration.created`,
-- que lisent Zoom, la confirmation par courriel et les rappels. Ses événements
-- sont marqués publiés dans la transaction, avant que le relais ne les voie.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-80', true) \g /dev/null

CREATE TEMP TABLE l80 ON COMMIT DROP AS
SELECT i.*,
       p.id AS proposal_id, p.status AS proposal_status, s.id AS session_id,
       legacy.map('auth.users', i.personne_v1::text) AS person_id,
       legacy.map('public.organizations', i.organisation_v1::text) AS organization_id,
       NULL::uuid AS registration_id, NULL::text AS disposition, NULL::text AS note
FROM lot80_inscription i
LEFT JOIN programme.proposals p ON p.id = legacy.map('public.activities', i.activity_id::text)
LEFT JOIN programme.sessions s ON s.proposal_id = p.id;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM l80 WHERE proposal_id IS NULL OR person_id IS NULL) THEN
        RAISE EXCEPTION 'lot 80 : activité ou personne sans correspondance';
    END IF;
    IF EXISTS (SELECT 1 FROM l80 GROUP BY id HAVING count(*) > 1) THEN
        RAISE EXCEPTION 'lot 80 : une proposition porte plusieurs séances';
    END IF;
    IF EXISTS (SELECT 1 FROM programme.sessions s JOIN l80 ON l80.session_id = s.id
               WHERE s.is_streamed OR s.status <> 'completed') THEN
        RAISE EXCEPTION 'lot 80 : séance diffusée ou non close, Zoom la reprendrait';
    END IF;
END $$;

UPDATE l80 SET disposition = 'skipped',
               note = CASE proposal_status WHEN 'rejected' THEN 'activité rejetée dans la v1'
                           ELSE 'proposition ' || proposal_status END || ' : aucune séance en v2'
WHERE session_id IS NULL;

-- Une personne une fois par séance (ux_registrations_person_session) : la plus
-- ancienne inscription l'emporte, les suivantes s'y rattachent.
UPDATE l80 SET disposition = d.disposition, note = d.note
FROM (
    SELECT id,
           CASE WHEN rang = 1 THEN 'migrated' ELSE 'merged' END AS disposition,
           CASE WHEN rang > 1 THEN 'doublon : même personne, même séance'
                WHEN invite THEN 'invité de la v1, rattaché au compte de même adresse' END AS note
    FROM (SELECT id, invite, row_number() OVER (PARTITION BY session_id, person_id
                                                ORDER BY registration_date, id) AS rang
          FROM l80 WHERE session_id IS NOT NULL) x
) d
WHERE l80.id = d.id;

UPDATE l80 SET registration_id = platform.uuid_v7() WHERE disposition = 'migrated';
UPDATE l80 AS m SET registration_id = g.registration_id
FROM l80 g
WHERE m.disposition = 'merged' AND g.disposition = 'migrated'
  AND g.session_id = m.session_id AND g.person_id = m.person_id;

INSERT INTO programme.registrations (id, session_id, person_id, organization_id, status, locale,
                                     source, created_at, updated_at)
SELECT registration_id, session_id, person_id, organization_id, 'registered', 'fr',
       'import', registration_date, registration_date
FROM l80 WHERE disposition = 'migrated';

-- Correspondances ----------------------------------------------------------------

SELECT legacy.remember('public.activity_registrations', id::text, 'programme', 'registrations',
                       registration_id, disposition, note)
FROM l80 \g /dev/null
SELECT legacy.remember('public.activity_registrations', id, 'programme', 'registrations', NULL,
                       'skipped', 'inscription PACO (D9)')
FROM lot80_paco \g /dev/null

-- Outbox et travaux : rien ne part ------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    FOR r IN SELECT event_type, count(*) AS n FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-80' GROUP BY event_type ORDER BY event_type
    LOOP
        RAISE NOTICE 'lot 80 : outbox % : % événement(s), marqués publiés', r.event_type, r.n;
    END LOOP;
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-80'
               AND event_type <> 'programme.registration.created') THEN
        RAISE EXCEPTION 'lot 80 : événement d''outbox inattendu';
    END IF;
    -- `now()` est l'heure de la transaction : un travail qui la porte vient d'elle.
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'lot 80 : travail créé par la transaction';
    END IF;
END $$;

UPDATE platform.outbox_events SET published_at = now()
WHERE correlation_id = 'reprise-v1-80' AND published_at IS NULL;

-- Compte rendu ---------------------------------------------------------------------

DO $$
DECLARE
    v_attendues int := current_setting('lot80.attendues')::int;
    v_consignees int := (SELECT count(*) FROM legacy.id_map
                         WHERE legacy_table = 'public.activity_registrations');
BEGIN
    RAISE NOTICE 'lot 80 : public.activity_registrations extraites %, migrated %, merged %, skipped % (dont PACO %)',
        v_attendues,
        (SELECT count(*) FROM l80 WHERE disposition = 'migrated'),
        (SELECT count(*) FROM l80 WHERE disposition = 'merged'),
        (SELECT count(*) FROM l80 WHERE disposition = 'skipped') + (SELECT count(*) FROM lot80_paco),
        (SELECT count(*) FROM lot80_paco);
    RAISE NOTICE 'lot 80 : séances avec inscrits %, invités rattachés à un compte %, avec organisation %',
        (SELECT count(DISTINCT session_id) FROM l80 WHERE disposition = 'migrated'),
        (SELECT count(*) FROM l80 WHERE invite AND disposition = 'migrated'),
        (SELECT count(*) FROM l80 WHERE disposition = 'migrated' AND organization_id IS NOT NULL);
    IF v_consignees <> v_attendues THEN
        RAISE EXCEPTION 'lot 80 : % lignes v1, % consignées', v_attendues, v_consignees;
    END IF;
END $$;

COMMIT;
