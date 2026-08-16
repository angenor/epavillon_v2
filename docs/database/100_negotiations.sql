-- =============================================================================
-- ePavillon v2 — 100_negotiations.sql
-- Module Négociations : espaces réservés aux négociateurs francophones,
-- réunions unifiées, documents d'aide, canaux d'échange temps réel.
--
-- Dépend de : 000_bootstrap, 010_platform, 020_reference, 030_identity,
--             040_organizations, ainsi que des modules `event`, `media` et
--             `live` (FK inter-modules nommées xmod_fk_*).
--
-- CADRAGE — « Espace où on publie : sessions de négociation ; documents d'aide
-- (lien ou fichier uploadé) ; réunions Francophonie (session avec lien zoom) ;
-- groupes d'échange temps réel comme WhatsApp, par thématique et parfois par
-- promotion, créés dynamiquement ; outils pour aider les négociateurs (agent IA
-- + RAG). Cet espace est réservé aux personnes ayant le rôle négociateur. »
--
-- QUATRE DÉFAUTS DE LA V1 CORRIGÉS ICI
--
--   D1. LA FILIÈRE ÉTAIT UN ENUM FERMÉ
--       `session_category_v2 ENUM('climate','biodiversity','desertification')`
--       était répété sur quatre tables. Ouvrir un espace « Traité plastique » ou
--       « Océans » imposait un ALTER TYPE, un redéploiement et la reprise des
--       filtres du frontend. En v2 la brique de base devient l'ESPACE
--       (negotiation.spaces), rattaché à une filière par un terme de la
--       taxonomie `negotiation_track`. Créer un espace = une ligne, zéro DDL.
--
--   D2. DEUX TABLES JUMELLES POUR LE MÊME CONCEPT
--       `negotiation_sessions` et `francophonie_meetings` portaient les mêmes
--       colonnes (titre, description, dates, lieu, catégorie, zoom, créateur),
--       avec deux tables d'inscription identiques. Toute évolution devait être
--       écrite deux fois, et « mes inscriptions » exigeait un UNION. En v2 :
--       UNE table `negotiation.meetings` discriminée par `kind`, UNE table
--       `negotiation.meeting_registrations`.
--
--   D3. FICHIER ET LIEN CONFONDUS
--       `negotiation_documents.file_url TEXT NOT NULL` accueillait aussi bien
--       une URL de stockage qu'un lien externe : impossible de savoir ce qu'on
--       devait purger, sauvegarder ou indexer. En v2, XOR explicite entre
--       `asset_id` (objet stocké, module media) et `external_url` (lien tiers).
--
--   D4. LES GROUPES D'ÉCHANGE N'ÉTAIENT PAS UNE MESSAGERIE
--       `message_groups` / `group_messages` : ni canal thématique, ni fil de
--       discussion, ni accusé de lecture, ni modération, ni maîtrise de la
--       volumétrie. En v2, canaux typés (thématique / promotion), fils de
--       réponses, `last_read_at` par membre, modération douce, et table de
--       messages PARTITIONNÉE PAR MOIS comme platform.audit_log.
--
-- AUTORISATION — aucun système de droits parallèle n'est défini ici.
-- L'accès à un espace se teste par
--   identity.has_permission(person_id, 'negotiation.space.access',
--                           'negotiation_space', space_id)
-- La valeur `negotiation_space` du type identity.scope_type fait des lignes de
-- `negotiation.spaces` des CIBLES DE PORTÉE RBAC : `role_assignments.scope_id`
-- pointe sur `spaces.id` (sans FK, la portée traversant les modules).
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 0. Garde-fou taxonomique
--
-- Un CHECK ne peut pas interroger une autre table : ce trigger générique vérifie
-- qu'un terme référencé relève bien de la taxonomie attendue.
-- Usage : EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('colonne', 'taxonomie')
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION negotiation.tg_check_term_taxonomy()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_term uuid := (to_jsonb(NEW) ->> TG_ARGV[0])::uuid;
BEGIN
    IF v_term IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM reference.taxonomy_terms t
        WHERE t.id = v_term AND t.taxonomy_code = TG_ARGV[1]
    ) THEN
        RAISE EXCEPTION 'Le terme % ne relève pas de la taxonomie « % » (colonne %.%)',
            v_term, TG_ARGV[1], TG_TABLE_NAME, TG_ARGV[0]
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

-- -----------------------------------------------------------------------------
-- 1. Espaces de négociation  (correction D1)
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.space_visibility AS ENUM (
    'private',  -- ni listé ni consultable hors membres
    'listed',   -- listé publiquement, contenus réservés aux membres
    'public'    -- contenus non restreints consultables sans adhésion
);

CREATE TABLE negotiation.spaces (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    slug               platform.slug NOT NULL,
    name               platform.i18n_text NOT NULL,
    tagline            platform.i18n_text,
    description        platform.i18n_text,
    -- Filière de rattachement : terme de la taxonomie `negotiation_track`
    -- (climate, biodiversity, desertification, et tout code ajouté ensuite).
    track_term_id      uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    cover_asset_id     uuid        CONSTRAINT xmod_fk_spaces_cover
                                   REFERENCES media.assets(id) ON DELETE SET NULL,
    visibility         negotiation.space_visibility NOT NULL DEFAULT 'listed',
    -- Une adhésion peut-elle être demandée depuis l'espace public ?
    is_membership_open boolean     NOT NULL DEFAULT false,
    opened_at          timestamptz NOT NULL DEFAULT now(),
    archived_at        timestamptz,        -- espace clos : lecture seule
    created_by         uuid        CONSTRAINT xmod_fk_spaces_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_spaces_slug UNIQUE (slug)
);

CREATE INDEX ix_spaces_track  ON negotiation.spaces (track_term_id);
CREATE INDEX ix_spaces_active ON negotiation.spaces (visibility, opened_at DESC) WHERE archived_at IS NULL;

CREATE TRIGGER tg_spaces_updated_at BEFORE UPDATE ON negotiation.spaces
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_spaces_check_track BEFORE INSERT OR UPDATE OF track_term_id ON negotiation.spaces
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('track_term_id', 'negotiation_track');

COMMENT ON TABLE negotiation.spaces IS
    'Espace thématique réservé aux négociateurs. Remplace l''ENUM de filière de la v1 et sert de portée RBAC (identity.scope_type = negotiation_space).';
COMMENT ON COLUMN negotiation.spaces.track_term_id IS
    'Filière (taxonomie negotiation_track). Un nouvel espace « Traité plastique » ne demande qu''un terme, jamais de migration.';

-- -----------------------------------------------------------------------------
-- 2. Appartenance à un espace
--
-- ARTICULATION AVEC LE RBAC — cette table décrit l'ANNUAIRE de l'espace (qui en
-- fait partie, à quel titre, depuis quand, pour quelle délégation). Elle ne
-- donne AUCUN droit par elle-même : l'autorisation reste portée par
-- `identity.role_assignments` (rôle `negotiator` ou `space_lead`, portée
-- `negotiation_space` + scope_id = spaces.id). L'API crée les deux dans la même
-- transaction ; en cas de divergence, le RBAC fait foi.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.space_member_role AS ENUM ('lead', 'facilitator', 'negotiator', 'observer');

CREATE TABLE negotiation.space_members (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    space_id              uuid        NOT NULL REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    person_id             uuid        NOT NULL CONSTRAINT xmod_fk_space_members_person
                                      REFERENCES identity.people(id) ON DELETE CASCADE,
    -- Profil négociateur associé : désignations annuelles et spécialisations
    -- restent dans le module identity, on ne les recopie pas.
    negotiator_profile_id uuid        CONSTRAINT xmod_fk_space_members_profile
                                      REFERENCES identity.negotiator_profiles(id) ON DELETE SET NULL,
    role                  negotiation.space_member_role NOT NULL DEFAULT 'negotiator',
    -- Délégation représentée dans l'espace (peut différer du pays de résidence).
    country_id            uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    organization_id       uuid        CONSTRAINT xmod_fk_space_members_organization
                                      REFERENCES org.organizations(id) ON DELETE SET NULL,
    cohort_label          text,       -- promotion de formation, ex. « Promotion 2026 »
    admitted_at           timestamptz NOT NULL DEFAULT now(),
    admitted_by           uuid        CONSTRAINT xmod_fk_space_members_admitter
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    left_at               timestamptz,
    CONSTRAINT ck_space_members_period CHECK (left_at IS NULL OR left_at >= admitted_at)
);

CREATE UNIQUE INDEX ux_space_members_active
    ON negotiation.space_members (space_id, person_id) WHERE left_at IS NULL;
CREATE INDEX ix_space_members_person ON negotiation.space_members (person_id) WHERE left_at IS NULL;
CREATE INDEX ix_space_members_cohort ON negotiation.space_members (space_id, cohort_label) WHERE cohort_label IS NOT NULL;

CREATE TRIGGER tg_space_members_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.space_members
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.space_members IS
    'Annuaire d''un espace. N''accorde aucun droit : l''autorisation passe par identity.has_permission(..., ''negotiation_space'', space_id).';

-- -----------------------------------------------------------------------------
-- 3. Réunions unifiées  (correction D2)
--
-- Une seule table remplace `negotiation_sessions` + `francophonie_meetings`.
-- `kind` porte la distinction éditoriale, `format` la modalité de participation.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.meeting_kind AS ENUM (
    'negotiation_session',       -- session officielle de la convention (COP, SB, pré-session)
    'francophone_consultation',  -- concertation francophone (ex-francophonie_meetings)
    'preparatory_workshop',      -- atelier préparatoire
    'field_training',            -- atelier de formation terrain
    'innovation'                 -- rencontre innovation
);
CREATE TYPE negotiation.meeting_format AS ENUM ('onsite', 'online', 'hybrid');
CREATE TYPE negotiation.meeting_status AS ENUM ('draft', 'scheduled', 'ongoing', 'completed', 'cancelled');

CREATE TABLE negotiation.meetings (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    space_id              uuid        NOT NULL REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    kind                  negotiation.meeting_kind NOT NULL,
    slug                  platform.slug NOT NULL,
    title                 platform.i18n_text NOT NULL,
    description           platform.i18n_text,
    start_at              timestamptz NOT NULL,
    end_at                timestamptz NOT NULL,
    timezone              platform.timezone_name NOT NULL DEFAULT 'UTC',
    format                negotiation.meeting_format NOT NULL DEFAULT 'online',
    venue_label           text,
    city                  text,
    country_id            uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    -- Visioconférence pilotée par la plateforme (création Zoom, présence, replay).
    live_meeting_id       uuid        CONSTRAINT xmod_fk_negotiation_meetings_live
                                      REFERENCES live.meetings(id) ON DELETE SET NULL,
    -- Lien de connexion tiers, quand la réunion n'est pas hébergée par l'IFDD.
    external_url          platform.url,
    -- Rattachement facultatif à une édition (COP) du module event.
    event_id              uuid        CONSTRAINT xmod_fk_negotiation_meetings_event
                                      REFERENCES event.events(id) ON DELETE SET NULL,
    organizer_org_id      uuid        CONSTRAINT xmod_fk_negotiation_meetings_organizer
                                      REFERENCES org.organizations(id) ON DELETE SET NULL,
    is_ifdd_organized     boolean     NOT NULL DEFAULT true,
    host_person_id        uuid        CONSTRAINT xmod_fk_negotiation_meetings_host
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    capacity              integer     CHECK (capacity IS NULL OR capacity > 0),
    registration_opens_at timestamptz,
    registration_closes_at timestamptz,
    registered_count      integer     NOT NULL DEFAULT 0,
    status                negotiation.meeting_status NOT NULL DEFAULT 'draft',
    cancellation_reason   text,
    created_by            uuid        CONSTRAINT xmod_fk_negotiation_meetings_creator
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_meetings_slug UNIQUE (space_id, slug),
    CONSTRAINT ck_meetings_period CHECK (end_at > start_at),
    CONSTRAINT ck_meetings_registration_window
        CHECK (registration_opens_at IS NULL OR registration_closes_at IS NULL
            OR registration_closes_at > registration_opens_at),
    -- Une réunion publiée à distance doit offrir un moyen de s'y connecter,
    -- et une réunion sur site doit indiquer où elle se tient.
    CONSTRAINT ck_meetings_online_access
        CHECK (status = 'draft' OR format = 'onsite'
            OR live_meeting_id IS NOT NULL OR external_url IS NOT NULL),
    CONSTRAINT ck_meetings_onsite_venue
        CHECK (status = 'draft' OR format = 'online' OR venue_label IS NOT NULL),
    CONSTRAINT ck_meetings_cancellation
        CHECK (status <> 'cancelled' OR cancellation_reason IS NOT NULL),
    -- Une même salle Zoom ne peut pas héberger deux réunions qui se chevauchent :
    -- le conflit est refusé par la base, pas détecté après coup par un humain.
    CONSTRAINT ex_meetings_live_room_overlap EXCLUDE USING gist (
        live_meeting_id WITH =,
        tstzrange(start_at, end_at, '[)') WITH &&
    ) WHERE (live_meeting_id IS NOT NULL AND status <> 'cancelled')
);

CREATE INDEX ix_meetings_agenda   ON negotiation.meetings (space_id, start_at DESC)
    WHERE status IN ('scheduled', 'ongoing');
CREATE INDEX ix_meetings_kind     ON negotiation.meetings (kind, start_at DESC);
CREATE INDEX ix_meetings_upcoming ON negotiation.meetings (start_at) WHERE status = 'scheduled';

CREATE TRIGGER tg_meetings_updated_at BEFORE UPDATE ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_meetings_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.meetings IS
    'Événements d''un espace de négociation : sessions officielles, concertations francophones, ateliers, formations, innovation. Unifie les deux tables jumelles de la v1.';
COMMENT ON COLUMN negotiation.meetings.kind IS
    'ENUM fermé assumé : chaque valeur engage un parcours applicatif distinct (contrairement aux filières, ouvertes par taxonomie).';

-- Publication d'un événement de domaine à l'annulation : le module engagement
-- prévient les inscrits sans que ce module connaisse l'email.
CREATE OR REPLACE FUNCTION negotiation.tg_meeting_status_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status IS DISTINCT FROM OLD.status AND NEW.status IN ('scheduled', 'cancelled') THEN
        PERFORM platform.emit_event(
            'negotiation', 'meeting', NEW.id,
            'negotiation.meeting.' || CASE WHEN NEW.status = 'cancelled' THEN 'cancelled' ELSE 'published' END,
            jsonb_build_object('space_id', NEW.space_id, 'kind', NEW.kind,
                               'start_at', NEW.start_at, 'reason', NEW.cancellation_reason)
        );
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_meetings_status_event AFTER UPDATE OF status ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_meeting_status_event();

-- -----------------------------------------------------------------------------
-- 4. Inscriptions — une seule table pour tous les types de réunion
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.registration_status AS ENUM ('registered', 'waitlisted', 'cancelled');
CREATE TYPE negotiation.attendance_state AS ENUM ('unknown', 'present', 'partial', 'absent');

CREATE TABLE negotiation.meeting_registrations (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    meeting_id         uuid        NOT NULL REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    -- Pas de colonnes `guest_*` comme en v1 : une personne existe toujours dans
    -- identity.people, avec ou sans compte.
    person_id          uuid        NOT NULL CONSTRAINT xmod_fk_meeting_registrations_person
                                   REFERENCES identity.people(id) ON DELETE CASCADE,
    status             negotiation.registration_status NOT NULL DEFAULT 'registered',
    registered_at      timestamptz NOT NULL DEFAULT now(),
    cancelled_at       timestamptz,
    attendance         negotiation.attendance_state NOT NULL DEFAULT 'unknown',
    attendance_minutes integer     CHECK (attendance_minutes IS NULL OR attendance_minutes >= 0),
    first_joined_at    timestamptz,
    note               text,
    CONSTRAINT ux_meeting_registrations UNIQUE (meeting_id, person_id),
    CONSTRAINT ck_meeting_registrations_cancel
        CHECK ((status = 'cancelled') = (cancelled_at IS NOT NULL))
);

CREATE INDEX ix_meeting_registrations_person ON negotiation.meeting_registrations (person_id, registered_at DESC);
CREATE INDEX ix_meeting_registrations_active ON negotiation.meeting_registrations (meeting_id) WHERE status = 'registered';

-- Compteur dénormalisé maintenu par la base : la liste des réunions n'a plus à
-- déclencher un COUNT(*) par ligne.
CREATE OR REPLACE FUNCTION negotiation.tg_sync_registered_count()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_meeting_id uuid;
BEGIN
    -- NEW n'est pas affecté sur un DELETE : la branche est explicite.
    IF TG_OP = 'DELETE' THEN
        v_meeting_id := OLD.meeting_id;
    ELSE
        v_meeting_id := NEW.meeting_id;
    END IF;

    UPDATE negotiation.meetings m
    SET registered_count = (
        SELECT count(*) FROM negotiation.meeting_registrations r
        WHERE r.meeting_id = m.id AND r.status = 'registered'
    )
    WHERE m.id = v_meeting_id;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_meeting_registrations_count
    AFTER INSERT OR UPDATE OF status OR DELETE ON negotiation.meeting_registrations
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_sync_registered_count();

-- -----------------------------------------------------------------------------
-- 5. Documents d'aide à la négociation  (correction D3)
-- -----------------------------------------------------------------------------
CREATE TABLE negotiation.documents (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- NULL = document transversal, visible depuis tous les espaces.
    space_id              uuid        REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    slug                  platform.slug NOT NULL,
    title                 platform.i18n_text NOT NULL,
    summary               platform.i18n_text,
    -- Type via la taxonomie `document_type` (guide, note technique, rapport...).
    document_type_term_id uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    track_term_id         uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    locale_code           text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    version               text        NOT NULL DEFAULT '1',
    -- Chaînage des versions : on ne remplace jamais un document cité ailleurs.
    supersedes_id         uuid        REFERENCES negotiation.documents(id) ON DELETE SET NULL,

    -- XOR STRUCTUREL : soit un objet stocké (Garage/S3 via le module media),
    -- soit un lien externe. Jamais les deux, jamais aucun des deux.
    asset_id              uuid        CONSTRAINT xmod_fk_documents_asset
                                      REFERENCES media.assets(id) ON DELETE RESTRICT,
    external_url          platform.url,
    external_publisher    text,       -- ex. « Secrétariat de la CCNUCC »
    cover_asset_id        uuid        CONSTRAINT xmod_fk_documents_cover
                                      REFERENCES media.assets(id) ON DELETE SET NULL,

    published_at          timestamptz,
    is_restricted         boolean     NOT NULL DEFAULT true,   -- réservé aux membres de l'espace
    download_count        integer     NOT NULL DEFAULT 0,
    -- Éligibilité et état d'indexation pour le RAG de l'assistant négociateur :
    -- les embeddings (pgvector) vivent dans le module `tool`, ce module ne
    -- publie que le signal d'indexation.
    is_rag_eligible       boolean     NOT NULL DEFAULT true,
    rag_indexed_at        timestamptz,
    migrated_from_v1      boolean     NOT NULL DEFAULT false,
    uploaded_by           uuid        CONSTRAINT xmod_fk_documents_uploader
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    search_vector         tsvector    GENERATED ALWAYS AS (
        to_tsvector('french',
            coalesce(title ->> 'fr', '')   || ' ' ||
            coalesce(title ->> 'en', '')   || ' ' ||
            coalesce(summary ->> 'fr', '') || ' ' ||
            coalesce(external_publisher, ''))
    ) STORED,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_documents_source_xor CHECK (num_nonnulls(asset_id, external_url) = 1),
    CONSTRAINT ck_documents_not_self_superseding CHECK (supersedes_id IS DISTINCT FROM id)
);

-- Unicité du couple (espace, slug, version) — COALESCE pour traiter les
-- documents transversaux (space_id NULL) comme un espace virtuel unique.
CREATE UNIQUE INDEX ux_documents_slug
    ON negotiation.documents (COALESCE(space_id, '00000000-0000-0000-0000-000000000000'::uuid), slug, version);
CREATE INDEX ix_documents_published ON negotiation.documents (space_id, published_at DESC)
    WHERE published_at IS NOT NULL;
CREATE INDEX ix_documents_type      ON negotiation.documents (document_type_term_id, published_at DESC);
CREATE INDEX ix_documents_track     ON negotiation.documents (track_term_id) WHERE track_term_id IS NOT NULL;
CREATE INDEX ix_documents_search    ON negotiation.documents USING gin (search_vector);
CREATE INDEX ix_documents_rag_queue ON negotiation.documents (created_at)
    WHERE is_rag_eligible AND rag_indexed_at IS NULL AND published_at IS NOT NULL;

CREATE TRIGGER tg_documents_updated_at BEFORE UPDATE ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_documents_check_type BEFORE INSERT OR UPDATE OF document_type_term_id ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('document_type_term_id', 'document_type');
CREATE TRIGGER tg_documents_check_track BEFORE INSERT OR UPDATE OF track_term_id ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('track_term_id', 'negotiation_track');
CREATE TRIGGER tg_documents_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.documents IS
    'Documents d''aide : fichier stocké OU lien externe, jamais les deux (ck_documents_source_xor). Versionnés, typés par taxonomie, indexés plein texte.';
COMMENT ON CONSTRAINT ck_documents_source_xor ON negotiation.documents IS
    'Corrige la v1 où `file_url` accueillait indifféremment une URL de stockage et un lien tiers, rendant purge et indexation impossibles à automatiser.';

-- Incrément du compteur de téléchargements. Les statistiques fines (qui, quand,
-- depuis où) relèvent du module analytics, pas de cette colonne.
CREATE OR REPLACE FUNCTION negotiation.register_document_download(p_document_id uuid)
RETURNS void
LANGUAGE sql
AS $$
    UPDATE negotiation.documents SET download_count = download_count + 1 WHERE id = p_document_id;
$$;

-- Favoris — remplace `user_favorite_documents` (clé de substitution inutile,
-- l'unicité (personne, document) EST la clé).
CREATE TABLE negotiation.document_bookmarks (
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_document_bookmarks_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    document_id uuid        NOT NULL REFERENCES negotiation.documents(id) ON DELETE CASCADE,
    note        text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, document_id)
);

CREATE INDEX ix_document_bookmarks_document ON negotiation.document_bookmarks (document_id);
CREATE INDEX ix_document_bookmarks_recent   ON negotiation.document_bookmarks (person_id, created_at DESC);

-- -----------------------------------------------------------------------------
-- 6. Canaux d'échange temps réel  (correction D4)
--
-- Créés dynamiquement par les animateurs : par thématique, par promotion de
-- formation, ou en groupe de travail éphémère.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.channel_kind AS ENUM (
    'thematic',      -- adossé à une thématique / filière
    'cohort',        -- promotion ou cohorte de formation
    'working_group', -- groupe de travail restreint et temporaire
    'announcement',  -- diffusion descendante, écriture réservée
    'support'        -- entraide et questions aux animateurs
);

CREATE TABLE negotiation.channels (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    space_id        uuid        NOT NULL REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    kind            negotiation.channel_kind NOT NULL DEFAULT 'thematic',
    slug            platform.slug NOT NULL,
    name            platform.i18n_text NOT NULL,
    topic           text,       -- sujet courant, modifiable par les modérateurs
    track_term_id   uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE SET NULL,
    cohort_label    text,       -- ex. « Promotion 2026 »
    cohort_year     smallint    CHECK (cohort_year IS NULL OR cohort_year BETWEEN 2000 AND 2100),
    is_private      boolean     NOT NULL DEFAULT true,
    is_read_only    boolean     NOT NULL DEFAULT false,
    -- Compteurs dénormalisés : la liste des canaux se rend sans agrégat sur la
    -- table partitionnée des messages.
    message_count   bigint      NOT NULL DEFAULT 0,
    last_message_at timestamptz,
    archived_at     timestamptz,
    created_by      uuid        CONSTRAINT xmod_fk_channels_creator
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_channels_slug UNIQUE (space_id, slug),
    CONSTRAINT ck_channels_cohort CHECK (kind <> 'cohort' OR cohort_label IS NOT NULL),
    CONSTRAINT ck_channels_announcement_read_only CHECK (kind <> 'announcement' OR is_read_only)
);

-- Index partiel : la barre latérale ne liste que les canaux vivants.
CREATE INDEX ix_channels_active
    ON negotiation.channels (space_id, last_message_at DESC NULLS LAST)
    WHERE archived_at IS NULL;
CREATE INDEX ix_channels_cohort ON negotiation.channels (cohort_year, cohort_label)
    WHERE kind = 'cohort' AND archived_at IS NULL;

CREATE TRIGGER tg_channels_updated_at BEFORE UPDATE ON negotiation.channels
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE negotiation.channels IS
    'Canal d''échange temps réel créé dynamiquement, par thématique ou par promotion. Remplace `message_groups` de la v1.';

CREATE TYPE negotiation.channel_member_role AS ENUM ('owner', 'moderator', 'member');
CREATE TYPE negotiation.notification_level AS ENUM ('all', 'mentions', 'none');

CREATE TABLE negotiation.channel_members (
    channel_id           uuid        NOT NULL REFERENCES negotiation.channels(id) ON DELETE CASCADE,
    person_id            uuid        NOT NULL CONSTRAINT xmod_fk_channel_members_person
                                     REFERENCES identity.people(id) ON DELETE CASCADE,
    role                 negotiation.channel_member_role NOT NULL DEFAULT 'member',
    joined_at            timestamptz NOT NULL DEFAULT now(),
    -- Accusés de lecture : borne à partir de laquelle les messages sont non lus.
    last_read_at         timestamptz,
    last_read_message_id uuid,       -- sans FK : cible partitionnée (voir §7)
    is_muted             boolean     NOT NULL DEFAULT false,
    muted_until          timestamptz,
    notification_level   negotiation.notification_level NOT NULL DEFAULT 'all',
    left_at              timestamptz,
    PRIMARY KEY (channel_id, person_id)
);

CREATE INDEX ix_channel_members_person ON negotiation.channel_members (person_id) WHERE left_at IS NULL;

COMMENT ON COLUMN negotiation.channel_members.last_read_at IS
    'Horodatage du dernier message lu. Base du badge « non lus », absent de la v1.';

-- -----------------------------------------------------------------------------
-- 7. Messages — table partitionnée par mois
--
-- Table à plus forte volumétrie du module : mêmes règles que platform.audit_log.
--   * PRIMARY KEY (created_at, id) — la clé primaire d'une table partitionnée
--     DOIT contenir la colonne de partitionnement ;
--   * partition DEFAULT pour qu'aucune écriture ne puisse échouer si le worker
--     de maintenance a pris du retard ;
--   * AUCUNE FK ENTRANTE : elle imposerait de référencer le couple
--     (created_at, id) et interdirait DETACH/DROP des partitions anciennes.
--     Les références vers un message (réponse, accusé de lecture, réaction) sont
--     donc de simples colonnes uuid, accompagnées de l'horodatage du message
--     cible pour permettre l'élagage de partitions lors des jointures.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.moderation_state AS ENUM ('visible', 'flagged', 'hidden', 'removed');

CREATE TABLE negotiation.channel_messages (
    id                  uuid        NOT NULL DEFAULT platform.uuid_v7(),
    created_at          timestamptz NOT NULL DEFAULT now(),
    channel_id          uuid        NOT NULL REFERENCES negotiation.channels(id) ON DELETE CASCADE,
    sender_person_id    uuid        NOT NULL CONSTRAINT xmod_fk_channel_messages_sender
                                    REFERENCES identity.people(id) ON DELETE CASCADE,
    body                text,
    -- Fil de discussion : réponse à un message. Couple (id, created_at) recopié
    -- pour cibler directement la bonne partition lors des mises à jour.
    parent_message_id   uuid,
    parent_created_at   timestamptz,
    thread_root_id      uuid,
    reply_count         integer     NOT NULL DEFAULT 0,
    attachment_asset_id uuid        CONSTRAINT xmod_fk_channel_messages_attachment
                                    REFERENCES media.assets(id) ON DELETE SET NULL,
    mentioned_person_ids uuid[]     NOT NULL DEFAULT '{}',
    -- Référence d'envoi optimiste côté client (déduplication des rejeux).
    client_reference    text,
    edited_at           timestamptz,
    deleted_at          timestamptz,           -- suppression douce
    deleted_by          uuid,
    moderation_state    negotiation.moderation_state NOT NULL DEFAULT 'visible',
    moderated_by        uuid,
    moderation_reason   text,
    PRIMARY KEY (created_at, id),
    CONSTRAINT ck_channel_messages_content
        CHECK (num_nonnulls(nullif(btrim(coalesce(body, '')), ''), attachment_asset_id) >= 1),
    CONSTRAINT ck_channel_messages_parent_shape
        CHECK ((parent_message_id IS NULL) = (parent_created_at IS NULL)),
    CONSTRAINT ck_channel_messages_not_self_parent CHECK (parent_message_id IS DISTINCT FROM id)
) PARTITION BY RANGE (created_at);

-- Fil de lecture d'un canal (index partiel : les messages retirés ne sont
-- jamais rendus, ils n'ont pas à peser dans l'index).
CREATE INDEX ix_channel_messages_timeline
    ON negotiation.channel_messages (channel_id, created_at DESC)
    WHERE deleted_at IS NULL AND moderation_state = 'visible';
CREATE INDEX ix_channel_messages_thread
    ON negotiation.channel_messages (thread_root_id, created_at)
    WHERE thread_root_id IS NOT NULL;
CREATE INDEX ix_channel_messages_sender
    ON negotiation.channel_messages (sender_person_id, created_at DESC);
CREATE INDEX ix_channel_messages_moderation
    ON negotiation.channel_messages (channel_id, created_at DESC)
    WHERE moderation_state = 'flagged';
CREATE INDEX ix_channel_messages_mentions
    ON negotiation.channel_messages USING gin (mentioned_person_ids);

COMMENT ON TABLE negotiation.channel_messages IS
    'Messages des canaux, partitionnés par mois. Purge et archivage par DROP/DETACH PARTITION, jamais par DELETE massif.';
COMMENT ON COLUMN negotiation.channel_messages.client_reference IS
    'Aucune contrainte d''unicité possible ici : elle devrait inclure created_at (clé de partitionnement) et perdrait tout effet. Le rejeu est filtré par l''API sur une fenêtre courte.';

CREATE TABLE negotiation.channel_messages_default PARTITION OF negotiation.channel_messages DEFAULT;

-- Partitions du mois courant et des trois suivants ; le worker de maintenance
-- appelle ensuite platform.ensure_month_partition() en continu.
DO $$
DECLARE
    v_month date;
BEGIN
    FOR v_month IN
        SELECT generate_series(date_trunc('month', now()),
                               date_trunc('month', now()) + interval '3 months',
                               interval '1 month')::date
    LOOP
        PERFORM platform.ensure_month_partition('negotiation', 'channel_messages', v_month);
    END LOOP;
END
$$;

-- Diffusion temps réel + compteurs. Un trigger FOR EACH ROW posé sur la table
-- partitionnée est propagé à toutes ses partitions (PostgreSQL 13+).
CREATE OR REPLACE FUNCTION negotiation.tg_channel_message_fanout()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE negotiation.channels
    SET message_count   = message_count + 1,
        last_message_at = greatest(coalesce(last_message_at, NEW.created_at), NEW.created_at)
    WHERE id = NEW.channel_id;

    IF NEW.parent_message_id IS NOT NULL THEN
        UPDATE negotiation.channel_messages
        SET reply_count = reply_count + 1
        WHERE id = NEW.parent_message_id
          AND created_at = NEW.parent_created_at;   -- élagage de partition
    END IF;

    -- Le serveur WebSocket écoute ce canal LISTEN et relaie aux membres
    -- connectés : aucune scrutation périodique côté client.
    PERFORM pg_notify('negotiation_channel_message', jsonb_build_object(
        'channel_id', NEW.channel_id, 'message_id', NEW.id,
        'sender_id', NEW.sender_person_id, 'created_at', NEW.created_at,
        'parent_message_id', NEW.parent_message_id
    )::text);
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_channel_messages_fanout AFTER INSERT ON negotiation.channel_messages
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_channel_message_fanout();

-- Réactions emoji. Volontairement SANS clé étrangère vers channel_messages :
-- une FK entrante vers une table partitionnée fige les partitions (impossible
-- de les détacher ou de les supprimer). `message_created_at` est recopié pour
-- retrouver la partition du message et purger les réactions en même temps que
-- lui. L'intégrité est assurée applicativement, le coût d'une orpheline étant
-- nul (une réaction sans message ne s'affiche jamais).
CREATE TABLE negotiation.channel_message_reactions (
    message_id         uuid        NOT NULL,
    message_created_at timestamptz NOT NULL,
    channel_id         uuid        NOT NULL REFERENCES negotiation.channels(id) ON DELETE CASCADE,
    person_id          uuid        NOT NULL CONSTRAINT xmod_fk_channel_message_reactions_person
                                   REFERENCES identity.people(id) ON DELETE CASCADE,
    emoji              text        NOT NULL CHECK (length(emoji) BETWEEN 1 AND 16),
    created_at         timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, person_id, emoji)
);

CREATE INDEX ix_channel_message_reactions_channel
    ON negotiation.channel_message_reactions (channel_id, message_created_at DESC);

-- -----------------------------------------------------------------------------
-- 8. Messages non lus
-- -----------------------------------------------------------------------------
-- Compteur de non-lus par canal pour une personne : une seule requête alimente
-- toute la barre latérale. La fenêtre `p_since` borne le balayage aux partitions
-- récentes — un membre absent six mois n'entraîne pas la lecture de l'historique
-- complet.
CREATE OR REPLACE FUNCTION negotiation.unread_message_counts(
    p_person_id uuid,
    p_since     interval DEFAULT '180 days'
)
RETURNS TABLE (
    channel_id      uuid,
    channel_name    platform.i18n_text,
    unread_count    bigint,
    mention_count   bigint,
    last_message_at timestamptz,
    is_muted        boolean
)
LANGUAGE sql
STABLE
AS $$
    SELECT c.id,
           c.name,
           count(m.id),
           count(m.id) FILTER (WHERE p_person_id = ANY (m.mentioned_person_ids)),
           c.last_message_at,
           cm.is_muted AND (cm.muted_until IS NULL OR cm.muted_until > now())
    FROM negotiation.channel_members cm
    JOIN negotiation.channels c ON c.id = cm.channel_id
    LEFT JOIN negotiation.channel_messages m
           ON m.channel_id = c.id
          AND m.created_at > greatest(coalesce(cm.last_read_at, cm.joined_at), now() - p_since)
          AND m.sender_person_id <> p_person_id
          AND m.deleted_at IS NULL
          AND m.moderation_state = 'visible'
    WHERE cm.person_id = p_person_id
      AND cm.left_at IS NULL
      AND c.archived_at IS NULL
    GROUP BY c.id, c.name, c.last_message_at, cm.is_muted, cm.muted_until;
$$;

COMMENT ON FUNCTION negotiation.unread_message_counts(uuid, interval) IS
    'Badges « non lus » et « mentions » par canal pour une personne, calculés depuis son last_read_at.';

-- Marquage d'un canal comme lu jusqu'au dernier message reçu.
CREATE OR REPLACE FUNCTION negotiation.mark_channel_read(
    p_channel_id uuid,
    p_person_id  uuid,
    p_read_at    timestamptz DEFAULT now()
)
RETURNS void
LANGUAGE sql
AS $$
    UPDATE negotiation.channel_members
    SET last_read_at = greatest(coalesce(last_read_at, '-infinity'::timestamptz), p_read_at)
    WHERE channel_id = p_channel_id AND person_id = p_person_id;
$$;

-- -----------------------------------------------------------------------------
-- 9. Amorçage — permissions complémentaires et espaces d'origine
-- -----------------------------------------------------------------------------
INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('negotiation.space.manage',    '{"fr":"Administrer un espace de négociation","en":"Manage a negotiation space"}', 'negotiation'),
    ('negotiation.meeting.manage',  '{"fr":"Gérer les réunions de négociation","en":"Manage negotiation meetings"}',   'negotiation'),
    ('negotiation.document.publish','{"fr":"Publier un document d''aide","en":"Publish a support document"}',          'negotiation'),
    ('negotiation.channel.moderate','{"fr":"Modérer les canaux d''échange","en":"Moderate discussion channels"}',      'negotiation')
ON CONFLICT (code) DO NOTHING;

-- Rôle attribuable à la portée `negotiation_space` : anime un espace donné sans
-- rien pouvoir faire sur les autres.
INSERT INTO identity.roles (code, label, description, allowed_scopes, is_system) VALUES
    ('space_lead',
     '{"fr":"Animateur d''espace","en":"Space lead"}',
     '{"fr":"Anime un espace de négociation : réunions, documents, canaux","en":"Runs a negotiation space: meetings, documents, channels"}',
     '{negotiation_space}', false)
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('space_lead', 'negotiation.space.access'),
    ('space_lead', 'negotiation.content.manage'),
    ('space_lead', 'negotiation.meeting.manage'),
    ('space_lead', 'negotiation.document.publish'),
    ('space_lead', 'negotiation.channel.moderate'),
    ('admin',      'negotiation.space.manage'),
    ('admin',      'negotiation.meeting.manage'),
    ('admin',      'negotiation.document.publish'),
    ('admin',      'negotiation.channel.moderate'),
    ('trainer',    'negotiation.space.access')
ON CONFLICT DO NOTHING;

-- Les trois valeurs de l'ex-ENUM `session_category_v2` deviennent trois lignes :
-- la reprise de données v1 est une simple correspondance, et le quatrième espace
-- s'ajoutera par un INSERT.
INSERT INTO negotiation.spaces (slug, name, description, track_term_id, visibility, is_membership_open)
SELECT v.slug, v.name::jsonb::platform.i18n_text, v.description::jsonb::platform.i18n_text,
       t.id, 'listed'::negotiation.space_visibility, false
FROM (VALUES
    ('climat',
     '{"fr":"Espace Climat (CCNUCC)","en":"Climate space (UNFCCC)"}',
     '{"fr":"Négociations climatiques : COP, organes subsidiaires et concertations francophones.","en":"Climate negotiations: COP, subsidiary bodies and Francophone consultations."}',
     'climate'),
    ('biodiversite',
     '{"fr":"Espace Biodiversité (CDB)","en":"Biodiversity space (CBD)"}',
     '{"fr":"Négociations sur la diversité biologique et suivi du cadre mondial.","en":"Biological diversity negotiations and global framework follow-up."}',
     'biodiversity'),
    ('desertification',
     '{"fr":"Espace Désertification (CNULCD)","en":"Desertification space (UNCCD)"}',
     '{"fr":"Négociations sur la lutte contre la désertification et la dégradation des terres.","en":"Negotiations on desertification and land degradation."}',
     'desertification')
) AS v(slug, name, description, track_code)
JOIN reference.taxonomy_terms t
  ON t.taxonomy_code = 'negotiation_track' AND t.code = v.track_code
ON CONFLICT (slug) DO NOTHING;
