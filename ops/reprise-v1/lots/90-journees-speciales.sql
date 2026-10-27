-- Lot 90 (phase 2) : journées spéciales de la CdP30 (D14).
--
-- La v1 les écrivait en dur dans deux pages Vue. Leur relevé
-- (`ops/reprise-v1/journees-speciales/cdp30-*.json`, textes publics de la v1)
-- arrive par 90-journees-speciales.sh dans `lot90_releve` (code du fil, document).
--
-- Affichage : direction actuelle gardée (commanditaire, 27/10) ; chaque temps de
-- la journée est une séance ordinaire de la programmation, rattachée à un fil
-- `special_day`.
--
-- Correspondances (`l90_corr`) :
--   - les activités soumises puis versées dans une journée existent dans Supabase,
--     jamais validées (les valider les aurait affichées sur la v1 hors du dessin
--     voulu) ; repérées par titre, porteur et date (posées au 12 ou au 14/11, de
--     9 h à 17 h). Leur proposition passe à `accepted` (celle de LOJIQ, rejetée,
--     par `under_review` : seule transition permise) ; leur séance prend le titre,
--     la présentation et les thématiques de la proposition, l'heure du programme ;
--   - les autres temps (accueil, ouverture, pitchs, panels, réseautage, clôture)
--     n'ont pas d'activité : séance sans proposition portée par l'IFDD (une par
--     le ministère tunisien), titre du relevé, intervenants et organisations du
--     relevé en présentation (personnes sans adresse : D16, phase 2) ;
--   - heures corrigées : « 10h11 » lu 10h10 ; clôtures sans fin écrite : 17h45 (fin
--     annoncée de la journée Jeunesse) et 18h00 pour la Finance (estimée).
-- Laissés en `under_review` : les dossiers posés au 12/11 absents du programme
-- final, ceux d'Irokko et d'Ecolotrip fondus dans un panel, les deux dossiers
-- « chapeau » de l'IFDD (« À définir »).
-- Rien ne part : séances `completed` non diffusées ; `programme.session.created`,
-- `programme.proposal.accepted` et `.under_review` marqués publiés.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-90', true) \g /dev/null

CREATE FUNCTION pg_temp.html(t text) RETURNS text LANGUAGE sql IMMUTABLE
AS $$ SELECT replace(replace(replace(t, '&', '&amp;'), '<', '&lt;'), '>', '&gt;') $$;

-- Temps du relevé (code du fil, rang) → activité v1 ou porteur v1 ; heure locale corrigée.
CREATE TEMP TABLE l90_corr (
    code text, rang int, activite_v1 uuid, porteur_v1 uuid, debut time, fin time, note text,
    PRIMARY KEY (code, rang)
) ON COMMIT DROP;
INSERT INTO l90_corr VALUES
    ('journee_jeunesse_climat',  1, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_jeunesse_climat',  2, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_jeunesse_climat',  3, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_jeunesse_climat',  4, '6a2849ab-8d54-4dd7-a217-78f4d3c49dd2', NULL, NULL, NULL, NULL),
    ('journee_jeunesse_climat',  5, '29833dad-af4a-4a0a-a1b6-fbc2b753ac1d', NULL, NULL, NULL, NULL),
    ('journee_jeunesse_climat',  6, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_jeunesse_climat',  7, NULL, 'c703c425-49a0-4118-b629-3cc1b49731d7', '10:10', NULL,
        'début écrit 10h11 dans la v1, lu 10h10'),
    ('journee_jeunesse_climat',  8, '41fb10e4-7519-471b-bdd2-767a36486da4', NULL, NULL, NULL, NULL),
    ('journee_jeunesse_climat',  9, 'cc7c002a-5141-4b33-b677-b1b8ad5cde2d', NULL, NULL, NULL,
        'activité rejetée dans la v1 mais tenue dans la journée'),
    ('journee_jeunesse_climat', 10, '5f194b25-1b3e-4fb6-9e9b-4c5dd1fe0986', NULL, NULL, NULL, NULL),
    ('journee_jeunesse_climat', 11, '2a2dafe0-988a-4f8b-b40d-1b5a99adcad3', NULL, NULL, NULL,
        'activité posée au 11/11 dans la v1, tenue dans la journée'),
    ('journee_jeunesse_climat', 12, '5c5c2d7b-17eb-4063-a50a-07b5f11f46ed', NULL, NULL, NULL,
        'activité posée au 18/11 dans la v1, tenue dans la journée'),
    ('journee_jeunesse_climat', 13, 'db46f6f6-a011-4bbd-9271-4cf2b96e072c', NULL, NULL, NULL, NULL),
    ('journee_jeunesse_climat', 14, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_jeunesse_climat', 15, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, '17:45',
        'fin non écrite, posée à 17h45, fin annoncée de la journée'),
    ('journee_finance_durable',  1, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_finance_durable',  2, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_finance_durable',  3, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_finance_durable',  4, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, NULL, NULL),
    ('journee_finance_durable',  5, 'b33e04ce-acfc-4daa-b062-f9c14bc64797', NULL, NULL, NULL, NULL),
    ('journee_finance_durable',  6, '6504b3c2-ba0a-4855-b640-524e7ec8bcee', NULL, NULL, NULL, NULL),
    ('journee_finance_durable',  7, '1652afaa-ed5f-49bc-a0ee-fce734a7935a', NULL, NULL, NULL, NULL),
    ('journee_finance_durable',  8, '5c14750e-c7c8-4b0e-a002-841aa5967066', NULL, NULL, NULL, NULL),
    ('journee_finance_durable',  9, '4e7b906a-ca12-4ade-b5a5-29eca678df59', NULL, NULL, NULL,
        'activité posée au 12/11 dans la v1, tenue dans la journée Finance'),
    ('journee_finance_durable', 10, '29c653fd-b710-4aa6-a896-999684d1cd7f', NULL, NULL, NULL, NULL),
    ('journee_finance_durable', 11, NULL, 'a4b79b51-0e35-4abc-b666-bb32e6b5cf78', NULL, '18:00',
        'fin non écrite, estimée à 18h00');

-- Fils ------------------------------------------------------------------------------

CREATE TEMP TABLE l90_fil ON COMMIT DROP AS
SELECT platform.uuid_v7() AS track_id, e.id AS event_id, e.timezone, e.programme_published_at,
       r.code, r.doc->'programme_track' AS t,
       (r.doc->'programme_track'->>'starts_on')::date AS jour,
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

-- Séances ---------------------------------------------------------------------------

CREATE TEMP TABLE l90 ON COMMIT DROP AS
SELECT c.*, f.track_id, f.event_id, f.timezone, f.programme_published_at, f.t->>'slug' AS fil_slug,
       s.value AS doc,
       platform.uuid_v7() AS session_id,
       legacy.map('public.activities', c.activite_v1::text) AS proposal_id,
       CASE WHEN c.debut IS NOT NULL THEN (f.jour + c.debut) AT TIME ZONE f.timezone
            ELSE (s.value->>'starts_at')::timestamptz END AS starts_at,
       CASE WHEN c.fin IS NOT NULL THEN (f.jour + c.fin) AT TIME ZONE f.timezone
            ELSE (s.value->>'ends_at')::timestamptz END AS ends_at
FROM lot90_releve r
JOIN l90_fil f ON f.code = r.code
CROSS JOIN LATERAL jsonb_array_elements(r.doc->'sessions') AS s(value)
FULL JOIN l90_corr c ON c.code = r.code AND c.rang = (s.value->>'sort_order')::int;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM l90 WHERE code IS NULL OR doc IS NULL) THEN
        RAISE EXCEPTION 'lot 90 : temps du relevé sans correspondance, ou l''inverse';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 WHERE (activite_v1 IS NULL) = (porteur_v1 IS NULL)) THEN
        RAISE EXCEPTION 'lot 90 : un temps a une activité v1 ou un porteur, jamais les deux';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 WHERE activite_v1 IS NOT NULL AND proposal_id IS NULL)
       OR EXISTS (SELECT 1 FROM l90 WHERE porteur_v1 IS NOT NULL
                  AND legacy.map('public.organizations', porteur_v1::text) IS NULL) THEN
        RAISE EXCEPTION 'lot 90 : activité ou porteur v1 absent de legacy.id_map';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 x JOIN programme.proposals p ON p.id = x.proposal_id
               WHERE p.event_id <> x.event_id OR p.status NOT IN ('under_review', 'rejected')
                  OR p.deleted_at IS NOT NULL)
       OR EXISTS (SELECT 1 FROM programme.sessions s JOIN l90 x ON s.proposal_id = x.proposal_id) THEN
        RAISE EXCEPTION 'lot 90 : proposition déjà décidée, supprimée ou dotée d''une séance';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 WHERE starts_at IS NULL OR ends_at IS NULL OR ends_at <= starts_at) THEN
        RAISE EXCEPTION 'lot 90 : horaire manquant ou inversé';
    END IF;
    IF EXISTS (SELECT 1 FROM l90 x JOIN l90_corr c USING (code, rang)
               CROSS JOIN LATERAL jsonb_array_elements(x.doc->'speakers') sp
               WHERE sp->>'role' NOT IN ('moderator', 'facilitator', 'speaker', 'panelist')) THEN
        RAISE EXCEPTION 'lot 90 : rôle d''intervenant inconnu';
    END IF;
END $$;

ALTER TABLE l90
    ADD COLUMN organization_id uuid, ADD COLUMN format event.participation_mode,
    ADD COLUMN titre jsonb, ADD COLUMN slug text, ADD COLUMN resume jsonb, ADD COLUMN presentation jsonb;

-- Activité v1 : la proposition donne porteur, format, titre, slug et présentation.
-- Un titre v1 commence par une puce collée d'une liste : la séance l'écrit sans.
UPDATE l90 x
SET organization_id = p.organization_id, format = p.format,
    titre = jsonb_set(p.title, '{fr}', to_jsonb(regexp_replace(p.title->>'fr', '^[[:space:]•·]+', ''))),
    slug = p.slug,
    presentation = p.detailed_presentation
FROM programme.proposals p
WHERE p.id = x.proposal_id;

-- Temps de l'IFDD : le relevé, intervenants et organisations partenaires en présentation.
UPDATE l90 x
SET organization_id = legacy.map('public.organizations', x.porteur_v1::text),
    format = 'hybrid',
    titre = x.doc->'title',
    slug = x.fil_slug || '-' || rtrim(left(platform.slugify(x.doc->'title'->>'fr'), 120), '-'),
    presentation = NULLIF(jsonb_strip_nulls(jsonb_build_object(
        'fr', NULLIF(concat(i.fr, o.fr), ''),
        'en', NULLIF(concat(i.en, o.en), ''))), '{}'::jsonb)
FROM l90 y
LEFT JOIN LATERAL (
    SELECT string_agg(format('<p>%s : %s</p>', l.fr, g.noms), '' ORDER BY l.ordre) AS fr,
           string_agg(format('<p>%s: %s</p>', l.en, g.noms), '' ORDER BY l.ordre) AS en
    FROM (SELECT sp->>'role' AS role,
                 string_agg(pg_temp.html(sp->>'name_as_written'), ' ; ' ORDER BY n) AS noms
          FROM jsonb_array_elements(y.doc->'speakers') WITH ORDINALITY AS a(sp, n)
          GROUP BY 1) g
    JOIN (VALUES ('moderator', 'Modération', 'Moderation', 1), ('facilitator', 'Animation', 'Facilitation', 2),
                 ('speaker', 'Intervenants', 'Speakers', 3), ('panelist', 'Panélistes', 'Panellists', 4))
         AS l(role, fr, en, ordre) USING (role)
) i ON true
LEFT JOIN LATERAL (
    SELECT format('<p>Avec : %s</p>', string_agg(pg_temp.html(concat_ws(', ', og->>'name_as_written',
                                                          og->>'descriptor_as_written')), ' ; ' ORDER BY n)) AS fr,
           format('<p>With: %s</p>', string_agg(pg_temp.html(concat_ws(', ', og->>'name_as_written',
                                                         og->>'descriptor_as_written')), ' ; ' ORDER BY n)) AS en
    FROM jsonb_array_elements(y.doc->'organizations') WITH ORDINALITY AS a(og, n)
    WHERE og->>'role' = 'partner'
    HAVING count(*) > 0
) o ON true
WHERE y.session_id = x.session_id AND x.proposal_id IS NULL;

UPDATE l90 SET resume = doc->'summary' WHERE jsonb_typeof(doc->'summary') = 'object';

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM l90 WHERE organization_id IS NULL OR titre->>'fr' IS NULL
               OR slug IS NULL OR slug !~ '^[a-z0-9]+(-[a-z0-9]+)*$')
       OR EXISTS (SELECT 1 FROM l90 GROUP BY slug HAVING count(*) > 1)
       OR EXISTS (SELECT 1 FROM l90 x JOIN programme.sessions s
                  ON s.event_id = x.event_id AND s.slug::text = x.slug)
       OR EXISTS (SELECT 1 FROM l90 x JOIN programme.proposals p
                  ON p.event_id = x.event_id AND p.slug::text = x.slug AND p.id IS DISTINCT FROM x.proposal_id) THEN
        RAISE EXCEPTION 'lot 90 : porteur, titre ou slug manquant, ou slug déjà pris';
    END IF;
END $$;

-- Décisions : la journée a tenu ces activités.
UPDATE programme.proposals p
SET status = 'under_review',
    decision_reason = 'Activité tenue dans une journée spéciale de la CdP30 (reprise v1)'
FROM l90 x
WHERE p.id = x.proposal_id AND p.status = 'rejected';

UPDATE programme.proposals p
SET status = 'accepted', decision_reason = NULL, decided_by = NULL
FROM l90 x
WHERE p.id = x.proposal_id;

INSERT INTO programme.sessions (
    id, event_id, proposal_id, organization_id, sequence_number,
    title, slug, summary, description, status, format, starts_at, ends_at, timezone,
    listing_changed_at, published_at, created_by, created_at, updated_at)
SELECT session_id, event_id, proposal_id, organization_id, 1,
       titre, slug, resume, presentation, 'completed', format, starts_at, ends_at, timezone,
       now(), programme_published_at, platform.current_actor_id(), now(), now()
FROM l90
ORDER BY code, rang;

-- Thématiques seules, comme à la naissance d'une séance par l'API (lot 50).
INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order, created_at)
SELECT 'programme', 'sessions', x.session_id, et.term_id, et.sort_order, now()
FROM l90 x
JOIN reference.entity_terms et
  ON et.entity_schema = 'programme' AND et.entity_table = 'proposals' AND et.entity_id = x.proposal_id
JOIN reference.taxonomy_terms t ON t.id = et.term_id AND t.taxonomy_code = 'activity_theme';

INSERT INTO programme.session_tracks (session_id, track_id, sort_order, is_highlight, added_by, added_at)
SELECT session_id, track_id, rang, false, platform.current_actor_id(), now()
FROM l90;

-- Correspondance ----------------------------------------------------------------------

SELECT legacy.remember('code.journees-speciales', code, 'event', 'programme_tracks', track_id)
FROM l90_fil \g /dev/null

SELECT legacy.remember('code.journees-speciales', format('%s/%s', code, lpad(rang::text, 2, '0')),
                       'programme', 'sessions', session_id, 'migrated',
                       concat_ws(' ; ',
                           CASE WHEN activite_v1 IS NOT NULL THEN 'séance de l''activité v1 ' || activite_v1
                                ELSE 'temps de la journée sans activité v1' END,
                           note))
FROM l90 \g /dev/null

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
               AND event_type NOT IN ('programme.session.created', 'programme.proposal.accepted',
                                      'programme.proposal.under_review')) THEN
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
        SELECT f.code, count(x.*) AS seances,
               count(x.proposal_id) AS avec_proposition,
               min(x.starts_at AT TIME ZONE f.timezone)::time AS debut,
               max(x.ends_at AT TIME ZONE f.timezone)::time AS fin
        FROM l90_fil f JOIN l90 x USING (code) GROUP BY f.code ORDER BY f.code
    LOOP
        RAISE NOTICE 'lot 90 : % : % séances (% d''une activité v1, % de l''IFDD), % → %',
            r.code, r.seances, r.avec_proposition, r.seances - r.avec_proposition, r.debut, r.fin;
    END LOOP;
    RAISE NOTICE 'lot 90 : propositions acceptées %, séances publiques de la CdP30 %',
        (SELECT count(*) FROM programme.proposals p JOIN l90 x ON x.proposal_id = p.id WHERE p.status = 'accepted'),
        (SELECT count(*) FROM programme.sessions s JOIN l90_fil f ON f.event_id = s.event_id
         WHERE f.rang = 1 AND s.published_at IS NOT NULL);
    IF (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'code.journees-speciales')
       <> (SELECT count(*) FROM l90_fil) + (SELECT count(*) FROM l90) THEN
        RAISE EXCEPTION 'lot 90 : correspondance incomplète';
    END IF;
END $$;

COMMIT;
