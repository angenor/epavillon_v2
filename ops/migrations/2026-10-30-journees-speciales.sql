-- La page publique d'un fil : journées spéciales et atelier préparatoire
-- (30/10). Rejouable, additive, aucune donnée touchée.
-- Modèle : docs/database/060_events.sql § 3 bis, § 4 et § 4 ter,
-- docs/database/075_programme_sessions.sql § 2 bis,
-- docs/database/050_media.sql § 4 (attached_documents) et table blanche.
--
--   - event.programme_tracks : accroche, publics, mention de soutien, encart
--     mis en avant, lieu (venue_id) ;
--   - event.venues : secteur, consigne d'accès, coordonnées ;
--   - event.programme_track_parts, _moments, _partners (+ taxonomie
--     track_partner_role, registre de fusion des organisations) ;
--   - programme.session_tracks.part_id ;
--   - media.attached_documents() et trois rôles pour event.programme_tracks.
--
-- Jouer AVANT de démarrer la nouvelle API : ses requêtes nomment ces colonnes.
BEGIN;

-- --- event.programme_tracks : colonnes de la page ---------------------------
ALTER TABLE event.programme_tracks
    ADD COLUMN IF NOT EXISTS tagline            platform.i18n_text,
    ADD COLUMN IF NOT EXISTS target_audiences   platform.i18n_text[] NOT NULL DEFAULT '{}',
    ADD COLUMN IF NOT EXISTS support_note       platform.i18n_text,
    ADD COLUMN IF NOT EXISTS feature_kicker     platform.i18n_text,
    ADD COLUMN IF NOT EXISTS feature_title      platform.i18n_text,
    ADD COLUMN IF NOT EXISTS feature_body       platform.i18n_text,
    ADD COLUMN IF NOT EXISTS feature_link_label platform.i18n_text,
    ADD COLUMN IF NOT EXISTS feature_link_url   text,
    ADD COLUMN IF NOT EXISTS venue_id           uuid REFERENCES event.venues(id) ON DELETE SET NULL;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ck_programme_tracks_feature_link') THEN
        ALTER TABLE event.programme_tracks ADD CONSTRAINT ck_programme_tracks_feature_link
            CHECK ((feature_link_url IS NULL) = (feature_link_label IS NULL));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ck_programme_tracks_feature_link_url') THEN
        ALTER TABLE event.programme_tracks ADD CONSTRAINT ck_programme_tracks_feature_link_url
            CHECK (feature_link_url IS NULL
                   OR (feature_link_url ~ '^(https?://|/)[^\s]*$' AND length(feature_link_url) <= 2048));
    END IF;
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ck_programme_tracks_feature_title') THEN
        ALTER TABLE event.programme_tracks ADD CONSTRAINT ck_programme_tracks_feature_title
            CHECK (feature_title IS NOT NULL
                   OR (feature_kicker IS NULL AND feature_body IS NULL AND feature_link_url IS NULL));
    END IF;
END;
$$;

COMMENT ON COLUMN event.programme_tracks.venue_id IS
    'Lieu du fil quand il n''est pas le pavillon : l''atelier préparatoire à l''hôtel. NULL = le pavillon de l''édition.';

CREATE OR REPLACE FUNCTION event.tg_check_track_venue()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.venue_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM event.venues v WHERE v.id = NEW.venue_id AND v.event_id = NEW.event_id
    ) THEN
        RAISE EXCEPTION 'Lieu refusé : il n''appartient pas à l''édition de ce fil.'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_programme_tracks_check_venue ON event.programme_tracks;
CREATE TRIGGER tg_programme_tracks_check_venue
    BEFORE INSERT OR UPDATE OF venue_id, event_id ON event.programme_tracks
    FOR EACH ROW EXECUTE FUNCTION event.tg_check_track_venue();

-- --- event.venues : plan d'accès --------------------------------------------
ALTER TABLE event.venues
    ADD COLUMN IF NOT EXISTS area        platform.i18n_text,
    ADD COLUMN IF NOT EXISTS access_note platform.i18n_text,
    ADD COLUMN IF NOT EXISTS latitude    numeric(9,6) CHECK (latitude  IS NULL OR latitude  BETWEEN -90  AND 90),
    ADD COLUMN IF NOT EXISTS longitude   numeric(9,6) CHECK (longitude IS NULL OR longitude BETWEEN -180 AND 180);

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'ck_venues_coordinates') THEN
        ALTER TABLE event.venues ADD CONSTRAINT ck_venues_coordinates
            CHECK ((latitude IS NULL) = (longitude IS NULL));
    END IF;
END;
$$;

-- --- Parties du déroulé -----------------------------------------------------
CREATE TABLE IF NOT EXISTS event.programme_track_parts (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    track_id    uuid        NOT NULL REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    label       platform.i18n_text,
    title       platform.i18n_text,
    summary     platform.i18n_text,
    sort_order  smallint    NOT NULL DEFAULT 0,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_programme_track_parts_heading CHECK (label IS NOT NULL OR title IS NOT NULL),
    CONSTRAINT ux_programme_track_parts_track UNIQUE (id, track_id)
);

CREATE INDEX IF NOT EXISTS ix_programme_track_parts_track ON event.programme_track_parts (track_id, sort_order);

DROP TRIGGER IF EXISTS tg_programme_track_parts_updated_at ON event.programme_track_parts;
CREATE TRIGGER tg_programme_track_parts_updated_at
    BEFORE UPDATE ON event.programme_track_parts
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE event.programme_track_parts IS
    'Parties du déroulé d''un fil (« Partie 1 », « Matin »). Regroupement éditorial : leurs heures se lisent sur leur contenu.';

-- --- Moments sans activité --------------------------------------------------
CREATE TABLE IF NOT EXISTS event.programme_track_moments (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    track_id    uuid        NOT NULL REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    part_id     uuid,
    starts_at   timestamptz NOT NULL,
    ends_at     timestamptz,
    label       platform.i18n_text NOT NULL,
    description platform.i18n_text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_programme_track_moments_period CHECK (ends_at IS NULL OR ends_at > starts_at),
    CONSTRAINT fk_programme_track_moments_part FOREIGN KEY (part_id, track_id)
        REFERENCES event.programme_track_parts (id, track_id) ON DELETE SET NULL (part_id)
);

CREATE INDEX IF NOT EXISTS ix_programme_track_moments_track ON event.programme_track_moments (track_id, starts_at);

DROP TRIGGER IF EXISTS tg_programme_track_moments_updated_at ON event.programme_track_moments;
CREATE TRIGGER tg_programme_track_moments_updated_at
    BEFORE UPDATE ON event.programme_track_moments
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE event.programme_track_moments IS
    'Moments du déroulé d''un fil qui ne sont pas des activités : accueil, déjeuner, pause, clôture. Ni inscription, ni conflit, ni programmation publique.';

-- --- Partenaires ------------------------------------------------------------
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('track_partner_role', '{"fr":"Rôles des partenaires d''une journée","en":"Day partner roles"}',
     '{"fr":"Ce qu''apporte une organisation à une journée spéciale ou à un atelier","en":"What an organisation brings to a special day or a workshop"}',
     false, false, false)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('track_partner_role', 'organizer',         '{"fr":"Organisateur","en":"Organiser"}', 10),
    ('track_partner_role', 'partner',           '{"fr":"Partenaire","en":"Partner"}', 20),
    ('track_partner_role', 'technical_partner', '{"fr":"Partenaire technique","en":"Technical partner"}', 30),
    ('track_partner_role', 'financial_support', '{"fr":"Soutien financier","en":"Financial support"}', 40)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

CREATE TABLE IF NOT EXISTS event.programme_track_partners (
    track_id        uuid        NOT NULL REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    organization_id uuid        NOT NULL CONSTRAINT xmod_fk_programme_track_partners_org
                                REFERENCES org.organizations(id) ON DELETE RESTRICT,
    role_term_id    uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    sort_order      smallint    NOT NULL DEFAULT 0,
    added_at        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (track_id, organization_id)
);

CREATE INDEX IF NOT EXISTS ix_programme_track_partners_org ON event.programme_track_partners (organization_id);

COMMENT ON TABLE event.programme_track_partners IS
    'Partenaires d''un fil (« Avec » : OIF, GIZ…), distincts des organisations de ses activités. Une organisation au plus une fois par fil.';

CREATE OR REPLACE FUNCTION event.tg_check_track_partner_role()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM reference.taxonomy_terms t
        WHERE t.id = NEW.role_term_id AND t.taxonomy_code = 'track_partner_role'
    ) THEN
        RAISE EXCEPTION 'Rôle refusé : il ne relève pas des rôles de partenaire d''une journée.'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_programme_track_partners_check_role ON event.programme_track_partners;
CREATE TRIGGER tg_programme_track_partners_check_role
    BEFORE INSERT OR UPDATE OF role_term_id ON event.programme_track_partners
    FOR EACH ROW EXECUTE FUNCTION event.tg_check_track_partner_role();

INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy, dedupe_on) VALUES
    ('event', 'programme_track_partners', 'organization_id', 'reassign', '{track_id}')
ON CONFLICT DO NOTHING;

-- --- programme.session_tracks.part_id ---------------------------------------
ALTER TABLE programme.session_tracks ADD COLUMN IF NOT EXISTS part_id uuid;

DO $$
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'xmod_fk_session_tracks_part') THEN
        ALTER TABLE programme.session_tracks ADD CONSTRAINT xmod_fk_session_tracks_part
            FOREIGN KEY (part_id, track_id)
            REFERENCES event.programme_track_parts (id, track_id) ON DELETE SET NULL (part_id);
    END IF;
END;
$$;

-- --- Médias ------------------------------------------------------------------
CREATE OR REPLACE FUNCTION media.attached_documents(
    p_owner_schema text,
    p_owner_table  text,
    p_owner_id     uuid,
    p_role         media.attachment_role DEFAULT 'document'
)
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
               'asset_id',  a.id,
               'url',       media.object_url(a.bucket, a.object_key),
               'mime_type', a.mime_type,
               'bytes',     a.byte_size,
               'filename',  a.original_filename,
               'caption',   COALESCE(t.caption_override, a.caption))
             ORDER BY t.sort_order, t.created_at), '[]'::jsonb)
    FROM media.attachments t
    JOIN media.assets a ON a.id = t.asset_id
    WHERE t.owner_schema = p_owner_schema
      AND t.owner_table  = p_owner_table
      AND t.owner_id     = p_owner_id
      AND t.role         = p_role
      AND a.deleted_at IS NULL
      AND a.status = 'ready'
      AND a.visibility = 'public';
$$;

COMMENT ON FUNCTION media.attached_documents(text, text, uuid, media.attachment_role) IS
    'Documents publics et servables d''un rôle (PDF), dans l''ordre réglé : adresse, type, poids, nom de fichier, légende résolue. Tableau vide sans document.';

INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size, expected_aspect_ratio) VALUES
    ('event', 'programme_tracks', 'cover',     '{"fr":"Photo du bandeau","en":"Header photo"}',  false, '{image/*}',              15728640, NULL),
    ('event', 'programme_tracks', 'thumbnail', '{"fr":"Visuel de l''encart mis en avant","en":"Featured block visual"}', false, '{image/*}', 5242880, NULL),
    ('event', 'programme_tracks', 'document',  '{"fr":"Document de la journée","en":"Day document"}', true, '{application/pdf}',   26214400, NULL)
ON CONFLICT DO NOTHING;

COMMIT;
