-- Séances communes (01/11). Rejouable.
-- Modèle : docs/database/075_programme_sessions.sql § 2, 2 ter, 2 quater et
-- proposal_needs_deadline ; 080_live.sql (v_public_schedule, colonne `proposals`).
--
-- Faute de créneaux pour tous, l'IFDD réunit deux dossiers retenus ou plus dans
-- un créneau, sous un titre commun. La séance commune est une séance sans
-- dossier propre ; programme.session_proposals dit quels dossiers elle réunit,
-- et intervenants et organisations recopiés y sont rangés par dossier.
-- Additive : les séances ordinaires ne changent pas (dossier NULL partout).
-- Jouer AVANT de démarrer l'API, qui lit ces colonnes.
BEGIN;

ALTER TABLE programme.session_speakers
    ADD COLUMN IF NOT EXISTS proposal_id uuid REFERENCES programme.proposals(id) ON DELETE CASCADE;
ALTER TABLE programme.session_organizations
    ADD COLUMN IF NOT EXISTS proposal_id uuid REFERENCES programme.proposals(id) ON DELETE CASCADE;

-- Un porteur par dossier (NULL pour une séance ordinaire).
DROP INDEX IF EXISTS programme.ux_session_organizations_lead;
CREATE UNIQUE INDEX ux_session_organizations_lead
    ON programme.session_organizations (session_id, proposal_id) NULLS NOT DISTINCT
    WHERE role = 'lead';

CREATE OR REPLACE FUNCTION programme.tg_sync_session_lead_organization()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.organization_id IS NULL THEN
        RETURN NULL;
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.organization_id IS NOT DISTINCT FROM OLD.organization_id THEN
        RETURN NULL;
    END IF;

    DELETE FROM programme.session_organizations
    WHERE session_id = NEW.id AND role = 'lead' AND proposal_id IS NULL
      AND organization_id <> NEW.organization_id;

    INSERT INTO programme.session_organizations (session_id, organization_id, role, sort_order)
    VALUES (NEW.id, NEW.organization_id, 'lead', 0)
    ON CONFLICT (session_id, organization_id) DO UPDATE SET role = 'lead';

    RETURN NULL;
END;
$$;

CREATE TABLE IF NOT EXISTS programme.session_proposals (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    session_id  uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    proposal_id uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE RESTRICT,
    -- Ordre d'affichage des dossiers dans la séance (1, 2…).
    sort_order  smallint    NOT NULL DEFAULT 1 CHECK (sort_order > 0),
    -- Titre d'origine et présentation du dossier, recopiés à la réunion puis à
    -- chaque correction, comme ceux d'une séance ordinaire (« les textes
    -- suivent l'activité »).
    title       platform.i18n_text NOT NULL,
    description platform.i18n_text,
    added_by    uuid        CONSTRAINT xmod_fk_session_proposals_actor
                            REFERENCES identity.people(id) ON DELETE SET NULL,
    added_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_session_proposals UNIQUE (session_id, proposal_id),
    -- Un dossier n'est réuni que dans une séance commune à la fois.
    CONSTRAINT ux_session_proposals_proposal UNIQUE (proposal_id)
);

CREATE INDEX IF NOT EXISTS ix_session_proposals_session ON programme.session_proposals (session_id, sort_order);

DROP TRIGGER IF EXISTS tg_session_proposals_audit ON programme.session_proposals;
CREATE TRIGGER tg_session_proposals_audit
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE programme.session_proposals IS
    'Dossiers réunis dans une séance commune : un créneau, un titre commun choisi par l''IFDD, chaque dossier gardant sa présentation, sa couverture, ses organisations et ses intervenants.';
COMMENT ON COLUMN programme.session_proposals.sort_order IS
    'Ordre d''affichage du dossier dans la séance commune ; aucun horaire de passage.';

-- Une séance commune n'a pas de dossier propre, et ne réunit que des dossiers
-- de son édition : sans ce contrôle, une URL forgée glisserait un dossier de
-- la COP30 dans une séance de la COP31.
CREATE OR REPLACE FUNCTION programme.tg_check_session_proposal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM programme.sessions s
        JOIN programme.proposals p ON p.id = NEW.proposal_id
        WHERE s.id = NEW.session_id
          AND s.proposal_id IS NULL
          AND s.event_id = p.event_id
    ) THEN
        RAISE EXCEPTION 'Réunion impossible : la séance commune a déjà un dossier propre, ou le dossier relève d''une autre édition.'
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_session_proposals_check ON programme.session_proposals;
CREATE TRIGGER tg_session_proposals_check
    BEFORE INSERT OR UPDATE OF session_id, proposal_id ON programme.session_proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_session_proposal();

CREATE OR REPLACE FUNCTION programme.proposal_needs_deadline(p_proposal_id uuid)
RETURNS timestamptz
LANGUAGE sql
STABLE
AS $$
    SELECT min(s.starts_at) - interval '10 days'
      FROM programme.sessions s
     WHERE (s.proposal_id = p_proposal_id
            OR EXISTS (SELECT 1 FROM programme.session_proposals sp
                        WHERE sp.session_id = s.id AND sp.proposal_id = p_proposal_id))
       AND s.status <> 'cancelled';
$$;

COMMENT ON FUNCTION programme.proposal_needs_deadline IS
    'Dix jours avant la première séance non annulée du dossier, séance commune comprise ; NULL sans séance. Indicative.';

CREATE OR REPLACE VIEW programme.v_public_schedule AS
SELECT
    s.id,
    s.event_id,
    s.event_day_id,
    s.proposal_id,
    s.slug,
    s.title,
    s.summary,
    s.starts_at,
    s.ends_at,
    s.timezone,
    s.format,
    s.status,
    s.room_id,
    r.name                AS room_name,
    s.organization_id,
    o.legal_name          AS organization_name,
    o.acronym             AS organization_acronym,
    c.iso2                AS organization_country_code,
    c.name                AS organization_country,
    s.is_streamed,
    s.broadcast_channel_id,
    s.capacity,
    COALESCE((
        SELECT jsonb_agg(jsonb_build_object(
                   'slug', t.slug, 'title', t.title, 'color', t.color_hex, 'kind', t.kind)
               ORDER BY st.sort_order, t.sort_order)
        FROM programme.session_tracks st
        JOIN event.programme_tracks t ON t.id = st.track_id
        WHERE st.session_id = s.id AND t.published_at IS NOT NULL
    ), '[]'::jsonb) AS tracks,
    COALESCE(
        media.attached_image('programme', 'sessions',  s.id,          'cover'),
        media.attached_image('programme', 'proposals', s.proposal_id, 'cover')
    ) AS cover,
    CASE
        WHEN s.status = 'cancelled'          THEN 'cancelled'
        WHEN s.status = 'postponed'          THEN 'postponed'
        WHEN now() < s.starts_at             THEN 'upcoming'
        WHEN now() BETWEEN s.starts_at AND s.ends_at THEN 'ongoing'
        ELSE 'past'
    END AS temporal_state,
    (SELECT count(*) FROM programme.registrations rg
      WHERE rg.session_id = s.id AND rg.status IN ('registered', 'attended')) AS registered_count,
    reference.terms_of('programme', 'sessions', s.id, 'activity_theme') AS theme_codes,
    reference.term_badges('programme', 'sessions', s.id, 'activity_theme') AS themes,
    s.waitlist_enabled,
    s.registration_required,
    s.registration_opens_at,
    s.registration_closes_at,
    (SELECT count(*) FROM programme.registrations rg
      WHERE rg.session_id = s.id AND rg.status = 'waitlisted') AS waitlisted_count,
    s.listing_changed_at,
    COALESCE(p.language_codes, (
        SELECT array_agg(DISTINCT l ORDER BY l)
        FROM programme.session_proposals sp
        JOIN programme.proposals pp ON pp.id = sp.proposal_id
        CROSS JOIN unnest(pp.language_codes) AS l
        WHERE sp.session_id = s.id
    )) AS language_codes,
    rp.url              AS replay_url,
    rp.duration_seconds AS replay_duration_seconds,
    media.attached_image('org', 'organizations', o.id, 'logo') AS organization_logo,
    o.organization_type_code,
    -- Séance commune (01/11) : les dossiers réunis dans l'ordre d'affichage,
    -- chacun avec son titre d'origine, sa couverture et son organisation
    -- porteuse ; [] pour une séance ordinaire. Les colonnes organization_*
    -- portent alors le premier dossier (organization_id de la séance).
    COALESCE((
        SELECT jsonb_agg(jsonb_build_object(
                   'proposal_id', sp.proposal_id,
                   'sort_order', sp.sort_order,
                   'title', sp.title,
                   'cover', media.attached_image('programme', 'proposals', sp.proposal_id, 'cover'),
                   'organization', jsonb_build_object(
                       'id', po.id,
                       'name', po.legal_name,
                       'acronym', po.acronym,
                       'country_code', pc.iso2,
                       'country', pc.name,
                       'type_code', po.organization_type_code,
                       'logo', media.attached_image('org', 'organizations', po.id, 'logo')))
               ORDER BY sp.sort_order, sp.added_at)
        FROM programme.session_proposals sp
        JOIN programme.proposals pp   ON pp.id = sp.proposal_id
        JOIN org.organizations po     ON po.id = pp.organization_id
        LEFT JOIN reference.countries pc ON pc.id = po.country_id
        WHERE sp.session_id = s.id
    ), '[]'::jsonb) AS proposals
FROM programme.sessions s
LEFT JOIN event.rooms r          ON r.id = s.room_id
LEFT JOIN org.organizations o    ON o.id = s.organization_id
LEFT JOIN reference.countries c  ON c.id = o.country_id
LEFT JOIN programme.proposals p  ON p.id = s.proposal_id
LEFT JOIN LATERAL (
    SELECT COALESCE(ls.replay_url::text, ls.watch_url::text,
                    live.build_embed_url(ls.provider, ls.embed_id)) AS url,
           COALESCE(round(a.duration_seconds)::int,
                    round(oa.duration_seconds)::int,
                    round(extract(epoch FROM ls.ended_at - ls.started_at))::int,
                    round(extract(epoch FROM og.ended_at - og.started_at))::int) AS duration_seconds
      FROM live.streams ls
      LEFT JOIN media.assets a  ON a.id  = ls.recording_asset_id
      LEFT JOIN live.streams og ON og.id = ls.replay_of_id
      LEFT JOIN media.assets oa ON oa.id = og.recording_asset_id
     WHERE ls.session_id = s.id
       AND ls.status <> 'cancelled'
       AND (ls.kind = 'replay' OR ls.replay_url IS NOT NULL)
       AND (ls.replay_available_at IS NULL OR ls.replay_available_at <= now())
       AND COALESCE(ls.replay_url::text, ls.watch_url::text,
                    live.build_embed_url(ls.provider, ls.embed_id)) IS NOT NULL
     ORDER BY ls.is_primary DESC, ls.replay_available_at DESC NULLS LAST, ls.id DESC
     LIMIT 1
) rp ON true
WHERE s.published_at IS NOT NULL;

COMMENT ON COLUMN programme.v_public_schedule.proposals IS
    'Séance commune : dossiers réunis (titre d''origine, couverture, organisation porteuse avec logo et pays), dans l''ordre d''affichage ; [] pour une séance ordinaire.';

COMMIT;
