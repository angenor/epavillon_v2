-- Lot 92 (phase 2) : intervenants écartés au lot 50 pour D16.
--
-- Leur adresse v1 était celle d'une autre personne (le déposant, souvent, qui la
-- saisissait pour tous ses intervenants). Depuis la migration
-- 2026-10-27-intervenant-sans-adresse, un intervenant sans adresse est une ligne
-- de programme.proposal_speakers sans personne, qui porte civilité, prénom et nom.
--
-- `lot92_int`, écrit par 92-intervenants-sans-adresse.sh : tous les intervenants
-- v1 des activités concernées (rang v1, pour l'ordre), l'identité des seuls
-- écartés ; `lot92.attendus` : leur nombre.
--
-- Choix :
--   - même nom deux fois dans une activité, ou nom d'un intervenant déjà repris
--     dans le dossier : une seule ligne (`merged`) ;
--   - fonction et organisation déclarées : instantanés de la ligne, comme au lot 50 ;
--   - ordre des intervenants recalculé sur le rang v1, dossier et séances ;
--   - séances de ces dossiers : la ligne y est recopiée, comme à la programmation ;
--   - photo v1 (3 sur 149) : non reprise, un intervenant sans personne n'en a pas.
-- Aucun événement d'outbox : ces tables n'en émettent pas (vérifié en fin de lot).

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-92', true) \g /dev/null

CREATE TEMP TABLE l92 ON COMMIT DROP AS
SELECT i.*, i.prenom IS NOT NULL AS a_reprendre,
       m.disposition AS disposition_v1, m.note AS note_v1, m.target_id AS cible_v1,
       legacy.map('public.activities', i.activity_id::text) AS proposal_id,
       platform.normalize_label(i.prenom || ' ' || i.nom) AS nom_n,
       NULL::uuid AS speaker_id, NULL::text AS disposition, NULL::text AS note
FROM lot92_int i
LEFT JOIN legacy.id_map m ON m.legacy_table = 'public.activity_speakers' AND m.legacy_id = i.id::text;

DO $$
BEGIN
    IF (SELECT count(*) FROM l92 WHERE a_reprendre) <> current_setting('lot92.attendus')::int THEN
        RAISE EXCEPTION 'lot 92 : % écartés attendus, % reçus',
            current_setting('lot92.attendus'), (SELECT count(*) FROM l92 WHERE a_reprendre);
    END IF;
    IF EXISTS (SELECT 1 FROM l92 WHERE a_reprendre
               AND (disposition_v1 IS DISTINCT FROM 'skipped' OR note_v1 NOT LIKE 'D16%')) THEN
        RAISE EXCEPTION 'lot 92 : intervenant déjà repris, ou écarté pour une autre raison';
    END IF;
    IF EXISTS (SELECT 1 FROM l92 WHERE disposition_v1 IS NULL OR proposal_id IS NULL) THEN
        RAISE EXCEPTION 'lot 92 : intervenant ou activité absent de legacy.id_map';
    END IF;
    IF EXISTS (SELECT 1 FROM l92 WHERE a_reprendre
               AND (btrim(COALESCE(prenom, '')) = '' OR btrim(COALESCE(nom, '')) = '')) THEN
        RAISE EXCEPTION 'lot 92 : intervenant sans prénom ou sans nom';
    END IF;
END $$;

-- Même personne : déjà intervenante du dossier (personne reprise), ou deux fois écartée.
UPDATE l92 x
SET disposition = 'merged', speaker_id = ps.id,
    note = 'D16 : même nom qu''un intervenant déjà repris dans le dossier'
FROM programme.proposal_speakers ps
JOIN identity.people p ON p.id = ps.person_id
WHERE x.a_reprendre AND ps.proposal_id = x.proposal_id
  AND platform.normalize_label(p.first_name || ' ' || p.last_name) = x.nom_n;

WITH r AS (
    SELECT id, row_number() OVER (PARTITION BY proposal_id, nom_n ORDER BY rang) AS occurrence
    FROM l92 WHERE a_reprendre AND disposition IS NULL
)
UPDATE l92 x
SET disposition = CASE WHEN r.occurrence = 1 THEN 'migrated' ELSE 'merged' END,
    speaker_id = CASE WHEN r.occurrence = 1 THEN platform.uuid_v7() END,
    note = CASE WHEN r.occurrence = 1 THEN 'D16 : intervenant sans adresse, identité portée par la ligne'
                ELSE 'D16 : même personne deux fois dans l''activité' END
FROM r WHERE r.id = x.id;

UPDATE l92 x
SET speaker_id = y.speaker_id
FROM l92 y
WHERE x.disposition = 'merged' AND x.speaker_id IS NULL
  AND y.disposition = 'migrated' AND y.proposal_id = x.proposal_id AND y.nom_n = x.nom_n;

INSERT INTO programme.proposal_speakers (
    id, proposal_id, person_id, civility, first_name, last_name, role,
    job_title_snapshot, organization_snapshot, is_available_for_questions, sort_order, created_at)
SELECT speaker_id, proposal_id, NULL, civilite, prenom, nom, 'speaker',
       fonction, organisation, disponible, 0, created_at
FROM l92
WHERE disposition = 'migrated';

-- Ordre : rang v1 ; une ligne ajoutée en v2 depuis garde sa place relative, après.
WITH origine AS (
    SELECT cible_v1 AS speaker_id, min(rang) AS rang FROM l92
    WHERE disposition_v1 IN ('migrated', 'merged') AND NOT a_reprendre GROUP BY 1
    UNION ALL
    SELECT speaker_id, rang FROM l92 WHERE disposition = 'migrated'
),
ordre AS (
    SELECT ps.id, (row_number() OVER (PARTITION BY ps.proposal_id
                   ORDER BY o.rang NULLS LAST, ps.sort_order, ps.created_at) - 1)::smallint AS rang
    FROM programme.proposal_speakers ps
    LEFT JOIN origine o ON o.speaker_id = ps.id
    WHERE ps.proposal_id IN (SELECT proposal_id FROM l92 WHERE disposition = 'migrated')
)
UPDATE programme.proposal_speakers ps SET sort_order = o.rang
FROM ordre o WHERE o.id = ps.id AND ps.sort_order IS DISTINCT FROM o.rang;

-- Séances : la ligne y est recopiée comme à la programmation, l'ordre suit le dossier.
CREATE TEMP TABLE l92_seance ON COMMIT DROP AS
SELECT s.id AS session_id, x.speaker_id
FROM l92 x JOIN programme.sessions s ON s.proposal_id = x.proposal_id
WHERE x.disposition = 'migrated';

INSERT INTO programme.session_speakers (
    session_id, person_id, civility, first_name, last_name, role,
    job_title_snapshot, organization_snapshot, bio, sort_order, created_at)
SELECT k.session_id, NULL, ps.civility, ps.first_name, ps.last_name, ps.role,
       ps.job_title_snapshot, ps.organization_snapshot, ps.bio, ps.sort_order, ps.created_at
FROM l92_seance k
JOIN programme.proposal_speakers ps ON ps.id = k.speaker_id;

UPDATE programme.session_speakers ss
SET sort_order = ps.sort_order
FROM programme.sessions s
JOIN programme.proposal_speakers ps ON ps.proposal_id = s.proposal_id
WHERE ss.session_id = s.id AND s.id IN (SELECT session_id FROM l92_seance)
  AND ss.person_id IS NOT NULL AND ss.person_id = ps.person_id AND ss.role = ps.role
  AND ss.sort_order IS DISTINCT FROM ps.sort_order;

-- Correspondance ------------------------------------------------------------------

UPDATE legacy.id_map m
SET disposition = x.disposition, target_id = x.speaker_id, note = x.note
FROM l92 x
WHERE x.a_reprendre AND m.legacy_table = 'public.activity_speakers' AND m.legacy_id = x.id::text;

-- Outbox et travaux : rien ne doit naître de ces lignes.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-92') THEN
        RAISE EXCEPTION 'lot 92 : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-92');
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'lot 92 : travail créé par la transaction';
    END IF;
END $$;

-- Compte rendu ---------------------------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    RAISE NOTICE 'lot 92 : écartés D16 %, migrated %, merged %, dossiers complétés %, lignes de séance ajoutées %',
        (SELECT count(*) FROM l92 WHERE a_reprendre),
        (SELECT count(*) FROM l92 WHERE disposition = 'migrated'),
        (SELECT count(*) FROM l92 WHERE disposition = 'merged'),
        (SELECT count(DISTINCT proposal_id) FROM l92 WHERE disposition = 'migrated'),
        (SELECT count(*) FROM l92_seance);
    FOR r IN
        SELECT e.slug, count(*) FILTER (WHERE NOT EXISTS (
                   SELECT 1 FROM programme.proposal_speakers ps WHERE ps.proposal_id = p.id)) AS sans,
               count(*) AS dossiers
        FROM programme.proposals p JOIN event.events e ON e.id = p.event_id
        WHERE p.id IN (SELECT proposal_id FROM l92)
        GROUP BY e.slug ORDER BY e.slug
    LOOP
        RAISE NOTICE 'lot 92 : % : % dossiers touchés, % encore sans intervenant', r.slug, r.dossiers, r.sans;
    END LOOP;
    IF EXISTS (SELECT 1 FROM legacy.id_map WHERE legacy_table = 'public.activity_speakers'
               AND disposition = 'skipped' AND note LIKE 'D16%') THEN
        RAISE EXCEPTION 'lot 92 : écartés D16 restants';
    END IF;
    IF EXISTS (SELECT 1 FROM l92 WHERE a_reprendre AND speaker_id IS NULL) THEN
        RAISE EXCEPTION 'lot 92 : intervenant sans ligne cible';
    END IF;
END $$;

COMMIT;
