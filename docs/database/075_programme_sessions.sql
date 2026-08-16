-- =============================================================================
-- ePavillon v2 — 075_programme_sessions.sql
-- Module Programmation, partie 2 : sessions programmées, détection des conflits
-- de créneaux, inscriptions, présence, questions du public, comptes rendus.
--
-- Dépend de : 000, 010, 020, 030, 040, 050, 060, 070
--
-- DÉCISION STRUCTURANTE N°1 — les conflits de créneaux sont SIGNALÉS, jamais
-- bloqués
-- Consigne explicite de l'IFDD : « Il ne faut pas rejeter systématiquement les
-- chevauchements de créneaux. Les organisations peuvent proposer des créneaux
-- sans se soucier de la date et de l'heure ; c'est à l'admin de réorganiser
-- dans le back-office, visuellement, par glisser-déposer. »
--
-- Une première version de ce modèle posait une contrainte d'exclusion GiST qui
-- rendait le chevauchement impossible à écrire. C'était une erreur de
-- conception : elle transformait un outil d'arbitrage en mur. Un planificateur
-- travaille par déplacements successifs, et un état transitoire incohérent —
-- deux blocs superposés le temps de recaler le second — fait partie du travail.
-- Une base qui refuse l'écriture oblige à contourner l'outil.
--
-- Le modèle retenu : `programme.detect_conflicts()` recense tous les
-- chevauchements avec leur gravité, et l'interface les rend visibles en
-- permanence dans le calendrier. Rien n'est refusé ; rien n'est caché.
-- Le garde-fou se déplace au moment qui compte vraiment, la PUBLICATION du
-- programme : `programme.publication_readiness()` liste ce qui doit être réglé
-- avant de rendre la programmation publique.
--
-- DÉCISION STRUCTURANTE N°2 — un formulaire d'inscription configurable
-- La v1 a vu `activity_registrations` grossir au fil des besoins : six colonnes
-- `guest_*`, `session_edition`, `referral_source`, `referral_source_other`,
-- `fallback_payload`, `fallback_error`, `recovered_at`, plus une table annexe
-- `paco_demographic_data`. Chaque nouvelle question posée aux inscrits coûtait
-- une migration et un déploiement. En v2, les questions sont des données
-- (`registration_form_fields`) et les réponses un document JSON validé.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Sessions programmées
-- -----------------------------------------------------------------------------
CREATE TYPE programme.session_status AS ENUM (
    'planned',    -- créneau pressenti, non public
    'scheduled',  -- programmé et publié
    'live',       -- en cours de diffusion
    'completed',  -- terminé
    'postponed',  -- reporté, nouveau créneau à venir
    'cancelled'
);

CREATE TABLE programme.sessions (
    id               uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id         uuid        NOT NULL CONSTRAINT xmod_fk_sessions_event
                                 REFERENCES event.events(id) ON DELETE CASCADE,
    -- NULL quand l'IFDD programme directement une activité, sans appel.
    proposal_id      uuid        REFERENCES programme.proposals(id) ON DELETE SET NULL,
    event_day_id     uuid        CONSTRAINT xmod_fk_sessions_event_day
                                 REFERENCES event.event_days(id) ON DELETE SET NULL,
    organization_id  uuid        CONSTRAINT xmod_fk_sessions_organization
                                 REFERENCES org.organizations(id) ON DELETE SET NULL,

    -- Rang de l'occurrence au sein d'une proposition multi-sessions. Remplace la
    -- colonne `session_edition` greffée dans les inscriptions de la v1.
    sequence_number  smallint    NOT NULL DEFAULT 1 CHECK (sequence_number > 0),

    title            platform.i18n_text NOT NULL,
    slug             platform.slug NOT NULL,
    summary          platform.i18n_text,
    description      platform.i18n_text,

    status           programme.session_status NOT NULL DEFAULT 'planned',
    format           event.participation_mode NOT NULL,

    starts_at        timestamptz NOT NULL,
    ends_at          timestamptz NOT NULL,
    -- Intervalle dérivé : support de la détection des chevauchements (opérateur
    -- && dans detect_conflicts()) et des requêtes de calendrier (« que se
    -- passe-t-il entre 14 h et 16 h ? »).
    time_range       tstzrange   GENERATED ALWAYS AS (tstzrange(starts_at, ends_at, '[)')) STORED,
    timezone         platform.timezone_name NOT NULL,

    room_id          uuid        CONSTRAINT xmod_fk_sessions_room
                                 REFERENCES event.rooms(id) ON DELETE SET NULL,
    -- Alimentée depuis event.rooms.is_virtual : une salle virtuelle accepte des
    -- sessions simultanées, une salle physique non. Dénormalisée pour que le
    -- calendrier du back-office sache colorer un chevauchement sans joindre
    -- event.rooms à chaque bloc affiché. Purement indicative : elle ne bloque
    -- aucune écriture, l'arbitrage passe par detect_conflicts().
    enforce_room_exclusivity boolean NOT NULL DEFAULT true,
    location_note    platform.i18n_text,

    -- Inscription
    registration_required boolean NOT NULL DEFAULT true,
    registration_opens_at  timestamptz,
    registration_closes_at timestamptz,
    capacity         integer     CHECK (capacity IS NULL OR capacity > 0),
    waitlist_enabled boolean     NOT NULL DEFAULT false,
    registration_form_id uuid,   -- FK ajoutée après création de la table des formulaires

    -- Diffusion : les identifiants techniques vivent dans le module `live`,
    -- seules les intentions restent ici.
    is_streamed      boolean     NOT NULL DEFAULT false,
    -- Canal occupé pendant la diffusion. Renseigné automatiquement avec le canal
    -- par défaut de l'événement dès que is_streamed passe à vrai : la règle
    -- « un seul direct à la fois » ne doit pas dépendre d'une saisie.
    broadcast_channel_id uuid    CONSTRAINT xmod_fk_sessions_broadcast_channel
                                 REFERENCES event.broadcast_channels(id) ON DELETE SET NULL,
    is_recorded      boolean     NOT NULL DEFAULT true,
    allows_questions boolean     NOT NULL DEFAULT true,

    -- Suivi post-activité (ex-`completion_report`).
    report           platform.i18n_text,
    report_submitted_at timestamptz,
    attendee_count   integer,
    view_count       integer     NOT NULL DEFAULT 0,

    search_vector    tsvector    GENERATED ALWAYS AS (
        to_tsvector('french',
            coalesce(title ->> 'fr', '') || ' ' || coalesce(summary ->> 'fr', ''))
    ) STORED,

    published_at     timestamptz,
    cancelled_reason platform.i18n_text,
    created_by       uuid        CONSTRAINT xmod_fk_sessions_creator
                                 REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_sessions_period CHECK (ends_at > starts_at),
    CONSTRAINT ck_sessions_registration_window
        CHECK (registration_closes_at IS NULL OR registration_opens_at IS NULL
               OR registration_closes_at > registration_opens_at),
    CONSTRAINT ck_sessions_cancelled_reason
        CHECK (status <> 'cancelled' OR cancelled_reason IS NOT NULL),
    CONSTRAINT ux_sessions_slug UNIQUE (event_id, slug),
    CONSTRAINT ux_sessions_proposal_sequence UNIQUE (proposal_id, sequence_number)
);

-- Aucune contrainte d'exclusion n'est posée sur les créneaux : voir la décision
-- structurante n°1 en tête de fichier. Les chevauchements sont détectés et
-- affichés par programme.detect_conflicts(), pas refusés à l'écriture.

CREATE INDEX ix_sessions_event_time  ON programme.sessions (event_id, starts_at);
CREATE INDEX ix_sessions_public      ON programme.sessions (event_id, starts_at)
    WHERE published_at IS NOT NULL AND status <> 'cancelled';
CREATE INDEX ix_sessions_day         ON programme.sessions (event_day_id, starts_at);
CREATE INDEX ix_sessions_proposal    ON programme.sessions (proposal_id);
CREATE INDEX ix_sessions_organization ON programme.sessions (organization_id, starts_at DESC);
CREATE INDEX ix_sessions_search      ON programme.sessions USING gin (search_vector);
CREATE INDEX ix_sessions_time_range  ON programme.sessions USING gist (time_range);

CREATE TRIGGER tg_sessions_updated_at
    BEFORE UPDATE ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_sessions_audit
    AFTER INSERT OR UPDATE OR DELETE ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN programme.sessions.sequence_number IS
    'Rang de l''occurrence dans une proposition multi-sessions (cycles de webinaires).';

-- Alignement automatique de l'exclusivité de salle, de la journée de
-- rattachement et du canal de diffusion : ces trois valeurs sont déductibles,
-- elles ne doivent pas reposer sur une saisie.
CREATE OR REPLACE FUNCTION programme.tg_sessions_derive_fields()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.room_id IS NOT NULL THEN
        SELECT NOT r.is_virtual INTO NEW.enforce_room_exclusivity
        FROM event.rooms r WHERE r.id = NEW.room_id;
    ELSE
        NEW.enforce_room_exclusivity := false;
    END IF;

    IF NEW.event_day_id IS NULL THEN
        SELECT d.id INTO NEW.event_day_id
        FROM event.event_days d
        JOIN event.events e ON e.id = d.event_id
        WHERE d.event_id = NEW.event_id
          AND d.day_date = (NEW.starts_at AT TIME ZONE e.timezone)::date;
    END IF;

    -- Une session diffusée occupe le canal par défaut de son événement, à
    -- défaut le canal général de la plateforme. Sans cette affectation
    -- automatique, une session marquée « diffusée » sans canal échapperait à la
    -- détection des conflits de diffusion : le contrôle serait effectif pour
    -- ceux qui connaissent la règle et contourné par distraction.
    IF NEW.is_streamed AND NEW.broadcast_channel_id IS NULL THEN
        SELECT c.id INTO NEW.broadcast_channel_id
        FROM event.broadcast_channels c
        WHERE c.is_active
          AND c.is_default
          AND (c.event_id = NEW.event_id OR c.event_id IS NULL)
        ORDER BY (c.event_id IS NOT NULL) DESC
        LIMIT 1;
    ELSIF NOT NEW.is_streamed THEN
        NEW.broadcast_channel_id := NULL;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_sessions_derive
    BEFORE INSERT OR UPDATE OF room_id, starts_at, event_id, is_streamed, broadcast_channel_id
    ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sessions_derive_fields();

-- Publication d'un événement de domaine à chaque changement d'état : c'est ce
-- qui déclenche la création de la réunion visio, les rappels et les
-- notifications, sans que ce module connaisse les modules concernés.
CREATE OR REPLACE FUNCTION programme.tg_sessions_emit_events()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.status IS NOT DISTINCT FROM OLD.status
       AND NEW.starts_at IS NOT DISTINCT FROM OLD.starts_at THEN
        RETURN NULL;
    END IF;

    PERFORM platform.emit_event(
        'programme', 'session', NEW.id,
        CASE
            WHEN TG_OP = 'INSERT' THEN 'programme.session.created'
            WHEN NEW.status IS DISTINCT FROM OLD.status THEN 'programme.session.' || NEW.status::text
            ELSE 'programme.session.rescheduled'
        END,
        jsonb_build_object(
            'event_id', NEW.event_id,
            'title', NEW.title,
            'starts_at', NEW.starts_at,
            'ends_at', NEW.ends_at,
            'timezone', NEW.timezone,
            'format', NEW.format,
            'previous_starts_at', CASE WHEN TG_OP = 'UPDATE' THEN OLD.starts_at END
        )
    );
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_sessions_events
    AFTER INSERT OR UPDATE OF status, starts_at, ends_at ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sessions_emit_events();

-- -----------------------------------------------------------------------------
-- 2. Intervenants de la session
--
-- Recopiés depuis la proposition à la programmation, puis modifiables : les
-- intervenants annoncés dans le dossier ne sont pas toujours ceux du jour.
-- -----------------------------------------------------------------------------
CREATE TABLE programme.session_speakers (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    session_id        uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    person_id         uuid        NOT NULL CONSTRAINT xmod_fk_session_speakers_person
                                  REFERENCES identity.people(id) ON DELETE RESTRICT,
    role              programme.speaker_role NOT NULL DEFAULT 'speaker',
    job_title_snapshot text,
    organization_snapshot text,
    bio               platform.i18n_text,
    confirmed_at      timestamptz,
    attended          boolean,
    sort_order        smallint    NOT NULL DEFAULT 0,
    created_at        timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_session_speakers UNIQUE (session_id, person_id, role)
);

CREATE INDEX ix_session_speakers_session ON programme.session_speakers (session_id, sort_order);
CREATE INDEX ix_session_speakers_person  ON programme.session_speakers (person_id);

-- -----------------------------------------------------------------------------
-- 2 ter. Co-organisation de la session
--
-- Reprise depuis la proposition à la programmation, puis modifiable : la liste
-- des co-organisateurs annoncée dans le dossier n'est pas toujours celle du
-- jour. Même modèle que programme.proposal_organizations.
-- -----------------------------------------------------------------------------
CREATE TABLE programme.session_organizations (
    session_id      uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    organization_id uuid        NOT NULL CONSTRAINT xmod_fk_session_organizations_org
                                REFERENCES org.organizations(id) ON DELETE RESTRICT,
    role            programme.organization_role NOT NULL DEFAULT 'co_organizer',
    sort_order      smallint    NOT NULL DEFAULT 0,
    added_at        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, organization_id)
);

CREATE UNIQUE INDEX ux_session_organizations_lead
    ON programme.session_organizations (session_id)
    WHERE role = 'lead';

CREATE INDEX ix_session_organizations_org
    ON programme.session_organizations (organization_id, role);

COMMENT ON TABLE programme.session_organizations IS
    'Organisations d''une session publiée, porteur principal compris. Alimente l''affichage public et le décompte par organisation.';

-- Synchronisation du porteur principal, comme sur les propositions.
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
    WHERE session_id = NEW.id AND role = 'lead' AND organization_id <> NEW.organization_id;

    INSERT INTO programme.session_organizations (session_id, organization_id, role, sort_order)
    VALUES (NEW.id, NEW.organization_id, 'lead', 0)
    ON CONFLICT (session_id, organization_id) DO UPDATE SET role = 'lead';

    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_sessions_sync_lead_organization
    AFTER INSERT OR UPDATE OF organization_id ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sync_session_lead_organization();

-- -----------------------------------------------------------------------------
-- 2 bis. Rattachement aux journées spéciales
--
-- La composition d'une journée spéciale (« Journée finance durable ») est faite
-- à la main par l'IFDD parmi les activités retenues. Une session peut relever
-- de plusieurs fils, et un fil peut piocher dans plusieurs jours.
--
-- On trace qui a rattaché quoi : la composition d'une journée thématique est un
-- choix éditorial, il arrive qu'on ait à l'expliquer à une organisation qui
-- s'étonne de ne pas y figurer.
-- -----------------------------------------------------------------------------
CREATE TABLE programme.session_tracks (
    session_id uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    track_id   uuid        NOT NULL CONSTRAINT xmod_fk_session_tracks_track
                           REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    -- Ordre de présentation dans la page du fil : le déroulé d'une journée
    -- spéciale n'est pas toujours l'ordre chronologique brut.
    sort_order smallint    NOT NULL DEFAULT 0,
    is_highlight boolean   NOT NULL DEFAULT false,
    added_by   uuid        CONSTRAINT xmod_fk_session_tracks_actor
                           REFERENCES identity.people(id) ON DELETE SET NULL,
    added_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (session_id, track_id)
);

CREATE INDEX ix_session_tracks_track ON programme.session_tracks (track_id, sort_order);

COMMENT ON TABLE programme.session_tracks IS
    'Composition des journées spéciales et fils thématiques : rattachement explicite décidé par l''IFDD, jamais déduit des dates.';

-- Une session ne peut rejoindre que les fils de son propre événement : sans ce
-- contrôle, un glisser-déposer maladroit dans le back-office ferait apparaître
-- une activité de la COP30 dans la programmation de la COP31.
CREATE OR REPLACE FUNCTION programme.tg_check_session_track_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_session_event uuid;
    v_track_event   uuid;
BEGIN
    SELECT event_id INTO v_session_event FROM programme.sessions WHERE id = NEW.session_id;
    SELECT event_id INTO v_track_event   FROM event.programme_tracks WHERE id = NEW.track_id;

    IF v_session_event IS DISTINCT FROM v_track_event THEN
        RAISE EXCEPTION 'Rattachement impossible : la session et le fil de programmation appartiennent à deux événements différents.'
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_session_tracks_check_event
    BEFORE INSERT OR UPDATE ON programme.session_tracks
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_session_track_event();

-- -----------------------------------------------------------------------------
-- 3. Formulaires d'inscription configurables
-- -----------------------------------------------------------------------------
CREATE TABLE programme.registration_forms (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    code        text        NOT NULL UNIQUE CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    name        platform.i18n_text NOT NULL,
    description platform.i18n_text,
    -- Formulaire appliqué par défaut aux sessions d'un événement donné.
    event_id    uuid        CONSTRAINT xmod_fk_registration_forms_event
                            REFERENCES event.events(id) ON DELETE CASCADE,
    is_default  boolean     NOT NULL DEFAULT false,
    -- Une personne non connectée peut-elle s'inscrire ? Le cas « invité » de la
    -- v1 devient un simple réglage : la personne est créée dans identity.people
    -- sans compte, et sera rattachée à son compte le jour où elle en ouvre un.
    allows_anonymous boolean NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_registration_forms_updated_at
    BEFORE UPDATE ON programme.registration_forms
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE UNIQUE INDEX ux_registration_forms_default
    ON programme.registration_forms (COALESCE(event_id, '00000000-0000-0000-0000-000000000000'::uuid))
    WHERE is_default;

CREATE TYPE programme.form_field_type AS ENUM (
    'text', 'long_text', 'email', 'phone', 'number', 'date',
    'single_choice', 'multiple_choice', 'boolean', 'country', 'taxonomy_term'
);

CREATE TABLE programme.registration_form_fields (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    form_id       uuid        NOT NULL REFERENCES programme.registration_forms(id) ON DELETE CASCADE,
    code          text        NOT NULL CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    label         platform.i18n_text NOT NULL,
    help_text     platform.i18n_text,
    field_type    programme.form_field_type NOT NULL,
    is_required   boolean     NOT NULL DEFAULT false,
    -- Options : soit une liste explicite `[{"value":"x","label":{"fr":"..."}}]`,
    -- soit une taxonomie référencée `{"taxonomy":"referral_source"}`.
    options       jsonb       NOT NULL DEFAULT '{}'::jsonb,
    validation    jsonb       NOT NULL DEFAULT '{}'::jsonb,  -- min, max, pattern...
    -- Donnée à caractère personnel sensible : impose le recueil d'un
    -- consentement et l'exclusion des exports non anonymisés.
    is_sensitive  boolean     NOT NULL DEFAULT false,
    sort_order    smallint    NOT NULL DEFAULT 0,
    is_active     boolean     NOT NULL DEFAULT true,
    CONSTRAINT ux_registration_form_fields UNIQUE (form_id, code)
);

CREATE INDEX ix_registration_form_fields ON programme.registration_form_fields (form_id, sort_order) WHERE is_active;

ALTER TABLE programme.sessions
    ADD CONSTRAINT fk_sessions_registration_form
    FOREIGN KEY (registration_form_id) REFERENCES programme.registration_forms(id) ON DELETE SET NULL;

COMMENT ON TABLE programme.registration_form_fields IS
    'Questions posées à l''inscription. Ajouter une question ne demande plus ni migration ni déploiement.';

-- -----------------------------------------------------------------------------
-- 4. Inscriptions
-- -----------------------------------------------------------------------------
CREATE TYPE programme.registration_status AS ENUM (
    'registered', 'waitlisted', 'cancelled', 'attended', 'no_show'
);

CREATE TABLE programme.registrations (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    session_id      uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    -- Une seule colonne, quel que soit le profil : la personne existe toujours,
    -- avec ou sans compte. Fin de la dualité utilisateur/invité de la v1.
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_registrations_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    organization_id uuid        CONSTRAINT xmod_fk_registrations_organization
                                REFERENCES org.organizations(id) ON DELETE SET NULL,

    status          programme.registration_status NOT NULL DEFAULT 'registered',
    -- Réponses au formulaire, indexées en GIN : les statistiques par canal
    -- d'acquisition ou par tranche d'âge se calculent sans table annexe.
    answers         jsonb       NOT NULL DEFAULT '{}'::jsonb,
    locale          text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),

    -- Position en liste d'attente ; NULL si inscription confirmée.
    waitlist_position integer,

    -- Participation. `joined_at` est écrit une seule fois (premier clic sur
    -- « Rejoindre »), ce qui donne un taux de présence réel exploitable.
    joined_at       timestamptz,
    attendance_minutes integer,
    certificate_asset_id uuid   CONSTRAINT xmod_fk_registrations_certificate
                                REFERENCES media.assets(id) ON DELETE SET NULL,

    -- Provenance technique de l'inscription, utile au diagnostic.
    source          text        NOT NULL DEFAULT 'web'
                                CHECK (source IN ('web', 'import', 'admin', 'api', 'partner')),
    cancelled_at    timestamptz,
    cancelled_reason text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_registrations_waitlist
        CHECK ((status = 'waitlisted') = (waitlist_position IS NOT NULL))
);

-- Une personne ne s'inscrit qu'une fois par session ; une annulation libère la place.
CREATE UNIQUE INDEX ux_registrations_person_session
    ON programme.registrations (session_id, person_id)
    WHERE status <> 'cancelled';

CREATE INDEX ix_registrations_session   ON programme.registrations (session_id, status);
CREATE INDEX ix_registrations_person    ON programme.registrations (person_id, created_at DESC);
CREATE INDEX ix_registrations_answers   ON programme.registrations USING gin (answers jsonb_path_ops);
CREATE INDEX ix_registrations_attendance ON programme.registrations (session_id)
    WHERE joined_at IS NOT NULL;

CREATE TRIGGER tg_registrations_updated_at
    BEFORE UPDATE ON programme.registrations
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON COLUMN programme.registrations.answers IS
    'Réponses au formulaire, clés = registration_form_fields.code. Indexé GIN pour les statistiques.';
COMMENT ON COLUMN programme.registrations.joined_at IS
    'Premier clic sur « Rejoindre » (écrit une seule fois). Base du taux de participation réel.';

-- Validation des réponses obligatoires et respect de la capacité, appliqués en
-- base : aucun chemin d'écriture ne peut créer une inscription incomplète ni
-- dépasser silencieusement la jauge.
CREATE OR REPLACE FUNCTION programme.tg_validate_registration()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_session  programme.sessions%ROWTYPE;
    v_missing  text[];
    v_taken    integer;
BEGIN
    SELECT * INTO v_session FROM programme.sessions WHERE id = NEW.session_id;

    IF v_session.status IN ('cancelled') THEN
        RAISE EXCEPTION 'Session annulée : inscription impossible.' USING ERRCODE = 'restrict_violation';
    END IF;

    IF TG_OP = 'INSERT' AND v_session.registration_closes_at IS NOT NULL
       AND now() > v_session.registration_closes_at THEN
        RAISE EXCEPTION 'Les inscriptions à cette session sont closes depuis le %.',
            v_session.registration_closes_at USING ERRCODE = 'restrict_violation';
    END IF;

    -- Champs obligatoires du formulaire rattaché.
    IF v_session.registration_form_id IS NOT NULL THEN
        SELECT array_agg(f.code ORDER BY f.sort_order) INTO v_missing
        FROM programme.registration_form_fields f
        WHERE f.form_id = v_session.registration_form_id
          AND f.is_active
          AND f.is_required
          AND COALESCE(NEW.answers ->> f.code, '') = '';

        IF v_missing IS NOT NULL THEN
            RAISE EXCEPTION 'Réponses obligatoires manquantes : %', array_to_string(v_missing, ', ')
                USING ERRCODE = 'not_null_violation';
        END IF;
    END IF;

    -- Jauge : bascule automatique en liste d'attente si elle est activée.
    IF TG_OP = 'INSERT' AND v_session.capacity IS NOT NULL AND NEW.status = 'registered' THEN
        SELECT count(*) INTO v_taken
        FROM programme.registrations r
        WHERE r.session_id = NEW.session_id AND r.status IN ('registered', 'attended');

        IF v_taken >= v_session.capacity THEN
            IF v_session.waitlist_enabled THEN
                NEW.status := 'waitlisted';
                SELECT COALESCE(max(waitlist_position), 0) + 1 INTO NEW.waitlist_position
                FROM programme.registrations WHERE session_id = NEW.session_id;
            ELSE
                RAISE EXCEPTION 'Capacité atteinte (% places).', v_session.capacity
                    USING ERRCODE = 'restrict_violation';
            END IF;
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_registrations_validate
    BEFORE INSERT OR UPDATE OF answers, status ON programme.registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_validate_registration();

-- Événement de domaine : déclenche l'inscription auprès du fournisseur visio,
-- la confirmation par courriel et la planification des rappels.
CREATE OR REPLACE FUNCTION programme.tg_registrations_emit_events()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM platform.emit_event(
        'programme', 'registration', NEW.id,
        CASE WHEN TG_OP = 'INSERT' THEN 'programme.registration.created'
             ELSE 'programme.registration.' || NEW.status::text END,
        jsonb_build_object(
            'session_id', NEW.session_id,
            'person_id', NEW.person_id,
            'status', NEW.status,
            'locale', NEW.locale
        )
    );
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_registrations_events
    AFTER INSERT OR UPDATE OF status ON programme.registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_registrations_emit_events();

-- Premier clic sur « Rejoindre » : écrit une seule fois, jamais écrasé.
CREATE OR REPLACE FUNCTION programme.record_join(p_registration_id uuid)
RETURNS timestamptz
LANGUAGE sql
AS $$
    UPDATE programme.registrations
    SET joined_at = COALESCE(joined_at, now())
    WHERE id = p_registration_id
    RETURNING joined_at;
$$;

-- Promotion depuis la liste d'attente lors d'une annulation.
CREATE OR REPLACE FUNCTION programme.promote_from_waitlist(p_session_id uuid, p_count integer DEFAULT 1)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    v_promoted integer;
BEGIN
    WITH next_in_line AS (
        SELECT id FROM programme.registrations
        WHERE session_id = p_session_id AND status = 'waitlisted'
        ORDER BY waitlist_position
        LIMIT p_count
        FOR UPDATE SKIP LOCKED
    )
    UPDATE programme.registrations r
    SET status = 'registered', waitlist_position = NULL
    FROM next_in_line n
    WHERE r.id = n.id;

    GET DIAGNOSTICS v_promoted = ROW_COUNT;
    RETURN v_promoted;
END;
$$;

-- -----------------------------------------------------------------------------
-- 5. Questions du public
-- -----------------------------------------------------------------------------
CREATE TABLE programme.session_questions (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    session_id     uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    person_id      uuid        CONSTRAINT xmod_fk_session_questions_person
                               REFERENCES identity.people(id) ON DELETE SET NULL,
    body           text        NOT NULL CHECK (length(btrim(body)) BETWEEN 3 AND 2000),
    target_speaker_ids uuid[]  NOT NULL DEFAULT '{}',
    upvotes        integer     NOT NULL DEFAULT 0,
    -- Modération : une question masquée reste tracée (qui, quand, pourquoi).
    is_visible     boolean     NOT NULL DEFAULT true,
    moderated_by   uuid        CONSTRAINT xmod_fk_session_questions_moderator
                               REFERENCES identity.people(id) ON DELETE SET NULL,
    moderated_at   timestamptz,
    moderation_reason text,
    answered_at    timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_session_questions ON programme.session_questions (session_id, upvotes DESC, created_at)
    WHERE is_visible;

CREATE TABLE programme.session_question_answers (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    question_id  uuid        NOT NULL REFERENCES programme.session_questions(id) ON DELETE CASCADE,
    answered_by  uuid        CONSTRAINT xmod_fk_question_answers_person
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    body         text        NOT NULL,
    is_official  boolean     NOT NULL DEFAULT true,
    created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE programme.session_question_votes (
    question_id uuid        NOT NULL REFERENCES programme.session_questions(id) ON DELETE CASCADE,
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_question_votes_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (question_id, person_id)
);

-- -----------------------------------------------------------------------------
-- 6. Vue calendrier
--
-- Alimente directement la vue calendrier du frontend (une ligne = un bloc), et
-- la vue grille. Le calcul de l'état d'avancement est fait une fois, en base,
-- plutôt que dans chaque composant.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW programme.v_public_schedule AS
SELECT
    s.id,
    s.event_id,
    s.event_day_id,
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
    s.is_streamed,
    s.broadcast_channel_id,
    s.capacity,
    -- Journées spéciales et fils thématiques auxquels la session est rattachée :
    -- alimente les pastilles « Journée finance durable » de la vue grille sans
    -- requête supplémentaire.
    COALESCE((
        SELECT jsonb_agg(jsonb_build_object(
                   'slug', t.slug, 'title', t.title, 'color', t.color_hex, 'kind', t.kind)
               ORDER BY st.sort_order, t.sort_order)
        FROM programme.session_tracks st
        JOIN event.programme_tracks t ON t.id = st.track_id
        WHERE st.session_id = s.id AND t.published_at IS NOT NULL
    ), '[]'::jsonb) AS tracks,
    -- État temporel dérivé : pilote la couleur du bloc dans le calendrier.
    CASE
        WHEN s.status = 'cancelled'          THEN 'cancelled'
        WHEN s.status = 'postponed'          THEN 'postponed'
        WHEN now() < s.starts_at             THEN 'upcoming'
        WHEN now() BETWEEN s.starts_at AND s.ends_at THEN 'ongoing'
        ELSE 'past'
    END AS temporal_state,
    (SELECT count(*) FROM programme.registrations rg
      WHERE rg.session_id = s.id AND rg.status IN ('registered', 'attended')) AS registered_count,
    reference.terms_of('programme', 'sessions', s.id, 'activity_theme') AS theme_codes
FROM programme.sessions s
LEFT JOIN event.rooms r        ON r.id = s.room_id
LEFT JOIN org.organizations o  ON o.id = s.organization_id
WHERE s.published_at IS NOT NULL;

COMMENT ON VIEW programme.v_public_schedule IS
    'Programmation publique prête à l''affichage (vue grille et vue calendrier), état temporel calculé en base.';

-- -----------------------------------------------------------------------------
-- 7. Détection des conflits
--
-- Aucun de ces conflits n'empêche l'écriture : ils alimentent l'écran
-- d'arbitrage et le bandeau d'alerte du planificateur. L'équipe déplace les
-- blocs jusqu'à ce que la liste soit vide.
--
-- Deux niveaux :
--   'blocking'  — matériellement impossible. L'IFDD tient UN SEUL stand : deux
--                 activités d'un même événement ne peuvent pas se tenir en même
--                 temps. Et une seule équipe technique : un seul direct à la
--                 fois, tous événements confondus. Ces conflits doivent être
--                 résolus avant publication.
--   'warning'   — gênant mais possible : un intervenant attendu à deux endroits,
--                 une organisation programmée deux fois. L'équipe juge.
--
-- Portée : les conflits de STAND sont cherchés au sein de l'événement — deux
-- événements distincts (un webinaire et une COP) peuvent parfaitement se tenir
-- en parallèle. Les conflits de DIFFUSION, eux, sont cherchés au-delà : le
-- direct est une ressource unique de la plateforme, pas de l'événement.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION programme.detect_conflicts(p_event_id uuid)
RETURNS TABLE (
    severity      text,
    conflict_kind text,
    subject_id    uuid,
    subject_label text,
    session_a     uuid,
    session_a_title text,
    session_b     uuid,
    session_b_title text,
    overlap       tstzrange
)
LANGUAGE sql
STABLE
AS $$
    WITH active AS (
        SELECT s.*, platform.t(s.title) AS label
        FROM programme.sessions s
        WHERE s.status IN ('planned', 'scheduled', 'live')
    )
    -- 1. Un seul stand : deux activités du même événement en même temps.
    SELECT 'blocking', 'venue_capacity',
           a.event_id, 'Stand unique de l''événement',
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM active a
    JOIN active b ON b.event_id = a.event_id AND b.id > a.id AND a.time_range && b.time_range
    WHERE a.event_id = p_event_id

    UNION ALL

    -- 2. Un seul direct, tous événements confondus.
    SELECT 'blocking', 'broadcast',
           a.broadcast_channel_id, 'Canal de diffusion occupé',
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM active a
    JOIN active b ON b.id > a.id
                 AND b.is_streamed
                 AND b.broadcast_channel_id IS NOT DISTINCT FROM a.broadcast_channel_id
                 AND a.time_range && b.time_range
    WHERE a.is_streamed
      AND a.broadcast_channel_id IS NOT NULL
      AND (a.event_id = p_event_id OR b.event_id = p_event_id)

    UNION ALL

    -- 3. Deux sessions dans la même salle physique.
    SELECT 'blocking', 'room',
           a.room_id, platform.t(r.name),
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM active a
    JOIN active b ON b.room_id = a.room_id AND b.id > a.id AND a.time_range && b.time_range
    JOIN event.rooms r ON r.id = a.room_id
    WHERE a.event_id = p_event_id
      AND a.room_id IS NOT NULL
      AND NOT r.is_virtual

    UNION ALL

    -- 4. Un intervenant attendu à deux endroits.
    SELECT 'warning', 'speaker',
           p.id, p.display_name,
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM programme.session_speakers sa
    JOIN programme.session_speakers sb ON sb.person_id = sa.person_id AND sb.session_id <> sa.session_id
    JOIN active a ON a.id = sa.session_id
    JOIN active b ON b.id = sb.session_id AND b.id > a.id AND a.time_range && b.time_range
    JOIN identity.people p ON p.id = sa.person_id
    WHERE a.event_id = p_event_id

    UNION ALL

    -- 5. Une organisation programmée deux fois sur le même créneau, en tenant
    --    compte des co-organisations.
    SELECT 'warning', 'organization',
           o.id, o.legal_name,
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM programme.session_organizations oa
    JOIN programme.session_organizations ob ON ob.organization_id = oa.organization_id
                                           AND ob.session_id <> oa.session_id
    JOIN active a ON a.id = oa.session_id
    JOIN active b ON b.id = ob.session_id AND b.id > a.id AND a.time_range && b.time_range
    JOIN org.organizations o ON o.id = oa.organization_id
    WHERE a.event_id = p_event_id;
$$;

COMMENT ON FUNCTION programme.detect_conflicts IS
    'Recense les chevauchements avec leur gravité. Aucun n''est bloqué à l''écriture : l''équipe arbitre dans le back-office.';

-- Contrôle avant publication : ce qui doit être réglé pour que la programmation
-- puisse être rendue publique. C'est ici que le garde-fou a du sens — pas
-- pendant que l'équipe déplace ses blocs.
CREATE OR REPLACE FUNCTION programme.publication_readiness(p_event_id uuid)
RETURNS TABLE (severity text, issue text, detail text, session_id uuid)
LANGUAGE sql
STABLE
AS $$
    SELECT c.severity,
           CASE c.conflict_kind
               WHEN 'venue_capacity' THEN 'Deux activités simultanées sur un stand unique'
               WHEN 'broadcast'      THEN 'Deux diffusions en direct simultanées'
               WHEN 'room'           THEN 'Salle réservée deux fois'
               WHEN 'speaker'        THEN 'Intervenant attendu à deux endroits'
               ELSE 'Organisation programmée deux fois'
           END,
           format('%s ↔ %s (%s)', c.session_a_title, c.session_b_title, c.overlap),
           c.session_a
    FROM programme.detect_conflicts(p_event_id) c

    UNION ALL

    SELECT 'blocking', 'Session sans créneau valide',
           platform.t(s.title), s.id
    FROM programme.sessions s
    WHERE s.event_id = p_event_id
      AND s.status IN ('planned', 'scheduled')
      AND (s.starts_at IS NULL OR s.ends_at <= s.starts_at)

    UNION ALL

    SELECT 'warning', 'Session diffusée sans canal assigné',
           platform.t(s.title), s.id
    FROM programme.sessions s
    WHERE s.event_id = p_event_id
      AND s.is_streamed
      AND s.broadcast_channel_id IS NULL
      AND s.status IN ('planned', 'scheduled')

    UNION ALL

    SELECT 'warning', 'Session sans intervenant déclaré',
           platform.t(s.title), s.id
    FROM programme.sessions s
    WHERE s.event_id = p_event_id
      AND s.status IN ('planned', 'scheduled')
      AND NOT EXISTS (SELECT 1 FROM programme.session_speakers sp WHERE sp.session_id = s.id);
$$;

COMMENT ON FUNCTION programme.publication_readiness IS
    'Liste ce qui doit être réglé avant de publier la programmation. Le seul moment où un contrôle bloquant a du sens.';

-- -----------------------------------------------------------------------------
-- 7 bis. Historique des modifications
--
-- « Il est important d'avoir l'historique des modifications des activités :
--   dates, textes, etc. »
-- Les triggers d'audit alimentent platform.audit_log ; ces deux fonctions le
-- restituent sous une forme directement affichable dans le back-office.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION programme.session_history(p_session_id uuid)
RETURNS TABLE (
    occurred_at timestamptz, actor_id uuid, actor_label text,
    action text, field text, old_value jsonb, new_value jsonb
)
LANGUAGE sql
STABLE
AS $$
    SELECT * FROM platform.entity_history(
        'programme', 'sessions', p_session_id,
        ARRAY['updated_at', 'search_vector', 'view_count', 'time_range',
              'enforce_room_exclusivity', 'attendee_count']
    );
$$;

-- Les reports de créneau méritent leur propre lecture : c'est la modification
-- la plus lourde de conséquences pour les inscrits, et celle qu'on doit
-- pouvoir retracer quand une organisation conteste un horaire.
CREATE OR REPLACE FUNCTION programme.session_schedule_history(p_session_id uuid)
RETURNS TABLE (
    occurred_at timestamptz, actor_label text,
    previous_start timestamptz, new_start timestamptz,
    previous_end timestamptz, new_end timestamptz
)
LANGUAGE sql
STABLE
AS $$
    SELECT a.occurred_at,
           a.actor_label,
           (a.old_data ->> 'starts_at')::timestamptz,
           (a.new_data ->> 'starts_at')::timestamptz,
           (a.old_data ->> 'ends_at')::timestamptz,
           (a.new_data ->> 'ends_at')::timestamptz
    FROM platform.audit_log a
    WHERE a.entity_schema = 'programme'
      AND a.entity_table = 'sessions'
      AND a.entity_id = p_session_id
      AND a.changed_fields && ARRAY['starts_at', 'ends_at', 'room_id']
    ORDER BY a.occurred_at DESC;
$$;

COMMENT ON FUNCTION programme.session_schedule_history IS
    'Historique des reports de créneau d''une session : qui a déplacé quoi, quand, et depuis quel horaire.';

-- -----------------------------------------------------------------------------
-- 8. Déclaration des références vers les organisations (pour la fusion)
-- -----------------------------------------------------------------------------
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy, dedupe_on) VALUES
    ('programme', 'sessions',              'organization_id', 'reassign', '{}'),
    ('programme', 'registrations',         'organization_id', 'reassign', '{}'),
    ('programme', 'session_organizations', 'organization_id', 'reassign', '{session_id}')
ON CONFLICT DO NOTHING;

-- Formulaire d'inscription minimal fourni par défaut.
INSERT INTO programme.registration_forms (code, name, description, is_default, allows_anonymous)
VALUES ('default', '{"fr":"Inscription standard","en":"Standard registration"}',
        '{"fr":"Formulaire par défaut appliqué aux sessions sans formulaire dédié","en":"Default form"}',
        true, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO programme.registration_form_fields (form_id, code, label, field_type, is_required, options, sort_order)
SELECT f.id, v.code, v.label::platform.i18n_text, v.field_type::programme.form_field_type, v.is_required, v.options::jsonb, v.sort_order
FROM programme.registration_forms f
CROSS JOIN (VALUES
    ('job_title',       '{"fr":"Fonction","en":"Job title"}',                     'text',          false, '{}', 10),
    ('organization',    '{"fr":"Organisation","en":"Organization"}',              'text',          false, '{}', 20),
    ('country',         '{"fr":"Pays","en":"Country"}',                           'country',       true,  '{}', 30),
    ('referral_source', '{"fr":"Comment avez-vous connu cette activité ?","en":"How did you hear about this activity?"}',
                        'single_choice', false, '{"taxonomy":"referral_source"}', 40)
) AS v(code, label, field_type, is_required, options, sort_order)
WHERE f.code = 'default'
ON CONFLICT (form_id, code) DO NOTHING;
