-- Lot 40 : événements, journées du calendrier, appels à propositions.
-- Carte : docs/progression/reprise-v1/carte-reelle-coeur.md §§ 5 et 13. Dépend du
-- lot 20 (créateurs) et du lot 10 (pays, s'il les consigne).
--
-- Les deux CdP entrent dans la série « cop_climate » semée ; PACO est écarté (D9).
-- Le lieu et la salle naissent avec l'édition (event.tg_create_default_venue) ;
-- les journées sont créées ici comme l'application le fait à l'enregistrement
-- d'une édition, pour que les séances du lot 50 s'y rattachent.
-- Bannières : lot 60. Logo d'événement : phase 2 (aucun rôle event.logo).

\set ON_ERROR_STOP on
BEGIN;
DO $$ BEGIN
    PERFORM set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true);
    PERFORM set_config('app.request_id', 'reprise-v1-40', true);
END $$;

CREATE TEMP TABLE e40 ON COMMIT DROP AS
SELECT e.id AS v1_id,
       e.title ILIKE 'PACO%' AS paco,
       platform.uuid_v7() AS nouvel_id,
       e.year::smallint AS annee,
       btrim(e.title) AS titre,
       e.description,
       btrim(e.acronym) AS sigle_v1,
       -- Sigle v2 : lettres, chiffres et tiret seulement (préfixe des numéros de dossier).
       regexp_replace(btrim(e.acronym), '[^A-Za-z0-9-]', '', 'g') AS sigle,
       e.participation_mode::text AS mode,
       e.timezone,
       -- Du premier jour à 00:00 au dernier à 23:59, heure locale : 24:00 tomberait
       -- le lendemain et ajouterait une journée au calendrier de l'édition.
       (e.in_person_start_date::timestamp) AT TIME ZONE e.timezone AS debut,
       (e.in_person_end_date + time '23:59') AT TIME ZONE e.timezone AS fin,
       e.in_person_end_date AS dernier_jour,
       COALESCE(legacy.map('public.countries', e.country_id::text),
                (SELECT r.id FROM v1.countries c JOIN reference.countries r ON r.iso2 = upper(c.code)
                  WHERE c.id = e.country_id)) AS pays_id,
       btrim(e.city) AS ville,
       NULLIF(NULLIF(btrim(e.address), ''), 'Bientôt disponible') AS adresse,
       e.event_status::text AS statut_v1,
       e.submission_status::text AS depot_v1,
       e.submission_deadline,
       e.is_programmation_available,
       legacy.map('auth.users', e.created_by::text) AS cree_par,
       e.created_at,
       e.updated_at
FROM v1.events e;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM e40 WHERE NOT paco
               AND (sigle !~ '^CdP[0-9]+$' OR debut IS NULL OR fin IS NULL OR pays_id IS NULL)) THEN
        RAISE EXCEPTION 'Lot 40 : événement v1 hors des deux CdP attendues, ou incomplet';
    END IF;
END $$;

INSERT INTO event.events
    (id, series_id, edition_label, edition_year, title, acronym, slug, description, status,
     participation_mode, timezone, starts_at, ends_at, country_id, city, address,
     has_pavilion, programme_published_at, created_by, created_at, updated_at)
SELECT e.nouvel_id,
       (SELECT s.id FROM event.event_series s WHERE s.code = 'cop_climate'),
       e.sigle, e.annee,
       jsonb_build_object('fr', e.titre),
       e.sigle,
       platform.slugify(e.sigle || ' ' || e.ville || ' ' || e.annee),
       jsonb_build_object('fr', e.description),
       CASE WHEN e.statut_v1 = 'suspended' THEN 'suspended'
            WHEN e.statut_v1 = 'completed' OR e.dernier_jour < current_date THEN 'completed'
            WHEN e.statut_v1 = 'ongoing' THEN 'ongoing'
            ELSE 'announced' END::event.event_status,
       e.mode::event.participation_mode, e.timezone, e.debut, e.fin,
       e.pays_id, e.ville, e.adresse,
       -- La v1 n'a pas de colonne : les deux CdP ont un pavillon (carte § 5).
       true,
       -- Sans source : date de dernière modification de l'événement (carte § 13).
       CASE WHEN e.is_programmation_available THEN e.updated_at END,
       e.cree_par, e.created_at, e.updated_at
FROM e40 e
WHERE NOT e.paco;

INSERT INTO event.event_days (event_id, day_date, sort_order)
SELECT e.nouvel_id, j::date, (row_number() OVER (PARTITION BY e.nouvel_id ORDER BY j) - 1)::smallint
FROM e40 e
CROSS JOIN LATERAL generate_series((e.debut AT TIME ZONE e.timezone)::date,
                                   (e.fin AT TIME ZONE e.timezone)::date,
                                   interval '1 day') AS j
WHERE NOT e.paco;

-- Un appel par édition à pavillon (règle 5). Ouverture sans source : création de
-- l'événement (carte § 13). Notation v1 sur 20, sans grille de critères.
-- Fenêtre du pavillon : 9 h à 18 h, la grille v1 courant jusqu'à 17 h 45.
INSERT INTO event.calls_for_proposals
    (event_id, code, title, status, opens_at, closes_at, uses_scoring_grid,
     daily_start_time, daily_end_time, created_by, created_at, updated_at)
SELECT e.nouvel_id,
       lower(e.sigle),
       jsonb_build_object('fr', 'Appel à propositions d''événements parallèles, ' || e.sigle_v1),
       CASE WHEN e.submission_deadline > now() AND e.depot_v1 = 'open' THEN 'open'
            WHEN e.submission_deadline > now() THEN 'closed'
            WHEN e.is_programmation_available THEN 'published'
            ELSE 'under_review' END::event.call_status,
       e.created_at,
       e.submission_deadline,
       false,
       '09:00', '18:00',
       e.cree_par, e.created_at, e.updated_at
FROM e40 e
WHERE NOT e.paco;

SELECT count(*) AS consignees FROM (
SELECT legacy.remember(
    'public.events', e.v1_id::text, 'event', 'events',
    CASE WHEN e.paco THEN NULL ELSE e.nouvel_id END,
    CASE WHEN e.paco THEN 'skipped' ELSE 'migrated' END,
    CASE WHEN e.paco THEN 'PACO (D9) : non repris, reste sur Supabase'
         ELSE concat_ws(' ; ',
             'ouverture de l''appel = création de l''événement (sans source)',
             CASE WHEN e.is_programmation_available
                  THEN 'publication du programme = dernière modification de l''événement (sans source)' END,
             CASE WHEN e.sigle <> e.sigle_v1 THEN 'sigle « ' || e.sigle_v1 || ' » écrit ' || e.sigle END,
             'logo d''événement : phase 2, rôle de média absent') END)
FROM e40 e
) x;

-- Compte rendu.
DO $$
DECLARE
    v_extraites bigint;
    v_rendues bigint;
BEGIN
    SELECT count(*) INTO v_extraites FROM v1.events;
    SELECT count(*) INTO v_rendues FROM legacy.id_map WHERE legacy_table = 'public.events';
    IF v_extraites <> v_rendues THEN
        RAISE EXCEPTION 'Lot 40 : public.events extraites %, consignées %', v_extraites, v_rendues;
    END IF;
    IF EXISTS (SELECT 1 FROM legacy.id_map m
               WHERE m.legacy_table = 'public.events' AND m.disposition = 'migrated'
                 AND (NOT EXISTS (SELECT 1 FROM event.calls_for_proposals c WHERE c.event_id = m.target_id)
                      OR NOT EXISTS (SELECT 1 FROM event.rooms r JOIN event.venues v ON v.id = r.venue_id
                                      WHERE v.event_id = m.target_id))) THEN
        RAISE EXCEPTION 'Lot 40 : édition sans appel ou sans salle';
    END IF;
    RAISE NOTICE 'public.events : % extraites, % reprises, % écartées ; journées %, appels %', v_extraites,
        (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'public.events' AND disposition = 'migrated'),
        (SELECT count(*) FROM legacy.id_map WHERE legacy_table = 'public.events' AND disposition = 'skipped'),
        (SELECT count(*) FROM event.event_days d JOIN legacy.id_map m
            ON m.target_id = d.event_id AND m.legacy_table = 'public.events'),
        (SELECT count(*) FROM event.calls_for_proposals c JOIN legacy.id_map m
            ON m.target_id = c.event_id AND m.legacy_table = 'public.events');
END $$;

COMMIT;
