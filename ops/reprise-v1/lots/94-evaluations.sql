-- Lot 94 (phase 2) : évaluations des activités, notes et consultations.
--
-- `lot94_note` (public.activity_ratings), `lot94_vue`
-- (public.revisionniste_activity_views), écrits par 94-evaluations.sh. Les
-- messages de révision et leurs lectures attendent les destinataires et le lu /
-- non lu par message de la v2 (commanditaire, 27/10) : lot 95.
--
-- Choix :
--   - note v1 (sur 20, sans grille ni recommandation) : revue `quick` soumise à sa
--     date, recommandation `neutral`. Les appels v2 n'ont pas de grille : le
--     classement se fait sur la moyenne sur 20 (`refresh_proposal_score`, appelé
--     ici puisque seul le service l'appelle). L'évaluateur garde ses notes même
--     si son rôle v1 a expiré : la revue désigne une personne, pas un rôle ;
--   - une affectation par revue reprise, datée de la note : sans elle l'évaluateur
--     n'apparaît pas dans la liste du dossier ;
--   - consultation d'un dossier par un évaluateur : `proposal_reads`.
-- Rien ne part : aucun déclencheur de ces tables n'émet d'événement (vérifié).

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-94', true) \g /dev/null

CREATE TEMP TABLE l94_note ON COMMIT DROP AS
SELECT n.*, platform.uuid_v7() AS review_id,
       legacy.map('public.activities', n.activity_id::text) AS proposal_id,
       legacy.map('auth.users', n.evaluateur::text) AS reviewer_id
FROM lot94_note n;

CREATE TEMP TABLE l94_vue ON COMMIT DROP AS
SELECT v.*, legacy.map('public.activities', v.activity_id::text) AS proposal_id,
       legacy.map('auth.users', v.personne::text) AS person_id
FROM lot94_vue v;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM l94_note WHERE proposal_id IS NULL OR reviewer_id IS NULL) THEN
        RAISE EXCEPTION 'lot 94 : note dont l''activité ou la personne manque à legacy.id_map';
    END IF;
    IF EXISTS (SELECT 1 FROM l94_vue WHERE person_id IS NULL)
       OR EXISTS (SELECT 1 FROM l94_vue v WHERE v.proposal_id IS NULL AND NOT EXISTS (
                      SELECT 1 FROM legacy.id_map m WHERE m.legacy_table = 'public.activities'
                         AND m.legacy_id = v.activity_id::text AND m.disposition = 'skipped')) THEN
        RAISE EXCEPTION 'lot 94 : consultation dont la personne, ou l''activité non écartée, manque';
    END IF;
    IF EXISTS (SELECT 1 FROM l94_note GROUP BY proposal_id, reviewer_id HAVING count(*) > 1) THEN
        RAISE EXCEPTION 'lot 94 : deux notes v1 pour une même proposition et une même personne v2';
    END IF;
    IF EXISTS (SELECT 1 FROM programme.reviews r JOIN l94_note n
               ON r.proposal_id = n.proposal_id AND r.reviewer_id = n.reviewer_id)
       OR EXISTS (SELECT 1 FROM legacy.id_map WHERE legacy_table IN (
                      'public.activity_ratings', 'public.revisionniste_activity_views')) THEN
        RAISE EXCEPTION 'lot 94 : évaluations déjà reprises, le lot ne se joue qu''une fois';
    END IF;
END $$;

-- Revues et affectations ----------------------------------------------------------

INSERT INTO programme.review_assignments (proposal_id, reviewer_id, assigned_at)
SELECT proposal_id, reviewer_id, created_at FROM l94_note
ON CONFLICT (proposal_id, reviewer_id) DO NOTHING;

INSERT INTO programme.reviews (id, proposal_id, reviewer_id, mode, recommendation,
                               score_out_of_20, comment, submitted_at, created_at, updated_at)
SELECT review_id, proposal_id, reviewer_id, 'quick', 'neutral',
       note, NULLIF(btrim(commentaire), ''), created_at, created_at, COALESCE(updated_at, created_at)
FROM l94_note;

SELECT programme.refresh_proposal_score(proposal_id)
FROM (SELECT DISTINCT proposal_id FROM l94_note) p \g /dev/null

-- Consultations des dossiers ------------------------------------------------------------

INSERT INTO programme.proposal_reads (proposal_id, person_id, first_read_at, last_read_at, read_count)
SELECT proposal_id, person_id, premiere, COALESCE(derniere, premiere), GREATEST(COALESCE(nombre, 1), 1)
FROM l94_vue
WHERE proposal_id IS NOT NULL
ON CONFLICT (proposal_id, person_id) DO UPDATE
    SET first_read_at = LEAST(proposal_reads.first_read_at, EXCLUDED.first_read_at),
        last_read_at = GREATEST(proposal_reads.last_read_at, EXCLUDED.last_read_at),
        read_count = proposal_reads.read_count + EXCLUDED.read_count;

-- Correspondance ----------------------------------------------------------------------

SELECT legacy.remember('public.activity_ratings', id::text, 'programme', 'reviews', review_id,
                       'migrated', 'revue rapide sur 20, recommandation neutre')
FROM l94_note \g /dev/null

SELECT legacy.remember('public.revisionniste_activity_views', id::text, 'programme', 'proposal_reads',
                       proposal_id, 'migrated', 'ligne (proposal_id, person_id) ; cible : la proposition')
FROM l94_vue WHERE proposal_id IS NOT NULL \g /dev/null

SELECT legacy.remember('public.revisionniste_activity_views', id::text, 'programme', 'proposal_reads',
                       NULL, 'skipped', 'activité écartée')
FROM l94_vue WHERE proposal_id IS NULL \g /dev/null

-- Outbox et travaux : rien ne doit naître de ces lignes.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-94') THEN
        RAISE EXCEPTION 'lot 94 : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-94');
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'lot 94 : travail créé par la transaction';
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
        FROM (VALUES ('public.activity_ratings', (SELECT count(*) FROM lot94_note)),
                     ('public.revisionniste_activity_views', (SELECT count(*) FROM lot94_vue))) AS s(t, extraites)
        LEFT JOIN legacy.id_map m ON m.legacy_table = s.t
        GROUP BY s.t, s.extraites ORDER BY s.t
    LOOP
        RAISE NOTICE 'lot 94 : % : extraites %, migrated %, skipped %', r.t, r.extraites, r.migrated, r.skipped;
        IF r.extraites <> r.migrated + r.skipped THEN
            RAISE EXCEPTION 'lot 94 : % : % extraites, % consignées', r.t, r.extraites, r.migrated + r.skipped;
        END IF;
    END LOOP;
    FOR r IN
        SELECT e.slug, count(DISTINCT p.id) FILTER (WHERE p.review_count > 0) AS notees,
               round(avg(p.average_score) FILTER (WHERE p.review_count > 0), 2) AS moyenne,
               (SELECT count(*) FROM programme.proposal_reads d JOIN programme.proposals q ON q.id = d.proposal_id
                 WHERE q.event_id = e.id) AS consultations
        FROM event.events e JOIN programme.proposals p ON p.event_id = e.id
        WHERE e.slug IN ('cdp30-belem-2025', 'cdp31-antalya-2026')
        GROUP BY e.id, e.slug ORDER BY e.slug
    LOOP
        RAISE NOTICE 'lot 94 : % : % dossiers notés (moyenne %), % consultations',
            r.slug, r.notees, r.moyenne, r.consultations;
    END LOOP;
END $$;

COMMIT;
