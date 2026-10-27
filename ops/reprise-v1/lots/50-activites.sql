-- Lot 50 : activités (public.activities, activity_speakers, activity_documents).
--
-- Choix retenus (carte du cœur § 13, sauf avis contraire) :
--   - `validation_status` mêle décision et déroulé : la décision va sur la
--     proposition (approved, live, completed → accepted), le déroulé sur la
--     séance (completed, édition passée) ;
--   - acceptée puis rejetée : rejected, sans séance ;
--   - jamais décidée : gardée dans son état (submitted, under_review) ;
--   - créneau sans acceptation : séance `planned` non publiée liée à la
--     proposition si l'édition est à venir (COP31) ; édition passée, aucune
--     séance, le créneau reste dans v1.activity_modifications (phase 2 : journal d'audit) ;
--   - historique : proposal_transitions reconstitué depuis activity_modifications
--     (état initial `submitted` à la création, répétitions fusionnées) ;
--   - intervenants : une personne par adresse ET nom. Une ligne dont le nom
--     diffère du titulaire de l'adresse exige une personne sans adresse (D16,
--     extension du modèle) : écartée, phase 2.
-- Documents : leur fichier n'existe qu'au lot 60 (media.assets exige empreinte
-- et taille). Ce lot prépare legacy.documents_activites, que le lot 60 lit pour
-- créer asset, proposal_documents et la ligne id_map de public.activity_documents.
--
-- Effets de bord : aucune inscription, aucune séance diffusée ; l'insertion
-- d'une séance émet `programme.session.created` (sans effet ici : ni inscrit,
-- ni diffusion). Les événements d'outbox du lot sont marqués publiés en fin de
-- lot, le worker ne les verra pas.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-50', true);

-- Garde-fous ------------------------------------------------------------------

-- Le HTML v1 est rendu tel quel par le site (l'API n'assainit qu'à l'écriture) :
-- un motif exécutable arrêterait la reprise plutôt que d'être publié.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM v1.activities a,
             LATERAL (VALUES (a.title), (a.objectives), (a.detailed_presentation)) AS t(x)
        WHERE t.x ~* '<\s*(script|iframe|object|embed|style|form|input|svg|math|link|meta|base|img)'
           OR t.x ~* '\son[a-z]+\s*='
           OR t.x ~* '(javascript|vbscript|data)\s*:'
    ) THEN
        RAISE EXCEPTION 'lot 50 : HTML exécutable dans une activité v1, assainir avant reprise';
    END IF;

    IF EXISTS (SELECT 1 FROM unnest(enum_range(NULL::v1.activity_categories)) c
               WHERE NOT EXISTS (SELECT 1 FROM reference.taxonomy_terms t
                                 WHERE t.taxonomy_code = 'activity_category' AND t.code = c::text)) THEN
        RAISE EXCEPTION 'lot 50 : catégorie v1 sans terme activity_category';
    END IF;

    IF EXISTS (SELECT 1 FROM v1.activities a, unnest(a.main_themes) th
               WHERE legacy.map('public.activity_theme', th::text) IS NULL)
    OR EXISTS (SELECT 1 FROM v1.activities a
               WHERE legacy.map('public.activity_type', a.activity_type::text) IS NULL) THEN
        RAISE EXCEPTION 'lot 50 : thématique ou type d''activité sans terme (lot 10)';
    END IF;
END $$;

-- Activités -------------------------------------------------------------------

CREATE TEMP TABLE l50_act ON COMMIT DROP AS
SELECT a.id                                              AS legacy_id,
       platform.uuid_v7()                                AS proposal_id,
       legacy.map('public.events', a.event_id::text)     AS event_id,
       (SELECT m.note FROM legacy.id_map m
         WHERE m.legacy_table = 'public.events' AND m.legacy_id = a.event_id::text) AS event_note,
       legacy.map('public.organizations', a.organization_id::text) AS organization_id,
       legacy.map('auth.users', a.submitted_by::text)    AS submitted_by,
       legacy.map('public.countries', a.country_id::text) AS country_id,
       legacy.map('auth.users', a.deleted_by::text)      AS deleted_by,
       CASE a.validation_status::text
           WHEN 'approved'  THEN 'accepted'
           WHEN 'live'      THEN 'accepted'
           WHEN 'completed' THEN 'accepted'
           ELSE a.validation_status::text
       END                                               AS statut,
       a.validation_status::text                         AS statut_v1,
       a.title, a.objectives, a.detailed_presentation, a.format::text AS format,
       a.proposed_start_date, a.proposed_end_date, a.final_start_date, a.final_end_date,
       a.activites_view_count, a.tags,
       CASE WHEN a.is_deleted THEN COALESCE(a.deleted_at, a.updated_at) END AS deleted_at,
       CASE WHEN a.is_deleted THEN a.deleted_reason END  AS deleted_reason,
       a.created_at, COALESCE(a.updated_at, a.created_at) AS updated_at,
       NULL::uuid        AS call_id,
       NULL::text        AS timezone,
       NULL::boolean     AS edition_passee,
       NULL::timestamptz AS programme_publie_le,
       NULL::text        AS slug,
       NULL::timestamptz AS decided_at,
       NULL::uuid        AS decided_by,
       NULL::text        AS seance,
       NULL::uuid        AS session_id,
       NULL::text[]      AS notes
FROM v1.activities a;

UPDATE l50_act x
SET call_id = (SELECT c.id FROM event.calls_for_proposals c
                WHERE c.event_id = x.event_id ORDER BY c.created_at LIMIT 1),
    timezone = e.timezone,
    edition_passee = e.ends_at < now(),
    programme_publie_le = e.programme_published_at
FROM event.events e
WHERE e.id = x.event_id;

DO $$
DECLARE n integer;
BEGIN
    SELECT count(*) INTO n FROM l50_act WHERE event_id IS NOT NULL AND organization_id IS NULL;
    IF n > 0 THEN
        RAISE EXCEPTION 'lot 50 : % activité(s) d''un porteur non repris (lot 30)', n;
    END IF;
    SELECT count(*) INTO n FROM l50_act WHERE event_id IS NOT NULL AND submitted_by IS NULL;
    IF n > 0 THEN
        RAISE EXCEPTION 'lot 50 : % activité(s) d''un déposant non repris (lot 20)', n;
    END IF;
    SELECT count(*) INTO n FROM l50_act WHERE event_id IS NOT NULL AND call_id IS NULL;
    IF n > 0 THEN
        RAISE NOTICE 'lot 50 : % activité(s) sans appel sur leur édition, reprises hors appel', n;
    END IF;
    IF EXISTS (SELECT 1 FROM l50_act WHERE event_id IS NOT NULL AND statut = 'accepted'
                                       AND NOT edition_passee) THEN
        RAISE EXCEPTION 'lot 50 : activité acceptée sur une édition à venir, règle de séance à écrire';
    END IF;
END $$;

-- Historique : état initial à la création (la v1 n'a pas de brouillon), puis
-- chaque changement de validation_status, répétitions consécutives fusionnées.
CREATE TEMP TABLE l50_hist ON COMMIT DROP AS
WITH brut AS (
    SELECT x.legacy_id, x.created_at AS occurred_at, 0 AS ordre, ''::text AS tri,
           'submitted'::text AS statut, x.submitted_by AS actor_id
    FROM l50_act x
    UNION ALL
    SELECT m.activity_id, m.modified_at, 1, m.id::text,
           CASE m.new_value #>> '{}'
               WHEN 'approved'  THEN 'accepted'
               WHEN 'live'      THEN 'accepted'
               WHEN 'completed' THEN 'accepted'
               ELSE m.new_value #>> '{}'
           END,
           legacy.map('auth.users', m.modified_by::text)
    FROM v1.activity_modifications m
    WHERE m.field_name = 'validation_status'
), suite AS (
    SELECT b.*, lag(statut) OVER (PARTITION BY legacy_id ORDER BY occurred_at, ordre, tri) AS precedent
    FROM brut b
)
SELECT legacy_id, occurred_at, ordre, tri, precedent AS from_status, statut AS to_status, actor_id
FROM suite
WHERE precedent IS DISTINCT FROM statut;

-- L'historique doit finir sur l'état final ; sinon, l'état final est daté de la
-- dernière modification de l'activité.
INSERT INTO l50_hist (legacy_id, occurred_at, ordre, tri, from_status, to_status, actor_id)
SELECT x.legacy_id, x.updated_at, 2, '', d.to_status, x.statut, NULL
FROM l50_act x
JOIN LATERAL (SELECT h.to_status FROM l50_hist h WHERE h.legacy_id = x.legacy_id
              ORDER BY h.occurred_at DESC, h.ordre DESC, h.tri DESC LIMIT 1) d ON true
WHERE d.to_status <> x.statut;

UPDATE l50_act x
SET decided_at = d.occurred_at, decided_by = d.actor_id
FROM (SELECT DISTINCT ON (legacy_id) legacy_id, to_status, occurred_at, actor_id
      FROM l50_hist ORDER BY legacy_id, occurred_at DESC, ordre DESC, tri DESC) dernier
JOIN LATERAL (SELECT h.occurred_at, h.actor_id FROM l50_hist h
              WHERE h.legacy_id = dernier.legacy_id AND h.to_status = dernier.to_status
              ORDER BY h.occurred_at DESC, h.ordre DESC, h.tri DESC LIMIT 1) d ON true
WHERE dernier.legacy_id = x.legacy_id
  AND x.statut IN ('accepted', 'rejected');

-- Séance : déroulé d'une acceptée (édition passée), ou créneau pressenti d'un
-- dossier non décidé d'une édition à venir.
UPDATE l50_act
SET seance = CASE
        WHEN statut = 'accepted' AND final_start_date IS NOT NULL THEN 'completed'
        WHEN statut IN ('submitted', 'under_review') AND final_start_date IS NOT NULL
             AND NOT edition_passee THEN 'planned'
    END
WHERE event_id IS NOT NULL;

UPDATE l50_act SET session_id = platform.uuid_v7() WHERE seance IS NOT NULL;

-- Adresse : titre normalisé, suffixe -2, -3 pour les homonymes d'une édition.
WITH base AS (
    SELECT legacy_id, event_id,
           COALESCE(NULLIF(rtrim(left(platform.slugify(title), 150), '-'), ''), 'activite') AS b
    FROM l50_act WHERE event_id IS NOT NULL
), rang AS (
    SELECT legacy_id, b, row_number() OVER (PARTITION BY event_id, b ORDER BY legacy_id) AS n
    FROM base
)
UPDATE l50_act x
SET slug = CASE WHEN r.n = 1 THEN r.b ELSE r.b || '-' || r.n END
FROM rang r WHERE r.legacy_id = x.legacy_id;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM l50_act x WHERE x.event_id IS NOT NULL
               GROUP BY x.event_id, x.slug HAVING count(*) > 1)
    OR EXISTS (SELECT 1 FROM l50_act x JOIN programme.proposals p
                 ON p.event_id = x.event_id AND p.slug::text = x.slug)
    OR EXISTS (SELECT 1 FROM l50_act x JOIN programme.sessions s
                 ON s.event_id = x.event_id AND s.slug::text = x.slug WHERE x.seance IS NOT NULL) THEN
        RAISE EXCEPTION 'lot 50 : adresse d''activité en collision';
    END IF;
END $$;

UPDATE l50_act SET notes = array_remove(ARRAY[
        CASE WHEN statut = 'rejected' AND EXISTS (
                SELECT 1 FROM l50_hist h WHERE h.legacy_id = l50_act.legacy_id AND h.to_status = 'accepted')
             THEN 'acceptée puis rejetée : rejected, sans séance' END,
        CASE WHEN statut_v1 IN ('live', 'completed', 'approved') AND seance = 'completed'
             THEN 'v1 ' || statut_v1 || ' : proposition accepted, séance completed' END,
        CASE WHEN statut IN ('submitted', 'under_review') AND final_start_date IS NOT NULL AND seance IS NULL
             THEN 'créneau sans acceptation, édition passée : pas de séance (phase 2, journal d''audit)' END,
        CASE WHEN seance = 'planned' THEN 'créneau sans acceptation : séance planned non publiée' END,
        CASE WHEN proposed_end_date <= proposed_start_date
             THEN 'créneau souhaité inversé : fin mise à nul' END,
        CASE WHEN cardinality(tags) > 0 THEN 'mots-clés libres non repris (phase 2)' END
    ], NULL)
WHERE event_id IS NOT NULL;

-- Propositions ----------------------------------------------------------------

-- Le contrôle de recevabilité refuse un dossier `submitted` hors de la fenêtre
-- d'un appel clos : il juge un dépôt d'aujourd'hui, pas un historique.
ALTER TABLE programme.proposals DISABLE TRIGGER tg_proposals_check_eligibility;

INSERT INTO programme.proposals (
    id, call_id, event_id, organization_id, submitted_by,
    title, slug, objectives, detailed_presentation, format, country_id,
    preferred_start_at, preferred_end_at, duration_minutes,
    status, submitted_at, decided_at, decided_by, view_count,
    deleted_at, deleted_by, deleted_reason, created_at, updated_at)
SELECT x.proposal_id, x.call_id, x.event_id, x.organization_id, x.submitted_by,
       jsonb_build_object('fr', btrim(x.title)), x.slug,
       jsonb_build_object('fr', x.objectives), jsonb_build_object('fr', x.detailed_presentation),
       x.format::event.participation_mode, x.country_id,
       x.proposed_start_date,
       CASE WHEN x.proposed_end_date > x.proposed_start_date THEN x.proposed_end_date END,
       CASE WHEN extract(epoch FROM x.proposed_end_date - x.proposed_start_date) / 60 BETWEEN 15 AND 600
            THEN (extract(epoch FROM x.proposed_end_date - x.proposed_start_date) / 60)::smallint END,
       x.statut::programme.proposal_status, x.created_at, x.decided_at, x.decided_by,
       LEAST(x.activites_view_count, 2147483647)::integer,
       x.deleted_at, CASE WHEN x.deleted_at IS NOT NULL THEN x.deleted_by END, x.deleted_reason,
       x.created_at, x.updated_at
FROM l50_act x
WHERE x.event_id IS NOT NULL
ORDER BY x.created_at, x.legacy_id;

ALTER TABLE programme.proposals ENABLE TRIGGER tg_proposals_check_eligibility;

-- Le porteur est posé par tg_proposals_sync_lead_organization, daté d'aujourd'hui.
UPDATE programme.proposal_organizations po
SET added_at = x.created_at, confirmed_at = x.created_at
FROM l50_act x WHERE po.proposal_id = x.proposal_id AND po.role = 'lead';

-- L'état initial journalisé par tg_proposals_log_initial_status est remplacé
-- par l'historique daté.
DELETE FROM programme.proposal_transitions t
USING l50_act x WHERE t.proposal_id = x.proposal_id;

INSERT INTO programme.proposal_transitions (proposal_id, from_status, to_status, actor_id, occurred_at)
SELECT x.proposal_id, h.from_status::programme.proposal_status, h.to_status::programme.proposal_status,
       h.actor_id, h.occurred_at
FROM l50_hist h
JOIN l50_act x ON x.legacy_id = h.legacy_id AND x.event_id IS NOT NULL
ORDER BY h.legacy_id, h.occurred_at, h.ordre, h.tri;

-- Thématiques, catégories, type d'activité.
INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order, created_at)
SELECT 'programme', 'proposals', x.proposal_id, t.term_id, min(t.rang)::smallint, x.created_at
FROM l50_act x
JOIN v1.activities a ON a.id = x.legacy_id
CROSS JOIN LATERAL (
    SELECT legacy.map('public.activity_theme', th::text), r FROM unnest(a.main_themes) WITH ORDINALITY AS u(th, r)
    UNION ALL
    SELECT tt.id, 100 + r FROM unnest(a.categories) WITH ORDINALITY AS u(c, r)
    JOIN reference.taxonomy_terms tt ON tt.taxonomy_code = 'activity_category' AND tt.code = c::text
    UNION ALL
    SELECT legacy.map('public.activity_type', a.activity_type::text), 200
) AS t(term_id, rang)
WHERE x.event_id IS NOT NULL
GROUP BY x.proposal_id, t.term_id, x.created_at;

-- Séances ---------------------------------------------------------------------

INSERT INTO programme.sessions (
    id, event_id, proposal_id, organization_id, sequence_number,
    title, slug, description, status, format, starts_at, ends_at, timezone,
    listing_changed_at, published_at, view_count, created_by, created_at, updated_at)
SELECT x.session_id, x.event_id, x.proposal_id, x.organization_id, 1,
       jsonb_build_object('fr', btrim(x.title)), x.slug, jsonb_build_object('fr', x.detailed_presentation),
       x.seance::programme.session_status, x.format::event.participation_mode,
       x.final_start_date, x.final_end_date, x.timezone,
       COALESCE(c.dernier, x.updated_at),
       CASE WHEN x.seance = 'completed' THEN COALESCE(x.programme_publie_le, x.decided_at, x.updated_at) END,
       CASE WHEN x.seance = 'completed' THEN LEAST(x.activites_view_count, 2147483647)::integer ELSE 0 END,
       COALESCE(x.decided_by, platform.current_actor_id()),
       COALESCE(c.premier, x.decided_at, x.updated_at),
       GREATEST(COALESCE(c.dernier, x.updated_at), COALESCE(c.premier, x.decided_at, x.updated_at))
FROM l50_act x
LEFT JOIN LATERAL (
    SELECT min(m.modified_at) AS premier, max(m.modified_at) AS dernier
    FROM v1.activity_modifications m
    WHERE m.activity_id = x.legacy_id AND m.field_name IN ('final_start_date', 'final_end_date')
) c ON true
WHERE x.seance IS NOT NULL
ORDER BY x.created_at, x.legacy_id;

UPDATE programme.session_organizations so
SET added_at = s.created_at
FROM programme.sessions s
JOIN l50_act x ON x.session_id = s.id
WHERE so.session_id = s.id;

-- Thématiques seules, comme à la naissance d'une séance par l'API.
INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order, created_at)
SELECT 'programme', 'sessions', x.session_id, et.term_id, et.sort_order, et.created_at
FROM l50_act x
JOIN reference.entity_terms et
  ON et.entity_schema = 'programme' AND et.entity_table = 'proposals' AND et.entity_id = x.proposal_id
JOIN reference.taxonomy_terms t ON t.id = et.term_id AND t.taxonomy_code = 'activity_theme'
WHERE x.session_id IS NOT NULL;

-- Intervenants ----------------------------------------------------------------

CREATE TEMP TABLE l50_int ON COMMIT DROP AS
SELECT s.id AS legacy_id, s.activity_id, x.proposal_id, x.session_id,
       x.event_id IS NULL AS activite_ecartee, x.event_note,
       lower(btrim(s.email)) AS email,
       btrim(s.first_name) AS prenom, btrim(s.last_name) AS nom,
       CASE lower(btrim(s.civility))
           WHEN 'mr' THEN 'm' WHEN 'm.' THEN 'm' WHEN 'm' THEN 'm' WHEN 'm. v2' THEN 'm'
           WHEN 'mrs' THEN 'mme' WHEN 'mme' THEN 'mme' WHEN 'ms' THEN 'mme'
           WHEN 'dr' THEN 'dr' WHEN 'dr.' THEN 'dr'
           WHEN 'prof' THEN 'pr'
       END AS civilite,
       NULLIF(btrim(s.position), '') AS fonction,
       NULLIF(btrim(s.organization), '') AS organisation,
       COALESCE(s.is_available_for_questions, true) AS disponible,
       s.confirmation_email_sent_at,
       COALESCE(s.created_at, x.created_at) AS created_at,
       COALESCE(replace(platform.normalize_label(s.first_name), ' ', ''), '') AS nf,
       COALESCE(replace(platform.normalize_label(s.last_name), ' ', ''), '') AS nl,
       COALESCE(platform.normalize_label(s.first_name || ' ' || s.last_name), '') AS nom_n,
       row_number() OVER (PARTITION BY s.activity_id ORDER BY s.created_at, s.id) AS rang,
       NULL::uuid AS titulaire,
       NULL::text AS t_nf,
       NULL::text AS t_nl,
       NULL::text AS t_nom_n,
       NULL::boolean AS titulaire_cree,
       NULL::boolean AS meme_nom,
       NULL::uuid AS speaker_id,
       NULL::text AS disposition,
       NULL::text AS note
FROM v1.activity_speakers s
JOIN l50_act x ON x.legacy_id = s.activity_id;

-- Titulaire déjà connu : personne (compte repris, production) portant l'adresse.
UPDATE l50_int i
SET titulaire = p.id, titulaire_cree = false,
    t_nf = COALESCE(replace(platform.normalize_label(p.first_name), ' ', ''), ''),
    t_nl = COALESCE(replace(platform.normalize_label(p.last_name), ' ', ''), ''),
    t_nom_n = COALESCE(platform.normalize_label(p.first_name || ' ' || p.last_name), '')
FROM identity.people p
WHERE p.primary_email = i.email::citext AND p.status <> 'anonymized'
  AND NOT i.activite_ecartee;

UPDATE l50_int i
SET titulaire = p.id, titulaire_cree = false,
    t_nf = COALESCE(replace(platform.normalize_label(p.first_name), ' ', ''), ''),
    t_nl = COALESCE(replace(platform.normalize_label(p.last_name), ' ', ''), ''),
    t_nom_n = COALESCE(platform.normalize_label(p.first_name || ' ' || p.last_name), '')
FROM identity.person_emails pe
JOIN identity.people p ON p.id = pe.person_id AND p.status <> 'anonymized'
WHERE pe.email = i.email::citext AND i.titulaire IS NULL
  AND NOT i.activite_ecartee;

-- Adresse inconnue : son titulaire est le nom qu'elle porte le plus souvent
-- (à égalité, le plus ancien), créé en personne sans compte.
CREATE TEMP TABLE l50_nouveaux ON COMMIT DROP AS
WITH groupes AS (
    SELECT email, nf || nl AS cle, count(*) AS n, min(created_at) AS premier,
           (array_agg(legacy_id ORDER BY created_at, legacy_id))[1] AS representant
    FROM l50_int
    WHERE titulaire IS NULL AND NOT activite_ecartee
    GROUP BY email, nf || nl
)
SELECT DISTINCT ON (g.email) platform.uuid_v7() AS person_id, g.email, g.representant
FROM groupes g
ORDER BY g.email, g.n DESC, g.premier, g.representant;

UPDATE l50_int i
SET titulaire = n.person_id, titulaire_cree = true, t_nf = r.nf, t_nl = r.nl, t_nom_n = r.nom_n
FROM l50_nouveaux n
JOIN l50_int r ON r.legacy_id = n.representant
WHERE i.email = n.email AND i.titulaire IS NULL AND NOT i.activite_ecartee;

-- Même personne : même nom à l'espacement près, prénom et nom inversés, l'un
-- des noms contenu dans l'autre (prénom composé, second nom de famille), faute
-- de frappe (trigrammes), ou prénom abrégé (« J. » pour « Jean »).
UPDATE l50_int
SET meme_nom = (nf || nl = t_nf || t_nl)
            OR (nl || nf = t_nf || t_nl)
            OR (least(cardinality(string_to_array(nom_n, ' ')), cardinality(string_to_array(t_nom_n, ' '))) >= 2
                AND (string_to_array(nom_n, ' ') <@ string_to_array(t_nom_n, ' ')
                     OR string_to_array(t_nom_n, ' ') <@ string_to_array(nom_n, ' ')))
            OR similarity(nom_n, t_nom_n) >= 0.75
            OR (nl = t_nl AND nl <> '' AND left(nf, 1) = left(t_nf, 1)
                AND (nf LIKE t_nf || '%' OR t_nf LIKE nf || '%'))
WHERE titulaire IS NOT NULL;

INSERT INTO identity.people (id, primary_email, first_name, last_name, civility, status,
                             created_at, updated_at)
SELECT n.person_id, n.email, r.prenom, r.nom,
       COALESCE(r.civilite, (SELECT i.civilite FROM l50_int i
                             WHERE i.titulaire = n.person_id AND i.meme_nom AND i.civilite IS NOT NULL
                             ORDER BY i.created_at LIMIT 1)),
       'active', m.premier, m.premier
FROM l50_nouveaux n
JOIN l50_int r ON r.legacy_id = n.representant
CROSS JOIN LATERAL (SELECT min(i.created_at) AS premier FROM l50_int i
                    WHERE i.titulaire = n.person_id AND i.meme_nom) m
ORDER BY m.premier, n.email;

-- Une même personne deux fois dans une activité : une seule ligne, la première.
WITH retenues AS (
    SELECT legacy_id,
           row_number() OVER (PARTITION BY activity_id, titulaire ORDER BY rang) AS occurrence
    FROM l50_int WHERE meme_nom
)
UPDATE l50_int i
SET disposition = CASE WHEN r.occurrence = 1 THEN 'migrated' ELSE 'merged' END,
    speaker_id = CASE WHEN r.occurrence = 1 THEN platform.uuid_v7() END
FROM retenues r WHERE r.legacy_id = i.legacy_id;

UPDATE l50_int i
SET speaker_id = d.speaker_id, note = 'même personne deux fois dans l''activité'
FROM l50_int d
WHERE i.disposition = 'merged' AND d.disposition = 'migrated'
  AND d.activity_id = i.activity_id AND d.titulaire = i.titulaire;

UPDATE l50_int
SET disposition = 'skipped',
    note = CASE WHEN activite_ecartee THEN 'activité écartée : ' || COALESCE(event_note, 'édition non reprise')
                ELSE 'D16, phase 2 : adresse portée par une autre personne' END
WHERE disposition IS NULL;

INSERT INTO programme.proposal_speakers (
    id, proposal_id, person_id, role, job_title_snapshot, organization_snapshot,
    confirmation_sent_at, is_available_for_questions, sort_order, created_at)
SELECT i.speaker_id, i.proposal_id, i.titulaire, 'speaker', i.fonction, i.organisation,
       i.confirmation_email_sent_at, i.disponible,
       (row_number() OVER (PARTITION BY i.proposal_id ORDER BY i.rang) - 1)::smallint,
       i.created_at
FROM l50_int i
WHERE i.disposition = 'migrated';

INSERT INTO programme.session_speakers (
    session_id, person_id, role, job_title_snapshot, organization_snapshot, sort_order, created_at)
SELECT x.session_id, ps.person_id, ps.role, ps.job_title_snapshot, ps.organization_snapshot,
       ps.sort_order, ps.created_at
FROM l50_act x
JOIN programme.proposal_speakers ps ON ps.proposal_id = x.proposal_id
WHERE x.session_id IS NOT NULL;

-- Documents : en attente de leur fichier (lot 60) ------------------------------

CREATE TABLE legacy.documents_activites (
    legacy_id          text PRIMARY KEY,   -- public.activity_documents.id
    proposal_id        uuid,               -- nul : activité écartée
    note_ecart         text,
    title              jsonb NOT NULL,
    document_type_code text,
    is_public          boolean NOT NULL,
    uploaded_by        uuid,
    uploaded_at        timestamptz,
    sort_order         smallint NOT NULL,
    file_url           text NOT NULL,
    file_type          text
);
COMMENT ON TABLE legacy.documents_activites IS
    'Lot 50 → lot 60 : documents d''activité prêts à rattacher. Le lot 60 crée l''asset, la ligne programme.proposal_documents et la ligne id_map de public.activity_documents.';

INSERT INTO legacy.documents_activites
SELECT d.id::text, CASE WHEN x.event_id IS NOT NULL THEN x.proposal_id END,
       CASE WHEN x.event_id IS NULL THEN 'activité écartée : ' || COALESCE(x.event_note, 'édition non reprise') END,
       jsonb_build_object('fr', btrim(d.title)),
       CASE d.types[1]
           WHEN 'autre' THEN 'other'
           WHEN 'presentation' THEN 'presentation'
           WHEN 'report' THEN 'report'
           WHEN 'additional_resource' THEN 'additional_resource'
           ELSE 'other'
       END,
       true,
       legacy.map('auth.users', d.uploaded_by::text),
       d.uploaded_at,
       (row_number() OVER (PARTITION BY d.activity_id ORDER BY d.uploaded_at, d.id) - 1)::smallint,
       d.file_url, d.file_type
FROM v1.activity_documents d
JOIN l50_act x ON x.legacy_id = d.activity_id;

-- Correspondance --------------------------------------------------------------

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table, target_id, disposition, note)
SELECT 'public.activities', x.legacy_id::text, 'programme', 'proposals',
       CASE WHEN x.event_id IS NOT NULL THEN x.proposal_id END,
       CASE WHEN x.event_id IS NOT NULL THEN 'migrated' ELSE 'skipped' END,
       CASE WHEN x.event_id IS NULL THEN 'édition écartée : ' || COALESCE(x.event_note, 'non reprise')
            ELSE NULLIF(array_to_string(x.notes, ' ; '), '') END
FROM l50_act x;

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table, target_id, disposition, note)
SELECT 'public.activity_speakers', i.legacy_id::text, 'programme', 'proposal_speakers',
       i.speaker_id, i.disposition,
       CASE WHEN i.disposition = 'migrated' AND i.titulaire_cree THEN NULL
            WHEN i.disposition = 'migrated' THEN 'personne existante, rapprochée par adresse et nom'
            ELSE i.note END
FROM l50_int i;

-- Outbox : seules les séances émettent à l'insertion. Rien à relayer : ni
-- inscrit (rappels), ni séance diffusée (Zoom).
DO $$
DECLARE r record;
BEGIN
    FOR r IN SELECT event_type, count(*) AS n FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-50' GROUP BY event_type ORDER BY event_type
    LOOP
        RAISE NOTICE 'lot 50 : outbox % : % événement(s), marqués publiés', r.event_type, r.n;
    END LOOP;
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-50'
               AND event_type <> 'programme.session.created') THEN
        RAISE EXCEPTION 'lot 50 : événement d''outbox inattendu';
    END IF;
    IF EXISTS (SELECT 1 FROM programme.sessions s JOIN l50_act x ON x.session_id = s.id
               WHERE s.is_streamed OR s.status IN ('scheduled', 'live')) THEN
        RAISE EXCEPTION 'lot 50 : séance diffusée ou programmée, Zoom la créerait';
    END IF;
END $$;

UPDATE platform.outbox_events SET published_at = now()
WHERE correlation_id = 'reprise-v1-50' AND published_at IS NULL;

-- Compte rendu ----------------------------------------------------------------

DO $$
DECLARE
    r record;
BEGIN
    FOR r IN
        SELECT s.t, s.extraites,
               count(m.*) FILTER (WHERE m.disposition = 'migrated') AS migrated,
               count(m.*) FILTER (WHERE m.disposition = 'merged')   AS merged,
               count(m.*) FILTER (WHERE m.disposition = 'skipped')  AS skipped
        FROM (VALUES
            ('public.activities',       (SELECT count(*) FROM v1.activities)),
            ('public.activity_speakers', (SELECT count(*) FROM v1.activity_speakers))
        ) AS s(t, extraites)
        LEFT JOIN legacy.id_map m ON m.legacy_table = s.t
        GROUP BY s.t, s.extraites ORDER BY s.t
    LOOP
        RAISE NOTICE 'lot 50 : % extraites %, migrated %, merged %, skipped %',
            r.t, r.extraites, r.migrated, r.merged, r.skipped;
        IF r.extraites <> r.migrated + r.merged + r.skipped THEN
            RAISE EXCEPTION 'lot 50 : % ne rend pas compte de toutes ses lignes', r.t;
        END IF;
    END LOOP;

    IF (SELECT count(*) FROM legacy.documents_activites) <> (SELECT count(*) FROM v1.activity_documents) THEN
        RAISE EXCEPTION 'lot 50 : documents d''activité incomplets dans legacy.documents_activites';
    END IF;
    RAISE NOTICE 'lot 50 : public.activity_documents %, en attente du lot 60',
        (SELECT count(*) FROM legacy.documents_activites);

    FOR r IN
        SELECT x.statut_v1, p.status::text AS v2, s.status::text AS seance, count(*) AS n
        FROM l50_act x
        LEFT JOIN programme.proposals p ON p.id = x.proposal_id
        LEFT JOIN programme.sessions s ON s.id = x.session_id
        GROUP BY 1, 2, 3 ORDER BY 1, 2, 3
    LOOP
        RAISE NOTICE 'lot 50 : v1 % → proposition %, séance % : %',
            r.statut_v1, COALESCE(r.v2, 'écartée'), COALESCE(r.seance, 'aucune'), r.n;
    END LOOP;

    RAISE NOTICE 'lot 50 : personnes créées pour des intervenants : %', (SELECT count(*) FROM l50_nouveaux);
END $$;

COMMIT;
