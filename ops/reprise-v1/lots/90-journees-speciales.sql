-- Lot 90 (phase 2) : journées spéciales de la CdP30 (D14).
--
-- La v1 les écrivait en dur dans deux pages Vue. Leur relevé
-- (`ops/reprise-v1/journees-speciales/cdp30-*.json`, textes publics de la v1)
-- arrive par 90-journees-speciales.sh dans `lot90_releve` (code du fil, document).
--
-- Affichage : direction actuelle gardée (commanditaire, 27/10) ; chaque activité
-- de la journée est une séance ordinaire de la programmation, rattachée à un fil
-- `special_day`.
--
-- Règles du commanditaire (27/10) :
--   - n'entrent que les activités de la page statique, quels que soient leur date
--     et leur statut dans la v1 : soumises puis versées dans la journée, jamais
--     validées (les valider les aurait affichées sur la v1 hors du dessin voulu).
--     Repérées par porteur, titre ou présentation, et date (`l90_corr`). Leur
--     proposition passe à `accepted` (celle de LOJIQ, rejetée, par `under_review` :
--     seule transition permise) ; leur séance prend le titre, la présentation et
--     les thématiques de la proposition, l'heure de la page statique (la date v1 ne
--     comptait plus) ;
--   - une activité de la page déjà validée dans la v1 à une autre date garde sa
--     séance, déplacée à l'heure de la page (`deplacee`) : celle du ministère
--     tunisien, titrée « Implication des jeunes… » mais présentée comme « le rôle
--     transformateur des jeunes… », le temps de 10h11 (lu 10h10) ;
--   - un temps de la page sans activité (accueil, ouverture, pitchs, panels,
--     réseautage, clôture…) n'est pas créé : consigné `skipped` ;
--   - une activité posée à la date d'une journée mais absente de la page reste
--     telle quelle (`under_review`, sans séance).
-- Rien ne part : séances `completed` non diffusées, sans inscrit pour la déplacée ;
-- `programme.session.created`, `.rescheduled`, `programme.proposal.accepted` et
-- `.under_review` marqués publiés.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-90', true) \g /dev/null

-- Temps de la page statique (code du fil, rang du relevé) → activité v1 ;
-- `deplacee` : sa séance existe déjà ; `debut` : heure locale corrigée.
CREATE TEMP TABLE l90_corr (code text, rang int, activite_v1 uuid NOT NULL, deplacee boolean NOT NULL,
                            debut time, note text, PRIMARY KEY (code, rang)) ON COMMIT DROP;
INSERT INTO l90_corr VALUES
    ('journee_jeunesse_climat',  4, '6a2849ab-8d54-4dd7-a217-78f4d3c49dd2', false, NULL, NULL),
    ('journee_jeunesse_climat',  5, '29833dad-af4a-4a0a-a1b6-fbc2b753ac1d', false, NULL, NULL),
    ('journee_jeunesse_climat',  7, '1e1e0463-9aa7-4930-b28d-b4f80da2a15e', true, '10:10',
        'validée dans la v1 au 13/11 13h15, séance déplacée à l''heure de la page ; début écrit 10h11, lu 10h10'),
    ('journee_jeunesse_climat',  8, '41fb10e4-7519-471b-bdd2-767a36486da4', false, NULL, NULL),
    ('journee_jeunesse_climat',  9, 'cc7c002a-5141-4b33-b677-b1b8ad5cde2d', false, NULL,
        'activité rejetée dans la v1 mais tenue dans la journée'),
    ('journee_jeunesse_climat', 10, '5f194b25-1b3e-4fb6-9e9b-4c5dd1fe0986', false, NULL, NULL),
    ('journee_jeunesse_climat', 11, '2a2dafe0-988a-4f8b-b40d-1b5a99adcad3', false, NULL, 'posée au 11/11 dans la v1'),
    ('journee_jeunesse_climat', 12, '5c5c2d7b-17eb-4063-a50a-07b5f11f46ed', false, NULL, 'posée au 18/11 dans la v1'),
    ('journee_jeunesse_climat', 13, 'db46f6f6-a011-4bbd-9271-4cf2b96e072c', false, NULL, NULL),
    ('journee_finance_durable',  5, 'b33e04ce-acfc-4daa-b062-f9c14bc64797', false, NULL, NULL),
    ('journee_finance_durable',  6, '6504b3c2-ba0a-4855-b640-524e7ec8bcee', false, NULL, NULL),
    ('journee_finance_durable',  7, '1652afaa-ed5f-49bc-a0ee-fce734a7935a', false, NULL, NULL),
    ('journee_finance_durable',  8, '5c14750e-c7c8-4b0e-a002-841aa5967066', false, NULL, NULL),
    ('journee_finance_durable',  9, '4e7b906a-ca12-4ade-b5a5-29eca678df59', false, NULL, 'posée au 12/11 dans la v1'),
    ('journee_finance_durable', 10, '29c653fd-b710-4aa6-a896-999684d1cd7f', false, NULL, NULL);

-- Fils ------------------------------------------------------------------------------

CREATE TEMP TABLE l90_fil ON COMMIT DROP AS
SELECT platform.uuid_v7() AS track_id, e.id AS event_id, e.timezone, e.programme_published_at,
       r.code, r.doc->'programme_track' AS t,
       CASE r.code WHEN 'journee_jeunesse_climat' THEN '#8C6D1F' ELSE '#1F5F8B' END AS couleur,
       row_number() OVER (ORDER BY r.doc->'programme_track'->>'starts_on')::smallint AS rang
FROM lot90_releve r
JOIN event.events e ON e.slug = 'cdp30-belem-2025';

DO $$
BEGIN
    IF (SELECT count(*) FROM l90_fil) <> 2
       OR EXISTS (SELECT 1 FROM l90_fil f WHERE NOT EXISTS (SELECT 1 FROM l90_corr c WHERE c.code = f.code)) THEN
        RAISE EXCEPTION 'lot 90 : relevé attendu, les deux journées de la CdP30';
    END IF;
    IF EXISTS (SELECT 1 FROM event.programme_tracks p JOIN l90_fil f
               ON p.event_id = f.event_id AND (p.code = f.code OR p.slug::text = f.t->>'slug')) THEN
        RAISE EXCEPTION 'lot 90 : fil déjà présent, le lot ne se joue qu''une fois';
    END IF;
END $$;

INSERT INTO event.programme_tracks (
    id, event_id, code, slug, kind, title, subtitle, description,
    starts_on, ends_on, color_hex, published_at, sort_order)
SELECT track_id, event_id, code, t->>'slug', 'special_day', t->'title', t->'subtitle', t->'description',
       (t->>'starts_on')::date, (t->>'ends_on')::date, couleur, programme_published_at, rang
FROM l90_fil;

-- Temps de la page --------------------------------------------------------------------

CREATE TEMP TABLE l90_temps ON COMMIT DROP AS
SELECT f.code, (s->>'sort_order')::int AS rang, s AS doc, c.activite_v1, c.deplacee, c.debut, c.note
FROM lot90_releve r
JOIN l90_fil f ON f.code = r.code
CROSS JOIN LATERAL jsonb_array_elements(r.doc->'sessions') AS s
LEFT JOIN l90_corr c ON c.code = r.code AND c.rang = (s->>'sort_order')::int;

CREATE TEMP TABLE l90 ON COMMIT DROP AS
SELECT x.code, x.rang, x.doc, x.activite_v1, x.deplacee, x.note,
       f.track_id, f.event_id, f.timezone, f.programme_published_at,
       NULL::uuid AS session_id,
       legacy.map('public.activities', x.activite_v1::text) AS proposal_id,
       CASE WHEN x.debut IS NULL THEN (x.doc->>'starts_at')::timestamptz
            ELSE (((x.doc->>'starts_at')::timestamptz AT TIME ZONE f.timezone)::date + x.debut)
                 AT TIME ZONE f.timezone END AS starts_at,
       (x.doc->>'ends_at')::timestamptz AS ends_at,
       CASE WHEN jsonb_typeof(x.doc->'summary') = 'object' THEN x.doc->'summary' END AS resume
FROM l90_temps x
JOIN l90_fil f USING (code)
WHERE x.activite_v1 IS NOT NULL;

DO $$
BEGIN
    IF (SELECT count(*) FROM l90) <> (SELECT count(*) FROM l90_corr) THEN
        RAISE EXCEPTION 'lot 90 : correspondance sans temps dans le relevé';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 WHERE proposal_id IS NULL) THEN
        RAISE EXCEPTION 'lot 90 : activité v1 absente de legacy.id_map';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 x JOIN programme.proposals p ON p.id = x.proposal_id
               WHERE p.event_id <> x.event_id OR p.deleted_at IS NOT NULL
                  OR p.status NOT IN ('under_review', 'rejected') AND NOT x.deplacee
                  OR p.status <> 'accepted' AND x.deplacee)
       OR EXISTS (SELECT 1 FROM programme.sessions s JOIN l90 x ON s.proposal_id = x.proposal_id
                  WHERE NOT x.deplacee) THEN
        RAISE EXCEPTION 'lot 90 : proposition déjà décidée, supprimée ou dotée d''une séance';
    END IF;
    -- Séance déplacée : la seule de sa proposition, close, non diffusée, sans inscrit.
    IF EXISTS (SELECT 1 FROM l90 x WHERE x.deplacee AND (
                   (SELECT count(*) FROM programme.sessions s WHERE s.proposal_id = x.proposal_id) <> 1
                   OR EXISTS (SELECT 1 FROM programme.sessions s WHERE s.proposal_id = x.proposal_id
                              AND (s.status <> 'completed' OR s.is_streamed OR s.event_id <> x.event_id))
                   OR EXISTS (SELECT 1 FROM programme.sessions s JOIN programme.registrations g
                              ON g.session_id = s.id WHERE s.proposal_id = x.proposal_id))) THEN
        RAISE EXCEPTION 'lot 90 : séance à déplacer absente, multiple, ouverte ou avec inscrits';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 WHERE starts_at IS NULL OR ends_at IS NULL OR ends_at <= starts_at) THEN
        RAISE EXCEPTION 'lot 90 : horaire manquant ou inversé';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 x JOIN programme.proposals p ON p.id = x.proposal_id
               JOIN programme.sessions s ON s.event_id = x.event_id AND s.slug::text = p.slug::text
               WHERE NOT x.deplacee) THEN
        RAISE EXCEPTION 'lot 90 : slug déjà pris par une séance';
    END IF;
END $$;

UPDATE l90 x
SET session_id = CASE WHEN x.deplacee
                      THEN (SELECT s.id FROM programme.sessions s WHERE s.proposal_id = x.proposal_id)
                      ELSE platform.uuid_v7() END;

-- Décisions : la journée a tenu ces activités.
UPDATE programme.proposals p
SET status = 'under_review',
    decision_reason = 'Activité tenue dans une journée spéciale de la CdP30 (reprise v1)'
FROM l90 x
WHERE p.id = x.proposal_id AND p.status = 'rejected';

UPDATE programme.proposals p
SET status = 'accepted', decision_reason = NULL, decided_by = NULL
FROM l90 x
WHERE p.id = x.proposal_id AND NOT x.deplacee;

UPDATE programme.sessions s
SET starts_at = x.starts_at, ends_at = x.ends_at, listing_changed_at = now()
FROM l90 x
WHERE s.id = x.session_id AND x.deplacee;

-- Un titre v1 commence par une puce collée d'une liste : la séance l'écrit sans.
INSERT INTO programme.sessions (
    id, event_id, proposal_id, organization_id, sequence_number,
    title, slug, summary, description, status, format, starts_at, ends_at, timezone,
    listing_changed_at, published_at, created_by, created_at, updated_at)
SELECT x.session_id, x.event_id, x.proposal_id, p.organization_id, 1,
       jsonb_set(p.title, '{fr}', to_jsonb(regexp_replace(p.title->>'fr', '^[[:space:]•·]+', ''))),
       p.slug, x.resume, p.detailed_presentation, 'completed', p.format, x.starts_at, x.ends_at, x.timezone,
       now(), x.programme_published_at, platform.current_actor_id(), now(), now()
FROM l90 x
JOIN programme.proposals p ON p.id = x.proposal_id
WHERE NOT x.deplacee
ORDER BY x.code, x.rang;

-- Thématiques seules, comme à la naissance d'une séance par l'API (lot 50).
INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order, created_at)
SELECT 'programme', 'sessions', x.session_id, et.term_id, et.sort_order, now()
FROM l90 x
JOIN reference.entity_terms et
  ON et.entity_schema = 'programme' AND et.entity_table = 'proposals' AND et.entity_id = x.proposal_id
JOIN reference.taxonomy_terms t ON t.id = et.term_id AND t.taxonomy_code = 'activity_theme'
WHERE NOT x.deplacee;

INSERT INTO programme.session_tracks (session_id, track_id, sort_order, is_highlight, added_by, added_at)
SELECT session_id, track_id, rang, false, platform.current_actor_id(), now()
FROM l90;

-- Correspondance ----------------------------------------------------------------------

SELECT legacy.remember('code.journees-speciales', code, 'event', 'programme_tracks', track_id)
FROM l90_fil \g /dev/null

SELECT legacy.remember('code.journees-speciales', format('%s/%s', t.code, lpad(t.rang::text, 2, '0')),
                       'programme', 'sessions', x.session_id,
                       CASE WHEN x.session_id IS NULL THEN 'skipped' ELSE 'migrated' END,
                       CASE WHEN x.session_id IS NULL
                            THEN 'temps de la page sans activité v1 : non créé (commanditaire)'
                            ELSE concat_ws(' ; ', 'séance de l''activité v1 ' || t.activite_v1, t.note) END)
FROM l90_temps t
LEFT JOIN l90 x USING (code, rang) \g /dev/null

-- Outbox et travaux : rien ne part ------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    FOR r IN SELECT event_type, count(*) AS n FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-90' GROUP BY event_type ORDER BY event_type
    LOOP
        RAISE NOTICE 'lot 90 : outbox % : % événement(s), marqués publiés', r.event_type, r.n;
    END LOOP;
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-90'
               AND event_type NOT IN ('programme.session.created', 'programme.session.rescheduled',
                                      'programme.proposal.accepted', 'programme.proposal.under_review')) THEN
        RAISE EXCEPTION 'lot 90 : événement d''outbox inattendu';
    END IF;
    IF EXISTS (SELECT 1 FROM programme.sessions s JOIN l90 x ON x.session_id = s.id
               WHERE s.is_streamed OR s.status <> 'completed') THEN
        RAISE EXCEPTION 'lot 90 : séance diffusée ou non close, Zoom la verrait';
    END IF;
    -- `now()` est l'heure de la transaction : un travail qui la porte vient d'elle.
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'lot 90 : travail créé par la transaction';
    END IF;
END $$;

UPDATE platform.outbox_events SET published_at = now()
WHERE correlation_id = 'reprise-v1-90' AND published_at IS NULL;

-- Compte rendu ---------------------------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    FOR r IN
        SELECT t.code, count(*) AS temps, count(x.session_id) AS seances,
               min(x.starts_at AT TIME ZONE f.timezone)::time AS debut,
               max(x.ends_at AT TIME ZONE f.timezone)::time AS fin
        FROM l90_temps t JOIN l90_fil f ON f.code = t.code
        LEFT JOIN l90 x ON x.code = t.code AND x.rang = t.rang
        GROUP BY t.code ORDER BY t.code
    LOOP
        RAISE NOTICE 'lot 90 : % : % temps sur la page, % séances (% → %), % sans activité non créés',
            r.code, r.temps, r.seances, r.debut, r.fin, r.temps - r.seances;
    END LOOP;
    RAISE NOTICE 'lot 90 : propositions acceptées %, séances publiques de la CdP30 %',
        (SELECT count(*) FROM programme.proposals p JOIN l90 x ON x.proposal_id = p.id WHERE p.status = 'accepted'),
        (SELECT count(*) FROM programme.sessions s JOIN l90_fil f ON f.event_id = s.event_id
         WHERE f.rang = 1 AND s.published_at IS NOT NULL);
    IF (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'code.journees-speciales')
       <> (SELECT count(*) FROM l90_fil) + (SELECT count(*) FROM l90_temps) THEN
        RAISE EXCEPTION 'lot 90 : correspondance incomplète';
    END IF;
END $$;

COMMIT;
