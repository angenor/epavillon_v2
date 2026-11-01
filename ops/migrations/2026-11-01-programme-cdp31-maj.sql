-- Programme de la CdP31 : mise à jour demandée par le commanditaire (01/11).
-- Données seules, aucun schéma touché. Toutes les heures en heure d'Antalya (+03).
--
-- Usage (une seule transaction, s'arrête à la première erreur) :
--   psql -v ON_ERROR_STOP=1 -d epavillon -f ops/migrations/2026-11-01-programme-cdp31-maj.sql
--
-- Chaque geste refait ce que fait l'API, déclencheurs compris (journal des
-- dossiers, audit, événements de l'outbox) :
--   1-3. trois déplacements, comme PUT /sessions/{id}/schedule (service/planner.rs) ;
--   4.   annulation du dossier « Agir pour une transition juste », comme
--        POST /proposals/{id}/decision vers `cancelled` (service/transition.rs) ;
--   5.   séance commune AIRF + CASAD-Benin, comme POST /admin/planner/joint-sessions
--        (service/joint.rs), après acceptation des deux dossiers ;
--   6.   deux activités de l'OIF hors appel, nées comme à l'acceptation
--        (service/birth.rs).
-- Les séances touchées sont posées dans le Stand, seule salle de l'édition.
-- Non rejouable : les gardes refusent un état de départ inattendu.

\set ON_ERROR_STOP on
\set QUIET on
\pset tuples_only on

BEGIN;

SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-01-programme-cdp31-maj', true);

-- -----------------------------------------------------------------------------
-- Outils de la transaction (schéma temporaire, rien ne reste en base)
-- -----------------------------------------------------------------------------

CREATE FUNCTION pg_temp.exiger(p_ok boolean, p_message text) RETURNS void
LANGUAGE plpgsql AS $$
BEGIN
    IF p_ok IS NOT TRUE THEN
        RAISE EXCEPTION 'Garde : %', p_message;
    END IF;
END $$;

-- La séance est au créneau attendu, non publiée, sans inscrit (aucun avis ne part).
CREATE FUNCTION pg_temp.exiger_seance(p_id uuid, p_debut timestamptz, p_fin timestamptz)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_temp.exiger(EXISTS (
        SELECT 1 FROM programme.sessions s
         WHERE s.id = p_id
           AND s.event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'
           AND s.starts_at = p_debut AND s.ends_at = p_fin
           AND s.status = 'planned' AND s.published_at IS NULL
           AND NOT EXISTS (SELECT 1 FROM programme.registrations r
                            WHERE r.session_id = s.id AND r.status <> 'cancelled')
           AND NOT EXISTS (SELECT 1 FROM programme.session_proposals sp
                            WHERE sp.session_id = s.id)),
        format('séance %s absente, déplacée, publiée, avec inscrits ou déjà commune', p_id));
END $$;

-- PUT /sessions/{id}/schedule : salle de l'édition, journée remise à nul pour
-- que tg_sessions_derive la redéduise de la date.
CREATE FUNCTION pg_temp.placer(p_id uuid, p_salle uuid, p_debut timestamptz, p_fin timestamptz)
RETURNS void LANGUAGE plpgsql AS $$
BEGIN
    PERFORM pg_temp.exiger(p_fin > p_debut, 'fin antérieure au début');
    PERFORM pg_temp.exiger(p_salle IS NULL OR EXISTS (
        SELECT 1 FROM event.rooms r
          JOIN event.venues v ON v.id = r.venue_id
          JOIN programme.sessions s ON s.event_id = v.event_id
         WHERE r.id = p_salle AND s.id = p_id), 'salle hors de l''édition');
    UPDATE programme.sessions
       SET room_id = p_salle, starts_at = p_debut, ends_at = p_fin, event_day_id = NULL
     WHERE id = p_id;
    PERFORM pg_temp.exiger(FOUND, format('séance %s introuvable', p_id));
END $$;

-- La base d'adresse d'URL (domain/slug.rs) : platform.slugify, repli « dossier ».
CREATE FUNCTION pg_temp.base_dadresse(p_titre text) RETURNS text
LANGUAGE plpgsql AS $$
DECLARE
    v text := btrim(COALESCE(platform.slugify(p_titre)::text, ''));
BEGIN
    IF v = '' THEN RETURN 'dossier'; END IF;
    PERFORM pg_temp.exiger(octet_length(v) <= 157, 'titre trop long pour l''adresse');
    RETURN v;
END $$;

-- Première adresse libre parmi base, base-2 … base-10 (slug::tentative).
CREATE FUNCTION pg_temp.adresse_libre(p_base text, p_table text) RETURNS text
LANGUAGE plpgsql AS $$
DECLARE
    v text;
    v_prise boolean;
BEGIN
    FOR n IN 0..9 LOOP
        v := CASE WHEN n = 0 THEN p_base ELSE p_base || '-' || (n + 1) END;
        IF p_table = 'sessions' THEN
            SELECT EXISTS (SELECT 1 FROM programme.sessions
                            WHERE event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'
                              AND slug = v::platform.slug) INTO v_prise;
        ELSE
            SELECT EXISTS (SELECT 1 FROM programme.proposals
                            WHERE event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'
                              AND slug = v::platform.slug) INTO v_prise;
        END IF;
        IF NOT v_prise THEN RETURN v; END IF;
    END LOOP;
    RAISE EXCEPTION 'dix adresses déjà prises pour %', p_base;
END $$;

-- session_parts::recopier_les_intervenants, mot pour mot.
CREATE FUNCTION pg_temp.recopier_les_intervenants(p_dossier uuid, p_seance uuid) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO programme.session_speakers
         (session_id, person_id, role, job_title_snapshot,
          organization_snapshot, bio, confirmed_at, sort_order,
          civility, first_name, last_name, proposal_id)
    SELECT p_seance, s.person_id, s.role, s.job_title_snapshot,
           s.organization_snapshot, s.bio, s.confirmed_at, s.sort_order,
           s.civility, s.first_name, s.last_name,
           CASE WHEN EXISTS (SELECT 1 FROM programme.session_proposals sp
                              WHERE sp.session_id = p_seance AND sp.proposal_id = p_dossier)
                THEN p_dossier END
      FROM programme.proposal_speakers s
     WHERE s.proposal_id = p_dossier
       AND (s.person_id IS NOT NULL OR NOT EXISTS (
           SELECT 1 FROM programme.session_speakers d
            WHERE d.session_id = p_seance AND d.person_id IS NULL AND d.role = s.role
              AND d.first_name = s.first_name AND d.last_name = s.last_name))
    ON CONFLICT (session_id, person_id, role) DO NOTHING;
$$;

-- session_parts::recopier_les_coorganisations, mot pour mot.
CREATE FUNCTION pg_temp.recopier_les_coorganisations(p_dossier uuid, p_seance uuid) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO programme.session_organizations
         (session_id, organization_id, role, sort_order, proposal_id)
    SELECT p_seance, o.organization_id, o.role, o.sort_order,
           CASE WHEN EXISTS (SELECT 1 FROM programme.session_proposals sp
                              WHERE sp.session_id = p_seance AND sp.proposal_id = p_dossier)
                THEN p_dossier END
      FROM programme.proposal_organizations o
     WHERE o.proposal_id = p_dossier AND o.role <> 'lead'
    ON CONFLICT (session_id, organization_id) DO NOTHING;
$$;

-- themes::recopier_sur_la_seance : les thématiques seules.
CREATE FUNCTION pg_temp.recopier_les_thematiques(p_dossier uuid, p_seance uuid) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO reference.entity_terms
         (entity_schema, entity_table, entity_id, term_id, sort_order)
    SELECT 'programme', 'sessions', p_seance, source.term_id, source.sort_order
      FROM reference.entity_terms source
      JOIN reference.taxonomy_terms t ON t.id = source.term_id
     WHERE source.entity_schema = 'programme'
       AND source.entity_table = 'proposals'
       AND source.entity_id = p_dossier
       AND t.taxonomy_code = 'activity_theme'
    ON CONFLICT DO NOTHING;
$$;

-- -----------------------------------------------------------------------------
-- 0. Gardes : l'état de départ est celui de la sauvegarde du 01/11 19:02
-- -----------------------------------------------------------------------------

SELECT pg_temp.exiger_seance('01a17da1-9bea-7aaa-a7ce-23bb9184fb84', '2026-11-10 13:00+03', '2026-11-10 14:00+03'); -- AEP
SELECT pg_temp.exiger_seance('01a17da1-9bea-7ba4-8030-0d4c26580a41', '2026-11-19 10:15+03', '2026-11-19 11:15+03'); -- Expertise France
SELECT pg_temp.exiger_seance('01a17da1-9bea-7eae-a833-bc3b74d15dca', '2026-11-17 13:00+03', '2026-11-17 15:15+03'); -- GLOBAL ACTIONS
SELECT pg_temp.exiger_seance('01a17da1-9bea-73fc-a1fe-510d0bf5b050', '2026-11-10 15:30+03', '2026-11-10 16:30+03'); -- Agir pour une transition juste
SELECT pg_temp.exiger_seance('01a17da1-9bea-7ada-aec5-8d97f1584311', '2026-11-13 11:30+03', '2026-11-13 12:30+03'); -- AIRF
SELECT pg_temp.exiger_seance('01a17da1-9bea-7d75-93ee-8312895f5dc8', '2026-11-13 11:30+03', '2026-11-13 12:30+03'); -- CASAD-Benin

SELECT pg_temp.exiger(
    (SELECT status FROM programme.proposals WHERE id = '01a17da1-9bda-7c15-afbd-82d9590eb346') = 'accepted',
    'le dossier « Agir pour une transition juste » n''est plus retenu');
SELECT pg_temp.exiger(
    (SELECT count(*) FROM programme.proposals
      WHERE id IN ('01a17da1-9bdb-7118-9696-ca75b9beae0b', '01a17da1-9bd9-7e13-8902-c9453c0d327f')
        AND status = 'under_review' AND deleted_at IS NULL) = 2,
    'AIRF et CASAD-Benin ne sont plus tous deux en évaluation');
SELECT pg_temp.exiger(
    NOT EXISTS (SELECT 1 FROM programme.session_proposals
                 WHERE proposal_id IN ('01a17da1-9bdb-7118-9696-ca75b9beae0b', '01a17da1-9bd9-7e13-8902-c9453c0d327f')),
    'AIRF ou CASAD-Benin déjà réuni');
SELECT pg_temp.exiger(
    NOT EXISTS (SELECT 1 FROM programme.proposals
                 WHERE event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'
                   AND organization_id = '01a18dc9-5da9-7bd9-9a7c-46022b3e602c'
                   AND deleted_at IS NULL),
    'l''OIF a déjà un dossier sur la CdP31 : script déjà joué ?');
SELECT pg_temp.exiger(
    EXISTS (SELECT 1 FROM org.organizations WHERE id = '01a18dc9-5da9-7bd9-9a7c-46022b3e602c')
    AND EXISTS (SELECT 1 FROM identity.people WHERE id = '01a17da0-b65a-7e87-bc9a-64e70e540c0a')
    AND EXISTS (SELECT 1 FROM event.rooms r JOIN event.venues v ON v.id = r.venue_id
                 WHERE r.id = '01a17da1-9b67-7e75-b5c0-d68db6d70feb'
                   AND v.event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'),
    'OIF, Issa Bado ou le Stand introuvable');

-- -----------------------------------------------------------------------------
-- 1-3. Déplacements (événement programme.session.rescheduled pour chacun)
-- -----------------------------------------------------------------------------

-- 1. AEP, « Bassin du Congo… » : 10/11 → 17/11 13:00-14:00.
SELECT pg_temp.placer('01a17da1-9bea-7aaa-a7ce-23bb9184fb84', '01a17da1-9b67-7e75-b5c0-d68db6d70feb',
                      '2026-11-17 13:00+03', '2026-11-17 14:00+03');
-- 2. Expertise France, « Gouvernance et financement climatique… » : 19/11 → 10/11 13:00-14:00.
SELECT pg_temp.placer('01a17da1-9bea-7ba4-8030-0d4c26580a41', '01a17da1-9b67-7e75-b5c0-d68db6d70feb',
                      '2026-11-10 13:00+03', '2026-11-10 14:00+03');
-- 3. GLOBAL ACTIONS, « Accélérer l'action climatique… » : 17/11 → 10/11 15:30-16:30.
SELECT pg_temp.placer('01a17da1-9bea-7eae-a833-bc3b74d15dca', '01a17da1-9b67-7e75-b5c0-d68db6d70feb',
                      '2026-11-10 15:30+03', '2026-11-10 16:30+03');

-- -----------------------------------------------------------------------------
-- 4. Expertise France annule « Agir pour une transition juste » (le créneau ne
--    lui convient pas). Chemin du dossier, pas de la régie : la régie refuse une
--    séance non publiée. Le garde journalise et émet programme.proposal.cancelled ;
--    la séance, jamais publiée et sans inscrit, est libérée (supprimée), comme
--    repo/sessions.rs liberer_les_creneaux.
-- -----------------------------------------------------------------------------

UPDATE programme.proposals
   SET status = 'cancelled',
       decision_reason = 'L''organisation a indiqué que le créneau ne lui convient pas.'
 WHERE id = '01a17da1-9bda-7c15-afbd-82d9590eb346' AND deleted_at IS NULL;

WITH liberees AS (
    DELETE FROM programme.sessions s
     WHERE s.proposal_id = '01a17da1-9bda-7c15-afbd-82d9590eb346'
       AND s.status = 'planned'
       AND s.published_at IS NULL
       AND NOT EXISTS (SELECT 1 FROM programme.registrations r WHERE r.session_id = s.id)
    RETURNING s.id
)
DELETE FROM reference.entity_terms et
 USING liberees l
 WHERE et.entity_schema = 'programme' AND et.entity_table = 'sessions' AND et.entity_id = l.id;

SELECT pg_temp.exiger(
    NOT EXISTS (SELECT 1 FROM programme.sessions WHERE proposal_id = '01a17da1-9bda-7c15-afbd-82d9590eb346'),
    'la séance du dossier annulé est restée');

-- -----------------------------------------------------------------------------
-- 5. Séance commune « Enjeux de financements pour les régions et les
--    municipalités » : AIRF puis CASAD-Benin, 13/11 11:30-12:30.
--
--    La réunion n'accepte que des dossiers retenus ; les deux sont en
--    évaluation, avec une séance déjà posée. On les retient d'abord, comme la
--    décision du back-office : leur séance existe déjà au rang 1, la naissance
--    n'en crée donc aucune.
-- -----------------------------------------------------------------------------

UPDATE programme.proposals
   SET status = 'accepted', decision_reason = NULL
 WHERE id IN ('01a17da1-9bdb-7118-9696-ca75b9beae0b', '01a17da1-9bd9-7e13-8902-c9453c0d327f')
   AND deleted_at IS NULL;

DO $$
DECLARE
    v_dossiers uuid[] := ARRAY['01a17da1-9bdb-7118-9696-ca75b9beae0b',  -- AIRF, en premier
                               '01a17da1-9bd9-7e13-8902-c9453c0d327f']; -- CASAD-Benin
    v_titre jsonb := '{"fr": "Enjeux de financements pour les régions et les municipalités"}';
    v_modele uuid;
    v_seance uuid;
    v_adresse text;
BEGIN
    -- joint::dossiers : verrouillés, retenus, non réunis, rien d'ouvert ni d'inscrit.
    PERFORM 1 FROM programme.proposals WHERE id = ANY (v_dossiers) FOR UPDATE;
    PERFORM pg_temp.exiger((
        SELECT count(*) FROM programme.proposals p
         WHERE p.id = ANY (v_dossiers) AND p.deleted_at IS NULL
           AND p.event_id = '01a17da1-9b46-75b2-895a-d9e226226be2'
           AND p.status = 'accepted'
           AND NOT EXISTS (SELECT 1 FROM programme.session_proposals sp WHERE sp.proposal_id = p.id)
           AND NOT EXISTS (SELECT 1 FROM programme.sessions s
                            WHERE s.proposal_id = p.id AND s.status IN ('live', 'completed'))
           AND NOT EXISTS (SELECT 1 FROM programme.registrations r
                             JOIN programme.sessions s ON s.id = r.session_id
                            WHERE s.proposal_id = p.id AND s.status <> 'cancelled'
                              AND r.status <> 'cancelled')) = 2,
        'dossiers non réunissables');

    -- joint::seance_modele : la première séance non annulée du premier dossier.
    SELECT id INTO v_modele FROM programme.sessions
     WHERE proposal_id = v_dossiers[1] AND status <> 'cancelled'
     ORDER BY sequence_number, starts_at LIMIT 1;
    PERFORM pg_temp.exiger(v_modele IS NOT NULL, 'AIRF sans séance modèle');

    v_adresse := pg_temp.adresse_libre(pg_temp.base_dadresse(v_titre ->> 'fr'), 'sessions');

    -- joint::creer : à l'image de la séance modèle, sans dossier propre.
    INSERT INTO programme.sessions
         (event_id, proposal_id, event_day_id, organization_id, title, slug,
          status, format, starts_at, ends_at, timezone, room_id, location_note,
          registration_required, registration_opens_at, registration_closes_at,
          capacity, waitlist_enabled, registration_form_id, is_streamed,
          broadcast_channel_id, is_recorded, allows_questions, published_at,
          created_by)
    SELECT s.event_id, NULL, s.event_day_id, s.organization_id, v_titre::platform.i18n_text,
           v_adresse::platform.slug,
           s.status, s.format, s.starts_at, s.ends_at, s.timezone, s.room_id,
           s.location_note, s.registration_required, s.registration_opens_at,
           s.registration_closes_at, s.capacity, s.waitlist_enabled,
           s.registration_form_id, s.is_streamed, s.broadcast_channel_id,
           s.is_recorded, s.allows_questions, s.published_at,
           platform.current_actor_id()
      FROM programme.sessions s
     WHERE s.id = v_modele
    RETURNING id INTO v_seance;

    FOR v_rang IN 1 .. array_length(v_dossiers, 1) LOOP
        -- joint::attacher : la ligne de réunion, textes du dossier recopiés…
        INSERT INTO programme.session_proposals
             (session_id, proposal_id, sort_order, title, description, added_by)
        SELECT v_seance, p.id, v_rang, p.title, p.detailed_presentation, platform.current_actor_id()
          FROM programme.proposals p
         WHERE p.id = v_dossiers[v_rang];

        -- … et le porteur rangé sous son dossier.
        INSERT INTO programme.session_organizations
             (session_id, organization_id, role, sort_order, proposal_id)
        SELECT v_seance, p.organization_id, 'lead', 0, p.id
          FROM programme.proposals p
         WHERE p.id = v_dossiers[v_rang]
        ON CONFLICT (session_id, organization_id) DO UPDATE
           SET role = 'lead', proposal_id = EXCLUDED.proposal_id
         WHERE programme.session_organizations.proposal_id IS NULL;

        -- joint::reprendre_les_fils : les journées spéciales suivent.
        INSERT INTO programme.session_tracks
             (session_id, track_id, sort_order, is_highlight, added_by, part_id)
        SELECT v_seance, st.track_id, st.sort_order, st.is_highlight, st.added_by, st.part_id
          FROM programme.session_tracks st
          JOIN programme.sessions s ON s.id = st.session_id
         WHERE s.proposal_id = v_dossiers[v_rang] AND s.status <> 'cancelled'
        ON CONFLICT (session_id, track_id) DO NOTHING;

        PERFORM pg_temp.recopier_les_intervenants(v_dossiers[v_rang], v_seance);
        PERFORM pg_temp.recopier_les_coorganisations(v_dossiers[v_rang], v_seance);
        PERFORM pg_temp.recopier_les_thematiques(v_dossiers[v_rang], v_seance);
    END LOOP;

    -- joint::retirer_les_seances : les séances propres partent, thématiques comprises.
    WITH retirees AS (
        DELETE FROM programme.sessions s
         WHERE s.proposal_id = ANY (v_dossiers) AND s.status <> 'cancelled'
        RETURNING s.id
    )
    DELETE FROM reference.entity_terms et
     USING retirees r
     WHERE et.entity_schema = 'programme' AND et.entity_table = 'sessions' AND et.entity_id = r.id;

    PERFORM set_config('maj.seance_commune', v_seance::text, true);
END $$;

-- Posée dans le Stand, au créneau des deux séances réunies.
SELECT pg_temp.placer(current_setting('maj.seance_commune')::uuid, '01a17da1-9b67-7e75-b5c0-d68db6d70feb',
                      '2026-11-13 11:30+03', '2026-11-13 12:30+03');

-- -----------------------------------------------------------------------------
-- 6. Deux activités de l'OIF, hors appel (l'appel est clos ; la base prévoit
--    le dossier « hors appel, créé par l'IFDD »). Chemin complet du graphe :
--    brouillon, déposé, en évaluation, retenu ; la séance naît à l'acceptation
--    au créneau souhaité, puis est posée dans le Stand.
--    Intervenant unique : Issa Bado (sa photo de profil s'affiche par repli).
-- -----------------------------------------------------------------------------

CREATE FUNCTION pg_temp.activite_oif(p_titre text, p_debut timestamptz, p_minutes int,
                                     p_thematiques text[], p_categories text[])
RETURNS uuid LANGUAGE plpgsql AS $$
DECLARE
    v_dossier uuid;
    v_seance uuid;
    v_adresse text;
BEGIN
    v_adresse := pg_temp.adresse_libre(pg_temp.base_dadresse(p_titre), 'proposals');

    -- repo/proposals.rs inserer : brouillon (statut par défaut).
    INSERT INTO programme.proposals
         (call_id, event_id, organization_id, submitted_by, contact_person_id,
          title, slug, objectives, detailed_presentation, expected_outcomes,
          target_audiences, format, language_codes, country_id,
          preferred_start_at, preferred_end_at,
          duration_minutes, requested_sessions, scheduling_constraints)
    VALUES (NULL, '01a17da1-9b46-75b2-895a-d9e226226be2',
            '01a18dc9-5da9-7bd9-9a7c-46022b3e602c',            -- OIF
            platform.current_actor_id(), NULL,
            jsonb_build_object('fr', p_titre)::platform.i18n_text,
            v_adresse::platform.slug,
            '{"fr": "Bientôt disponible"}'::jsonb::platform.i18n_text,
            '{"fr": "<p>Bientôt disponible</p>"}'::jsonb::platform.i18n_text,
            NULL,
            ARRAY['{"fr": "Négociatrices et négociateurs francophones"}'::jsonb::platform.i18n_text],
            'in_person', '{fr}',
            (SELECT id FROM reference.countries WHERE iso2 = 'TR'),  -- comme tous les dossiers CdP31
            p_debut, p_debut + make_interval(mins => p_minutes),
            p_minutes, 1, NULL)
    RETURNING id INTO v_dossier;

    -- themes::poser : thématiques et catégories, rang à partir de 1.
    INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order)
    SELECT 'programme', 'proposals', v_dossier, t.id, c.rang
      FROM unnest(p_thematiques) WITH ORDINALITY AS c(code, rang)
      JOIN reference.taxonomy_terms t ON t.code = c.code AND t.taxonomy_code = 'activity_theme'
    ON CONFLICT DO NOTHING;
    INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order)
    SELECT 'programme', 'proposals', v_dossier, t.id, c.rang
      FROM unnest(p_categories) WITH ORDINALITY AS c(code, rang)
      JOIN reference.taxonomy_terms t ON t.code = c.code AND t.taxonomy_code = 'activity_category'
    ON CONFLICT DO NOTHING;
    PERFORM pg_temp.exiger(
        (SELECT count(*) FROM reference.entity_terms
          WHERE entity_schema = 'programme' AND entity_table = 'proposals' AND entity_id = v_dossier)
        = cardinality(p_thematiques) + cardinality(p_categories),
        'thématique ou catégorie inconnue');

    -- repo/speakers.rs : personne liée, donc ni civilité ni nom sur la ligne.
    INSERT INTO programme.proposal_speakers
         (proposal_id, person_id, role, job_title_snapshot, organization_snapshot,
          organization_id, bio, sort_order)
    VALUES (v_dossier, '01a17da0-b65a-7e87-bc9a-64e70e540c0a', 'speaker',
            'Spécialiste de Programme',
            'Institut de la Francophonie pour le développement durable (IFDD)',
            NULL, NULL, 0);

    -- Le graphe, transition par transition (journal et événement à chacune).
    UPDATE programme.proposals SET status = 'submitted', decision_reason = NULL WHERE id = v_dossier;
    UPDATE programme.proposals SET status = 'under_review', decision_reason = NULL WHERE id = v_dossier;
    UPDATE programme.proposals SET status = 'accepted', decision_reason = NULL WHERE id = v_dossier;

    -- service/birth.rs faire_naitre : une séance au créneau souhaité.
    v_adresse := pg_temp.adresse_libre(
        pg_temp.base_dadresse((SELECT slug::text FROM programme.proposals WHERE id = v_dossier)),
        'sessions');
    INSERT INTO programme.sessions
         (event_id, proposal_id, organization_id, sequence_number,
          title, description, slug, format, timezone, starts_at, ends_at)
    SELECT p.event_id, p.id, p.organization_id, 1,
           p.title, p.detailed_presentation, v_adresse::platform.slug, p.format,
           e.timezone, p.preferred_start_at,
           p.preferred_start_at + make_interval(mins => p.duration_minutes::int)
      FROM programme.proposals p
      JOIN event.events e ON e.id = p.event_id
     WHERE p.id = v_dossier
    RETURNING id INTO v_seance;

    PERFORM pg_temp.recopier_les_intervenants(v_dossier, v_seance);
    PERFORM pg_temp.recopier_les_coorganisations(v_dossier, v_seance);
    PERFORM pg_temp.recopier_les_thematiques(v_dossier, v_seance);

    PERFORM pg_temp.placer(v_seance, '01a17da1-9b67-7e75-b5c0-d68db6d70feb',
                           p_debut, p_debut + make_interval(mins => p_minutes));
    RETURN v_seance;
END $$;

SELECT pg_temp.activite_oif(
    'Atelier de fixation des objectifs pour la semaine 1 pour négociateurs et négociatrices',
    '2026-11-09 13:00+03', 120,
    ARRAY['climate_ambition_ndc', 'climate_finance', 'adaptation'],
    ARRAY['capacity_building', 'concertation']) AS seance_atelier;

SELECT pg_temp.activite_oif(
    'Concertation francophone : bilan de la première semaine de négociations',
    '2026-11-16 13:00+03', 135,
    ARRAY['climate_ambition_ndc', 'climate_finance', 'adaptation'],
    ARRAY['concertation', 'results_sharing']) AS seance_bilan;

COMMIT;
