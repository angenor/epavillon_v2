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
    -- Dernier changement de ce que le PUBLIC lit d'une ligne du programme
    -- (16/09) : horaires, titre, salle, statut. `updated_at` bouge aussi pour
    -- une capacité ou un compte rendu, qui ne changent rien pour le visiteur.
    -- Tenue par `tg_sessions_listing_changed`.
    listing_changed_at timestamptz NOT NULL DEFAULT now(),

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
    -- Régie du jour (02/11). Instant et auteur de l'annulation, tenus par
    -- `tg_sessions_status_times` quel que soit le chemin (régie, dossier
    -- annulé). Raison et note : celles de la régie seule.
    cancelled_at     timestamptz,
    cancelled_by     uuid        CONSTRAINT xmod_fk_sessions_canceller
                                 REFERENCES identity.people(id) ON DELETE SET NULL,
    -- reference.taxonomy_terms.code (taxonomie session_cancellation_reason).
    cancellation_reason_code text,
    -- Note interne de l'équipe, jamais publiée.
    cancellation_note text,
    -- Bornes réelles du direct, tenues par `tg_sessions_status_times` : la
    -- reprise d'un direct terminé par erreur se décide sur `live_ended_at`.
    live_started_at  timestamptz,
    live_ended_at    timestamptz,
    created_by       uuid        CONSTRAINT xmod_fk_sessions_creator
                                 REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_sessions_period CHECK (ends_at > starts_at),
    CONSTRAINT ck_sessions_registration_window
        CHECK (registration_closes_at IS NULL OR registration_opens_at IS NULL
               OR registration_closes_at > registration_opens_at),
    -- Pas de `ck_sessions_cancelled_reason` depuis le 28/10/2026 : annuler un
    -- dossier retenu annule ses séances avec son motif, devenu facultatif. Une
    -- séance annulée sans motif s'affiche « annulée », sans explication.
    CONSTRAINT ck_sessions_cancellation_note CHECK (
        cancellation_note IS NULL OR length(cancellation_note) <= 2000),
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
COMMENT ON COLUMN programme.sessions.title IS
    'Titre public. Recopié du dossier à l''acceptation, puis à chaque enregistrement du dossier, par l''équipe comme par l''organisation (arbitrage du 31/10).';
COMMENT ON COLUMN programme.sessions.description IS
    'Présentation publique, en HTML restreint comme programme.proposals.detailed_presentation, dont elle est recopiée à l''acceptation puis à chaque enregistrement du dossier (31/10).';

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

-- UNE SÉANCE « À PLACER » EST HORS DU PROGRAMME (arbitrage du 02/11/2026).
-- À placer : ni salle ni précision de lieu, le panneau latéral du
-- planificateur. « Retirer du calendrier » ne posait que `room_id = NULL` :
-- la séance restait publiée, visible du public sans lieu, toujours en direct,
-- et bloquait la republication du programme. Pour une séance pressentie ou
-- programmée, désormais :
--   - QUITTER LA GRILLE la retire du site public (`published_at` nul,
--     `scheduled` redevient `planned`) et coupe son direct. La replacer ne
--     rallume rien : elle redevient publique à la prochaine publication ;
--   - À PLACER, ELLE N'EST JAMAIS PUBLIÉE, quel que soit l'écrivain
--     (publication du programme, script) : la date posée est effacée et son
--     état reste celui d'avant.
-- Transitions seulement, à la mise à jour : une insertion garde ce qu'on lui
-- donne (reprise de la v1 : 61 séances terminées de la CdP30, sans salle, sont
-- publiées), et les autres états ne bougent pas. Les inscriptions restent ;
-- une séance non publiée n'en reçoit plus de nouvelle.
-- Le canal s'efface ici : `tg_sessions_derive` passe avant (ordre alphabétique).
CREATE OR REPLACE FUNCTION programme.tg_sessions_unplaced()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.room_id IS NOT NULL OR NEW.location_note IS NOT NULL
       OR NEW.status NOT IN ('planned', 'scheduled') THEN
        RETURN NEW;
    END IF;

    IF OLD.room_id IS NOT NULL OR OLD.location_note IS NOT NULL THEN
        NEW.published_at := NULL;
        NEW.status := 'planned';
        NEW.is_streamed := false;
        NEW.broadcast_channel_id := NULL;
    ELSIF OLD.published_at IS NULL AND NEW.published_at IS NOT NULL THEN
        NEW.published_at := NULL;
        NEW.status := OLD.status;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_sessions_unplaced
    BEFORE UPDATE OF room_id, location_note, published_at ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sessions_unplaced();

CREATE OR REPLACE FUNCTION programme.tg_sessions_touch_listing()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.listing_changed_at := now();
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_sessions_listing_changed
    BEFORE UPDATE OF starts_at, ends_at, title, room_id, status ON programme.sessions
    FOR EACH ROW
    WHEN (NEW.starts_at IS DISTINCT FROM OLD.starts_at
       OR NEW.ends_at   IS DISTINCT FROM OLD.ends_at
       OR NEW.title     IS DISTINCT FROM OLD.title
       OR NEW.room_id   IS DISTINCT FROM OLD.room_id
       OR NEW.status    IS DISTINCT FROM OLD.status)
    EXECUTE FUNCTION programme.tg_sessions_touch_listing();

-- Régie du jour (02/11) : les instants d'un changement d'état, posés par la
-- base pour que la régie, l'annulation d'un dossier et la réunion Zoom
-- (`programme.direct`) les tiennent de la même façon. Une reprise (`completed`
-- vers `live`) garde le début du direct ; quitter l'annulation efface son
-- instant, son auteur, sa raison et sa note.
CREATE OR REPLACE FUNCTION programme.tg_sessions_status_times()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = 'live' THEN
        IF OLD.status <> 'completed' OR NEW.live_started_at IS NULL THEN
            NEW.live_started_at := now();
        END IF;
        NEW.live_ended_at := NULL;
    ELSIF NEW.status = 'completed' AND OLD.status = 'live' THEN
        NEW.live_ended_at := now();
    ELSIF OLD.status = 'live' THEN
        NEW.live_started_at := NULL;
    END IF;

    IF NEW.status = 'cancelled' THEN
        NEW.cancelled_at := now();
        NEW.cancelled_by := platform.current_actor_id();
    ELSIF OLD.status = 'cancelled' THEN
        NEW.cancelled_at := NULL;
        NEW.cancelled_by := NULL;
        NEW.cancellation_reason_code := NULL;
        NEW.cancellation_note := NULL;
        NEW.cancelled_reason := NULL;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_sessions_status_times
    BEFORE UPDATE OF status ON programme.sessions
    FOR EACH ROW
    WHEN (NEW.status IS DISTINCT FROM OLD.status)
    EXECUTE FUNCTION programme.tg_sessions_status_times();

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
-- Recopiés depuis la proposition à la programmation, puis tenus à jour à
-- chaque enregistrement du dossier (31/10) : l'organisation corrige ses
-- intervenants dans son dossier ou depuis l'éditeur de l'image, et la page de
-- l'activité suit. Reconnu par personne et rôle (ou nom et rôle, D16), un
-- intervenant garde sa ligne et sa présence notée (`attended`).
-- -----------------------------------------------------------------------------
CREATE TABLE programme.session_speakers (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    session_id        uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    -- NULL : intervenant sans adresse (D16), comme dans programme.proposal_speakers.
    person_id         uuid        CONSTRAINT xmod_fk_session_speakers_person
                                  REFERENCES identity.people(id) ON DELETE RESTRICT,
    role              programme.speaker_role NOT NULL DEFAULT 'speaker',
    job_title_snapshot text,
    organization_snapshot text,
    bio               platform.i18n_text,
    confirmed_at      timestamptz,
    attended          boolean,
    sort_order        smallint    NOT NULL DEFAULT 0,
    created_at        timestamptz NOT NULL DEFAULT now(),
    -- Identité d'un intervenant sans personne (D16) ; ajoutées en dernier, comme la migration.
    civility          text        CHECK (civility IN ('mme', 'm', 'dr', 'pr', 'other')),
    first_name        text,
    last_name         text,
    -- Dossier dont l'intervenant vient, dans une séance commune (§ 2 quater) ;
    -- NULL dans une séance ordinaire, où il n'y a qu'un dossier.
    proposal_id       uuid        REFERENCES programme.proposals(id) ON DELETE CASCADE,
    CONSTRAINT ux_session_speakers UNIQUE (session_id, person_id, role),
    CONSTRAINT ck_session_speakers_identity CHECK (
        (person_id IS NOT NULL AND civility IS NULL AND first_name IS NULL AND last_name IS NULL)
        OR (person_id IS NULL AND length(btrim(first_name)) > 0 AND length(btrim(last_name)) > 0))
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
    -- Dossier dont l'organisation vient, dans une séance commune (§ 2 quater) ;
    -- NULL dans une séance ordinaire. Une organisation présente dans deux
    -- dossiers d'une même séance commune n'y figure qu'une fois (clé primaire),
    -- sous le premier dossier réuni.
    proposal_id     uuid        REFERENCES programme.proposals(id) ON DELETE CASCADE,
    PRIMARY KEY (session_id, organization_id)
);

-- Un porteur par dossier : un seul dans une séance ordinaire (dossier NULL,
-- d'où NULLS NOT DISTINCT), un par dossier réuni dans une séance commune.
CREATE UNIQUE INDEX ux_session_organizations_lead
    ON programme.session_organizations (session_id, proposal_id) NULLS NOT DISTINCT
    WHERE role = 'lead';

CREATE INDEX ix_session_organizations_org
    ON programme.session_organizations (organization_id, role);

COMMENT ON TABLE programme.session_organizations IS
    'Organisations d''une session publiée, porteur principal compris. Alimente l''affichage public et le décompte par organisation.';

-- Synchronisation du porteur principal, comme sur les propositions. Dans une
-- séance commune, les porteurs des dossiers réunis (rangés sous leur dossier)
-- ne sont pas ceux de la séance : seul le porteur sans dossier est remplacé.
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

CREATE TRIGGER tg_sessions_sync_lead_organization
    AFTER INSERT OR UPDATE OF organization_id ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sync_session_lead_organization();

-- -----------------------------------------------------------------------------
-- 2 quater. Séances communes (01/11/2026)
--
-- Faute de créneaux pour tous, l'IFDD réunit parfois deux dossiers retenus ou
-- plus, d'organisations différentes et aux thèmes proches, dans UN créneau et
-- sous un titre commun qu'elle choisit. Le public voit une séance : une
-- inscription, une réunion visio, un direct. Chaque dossier y garde sa
-- présentation, sa couverture, ses organisations et ses intervenants, dans un
-- ordre d'affichage. Pas d'horaire de passage : les organisations s'entendent
-- entre elles, seuls le début et la fin de la séance comptent.
--
-- La séance commune est une ligne ordinaire de programme.sessions, sans dossier
-- (`proposal_id` NULL, le cas « l'IFDD programme directement »), dont le titre
-- est le titre commun et dont l'organisation est celle du premier dossier.
-- Créneau, inscriptions, diffusion et conflits restent donc ceux d'une séance,
-- sans rien dupliquer. Cette table dit quels dossiers elle réunit.
--
-- Pourquoi pas plusieurs `proposal_id` sur la séance : un dossier n'a qu'une
-- séance commune à la fois, mais une séance en réunit un nombre variable ; et
-- ce que chaque dossier apporte (titre d'origine, présentation) doit se
-- recopier et se tenir à jour comme les textes d'une séance ordinaire, ce
-- qu'un tableau d'identifiants ne porte pas.
--
-- Intervenants et organisations recopiés des dossiers vivent dans
-- session_speakers et session_organizations, rangés par leur `proposal_id`.
-- -----------------------------------------------------------------------------
CREATE TABLE programme.session_proposals (
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

CREATE INDEX ix_session_proposals_session ON programme.session_proposals (session_id, sort_order);

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

CREATE TRIGGER tg_session_proposals_check
    BEFORE INSERT OR UPDATE OF session_id, proposal_id ON programme.session_proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_session_proposal();

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
    -- PARTIE DU DÉROULÉ (30/10) : « Partie 2 : initiatives nationales ». Sur le
    -- rattachement et non sur la séance, parce qu'une activité de deux fils
    -- n'appartient pas à la même partie dans l'un et dans l'autre. La clé
    -- composite garantit que la partie est celle de CE fil ; retirer la partie
    -- remet l'activité hors partie sans la détacher du fil.
    part_id    uuid,
    -- MOMENT QUI REGROUPE (03/11/2026) : « Les 10 minutes qui inspirent »
    -- réunit trois interventions de dix minutes de la journée jeunesse. Les
    -- activités restent publiées (fiche, lien direct, compteurs) ; la page du
    -- fil les énumère sous le moment au lieu de les dérouler une à une, et
    -- l'inscription à la journée ne les propose plus à garder. Sur le
    -- rattachement, comme la partie : le moment est celui de CE fil (clé
    -- composite). Retirer le moment rend l'activité au déroulé. Clôt la table :
    -- arrivée par ALTER sur une base en service.
    moment_id  uuid,
    PRIMARY KEY (session_id, track_id),
    CONSTRAINT xmod_fk_session_tracks_part FOREIGN KEY (part_id, track_id)
        REFERENCES event.programme_track_parts (id, track_id) ON DELETE SET NULL (part_id),
    CONSTRAINT xmod_fk_session_tracks_moment FOREIGN KEY (moment_id, track_id)
        REFERENCES event.programme_track_moments (id, track_id) ON DELETE SET NULL (moment_id)
);

CREATE INDEX ix_session_tracks_track ON programme.session_tracks (track_id, sort_order);
-- Clé étrangère vers les moments : sans index, supprimer un moment parcourt la table.
CREATE INDEX ix_session_tracks_moment ON programme.session_tracks (moment_id) WHERE moment_id IS NOT NULL;

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

-- Signal du planificateur (temps réel, 31/10) : `event:<id>:planner`, la grille
-- d'un collègue se relit quand une séance est placée, déplacée, rattachée à une
-- journée, publiée, annulée ou née d'un dossier retenu. En base : les routes du
-- planificateur, la composition d'une journée (module Événements) et la
-- publication (travail différé) écrivent ces deux tables, aucun n'oublie. Un
-- NOTIFY identique est fusionné par PostgreSQL dans une transaction : une
-- publication de deux cents séances donne un signal.
CREATE OR REPLACE FUNCTION programme.tg_planner_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_event uuid;
BEGIN
    IF TG_TABLE_NAME = 'sessions' THEN
        v_event := CASE WHEN TG_OP = 'DELETE' THEN OLD.event_id ELSE NEW.event_id END;
    ELSE
        SELECT event_id INTO v_event FROM programme.sessions
         WHERE id = CASE WHEN TG_OP = 'DELETE' THEN OLD.session_id ELSE NEW.session_id END;
    END IF;
    IF v_event IS NOT NULL THEN
        PERFORM platform.signal('event:' || v_event || ':planner');
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_sessions_planner_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_planner_signal();

CREATE TRIGGER tg_session_tracks_planner_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_tracks
    FOR EACH ROW EXECUTE FUNCTION programme.tg_planner_signal();

-- Signal du programme PUBLIC (temps réel, 31/10) : les pages ouvertes
-- (programmation, accueil, activité, journée) relisent dans les 30 s, par le
-- pouls, sans session. Distinct du signal du planificateur : seule une séance
-- publiée, avant ou après l'écriture, et seule une colonne que le public voit,
-- comptent ; une séance en préparation ne réveille aucun visiteur.
--   `edition:<id>:public:live`     la séance entre en direct ou en sort (repère) ;
--   `edition:<id>:public:schedule` tout autre changement visible : créneau,
--                                  salle, titre, annulation, report, publication.
-- Mêmes écrivains que le planificateur (routes, journée, publication, réunion
-- Zoom par `programme.direct`, annulation d'un dossier, correction des textes),
-- d'où la base plutôt que chaque service.
CREATE OR REPLACE FUNCTION programme.signal_public_schedule(p_session_id uuid)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_event uuid;
BEGIN
    SELECT event_id INTO v_event FROM programme.sessions
     WHERE id = p_session_id AND published_at IS NOT NULL;
    IF v_event IS NOT NULL THEN
        PERFORM platform.signal('edition:' || v_event || ':public:schedule');
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION programme.tg_sessions_public_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_avant programme.sessions;
    v_apres programme.sessions;
    v_sujet text;
BEGIN
    IF TG_OP <> 'INSERT' THEN v_avant := OLD; END IF;
    IF TG_OP <> 'DELETE' THEN v_apres := NEW; END IF;
    IF v_avant.published_at IS NULL AND v_apres.published_at IS NULL THEN
        RETURN NULL;
    END IF;
    v_sujet := 'edition:' || COALESCE(v_apres.event_id, v_avant.event_id) || ':public:';

    IF COALESCE(v_avant.status = 'live', false) <> COALESCE(v_apres.status = 'live', false) THEN
        PERFORM platform.signal(v_sujet || 'live');
    ELSIF (v_avant.published_at, v_avant.status, v_avant.title, v_avant.slug, v_avant.summary,
           v_avant.description, v_avant.format, v_avant.starts_at, v_avant.ends_at,
           v_avant.timezone, v_avant.room_id, v_avant.location_note, v_avant.event_day_id,
           v_avant.organization_id, v_avant.registration_required, v_avant.registration_opens_at,
           v_avant.registration_closes_at, v_avant.capacity, v_avant.waitlist_enabled,
           v_avant.is_streamed, v_avant.broadcast_channel_id, v_avant.is_recorded,
           v_avant.allows_questions, v_avant.cancelled_reason)
          IS DISTINCT FROM
          (v_apres.published_at, v_apres.status, v_apres.title, v_apres.slug, v_apres.summary,
           v_apres.description, v_apres.format, v_apres.starts_at, v_apres.ends_at,
           v_apres.timezone, v_apres.room_id, v_apres.location_note, v_apres.event_day_id,
           v_apres.organization_id, v_apres.registration_required, v_apres.registration_opens_at,
           v_apres.registration_closes_at, v_apres.capacity, v_apres.waitlist_enabled,
           v_apres.is_streamed, v_apres.broadcast_channel_id, v_apres.is_recorded,
           v_apres.allows_questions, v_apres.cancelled_reason) THEN
        PERFORM platform.signal(v_sujet || 'schedule');
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_sessions_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sessions_public_signal();

-- Rattachement à une journée, partie, mise en avant : la page du fil le montre.
CREATE OR REPLACE FUNCTION programme.tg_session_tracks_public_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM programme.signal_public_schedule(
        CASE WHEN TG_OP = 'DELETE' THEN OLD.session_id ELSE NEW.session_id END);
    IF TG_OP = 'UPDATE' AND OLD.session_id <> NEW.session_id THEN
        PERFORM programme.signal_public_schedule(OLD.session_id);
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_session_tracks_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_tracks
    FOR EACH ROW EXECUTE FUNCTION programme.tg_session_tracks_public_signal();

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
--
-- UNE ACTIVITÉ GARDÉE D'UNE JOURNÉE (§ 4 bis, arbitré le 02/11) suit les règles
-- de sa journée : ni jauge (le direct est commun à toute la journée), ni
-- clôture propre, ni formulaire propre (les réponses sont celles de la
-- journée). Seule une activité annulée refuse une nouvelle ligne ; l'annuler
-- reste toujours possible, sans quoi une activité annulée bloquerait la
-- désinscription de toute la journée.
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

    IF NEW.track_registration_id IS NOT NULL THEN
        IF TG_OP = 'INSERT' AND v_session.status = 'cancelled' THEN
            RAISE EXCEPTION 'Session annulée : inscription impossible.' USING ERRCODE = 'restrict_violation';
        END IF;
        RETURN NEW;
    END IF;

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
--
-- Il porte sa journée depuis le 02/11 (`track_registration_id`, § 4 bis) : une
-- activité gardée d'une inscription à la journée n'a ni place Zoom propre ni
-- avis de confirmation à elle, et les consommateurs le savent sans relire la
-- ligne. Rattacher une inscription existante à une journée réémet l'état
-- courant, pour que le module Direct retire la place Zoom de l'activité.
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
            'locale', NEW.locale,
            'track_registration_id', NEW.track_registration_id
        )
    );
    RETURN NULL;
END;
$$;

-- Le déclencheur est posé au § 4 bis, une fois la colonne
-- `track_registration_id` créée : un déclencheur `UPDATE OF` ne se pose que sur
-- des colonnes existantes.

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
-- 4 bis. Inscription à une JOURNÉE (02/11/2026)
--
-- Demande du commanditaire (maquettes docs/maquettes/direction-condensee/
-- journee-inscription.html et bouton-inscription.html) : une journée spéciale,
-- et l'atelier préparatoire, se suivent avec UN SEUL LIEN DE DIRECT pour toute
-- la journée, accueil, pauses et clôture compris. On s'inscrit à la journée,
-- avec ou sans compte ; la fenêtre d'inscription liste les activités du jour,
-- toutes cochées, et la personne décoche ce qui ne l'intéresse pas. Les
-- activités gardées servent aux rappels et aux statistiques ; elles ne changent
-- RIEN à l'accès au direct, qui est celui de la journée.
--
-- QUELLE JOURNÉE, QUELLES ACTIVITÉS. Un fil `special_day` ou `side_programme`
-- (l'atelier, hors pavillon) a un direct de journée, d'office : aucun réglage,
-- c'est la nature du fil qui le dit (`programme.is_day_track_kind`). Un fil
-- `thematic_track` court sur plusieurs jours et ne change rien. Une activité
-- relève du direct de sa journée dès qu'elle est rattachée à un tel fil
-- (`programme.day_track_of`), que la journée soit publiée ou non : l'aiguillage
-- de la réunion ne bascule pas le jour de la publication. L'inscription, elle,
-- n'ouvre qu'avec la page publique de la journée (vérifié par l'API).
--
-- POURQUOI DEUX NIVEAUX PLUTÔT QU'UNE TABLE DE CHOIX. L'inscription à la
-- journée (`programme.track_registrations`) porte la personne, ses réponses,
-- le jeton de l'invité et l'accès au direct. Chaque activité gardée reste une
-- ligne ordinaire de `programme.registrations`, rattachée à la journée par
-- `track_registration_id`. Ainsi tout ce qui lit les inscriptions par activité
-- continue de marcher sans être réécrit : rappels (`engagement.
-- schedule_session_reminders`), compteurs et jauge de la programmation
-- publique (`registered_count`), liste nominative, statistiques
-- (`analytics.mv_daily_registrations`), courriels du back-office par activité.
-- Une table « activités choisies » à côté aurait demandé de doubler chacun de
-- ces lecteurs, et le premier oublié aurait compté faux sans le dire.
--
-- LES RÉPONSES AU FORMULAIRE vivent sur l'inscription à la journée, qui fait
-- foi ; elles sont recopiées sur chaque activité gardée à l'écriture, pour que
-- les statistiques par activité (index GIN de `registrations.answers`) restent
-- des requêtes d'une table. Le formulaire applicable à la journée est celui de
-- son édition, à défaut celui de la plateforme (même résolution qu'une activité
-- sans formulaire attaché).
--
-- SANS COMPTE, comme pour une activité : la personne est créée dans
-- identity.people sans compte. Elle reçoit un courriel portant le lien du direct
-- et un lien pour changer ses activités ou se désinscrire. Ce lien porte un
-- jeton dont la base ne garde que l'empreinte (`manage_token_hash`, SHA-256) ;
-- le jeton en clair ne vit que dans le courriel et dans la charge utile du
-- travail d'envoi, qui le déclare secret (`carries_secret`). Même construction
-- que tool.survey_invitations et identity.one_time_tokens. Un nouvel envoi du
-- lien remplace l'empreinte : l'ancien lien cesse de servir.
--
-- CE QUI RESTE PAR ACTIVITÉ : l'état « en direct » (règle n° 4, un seul direct
-- à la fois) et la régie, qui marque le début et la fin de chaque activité.
-- Seule la réunion Zoom passe à la journée (`live.meetings.track_id`, 080).
--
-- Ni jauge ni liste d'attente : la seule limite est celle de la réunion Zoom,
-- qui laisse entrer les premiers arrivés. Une activité gardée échappe aussi à la
-- jauge, à la clôture et au formulaire de son activité (arbitré le 02/11,
-- `tg_validate_registration`) : elle suit sa journée.
--
-- RAPPELS : ceux de plus d'un jour sont ceux de la journée, un par inscription ;
-- par activité gardée ne partent que ceux de moins d'un jour (110,
-- `engagement.schedule_session_reminders`).
-- -----------------------------------------------------------------------------

-- Les natures de fil qui ont un direct de journée. Une fonction plutôt qu'une
-- liste recopiée : le déclencheur d'inscription et l'aiguillage des séances
-- doivent dire la même chose.
CREATE OR REPLACE FUNCTION programme.is_day_track_kind(p_kind event.track_kind)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT p_kind IN ('special_day', 'side_programme');
$$;

COMMENT ON FUNCTION programme.is_day_track_kind(event.track_kind) IS
    'Vrai pour une journée spéciale ou un programme parallèle (atelier préparatoire) : un seul direct pour la journée, une inscription à la journée.';

-- La journée dont une activité suit le direct, ou NULL. Une activité n'est
-- rattachée qu'à une journée en pratique (une date, un créneau) ; si elle l'était
-- à deux, la première par date puis par rang gagne, de façon stable.
CREATE OR REPLACE FUNCTION programme.day_track_of(p_session_id uuid)
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT st.track_id
      FROM programme.session_tracks st
      JOIN event.programme_tracks t ON t.id = st.track_id
     WHERE st.session_id = p_session_id
       AND programme.is_day_track_kind(t.kind)
     ORDER BY t.starts_on NULLS LAST, t.sort_order, t.id
     LIMIT 1;
$$;

COMMENT ON FUNCTION programme.day_track_of(uuid) IS
    'Journée (fil special_day ou side_programme) dont l''activité suit le direct, publiée ou non ; NULL hors journée. Lue par la programmation et par le module Direct, une seule définition.';

CREATE TABLE programme.track_registrations (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    track_id        uuid        NOT NULL CONSTRAINT xmod_fk_track_registrations_track
                                REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_track_registrations_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    organization_id uuid        CONSTRAINT xmod_fk_track_registrations_organization
                                REFERENCES org.organizations(id) ON DELETE SET NULL,
    -- Même machine que l'inscription à une activité, sans la liste d'attente.
    status          programme.registration_status NOT NULL DEFAULT 'registered',
    -- Réponses au formulaire de l'édition, clés = registration_form_fields.code.
    -- Recopiées sur chaque activité gardée (voir l'en-tête).
    answers         jsonb       NOT NULL DEFAULT '{}'::jsonb,
    locale          text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    -- Empreinte SHA-256 du jeton du lien de gestion (32 octets), posée pour une
    -- inscription sans compte. Le jeton en clair n'est jamais stocké.
    manage_token_hash bytea,
    -- Premier clic sur « Rejoindre le direct de la journée », écrit une fois.
    joined_at       timestamptz,
    source          text        NOT NULL DEFAULT 'web'
                                CHECK (source IN ('web', 'import', 'admin', 'api', 'partner')),
    cancelled_at    timestamptz,
    cancelled_reason text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_track_registrations_status CHECK (status <> 'waitlisted'),
    CONSTRAINT ck_track_registrations_answers CHECK (jsonb_typeof(answers) = 'object'),
    CONSTRAINT ck_track_registrations_token
        CHECK (manage_token_hash IS NULL OR octet_length(manage_token_hash) = 32)
);

-- Une personne ne s'inscrit qu'une fois à une journée ; une annulation libère.
CREATE UNIQUE INDEX ux_track_registrations_person_track
    ON programme.track_registrations (track_id, person_id)
    WHERE status <> 'cancelled';
CREATE UNIQUE INDEX ux_track_registrations_manage_token
    ON programme.track_registrations (manage_token_hash)
    WHERE manage_token_hash IS NOT NULL;
CREATE INDEX ix_track_registrations_track  ON programme.track_registrations (track_id, status);
CREATE INDEX ix_track_registrations_person ON programme.track_registrations (person_id, created_at DESC);

CREATE TRIGGER tg_track_registrations_updated_at
    BEFORE UPDATE ON programme.track_registrations
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE programme.track_registrations IS
    'Inscription à une journée spéciale ou à l''atelier préparatoire : un seul direct pour la journée. Les activités gardées sont des programme.registrations rattachées par track_registration_id.';
COMMENT ON COLUMN programme.track_registrations.answers IS
    'Réponses au formulaire applicable (celui de l''édition, à défaut celui de la plateforme). Font foi ; recopiées sur chaque activité gardée.';
COMMENT ON COLUMN programme.track_registrations.manage_token_hash IS
    'Empreinte SHA-256 du jeton du lien « changer mes activités ou me désinscrire » d''une inscription sans compte. Le jeton en clair ne vit que dans le courriel ; un nouvel envoi remplace l''empreinte.';
COMMENT ON COLUMN programme.track_registrations.joined_at IS
    'Premier clic sur « Rejoindre le direct de la journée » (écrit une seule fois). Base du taux de participation de la journée.';

-- Seul un fil de journée prend des inscriptions : une inscription à un fil
-- thématique ne mènerait à aucun direct.
CREATE OR REPLACE FUNCTION programme.tg_check_track_registration()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM event.programme_tracks t
         WHERE t.id = NEW.track_id AND programme.is_day_track_kind(t.kind)
    ) THEN
        RAISE EXCEPTION 'Inscription impossible : ce fil n''est ni une journée spéciale ni un atelier.'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_track_registrations_check
    BEFORE INSERT OR UPDATE OF track_id ON programme.track_registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_track_registration();

-- Événement de domaine : réunion Zoom de la journée et place de l'inscrit
-- (module Direct), avis de confirmation (Engagement). Le courriel de l'invité,
-- qui porte le jeton en clair, ne passe pas par l'outbox : l'API le met en
-- file elle-même, dans sa transaction.
CREATE OR REPLACE FUNCTION programme.tg_track_registrations_emit_events()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM platform.emit_event(
        'programme', 'track_registration', NEW.id,
        CASE WHEN TG_OP = 'INSERT' THEN 'programme.track_registration.created'
             ELSE 'programme.track_registration.' || NEW.status::text END,
        jsonb_build_object(
            'track_id', NEW.track_id,
            'event_id', (SELECT t.event_id FROM event.programme_tracks t WHERE t.id = NEW.track_id),
            'person_id', NEW.person_id,
            'status', NEW.status,
            'locale', NEW.locale
        )
    );
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_track_registrations_events
    AFTER INSERT OR UPDATE OF status ON programme.track_registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_track_registrations_emit_events();

-- Les activités gardées : une inscription par activité, rattachée à la
-- journée. En queue de table, comme en production. SET NULL : la journée
-- supprimée, l'inscription à l'activité redevient ordinaire au lieu de
-- disparaître avec son historique de présence.
ALTER TABLE programme.registrations
    ADD COLUMN track_registration_id uuid
        CONSTRAINT fk_registrations_track_registration
        REFERENCES programme.track_registrations(id) ON DELETE SET NULL;

CREATE INDEX ix_registrations_track_registration
    ON programme.registrations (track_registration_id)
    WHERE track_registration_id IS NOT NULL;

COMMENT ON COLUMN programme.registrations.track_registration_id IS
    'Inscription à la journée dont cette activité est gardée ; NULL pour une inscription ordinaire. Une telle ligne n''a pas de place Zoom propre : le direct est celui de la journée.';

-- Une activité gardée appartient à la journée et à la personne de son
-- inscription à la journée. Vérifié à l'écriture seulement : une activité
-- retirée ensuite de la journée garde sa ligne, l'API la traite.
CREATE OR REPLACE FUNCTION programme.tg_check_registration_track()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_track  uuid;
    v_person uuid;
BEGIN
    IF NEW.track_registration_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT tr.track_id, tr.person_id INTO v_track, v_person
      FROM programme.track_registrations tr WHERE tr.id = NEW.track_registration_id;

    IF v_person IS DISTINCT FROM NEW.person_id THEN
        RAISE EXCEPTION 'Rattachement impossible : l''inscription à l''activité et l''inscription à la journée ne sont pas celles de la même personne.'
            USING ERRCODE = 'check_violation';
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM programme.session_tracks st
         WHERE st.session_id = NEW.session_id AND st.track_id = v_track
    ) THEN
        RAISE EXCEPTION 'Rattachement impossible : cette activité ne fait pas partie de la journée.'
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_registrations_check_track
    BEFORE INSERT OR UPDATE OF track_registration_id, session_id, person_id ON programme.registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_registration_track();

CREATE TRIGGER tg_registrations_events
    AFTER INSERT OR UPDATE OF status, track_registration_id ON programme.registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_registrations_emit_events();

-- LA COMPOSITION DE LA JOURNÉE BOUGE APRÈS LES INSCRIPTIONS (arbitré le 02/11).
-- Une activité ajoutée à une journée l'est d'office, cochée, à chacun de ses
-- inscrits ; une activité retirée voit leurs lignes rattachées annulées, sans
-- courriel (l'annulation d'une inscription n'en envoie aucun). En base : le
-- planificateur, la séance commune et le back-office des journées écrivent
-- tous programme.session_tracks, aucun n'oublie.
--
-- Une ligne ordinaire déjà prise par la personne sur l'activité ajoutée est
-- rattachée à sa journée plutôt que doublée ; le rattachement réémet son état,
-- et le module Direct retire sa place Zoom propre. Une séance ou une journée en
-- cours de suppression (cascade) ne touche à rien : ses lignes partent avec elle.
CREATE OR REPLACE FUNCTION programme.tg_session_tracks_follow_registrations()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_retire boolean := TG_OP = 'DELETE';
    v_ajoute boolean := TG_OP = 'INSERT';
BEGIN
    IF TG_OP = 'UPDATE' THEN
        v_retire := NEW.track_id <> OLD.track_id OR NEW.session_id <> OLD.session_id;
        v_ajoute := v_retire;
    END IF;

    IF v_retire
       AND EXISTS (SELECT 1 FROM programme.sessions s WHERE s.id = OLD.session_id) THEN
        UPDATE programme.registrations r
           SET status = 'cancelled', waitlist_position = NULL, cancelled_at = now(),
               cancelled_reason = 'Activité retirée de la journée.'
          FROM programme.track_registrations tr
         WHERE tr.id = r.track_registration_id
           AND tr.track_id = OLD.track_id
           AND r.session_id = OLD.session_id
           AND r.status <> 'cancelled';
    END IF;

    IF v_ajoute THEN
        INSERT INTO programme.registrations
               (session_id, person_id, organization_id, answers, locale, source, track_registration_id)
        SELECT NEW.session_id, tr.person_id, tr.organization_id, tr.answers, tr.locale,
               tr.source, tr.id
          FROM programme.track_registrations tr
          JOIN event.programme_tracks t ON t.id = tr.track_id
          JOIN programme.sessions s ON s.id = NEW.session_id
         WHERE tr.track_id = NEW.track_id
           AND tr.status IN ('registered', 'attended')
           AND programme.is_day_track_kind(t.kind)
           AND s.status <> 'cancelled'
        ON CONFLICT (session_id, person_id) WHERE status <> 'cancelled'
        DO UPDATE SET track_registration_id = EXCLUDED.track_registration_id,
                      waitlist_position = NULL,
                      status = CASE WHEN programme.registrations.status = 'waitlisted'
                                    THEN 'registered'::programme.registration_status
                                    ELSE programme.registrations.status END
                WHERE programme.registrations.track_registration_id IS NULL;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_session_tracks_follow_registrations
    AFTER INSERT OR UPDATE OF track_id, session_id OR DELETE ON programme.session_tracks
    FOR EACH ROW EXECUTE FUNCTION programme.tg_session_tracks_follow_registrations();

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

-- Signal des questions (temps réel, 31/10) : `edition:<id>:public:questions`,
-- sujet PUBLIC servi par le pouls comme la programmation. La lecture des
-- questions est publique, et un sujet par séance exigerait une autorisation en
-- base pour chaque visiteur anonyme ; l'édition suffit : une seule activité est
-- en direct à la fois (règle 4), c'est elle qui reçoit les questions, et les
-- autres pages relisent une route à empreinte (304). Une question posée,
-- soutenue (le décompte `upvotes` est réécrit), masquée, répondue.
CREATE OR REPLACE FUNCTION programme.signal_session_questions(p_session_id uuid)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_event uuid;
BEGIN
    SELECT event_id INTO v_event FROM programme.sessions
     WHERE id = p_session_id AND published_at IS NOT NULL;
    IF v_event IS NOT NULL THEN
        PERFORM platform.signal('edition:' || v_event || ':public:questions');
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION programme.tg_session_questions_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE'
       AND (OLD.body, OLD.upvotes, OLD.is_visible, OLD.answered_at, OLD.session_id)
           IS NOT DISTINCT FROM (NEW.body, NEW.upvotes, NEW.is_visible, NEW.answered_at, NEW.session_id) THEN
        RETURN NULL;
    END IF;
    PERFORM programme.signal_session_questions(
        CASE WHEN TG_OP = 'DELETE' THEN OLD.session_id ELSE NEW.session_id END);
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_session_questions_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_questions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_session_questions_signal();

CREATE OR REPLACE FUNCTION programme.tg_session_question_answers_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_session uuid;
BEGIN
    SELECT session_id INTO v_session FROM programme.session_questions
     WHERE id = CASE WHEN TG_OP = 'DELETE' THEN OLD.question_id ELSE NEW.question_id END;
    IF v_session IS NOT NULL THEN
        PERFORM programme.signal_session_questions(v_session);
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_session_question_answers_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_question_answers
    FOR EACH ROW EXECUTE FUNCTION programme.tg_session_question_answers_signal();

-- -----------------------------------------------------------------------------
-- 6. Vue calendrier
--
-- Alimente directement la vue calendrier du frontend (une ligne = un bloc), et
-- la vue grille. Le calcul de l'état d'avancement est fait une fois, en base,
-- plutôt que dans chaque composant.
--
-- RÈGLE DE CETTE VUE : elle répond à l'écran EN UNE REQUÊTE. Tout ce que la
-- carte de séance affiche doit donc s'y trouver déjà joint et déjà résolu —
-- salle, organisation et son pays, journées spéciales, thématiques avec leur
-- libellé et leur couleur, image de couverture. Chaque colonne manquante coûte
-- une requête par écran, ou un renoncement d'affichage ; les deux se sont
-- produits (voir les écarts n° 14, 14 bis et 15 de
-- docs/progression/ecrans/a0.4-composants.md).
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW programme.v_public_schedule AS
SELECT
    s.id,
    s.event_id,
    s.event_day_id,
    -- Exposé pour deux raisons : la page de détail renvoie au dossier d'origine,
    -- et le repli de couverture ci-dessous en a besoin.
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
    -- PAYS DE L'ORGANISATION PORTEUSE. Sur une COP, il situe l'organisation
    -- aussi sûrement que son nom : « Réseau ouest-africain… (Sénégal) » ne se
    -- confond pas avec son homonyme canadien. Sans lui ici, chaque écran de
    -- programmation devait charger org.organizations ET reference.countries pour
    -- un seul mot — et la vue perdait sa raison d'être, répondre à un écran en
    -- une requête.
    -- Deux colonnes, deux usages : le code ISO est stable et sert au filtre et
    -- au drapeau, le nom est multilingue et se résout à l'affichage comme tout
    -- platform.i18n_text.
    c.iso2                AS organization_country_code,
    c.name                AS organization_country,
    s.is_streamed,
    s.broadcast_channel_id,
    s.capacity,
    -- Journées spéciales et fils thématiques auxquels la session est rattachée :
    -- alimente les pastilles « Journée finance durable » de la vue grille sans
    -- requête supplémentaire. `is_highlight` (01/11) : l'activité est un temps
    -- fort DE CE FIL ; elle peut l'être dans une journée et pas dans l'autre.
    COALESCE((
        SELECT jsonb_agg(jsonb_build_object(
                   'slug', t.slug, 'title', t.title, 'color', t.color_hex, 'kind', t.kind,
                   'is_highlight', st.is_highlight)
               ORDER BY st.sort_order, t.sort_order)
        FROM programme.session_tracks st
        JOIN event.programme_tracks t ON t.id = st.track_id
        WHERE st.session_id = s.id AND t.published_at IS NOT NULL
    ), '[]'::jsonb) AS tracks,
    -- Couverture de la séance, à défaut celle de la proposition d'origine.
    --
    -- LE REPLI EST LA RÈGLE, pas une commodité : une organisation joint son
    -- image au DÉPÔT, et personne ne revient en téléverser une seconde après
    -- l'acceptation. Sans ce COALESCE, la programmation publique s'afficherait
    -- sans visuel alors que l'image existe déjà en base, à un identifiant près.
    -- Rendu par media.attached_image() plutôt que par une jointure écrite ici :
    -- le rattachement est polymorphe, et trois vues qui le déroulent à la main
    -- sont trois occasions de diverger. NULL quand il n'y a réellement aucune
    -- image — la carte doit rester lisible sans elle.
    COALESCE(
        media.attached_image('programme', 'sessions',  s.id,          'cover'),
        media.attached_image('programme', 'proposals', s.proposal_id, 'cover')
    ) AS cover,
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
    -- Thématiques, DEUX FOIS et pour deux usages distincts :
    --   · `theme_codes` pour FILTRER (opérateurs de tableau, indexables) ;
    --   · `themes` pour AFFICHER — libellé traduit et couleur viennent de
    --     reference.taxonomy_terms, où un administrateur les modifie.
    -- N'exposer que les codes forçait chaque écran à recharger la taxonomie et à
    -- refaire la correspondance : c'est ainsi que les libellés de thématiques
    -- se sont retrouvés figés dans le frontend de la v1. La vue agrégeait déjà
    -- complètement les fils de programmation ; l'asymétrie n'avait pas de raison.
    reference.terms_of('programme', 'sessions', s.id, 'activity_theme') AS theme_codes,
    reference.term_badges('programme', 'sessions', s.id, 'activity_theme') AS themes,
    -- Ajoutées par l'étape 5 de Guide Négo (spec 017), EN FIN DE LISTE : le site
    -- lit cette vue, et CREATE OR REPLACE VIEW n'admet qu'un ajout en queue.
    -- Conditions d'inscription d'une activité publiée : ce que le bouton dit
    -- (« Rejoindre la liste d'attente », « closes », « pas encore ouvertes »).
    s.waitlist_enabled,
    s.registration_required,
    s.registration_opens_at,
    s.registration_closes_at,
    (SELECT count(*) FROM programme.registrations rg
      WHERE rg.session_id = s.id AND rg.status = 'waitlisted') AS waitlisted_count,
    s.listing_changed_at,
    -- LA SEULE COLONNE TIRÉE DU DOSSIER DE PROPOSITION. La langue d'une activité
    -- est annoncée au public ; rien d'autre du dossier (coordonnées, pièces,
    -- évaluation) ne l'est. Séance commune : les langues de ses dossiers. Nulle
    -- quand l'IFDD programme sans dossier.
    COALESCE(p.language_codes, (
        SELECT array_agg(DISTINCT l ORDER BY l)
        FROM programme.session_proposals sp
        JOIN programme.proposals pp ON pp.id = sp.proposal_id
        CROSS JOIN unnest(pp.language_codes) AS l
        WHERE sp.session_id = s.id
    )) AS language_codes
FROM programme.sessions s
LEFT JOIN event.rooms r          ON r.id = s.room_id
LEFT JOIN org.organizations o    ON o.id = s.organization_id
LEFT JOIN reference.countries c  ON c.id = o.country_id
LEFT JOIN programme.proposals p  ON p.id = s.proposal_id
WHERE s.published_at IS NOT NULL;

-- LA VUE SE REDÉFINIT EN ENTIER DANS 080_live.sql (§ « Rediffusion dans la
-- programmation publique ») : elle y gagne, en fin de liste, `replay_url` et
-- `replay_duration_seconds`, lus dans live.streams — qui n'existe pas encore ici,
-- puis `organization_logo` et `organization_type_code` (30/09), qui n'existent
-- que là, comme `proposals` (séances communes, 01/11). Toute autre
-- modification de la vue se reporte dans les deux fichiers.

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
--                 activités d'un même événement ne peuvent pas occuper ses salles
--                 en même temps. Et une seule équipe technique : un seul direct à
--                 la fois, tous événements confondus. Ces conflits doivent être
--                 résolus avant publication.
--   'warning'   — gênant mais possible : un intervenant attendu à deux endroits,
--                 une organisation programmée deux fois. L'équipe juge.
--
-- Portée : les conflits de STAND sont cherchés au sein de l'événement — deux
-- événements distincts (un webinaire et une COP) peuvent parfaitement se tenir
-- en parallèle. Les conflits de DIFFUSION, eux, sont cherchés au-delà : le
-- direct est une ressource unique de la plateforme, pas de l'événement.
--
-- CE QU'OCCUPE UNE SÉANCE, ET CE QU'ELLE N'OCCUPE PAS (corrigé le 18/08 2026,
-- écart n° 10 de docs/progression/ecrans/a9-planificateur.md, avant
-- l'écriture du planificateur)
-- La première version de la branche « stand unique » visait TOUTE paire de
-- séances simultanées de l'édition. Elle remontait donc en gravité bloquante un
-- atelier en ligne tenu dans une salle virtuelle, qui n'occupe pourtant aucun
-- mètre carré du pavillon : le planificateur affichait un conflit matériel qui
-- n'en était pas un, et une alerte qu'on apprend à ignorer a cessé d'être une
-- alerte. La branche ne retient donc plus que les séances qui occupent
-- RÉELLEMENT le stand, c'est-à-dire celles installées dans une salle PHYSIQUE
-- (`enforce_room_exclusivity`, dérivée de `event.rooms.is_virtual` par trigger).
--
-- Pourquoi la salle et non le format : une table ronde `online` peut être
-- diffusée depuis le studio du pavillon — elle occupe alors bien le stand —, et
-- une séance `hybrid` posée en salle virtuelle ne l'occupe pas. C'est le LIEU
-- qui dit l'occupation, jamais le mode de participation.
--
-- Corollaire : une séance SANS SALLE n'occupe rien. C'est l'état normal d'une
-- activité retenue mais pas encore installée — le panneau « à placer » du
-- planificateur. Ces séances ne doivent pas saturer le bandeau de conflits avant
-- même que l'arbitrage ait commencé.
--
-- À PLACER, HORS DE TOUT CONFLIT (02/11/2026). La première branche les écartait
-- déjà ; pas les autres. Une séance retirée du calendrier gardait son créneau et
-- son direct : « deux diffusions simultanées » avec celle qui avait pris sa
-- place, et la séance restée sur la grille virait au rouge. Une séance sans
-- salle ni précision de lieu est hors du programme (`tg_sessions_unplaced`) :
-- elle n'entre dans aucune branche, intervenants et organisations compris.
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
          AND (s.room_id IS NOT NULL OR s.location_note IS NOT NULL)
    )
    -- 1. Un seul stand : deux activités du même événement OCCUPANT UNE SALLE
    --    PHYSIQUE en même temps. Voir l'en-tête : une séance en salle virtuelle
    --    ou pas encore installée n'occupe pas le stand.
    --
    --    Les deux séances dans la MÊME salle sont écartées ici : la branche 3 le
    --    dit mieux, en nommant la salle. Sans cette exclusion, chaque double
    --    réservation était comptée deux fois dans le bandeau du planificateur.
    SELECT 'blocking', 'venue_capacity',
           a.event_id, 'Stand unique de l''événement',
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM active a
    JOIN active b ON b.event_id = a.event_id AND b.id > a.id AND a.time_range && b.time_range
    WHERE a.event_id = p_event_id
      AND a.enforce_room_exclusivity
      AND b.enforce_room_exclusivity
      AND a.room_id IS DISTINCT FROM b.room_id

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
-- `occurs_at` PLUTÔT QU'UN INTERVALLE DANS LE TEXTE (corrigé le 18/08 2026, en
-- éprouvant le récapitulatif de publication du planificateur). La première
-- version glissait le `tstzrange` brut dans `detail`, ce qui s'affichait tel
-- quel à l'écran : « (["2027-11-12T14:00:00-03:00","2027-11-12T15:30:00-03:00") »
-- au milieu d'une phrase française. Un instant n'est pas un texte : la base rend
-- la DONNÉE, et l'interface la situe dans le fuseau de l'édition et la langue du
-- lecteur — la règle du projet, « toute date affichée porte son fuseau », ne
-- peut pas s'appliquer à une chaîne déjà figée.
CREATE OR REPLACE FUNCTION programme.publication_readiness(p_event_id uuid)
RETURNS TABLE (severity text, issue text, detail text, session_id uuid, occurs_at timestamptz)
LANGUAGE sql
STABLE
AS $$
    -- NE CONTRÔLE QUE CE QUI SERA PUBLIC (28/10 2026). Un dossier en évaluation
    -- ou rejeté peut porter un créneau pressenti (repris de la v1) : la
    -- publication le laisse privé, il ne doit donc pas la retenir. Est retenue
    -- une séance déjà publique, sans dossier (programmée par l'IFDD) ou de
    -- dossier accepté : le prédicat même de la publication (module event,
    -- `seances_a_publier`), plus ce qui est déjà en ligne. Le planificateur,
    -- lui, garde tous les conflits par `detect_conflicts()`.
    --
    -- Une séance à placer (ni salle ni précision de lieu) n'est jamais publiée
    -- (`tg_sessions_unplaced`, 02/11/2026) : elle ne retient donc rien.
    WITH retenue AS (
        SELECT s.*
        FROM programme.sessions s
        LEFT JOIN programme.proposals p ON p.id = s.proposal_id
        WHERE (s.room_id IS NOT NULL OR s.location_note IS NOT NULL)
          AND (s.published_at IS NOT NULL
               OR s.proposal_id IS NULL
               OR p.status = 'accepted')
    )
    SELECT c.severity,
           CASE c.conflict_kind
               WHEN 'venue_capacity' THEN 'Deux activités simultanées sur un stand unique'
               WHEN 'broadcast'      THEN 'Deux diffusions en direct simultanées'
               WHEN 'room'           THEN 'Salle réservée deux fois'
               WHEN 'speaker'        THEN 'Intervenant attendu à deux endroits'
               ELSE 'Organisation programmée deux fois'
           END,
           format('%s ↔ %s', c.session_a_title, c.session_b_title),
           c.session_a,
           lower(c.overlap)
    FROM programme.detect_conflicts(p_event_id) c
    WHERE c.session_a IN (SELECT r.id FROM retenue r)
      AND c.session_b IN (SELECT r.id FROM retenue r)

    UNION ALL

    SELECT 'blocking', 'Session sans créneau valide',
           platform.t(s.title), s.id, s.starts_at
    FROM retenue s
    WHERE s.event_id = p_event_id
      AND s.status IN ('planned', 'scheduled')
      AND (s.starts_at IS NULL OR s.ends_at <= s.starts_at)

    UNION ALL

    -- « Séance sans lieu ni précision de lieu », bloquante du 18/08 au
    -- 02/11/2026, a disparu : c'est la définition même d'une séance à placer,
    -- que la publication laisse désormais privée au lieu d'être retenue par elle.
    SELECT 'warning', 'Session diffusée sans canal assigné',
           platform.t(s.title), s.id, s.starts_at
    FROM retenue s
    WHERE s.event_id = p_event_id
      AND s.is_streamed
      AND s.broadcast_channel_id IS NULL
      AND s.status IN ('planned', 'scheduled')

    UNION ALL

    SELECT 'warning', 'Session sans intervenant déclaré',
           platform.t(s.title), s.id, s.starts_at
    FROM retenue s
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
              'enforce_room_exclusivity', 'attendee_count', 'listing_changed_at']
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
-- 7 ter. Date limite indicative des besoins techniques d'un dossier (29/10)
--
-- Dix jours avant la première séance non annulée du dossier, nulle sans séance.
-- Rien n'est refusé après : elle sert à rappeler à l'organisation ce qui manque
-- (070 § 4 bis). Une fonction plutôt qu'un calcul dans l'API : les rappels
-- automatiques la reliront. Ici et non dans 070 : elle lit les séances.
-- -----------------------------------------------------------------------------
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

-- -----------------------------------------------------------------------------
-- 8. Déclaration des références vers les organisations (pour la fusion)
-- -----------------------------------------------------------------------------
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy, dedupe_on) VALUES
    ('programme', 'sessions',              'organization_id', 'reassign', '{}'),
    ('programme', 'registrations',         'organization_id', 'reassign', '{}'),
    ('programme', 'track_registrations',   'organization_id', 'reassign', '{}'),
    ('programme', 'session_organizations', 'organization_id', 'reassign', '{session_id}')
ON CONFLICT DO NOTHING;

-- Rattachements média autorisés pour une séance programmée.
--
-- CE QUI MANQUAIT : `programme.proposals` déclarait son rôle `cover` depuis le
-- début (050 § 8), mais `programme.sessions` n'en déclarait aucun. Le trigger
-- de contrôle refusait donc toute image sur une séance — alors que c'est la
-- séance, et non le dossier, que le public regarde dans la programmation.
--
-- POURQUOI UN RÔLE SUR LA SÉANCE PLUTÔT QUE LE SEUL RENVOI À LA PROPOSITION :
-- l'image déposée au dépôt n'est pas toujours celle qu'on publie. Une
-- organisation joint volontiers une photo de projet à son dossier ; l'IFDD
-- publie parfois autre chose — un visuel de la journée spéciale, une photo de
-- l'édition précédente, ou simplement une image mieux cadrée. Le rôle sur la
-- séance permet ce remplacement SANS réécrire le dossier déposé, qui reste la
-- pièce contractuelle. La vue calendrier (§ 6) applique le repli : couverture de
-- la séance, à défaut celle de la proposition d'origine.
--
-- Pas de rôle `gallery` ici : les photos de compte rendu relèvent du bilan
-- d'activité, hors du jalon en cours. À ajouter le jour où l'écran existe.
INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size) VALUES
    ('programme', 'sessions', 'cover', '{"fr":"Image de couverture","en":"Cover image"}', false, '{image/*}', 10485760)
ON CONFLICT DO NOTHING;

-- Formulaire d'inscription minimal fourni par défaut.
-- Arbitrage du 03/10 : le public ne donne que nom, prénom, adresse et pays. La
-- fonction, l'organisation et la provenance restent déclarées, désactivées :
-- les rouvrir est un `UPDATE … SET is_active = true`, sans migration.
INSERT INTO programme.registration_forms (code, name, description, is_default, allows_anonymous)
VALUES ('default', '{"fr":"Inscription standard","en":"Standard registration"}',
        '{"fr":"Formulaire par défaut appliqué aux sessions sans formulaire dédié","en":"Default form"}',
        true, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO programme.registration_form_fields (form_id, code, label, field_type, is_required, options, sort_order, is_active)
SELECT f.id, v.code, v.label::platform.i18n_text, v.field_type::programme.form_field_type, v.is_required, v.options::jsonb, v.sort_order, v.is_active
FROM programme.registration_forms f
CROSS JOIN (VALUES
    ('job_title',       '{"fr":"Fonction","en":"Job title"}',                     'text',          false, '{}', 10, false),
    ('organization',    '{"fr":"Organisation","en":"Organization"}',              'text',          false, '{}', 20, false),
    ('country',         '{"fr":"Pays","en":"Country"}',                           'country',       true,  '{}', 30, true),
    ('referral_source', '{"fr":"Comment avez-vous connu cette activité ?","en":"How did you hear about this activity?"}',
                        'single_choice', false, '{"taxonomy":"referral_source"}', 40, false)
) AS v(code, label, field_type, is_required, options, sort_order, is_active)
WHERE f.code = 'default'
ON CONFLICT (form_id, code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- Statistiques de programmation par édition
--
-- Le pendant de `event.v_public_editions` (060 § 9), qui ne peut pas les porter :
-- `event` se charge avant `programme`, et la dépendance va dans ce sens-là — la
-- programmation connaît son édition, jamais l'inverse. Les deux vues se
-- joignent sur `event_id` en une requête.
--
-- Ce qu'elles servent : la frise d'historique de la page d'accueil, où « 42
-- activités » distingue une édition documentée d'une édition simplement
-- annoncée, et où les bornes réelles du programme diffèrent souvent des dates
-- officielles de la conférence.
--
-- Deux colonnes ajoutées le 16/09 pour le bandeau de `/programmations`, EN FIN
-- DE LISTE pour qu'un `CREATE OR REPLACE` suffise sur une base en service :
--   · `country_count` — pays DISTINCTS des organisations porteuses, comptés
--     comme `organization_count` (le porteur, pas les co-organisateurs) ;
--   · `programme_updated_at` — dernier changement visible d'une séance publiée
--     (`listing_changed_at` : horaires, titre, salle, statut), ou dernière
--     entrée d'une séance au programme. Une ANNULATION en est un, d'où le
--     filtre posé colonne par colonne et non dans le `WHERE`.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW programme.v_edition_stats AS
SELECT
    s.event_id,
    count(*) FILTER (WHERE s.status <> 'cancelled')                       AS published_session_count,
    count(*) FILTER (WHERE s.status <> 'cancelled' AND s.is_streamed)     AS streamed_session_count,
    count(DISTINCT s.organization_id) FILTER (WHERE s.status <> 'cancelled') AS organization_count,
    min(s.starts_at) FILTER (WHERE s.status <> 'cancelled')               AS programme_starts_at,
    max(s.ends_at)   FILTER (WHERE s.status <> 'cancelled')               AS programme_ends_at,
    count(DISTINCT o.country_id) FILTER (WHERE s.status <> 'cancelled')   AS country_count,
    max(greatest(s.listing_changed_at, s.published_at))                   AS programme_updated_at
FROM programme.sessions s
LEFT JOIN org.organizations o ON o.id = s.organization_id
WHERE s.published_at IS NOT NULL
GROUP BY s.event_id;

COMMENT ON VIEW programme.v_edition_stats IS
    'Volume du programme publié par édition. Complète event.v_public_editions, qui ne peut pas dépendre de programme (ordre de chargement).';

-- -----------------------------------------------------------------------------
-- 9. Régie du jour (02/11)
--
-- Pendant une CdP, l'équipe pilote la journée depuis une régie : présence des
-- organisateurs, annulation d'une activité, direct démarré, terminé, repris.
-- Le commanditaire a posé une exigence avant toute autre : POUVOIR RÉPARER
-- TOUTE ERREUR. D'où un journal où chaque geste garde ce qu'il faut pour être
-- défait, même plus tard, par un autre membre de l'équipe.
--
-- POURQUOI DANS `programme` : chaque geste écrit une séance (état, présence,
-- annulation), qui appartient à ce module. Les messages d'incident restent au
-- module `live` : la régie les lit dans `live.incidents` (publiés, retirés), ne
-- les recopie pas dans ce journal.
--
-- Les courriels aux inscrits ne partent pas d'ici : la régie émet
-- `programme.regie.cancellation_notice` (envoi dû dans cinq minutes) ou
-- `programme.regie.restoration_notice` (« finalement maintenue »), et le module
-- `engagement` envoie à l'heure dite, après avoir vérifié que le geste n'a pas
-- été défait entre-temps.
-- -----------------------------------------------------------------------------

-- Présence des organisateurs le jour J. Pas de ligne : présence inconnue. Une
-- ligne porte « confirmée » (vrai) ou « absente » (faux), l'instant et l'auteur.
-- Une table plutôt qu'un ENUM sur la séance : trois états sans transition
-- imposée ne font pas une machine à états.
CREATE TABLE programme.session_presence (
    session_id  uuid        PRIMARY KEY REFERENCES programme.sessions(id) ON DELETE CASCADE,
    is_present  boolean     NOT NULL,
    noted_at    timestamptz NOT NULL DEFAULT now(),
    noted_by    uuid        CONSTRAINT xmod_fk_session_presence_author
                            REFERENCES identity.people(id) ON DELETE SET NULL
);

COMMENT ON TABLE programme.session_presence IS
    'Présence des organisateurs d''une activité, notée en régie. Absence de ligne : inconnue.';

-- Journal de la régie : un geste par ligne, avec ce qu'il faut pour le défaire.
-- `data` selon le geste :
--   presence_confirmed  {previous}                  présence d'avant (true, false ou null)
--   session_cancelled   {previous_status, previous_presence, reason_code,
--                        notify_registrants, notify_at, registrants,
--                        restore_notice}             la dernière clé posée en le défaisant
--   live_started        {previous_status}
--   live_ended          {}
-- Ni adresse ni nom : l'auteur est une référence, les inscrits un nombre.
CREATE TABLE programme.control_room_actions (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id    uuid        NOT NULL CONSTRAINT xmod_fk_control_room_actions_event
                            REFERENCES event.events(id) ON DELETE CASCADE,
    session_id  uuid        NOT NULL REFERENCES programme.sessions(id) ON DELETE CASCADE,
    action_code text        NOT NULL,
    data        jsonb       NOT NULL DEFAULT '{}'::jsonb,
    acted_by    uuid        CONSTRAINT xmod_fk_control_room_actions_actor
                            REFERENCES identity.people(id) ON DELETE SET NULL,
    acted_at    timestamptz NOT NULL DEFAULT now(),
    undone_at   timestamptz,
    undone_by   uuid        CONSTRAINT xmod_fk_control_room_actions_undoer
                            REFERENCES identity.people(id) ON DELETE SET NULL,
    -- Liste fermée : chaque geste a son retour en arrière écrit dans l'API.
    CONSTRAINT ck_control_room_actions_code CHECK (action_code IN (
        'presence_confirmed', 'session_cancelled', 'live_started', 'live_ended')),
    CONSTRAINT ck_control_room_actions_data CHECK (jsonb_typeof(data) = 'object'),
    CONSTRAINT ck_control_room_actions_undone CHECK (undone_at IS NOT NULL OR undone_by IS NULL)
);

CREATE INDEX ix_control_room_actions_event   ON programme.control_room_actions (event_id, acted_at DESC);
CREATE INDEX ix_control_room_actions_session ON programme.control_room_actions (session_id, acted_at DESC);

COMMENT ON TABLE programme.control_room_actions IS
    'Journal de la régie du jour : chaque geste (présence, annulation, direct) et de quoi le défaire. Défaire pose undone_at, la ligne reste.';
COMMENT ON COLUMN programme.control_room_actions.data IS
    'Ce qu''il faut pour défaire le geste : état d''avant, choix d''envoi des courriels. Aucune donnée personnelle.';

-- `event:<id>:regie` : la régie d'un collègue se relit. Les changements d'une
-- séance passent déjà par `event:<id>:planner`.
CREATE OR REPLACE FUNCTION programme.tg_control_room_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_event uuid;
BEGIN
    IF TG_TABLE_NAME = 'control_room_actions' THEN
        v_event := CASE WHEN TG_OP = 'DELETE' THEN OLD.event_id ELSE NEW.event_id END;
    ELSE
        SELECT event_id INTO v_event FROM programme.sessions
         WHERE id = CASE WHEN TG_OP = 'DELETE' THEN OLD.session_id ELSE NEW.session_id END;
    END IF;
    IF v_event IS NOT NULL THEN
        PERFORM platform.signal('event:' || v_event || ':regie');
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_control_room_actions_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.control_room_actions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_control_room_signal();

CREATE TRIGGER tg_session_presence_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_presence
    FOR EACH ROW EXECUTE FUNCTION programme.tg_control_room_signal();

-- Raisons d'annulation proposées en régie. Vocabulaire ouvert : l'équipe peut
-- en ajouter sans migration.
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('session_cancellation_reason',
     '{"fr":"Raisons d''annulation d''une activité","en":"Activity cancellation reasons"}',
     '{"fr":"Choisies en régie le jour J ; la note qui les accompagne n''est jamais publiée.","en":"Chosen in the control room; the accompanying note is never published."}',
     false, false, false)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, description, sort_order) VALUES
    ('session_cancellation_reason', 'no_show',
     '{"fr":"Les organisateurs ne sont pas venus","en":"The organisers did not come"}',
     '{"fr":"Constaté sur place.","en":"Noted on site."}', 10),
    ('session_cancellation_reason', 'notified_absence',
     '{"fr":"Les organisateurs ont prévenu de leur absence","en":"The organisers notified their absence"}',
     '{"fr":"Par courriel, téléphone ou message.","en":"By email, phone or message."}', 20),
    ('session_cancellation_reason', 'other',
     '{"fr":"Autre raison","en":"Other reason"}',
     '{"fr":"À préciser pour l''équipe ; elle n''est pas publiée.","en":"To be specified for the team; it is not published."}', 30)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

COMMENT ON COLUMN programme.sessions.cancellation_reason_code IS
    'reference.taxonomy_terms.code (taxonomie session_cancellation_reason), posée par la régie. NULL pour une annulation venue du dossier.';
