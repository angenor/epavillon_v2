-- Programme de la CdP31 (01/11, demande du commanditaire) : la Fresque du
-- Climat et COP Bike Ride réunies en une séance commune, « Installation,
-- ouverture et accueil au Pavillon climatique de la Francophonie », le 09/11
-- de 09:00 à 11:15 (leurs deux créneaux). Même chemin que
-- POST /admin/planner/joint-sessions (service/joint.rs), repris du script
-- 2026-11-01-programme-cdp31-maj. Aucun inscrit, rien de publié.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
\set ON_ERROR_STOP on
\set QUIET on
\pset tuples_only on

BEGIN;

SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-01-programme-cdp31-installation', true);

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


DO $$
DECLARE
    v_dossiers uuid[] := ARRAY['01a17da1-9bd7-7ae8-8777-ffb4dc0477e1',  -- La Fresque du Climat, en premier
                               '01a17da1-9bda-7d81-b123-b14c35f3c8b7']; -- COP Bike Ride
    v_titre jsonb := '{"fr": "Installation, ouverture et accueil au Pavillon climatique de la Francophonie"}';
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
    PERFORM pg_temp.exiger(v_modele IS NOT NULL, 'Fresque sans séance modèle');

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


-- Posée dans le Stand, sur les deux créneaux réunis.
SELECT pg_temp.placer(current_setting('maj.seance_commune')::uuid, '01a17da1-9b67-7e75-b5c0-d68db6d70feb',
                      '2026-11-09 09:00+03', '2026-11-09 11:15+03');

COMMIT;
