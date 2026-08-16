-- =============================================================================
-- ePavillon v2 — 060_events.sql
-- Module Événements : séries, éditions, journées, lieux et salles,
-- appels à propositions et grilles d'évaluation.
--
-- Dépend de : 000, 010, 020, 030, 040, 050
--
-- DÉCISION STRUCTURANTE N°1 — séparer la SÉRIE de l'ÉDITION
-- La v1 posait une table `events` plate, avec une colonne `year`. Impossible
-- d'exprimer que la COP29, la COP30 et la COP31 sont trois éditions du MÊME
-- rendez-vous : chaque comparaison inter-annuelle (« combien d'organisations
-- reviennent d'une COP à l'autre ? ») demandait un rapprochement manuel sur les
-- titres. En v2, `event_series` porte l'identité durable, `events` porte
-- l'édition. Les tableaux de bord pluriannuels deviennent une simple jointure.
--
-- DÉCISION STRUCTURANTE N°2 — l'APPEL À PROPOSITIONS devient une entité
-- La v1 plaçait `submission_deadline` et `submission_status` directement sur
-- l'événement. Or le métier distingue les deux :
--   « Contrairement à la COP climat, les autres COP n'ont pas forcément droit à
--     un stand puis défilé d'organisations. Parfois oui et parfois non. Quand il
--     n'y a pas de stand, on ne fait pas d'appel à propositions. »
--
-- CARDINALITÉ : un événement porte ZÉRO ou UN appel — jamais plusieurs.
-- Zéro pour une COP sans pavillon, où l'IFDD n'envoie qu'un représentant ; un
-- pour une COP climat classique. La règle est garantie par index unique
-- (ux_calls_one_per_event) et non laissée à l'application.
--
-- Note de conception : une première version de ce modèle autorisait plusieurs
-- appels par événement, en imaginant qu'une journée thématique puisse ouvrir sa
-- propre fenêtre de soumission avec ses propres critères. L'IFDD a tranché :
-- une seule campagne de soumission par édition, les journées thématiques étant
-- composées APRÈS sélection, à partir du vivier commun (event.programme_tracks).
-- C'est plus simple et cela reflète la pratique réelle. Si le besoin d'un second
-- appel apparaissait un jour, il suffirait de retirer cet index unique : rien
-- d'autre dans le schéma ne présuppose l'unicité.
--
-- La grille de notation reste attachée à l'appel, ce qui permet de la faire
-- évoluer d'une année sur l'autre sans réécrire l'historique.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Séries d'événements
-- -----------------------------------------------------------------------------
CREATE TYPE event.series_kind AS ENUM (
    'cop_climate',        -- Conférence des Parties CCNUCC
    'cop_biodiversity',   -- Conférence des Parties CDB
    'cop_desertification',-- Conférence des Parties CNULCD
    'un_conference',      -- autre conférence onusienne (FPHN, Sommet de l'avenir...)
    'webinar_series',     -- cycle périodique organisé par l'IFDD (type PACO)
    'standalone',         -- rendez-vous ponctuel sans récurrence
    'other'
);

CREATE TABLE event.event_series (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    code            text        NOT NULL UNIQUE CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    kind            event.series_kind NOT NULL,
    name            platform.i18n_text NOT NULL,
    description     platform.i18n_text,
    slug            platform.slug NOT NULL UNIQUE,
    -- Convention internationale de rattachement (code de reference.taxonomy_terms,
    -- taxonomie `negotiation_track`) : permet de croiser programmation et espace
    -- de négociation sur la même filière.
    track_code      text,
    organizer_organization_id uuid CONSTRAINT xmod_fk_event_series_organizer
                                REFERENCES org.organizations(id) ON DELETE SET NULL,
    is_active       boolean     NOT NULL DEFAULT true,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_event_series_updated_at
    BEFORE UPDATE ON event.event_series
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE event.event_series IS
    'Rendez-vous récurrent (COP climat, cycle de webinaires...). Porte l''identité durable ; les éditions sont dans event.events.';

-- -----------------------------------------------------------------------------
-- 2. Éditions
-- -----------------------------------------------------------------------------
CREATE TYPE event.event_status AS ENUM ('draft', 'announced', 'ongoing', 'completed', 'cancelled', 'suspended');
CREATE TYPE event.participation_mode AS ENUM ('online', 'in_person', 'hybrid');

CREATE TABLE event.events (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    series_id          uuid        REFERENCES event.event_series(id) ON DELETE RESTRICT,
    -- Libellé de l'édition dans sa série : « COP30 », « Session 7 ».
    edition_label      text,
    edition_year       smallint    NOT NULL CHECK (edition_year BETWEEN 2000 AND 2100),

    title              platform.i18n_text NOT NULL,
    acronym            text,
    slug               platform.slug NOT NULL,
    description        platform.i18n_text NOT NULL,
    status             event.event_status NOT NULL DEFAULT 'draft',

    participation_mode event.participation_mode NOT NULL,
    -- Fuseau de référence de l'événement : toutes les heures affichées au public
    -- sont converties depuis l'UTC vers ce fuseau, puis vers celui du visiteur.
    timezone           platform.timezone_name NOT NULL,
    starts_at          timestamptz NOT NULL,
    ends_at            timestamptz NOT NULL,

    country_id         uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    city               text,
    address            text,

    -- L'OIF tient-elle un stand/pavillon sur cette édition ? Détermine, côté
    -- métier, s'il y a lieu d'ouvrir un appel à propositions.
    has_pavilion       boolean     NOT NULL DEFAULT false,
    -- La programmation publique est-elle visible ? Remplace
    -- `is_programmation_available`, avec en plus une date de publication.
    programme_published_at timestamptz,

    -- Contenus éditoriaux légers propres à l'édition (message d'accueil,
    -- consignes d'accès). Les visuels passent par media.attachments.
    highlights         platform.i18n_text,

    created_by         uuid        CONSTRAINT xmod_fk_events_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_events_period CHECK (ends_at > starts_at),
    CONSTRAINT ck_events_physical_location
        CHECK (participation_mode = 'online' OR (country_id IS NOT NULL AND city IS NOT NULL)),
    CONSTRAINT ux_events_slug UNIQUE (slug),
    CONSTRAINT ux_events_series_edition UNIQUE (series_id, edition_year, edition_label)
);

CREATE INDEX ix_events_series   ON event.events (series_id, edition_year DESC);
CREATE INDEX ix_events_status   ON event.events (status, starts_at DESC);
CREATE INDEX ix_events_year     ON event.events (edition_year DESC);
CREATE INDEX ix_events_public   ON event.events (starts_at DESC) WHERE programme_published_at IS NOT NULL;

CREATE TRIGGER tg_events_updated_at
    BEFORE UPDATE ON event.events
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_events_audit
    AFTER INSERT OR UPDATE OR DELETE ON event.events
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN event.events.has_pavilion IS
    'Vrai si l''OIF tient un stand sur cette édition. Sans pavillon, l''IFDD envoie un représentant et n''ouvre pas d''appel.';
COMMENT ON COLUMN event.events.timezone IS
    'Fuseau de référence. Les horaires sont stockés en timestamptz et convertis à l''affichage — jamais l''inverse.';

-- -----------------------------------------------------------------------------
-- 3. Journées du calendrier et JOURNÉES SPÉCIALES
--
-- Cette table est le CALENDRIER de l'événement : une ligne par jour, rien de
-- plus. Le rattachement d'une session à son jour se déduit de sa date.
--
-- Les journées spéciales (« Journée finance durable », « Journée jeunesse et
-- climat ») ne sont PAS modélisées ici, mais dans event.programme_tracks —
-- voir la section suivante et la justification qui l'accompagne.
--
-- La v1 codait ces journées en dur dans le routeur Vue :
-- `/programmations/2025/journee-jeunesse`, `/programmations/2025/journee-finance`.
-- Créer une journée demandait un déploiement du frontend. Ici, c'est une donnée :
-- le slug alimente directement la route Nuxt `/programmation/[event]/[day]`.
-- -----------------------------------------------------------------------------
CREATE TABLE event.event_days (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id     uuid        NOT NULL REFERENCES event.events(id) ON DELETE CASCADE,
    day_date     date        NOT NULL,
    title        platform.i18n_text,
    slug         platform.slug,
    description  platform.i18n_text,
    -- Jour remarquable du calendrier (ouverture, clôture, segment de haut niveau).
    is_featured  boolean     NOT NULL DEFAULT false,
    color_hex    text        CHECK (color_hex ~ '^#[0-9a-fA-F]{6}$'),
    sort_order   smallint    NOT NULL DEFAULT 0,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_event_days_date UNIQUE (event_id, day_date),
    CONSTRAINT ux_event_days_slug UNIQUE (event_id, slug)
);

CREATE INDEX ix_event_days_event ON event.event_days (event_id, day_date);

CREATE TRIGGER tg_event_days_updated_at
    BEFORE UPDATE ON event.event_days
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- -----------------------------------------------------------------------------
-- 3 bis. Fils de programmation — les JOURNÉES SPÉCIALES
--
-- RÈGLE MÉTIER : pendant une COP, l'IFDD construit des journées spéciales —
-- « Journée finance durable », « Journée jeunesse et climat ». Leur composition
-- est une DÉCISION ÉDITORIALE : c'est l'équipe qui choisit les activités qui en
-- font partie, parmi celles qui ont été retenues.
--
-- POURQUOI UNE TABLE À PART, ET NON UN DRAPEAU SUR event_days
-- Assimiler une journée spéciale à un jour du calendrier paraît naturel et
-- serait faux sur trois points, chacun observable dès la première édition :
--   1. une journée spéciale occupe rarement le jour entier — la Journée finance
--      durable peut n'être qu'une matinée, et les activités de l'après-midi ne
--      doivent pas s'y retrouver aspirées par leur seule date ;
--   2. un même jour peut porter deux fils (jeunesse le matin, genre l'après-midi) ;
--   3. certains fils débordent d'un jour (« Séminaire de Chypre », que la v1
--      avait dû coder en dur faute de pouvoir l'exprimer).
-- Le rattachement est donc explicite, N-N, et porte la main de l'équipe.
--
-- Les thématiques d'un fil passent par reference.entity_terms
-- ('event', 'programme_tracks', <id>) : aucune table de liaison à maintenir.
-- -----------------------------------------------------------------------------
CREATE TYPE event.track_kind AS ENUM (
    'special_day',     -- journée thématique (finance durable, jeunesse climat)
    'thematic_track',  -- fil transversal courant sur plusieurs jours
    'side_programme'   -- programme parallèle (séminaire, atelier hors pavillon)
);

CREATE TABLE event.programme_tracks (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id      uuid        NOT NULL REFERENCES event.events(id) ON DELETE CASCADE,
    code          text        NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    slug          platform.slug NOT NULL,
    kind          event.track_kind NOT NULL DEFAULT 'special_day',
    title         platform.i18n_text NOT NULL,
    subtitle      platform.i18n_text,
    description   platform.i18n_text,
    -- Portée indicative, affichée au public. Elle n'impose rien : une session
    -- hors de cette plage peut être rattachée si l'équipe le décide.
    starts_on     date,
    ends_on       date,
    color_hex     text        CHECK (color_hex ~ '^#[0-9a-fA-F]{6}$'),
    -- Responsable du fil au sein de l'IFDD.
    curated_by    uuid        CONSTRAINT xmod_fk_programme_tracks_curator
                              REFERENCES identity.people(id) ON DELETE SET NULL,
    -- Page publique du fil ; NULL tant qu'elle n'est pas ouverte.
    published_at  timestamptz,
    sort_order    smallint    NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_programme_tracks_code UNIQUE (event_id, code),
    CONSTRAINT ux_programme_tracks_slug UNIQUE (event_id, slug),
    CONSTRAINT ck_programme_tracks_period
        CHECK (ends_on IS NULL OR starts_on IS NULL OR ends_on >= starts_on)
);

CREATE INDEX ix_programme_tracks_event ON event.programme_tracks (event_id, sort_order);
CREATE INDEX ix_programme_tracks_public
    ON event.programme_tracks (event_id, starts_on)
    WHERE published_at IS NOT NULL;

CREATE TRIGGER tg_programme_tracks_updated_at
    BEFORE UPDATE ON event.programme_tracks
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE event.programme_tracks IS
    'Journée spéciale ou fil thématique composé par l''IFDD. La composition est explicite (programme.session_tracks), jamais déduite des dates.';
COMMENT ON COLUMN event.programme_tracks.starts_on IS
    'Portée annoncée au public. Purement indicative : elle ne contraint pas le rattachement des sessions.';

-- -----------------------------------------------------------------------------
-- 4. Lieux et salles
--
-- Absents de la v1 : le pavillon francophone n'existait nulle part en base, et
-- les conflits de créneaux se réglaient à l'œil dans le calendrier. Les salles
-- introduites ici sont ce qui permet, dans 070, de confier au SGBD la garantie
-- de non-chevauchement (contrainte d'exclusion GiST).
-- -----------------------------------------------------------------------------
CREATE TABLE event.venues (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id    uuid        NOT NULL REFERENCES event.events(id) ON DELETE CASCADE,
    name        platform.i18n_text NOT NULL,
    -- 'pavilion' : le stand de la Francophonie ; 'partner' : espace partenaire ;
    -- 'virtual' : salle en ligne sans existence physique.
    kind        text        NOT NULL DEFAULT 'pavilion'
                            CHECK (kind IN ('pavilion', 'partner', 'plenary', 'virtual', 'other')),
    address     text,
    map_url     platform.url,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE event.rooms (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    venue_id      uuid        NOT NULL REFERENCES event.venues(id) ON DELETE CASCADE,
    name          platform.i18n_text NOT NULL,
    code          text        NOT NULL,
    capacity      smallint    CHECK (capacity IS NULL OR capacity > 0),
    -- Salle virtuelle : plusieurs sessions peuvent s'y tenir simultanément, la
    -- contrainte de non-chevauchement ne s'y applique donc pas.
    is_virtual    boolean     NOT NULL DEFAULT false,
    has_streaming boolean     NOT NULL DEFAULT false,
    equipment     text[]      NOT NULL DEFAULT '{}',
    sort_order    smallint    NOT NULL DEFAULT 0,
    created_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_rooms_code UNIQUE (venue_id, code)
);

CREATE INDEX ix_rooms_venue ON event.rooms (venue_id);

COMMENT ON COLUMN event.rooms.is_virtual IS
    'Une salle virtuelle accepte des sessions simultanées ; la contrainte d''exclusion horaire ne s''y applique pas.';

-- -----------------------------------------------------------------------------
-- 4 bis. Canaux de diffusion
--
-- RÈGLE MÉTIER : l'IFDD ne diffuse jamais deux activités en direct simultanément.
-- Une seule équipe technique, un seul flux, un seul lecteur sur la page d'accueil.
--
-- Le canal de diffusion est donc modélisé comme une RESSOURCE RÉSERVABLE, au
-- même titre qu'une salle : deux sessions diffusées ne peuvent pas l'occuper sur
-- des créneaux qui se recouvrent (voir ex_sessions_no_broadcast_overlap dans
-- 075). Ce n'est pas une contrainte codée en dur sur l'événement : si l'IFDD
-- ouvre un second canal — une chaîne dédiée à une journée thématique, une
-- diffusion en langue étrangère — il suffit d'ajouter une ligne, et les deux
-- flux peuvent alors coexister sans aucune migration.
-- -----------------------------------------------------------------------------
CREATE TABLE event.broadcast_channels (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id    uuid        REFERENCES event.events(id) ON DELETE CASCADE,
    code        text        NOT NULL CHECK (code ~ '^[a-z][a-z0-9_-]*$'),
    name        platform.i18n_text NOT NULL,
    provider    text        NOT NULL DEFAULT 'youtube'
                            CHECK (provider IN ('youtube', 'vimeo', 'facebook', 'linkedin', 'dailymotion', 'custom')),
    -- Compte diffuseur (ex. '@ifddoif') : point de départ de la détection
    -- automatique du direct en cours.
    channel_ref text,
    locale      text        REFERENCES reference.locales(code),
    -- Canal retenu par défaut pour toute session marquée comme diffusée.
    -- L'équipe n'a donc rien à saisir pour que la règle « un seul direct »
    -- s'applique : c'est l'oubli qui serait dangereux, pas la saisie.
    is_default  boolean     NOT NULL DEFAULT false,
    is_active   boolean     NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),
    -- NULLS NOT DISTINCT : les canaux généraux de la plateforme portent
    -- event_id = NULL et doivent malgré tout rester uniques par code.
    CONSTRAINT ux_broadcast_channels_code UNIQUE NULLS NOT DISTINCT (event_id, code)
);

-- Un seul canal par défaut par événement (et un seul pour les diffusions hors
-- événement, portées par la ligne event_id IS NULL).
CREATE UNIQUE INDEX ux_broadcast_channels_default
    ON event.broadcast_channels (COALESCE(event_id, '00000000-0000-0000-0000-000000000000'::uuid))
    WHERE is_default AND is_active;

CREATE INDEX ix_broadcast_channels_event ON event.broadcast_channels (event_id) WHERE is_active;

CREATE TRIGGER tg_broadcast_channels_updated_at
    BEFORE UPDATE ON event.broadcast_channels
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE event.broadcast_channels IS
    'Canal de diffusion en direct, traité comme une ressource réservable : une seule session peut l''occuper à la fois.';

-- -----------------------------------------------------------------------------
-- 5. Appels à propositions
-- -----------------------------------------------------------------------------
CREATE TYPE event.call_status AS ENUM ('draft', 'open', 'closed', 'under_review', 'published', 'cancelled');

CREATE TABLE event.calls_for_proposals (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id          uuid        NOT NULL REFERENCES event.events(id) ON DELETE CASCADE,
    code              text        NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    title             platform.i18n_text NOT NULL,
    description       platform.i18n_text,
    status            event.call_status NOT NULL DEFAULT 'draft',
    opens_at          timestamptz NOT NULL,
    closes_at         timestamptz NOT NULL,
    -- Prolongation : conservée à part pour garder la trace de l'échéance
    -- initialement annoncée aux organisations.
    extended_until    timestamptz,
    results_expected_at date,

    -- Règles de recevabilité, appliquées à la soumission.
    max_proposals_per_organization smallint CHECK (max_proposals_per_organization IS NULL OR max_proposals_per_organization > 0),
    requires_verified_organization boolean NOT NULL DEFAULT false,
    min_speakers      smallint    NOT NULL DEFAULT 1,
    max_speakers      smallint    NOT NULL DEFAULT 10,
    default_duration_minutes smallint NOT NULL DEFAULT 90,
    allowed_formats   event.participation_mode[] NOT NULL DEFAULT '{online,in_person,hybrid}',
    -- Nombre de revues indépendantes exigé avant décision : matérialise la règle
    -- « au moins deux révisionnistes se prononcent ».
    required_reviews  smallint    NOT NULL DEFAULT 2 CHECK (required_reviews >= 0),
    -- Les révisionnistes voient-ils les notes de leurs pairs avant d'avoir posé
    -- la leur ? Faux = évaluation en aveugle, pour éviter l'effet d'ancrage.
    blind_review      boolean     NOT NULL DEFAULT true,

    guidelines_url    platform.url,
    created_by        uuid        CONSTRAINT xmod_fk_calls_creator
                                  REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_calls_code UNIQUE (event_id, code),
    CONSTRAINT ck_calls_window CHECK (closes_at > opens_at),
    CONSTRAINT ck_calls_extension CHECK (extended_until IS NULL OR extended_until > closes_at),
    CONSTRAINT ck_calls_speakers CHECK (max_speakers >= min_speakers)
);

-- CARDINALITÉ 0..1 : un événement n'ouvre qu'une seule campagne de soumission.
-- Les appels annulés sont exclus, ce qui permet de repartir après une annulation
-- sans avoir à supprimer l'historique.
CREATE UNIQUE INDEX ux_calls_one_per_event
    ON event.calls_for_proposals (event_id)
    WHERE status <> 'cancelled';

CREATE INDEX ix_calls_event  ON event.calls_for_proposals (event_id, status);
CREATE INDEX ix_calls_open   ON event.calls_for_proposals (closes_at) WHERE status = 'open';

CREATE TRIGGER tg_calls_updated_at
    BEFORE UPDATE ON event.calls_for_proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_calls_audit
    AFTER INSERT OR UPDATE OR DELETE ON event.calls_for_proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN event.calls_for_proposals.blind_review IS
    'Évaluation en aveugle : un révisionniste ne voit les notes des autres qu''après avoir soumis la sienne.';

-- Échéance effective, prolongation comprise : une seule expression, utilisée
-- partout (interface, contrôle de recevabilité, tâches de clôture).
CREATE OR REPLACE FUNCTION event.effective_deadline(p_call_id uuid)
RETURNS timestamptz
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(extended_until, closes_at)
    FROM event.calls_for_proposals WHERE id = p_call_id;
$$;

CREATE OR REPLACE FUNCTION event.is_call_open(p_call_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM event.calls_for_proposals c
        WHERE c.id = p_call_id
          AND c.status = 'open'
          AND now() BETWEEN c.opens_at AND COALESCE(c.extended_until, c.closes_at)
    );
$$;

-- -----------------------------------------------------------------------------
-- 6. Grille d'évaluation
--
-- La v1 se contentait d'une note libre sur 20 (`activity_ratings.rating`), sans
-- justification structurée : impossible de savoir pourquoi une proposition
-- l'emportait, ni d'expliquer un refus à l'organisation qui le conteste.
-- Ici, la note est la somme pondérée de critères explicites, définis par appel.
-- -----------------------------------------------------------------------------
CREATE TABLE event.review_criteria (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    call_id      uuid        NOT NULL REFERENCES event.calls_for_proposals(id) ON DELETE CASCADE,
    code         text        NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    label        platform.i18n_text NOT NULL,
    description  platform.i18n_text,
    max_score    numeric(5,2) NOT NULL DEFAULT 5 CHECK (max_score > 0),
    weight       numeric(5,2) NOT NULL DEFAULT 1 CHECK (weight > 0),
    -- Critère éliminatoire : une note nulle disqualifie la proposition quelle
    -- que soit la moyenne générale.
    is_knockout  boolean     NOT NULL DEFAULT false,
    sort_order   smallint    NOT NULL DEFAULT 0,
    CONSTRAINT ux_review_criteria UNIQUE (call_id, code)
);

CREATE INDEX ix_review_criteria_call ON event.review_criteria (call_id, sort_order);

-- Grille par défaut proposée à la création d'un appel, alignée sur les usages
-- observés à l'IFDD. Modifiable appel par appel.
CREATE OR REPLACE FUNCTION event.seed_default_criteria(p_call_id uuid)
RETURNS void
LANGUAGE sql
AS $$
    INSERT INTO event.review_criteria (call_id, code, label, max_score, weight, is_knockout, sort_order) VALUES
        (p_call_id, 'relevance',    '{"fr":"Pertinence au regard des priorités de l''IFDD","en":"Relevance to IFDD priorities"}', 5, 2.0, true,  10),
        (p_call_id, 'quality',      '{"fr":"Qualité et clarté de la proposition","en":"Quality and clarity of the proposal"}',    5, 1.5, false, 20),
        (p_call_id, 'impact',       '{"fr":"Impact et retombées attendues","en":"Expected impact and outcomes"}',                 5, 1.5, false, 30),
        (p_call_id, 'innovation',   '{"fr":"Caractère innovant","en":"Innovation"}',                                              5, 1.0, false, 40),
        (p_call_id, 'inclusiveness', '{"fr":"Inclusion (genre, jeunesse, sociétés civiles)","en":"Inclusiveness"}',               5, 1.0, false, 50),
        (p_call_id, 'feasibility',  '{"fr":"Faisabilité logistique","en":"Logistical feasibility"}',                              5, 1.0, false, 60)
    ON CONFLICT (call_id, code) DO NOTHING;
$$;

-- Note maximale atteignable sur un appel : sert à normaliser l'affichage
-- (« 17,5 / 20 ») quelle que soit la grille retenue.
CREATE OR REPLACE FUNCTION event.max_weighted_score(p_call_id uuid)
RETURNS numeric
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(sum(max_score * weight), 0)
    FROM event.review_criteria WHERE call_id = p_call_id;
$$;

-- -----------------------------------------------------------------------------
-- 7. Comité de sélection
--
-- Qui évalue quel appel. Complète le RBAC : le rôle `reviewer` peut être
-- attribué globalement, cette table dit qui siège effectivement sur cet appel
-- et permet de répartir la charge (voir programme.review_assignments).
-- -----------------------------------------------------------------------------
CREATE TABLE event.call_reviewers (
    call_id     uuid        NOT NULL REFERENCES event.calls_for_proposals(id) ON DELETE CASCADE,
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_call_reviewers_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    is_lead     boolean     NOT NULL DEFAULT false,
    -- Plafond indicatif de propositions confiées à ce membre.
    workload_cap smallint,
    added_at    timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (call_id, person_id)
);

COMMENT ON TABLE event.call_reviewers IS
    'Composition du comité de sélection d''un appel. L''autorisation reste portée par identity.role_assignments (portée event).';

-- -----------------------------------------------------------------------------
-- 8. Déclaration des références vers les organisations
--
-- Permet à org.merge_organizations() de réaffecter automatiquement ce module
-- lors d'une fusion de fiches.
-- -----------------------------------------------------------------------------
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy) VALUES
    ('event', 'event_series', 'organizer_organization_id', 'reassign')
ON CONFLICT DO NOTHING;
