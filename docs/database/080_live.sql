-- =============================================================================
-- ePavillon v2 — 080_live.sql
-- Module Direct : réunions de visioconférence, diffusions en direct (YouTube),
-- synchronisation des participants chez le fournisseur, journal des webhooks,
-- messages d'incident affichés aux téléspectateurs.
--
-- Dépend de : 000, 010, 020, 030 (identity), 040 (org), 050 (media), 060 (event),
--             070 et la seconde partie du module Programmation, qui fournit
--             programme.sessions et programme.registrations.
--
-- CE QUE LA V1 FAISAIT ET QUI EST CORRIGÉ ICI
--
--   D1. MONO-FOURNISSEUR. `public.zoom_meetings` portait Zoom dans son nom, dans
--       ses colonnes (`meeting_id`, `start_url`, `password`, `host_email`) et
--       dans la FK `activities.zoom_meeting_id`. Teams était déjà utilisé en
--       parallèle, hors base, via un lien collé à la main. Ici, une réunion est
--       décrite indépendamment du fournisseur : `provider` + `external_id` +
--       `provider_payload` (réponse brute de l'API). Ajouter Teams, Jitsi ou un
--       fournisseur interne ne demande aucune migration de schéma.
--
--   D2. ÉCHECS DE SYNCHRONISATION SILENCIEUX. L'inscription d'un participant
--       chez Zoom se faisait dans la même requête que l'inscription en base :
--       une panne réseau, un quota d'API ou un blocage WAF perdait l'inscrit.
--       La v1 avait dû greffer en urgence `fallback_payload` / `fallback_error`
--       / `recovered_at` sur `activity_registrations` (feature 005), c'est-à-dire
--       un état de synchronisation technique au milieu d'une table métier.
--       Ici, l'inscription plateforme n'échoue JAMAIS pour une raison externe :
--       elle crée une ligne `live.meeting_participants` en `pending`, un travail
--       est déposé dans `platform.jobs`, et le statut, le nombre de tentatives,
--       la dernière erreur et le rattrapage sont des colonnes de premier ordre.
--
--   D3. SECRETS EN CLAIR. `password` et `start_url` (qui vaut authentification
--       hôte : quiconque possède l'URL démarre la réunion en tant qu'animateur)
--       étaient stockés en clair et remontaient dans les réponses de l'API via
--       `SELECT *`. Ici, la frontière est explicite : les secrets sont chiffrés
--       côté application (bytea), les champs diffusables sont séparés, et la vue
--       `live.meetings_public` est le seul contrat exposable au front.
--
--   D4. WEBHOOKS NON JOURNALISÉS. Aucune trace des notifications Zoom reçues :
--       un webhook manqué pendant un redéploiement était définitivement perdu,
--       et un webhook rejoué par Zoom était appliqué deux fois. Ici, tout
--       webhook entrant est écrit avant traitement, dédoublonné sur
--       l'identifiant d'événement du fournisseur, et rejouable à la demande.
--
--   D5. INCIDENTS À PORTÉE FIGÉE. `incident_messages` exigeait un `event_id`,
--       acceptait éventuellement une `organization_id` et une `day_date` (une
--       date libre, non reliée aux journées de l'événement), dupliquait le texte
--       en `message_fr` / `message_en`, et n'avait qu'un booléen `is_active`
--       basculé à la main — donc oublié allumé après l'incident. Ici : portée
--       générique (global, événement, journée, session, organisation), message
--       en `platform.i18n_text`, fenêtre d'affichage bornée dans le temps, et
--       publication/dépublication horodatées et attribuées.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Vocabulaires
--
-- Les ENUM ci-dessous décrivent des ensembles fermés : chaque valeur de
-- `provider` correspond à un adaptateur écrit dans l'API (client OAuth, format
-- de webhook, appel d'inscription) — en ajouter une sans coder l'adaptateur n'a
-- aucun sens. Les statuts sont des machines à états. En revanche la NATURE d'un
-- incident (panne, retard, annulation…) est un vocabulaire ouvert : il vit dans
-- reference.taxonomy_terms, amorcé en fin de fichier.
-- -----------------------------------------------------------------------------
CREATE TYPE live.meeting_provider AS ENUM ('zoom', 'teams', 'jitsi', 'webex', 'custom');

CREATE TYPE live.meeting_status AS ENUM (
    'draft',        -- créée en base, pas encore poussée chez le fournisseur
    'scheduled',    -- existe chez le fournisseur, à venir
    'started',      -- en cours (webhook meeting.started)
    'ended',        -- terminée (webhook meeting.ended)
    'cancelled',    -- annulée volontairement
    'failed'        -- création impossible chez le fournisseur après tous les essais
);

-- Machine à états de la synchronisation avec l'API du fournisseur. Partagée par
-- la réunion elle-même et par chacun de ses participants.
CREATE TYPE live.sync_status AS ENUM (
    'not_required', -- réunion saisie à la main (lien Teams collé) : rien à appeler
    'pending',      -- en file d'attente
    'in_progress',  -- un worker a pris le travail
    'synced',       -- accepté par le fournisseur
    'failed',       -- en échec, sera réessayé (voir next_retry_at)
    'abandoned'     -- au-delà du nombre d'essais : à traiter manuellement
);

CREATE TYPE live.participant_role AS ENUM ('host', 'co_host', 'panelist', 'attendee', 'interpreter');

CREATE TYPE live.stream_provider AS ENUM ('youtube', 'vimeo', 'facebook', 'linkedin', 'dailymotion', 'custom');

CREATE TYPE live.stream_kind AS ENUM ('live', 'replay');

CREATE TYPE live.stream_status AS ENUM ('scheduled', 'live', 'ended', 'cancelled');

CREATE TYPE live.webhook_status AS ENUM ('received', 'processing', 'processed', 'ignored', 'failed');

-- Ordre volontaire : croissant en gravité. `ORDER BY severity DESC` remonte donc
-- le message le plus grave en tête du bandeau.
CREATE TYPE live.incident_severity AS ENUM ('info', 'warning', 'error', 'critical');

CREATE TYPE live.incident_scope AS ENUM ('global', 'event', 'event_day', 'session', 'organization');

-- -----------------------------------------------------------------------------
-- 2. D1 — Réunions de visioconférence, indépendantes du fournisseur
--
-- Deux cas d'usage cohabitent, comme en v1 (create-zoom-meeting et
-- create-standalone-zoom-meeting) :
--   - la réunion adossée à une session programmée, créée automatiquement à la
--     validation de l'activité (session_id renseignée) ;
--   - la réunion autonome de l'IFDD (coordination, répétition technique,
--     réunion francophone), qui porte alors son propre intitulé.
-- -----------------------------------------------------------------------------
CREATE TABLE live.meetings (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    -- Rattachement métier. Les deux peuvent être nuls : réunion purement interne.
    session_id          uuid        CONSTRAINT xmod_fk_meetings_session
                                    REFERENCES programme.sessions(id) ON DELETE CASCADE,
    event_id            uuid        CONSTRAINT xmod_fk_meetings_event
                                    REFERENCES event.events(id) ON DELETE SET NULL,

    provider            live.meeting_provider NOT NULL DEFAULT 'zoom',
    -- Identifiant de la réunion chez le fournisseur (numérique chez Zoom,
    -- GUID chez Teams). Toujours du texte : aucun fournisseur ne garantit un
    -- entier, et la v1 avait déjà dû stocker `meeting_id` en TEXT.
    external_id         text,
    -- Identifiant d'occurrence (Zoom `uuid`) : change à chaque démarrage et sert
    -- à rapprocher les webhooks d'une instance précise d'une réunion récurrente.
    external_uuid       text,
    -- Compte hôte utilisé côté fournisseur (Zoom user id / UPN Teams) : la
    -- plateforme exploite plusieurs licences en parallèle pendant une COP.
    provider_account    text,
    -- Réponse brute de l'API, conservée telle quelle. Tout champ propre au
    -- fournisseur (settings, tracking_fields, occurrences) y reste accessible
    -- sans nouvelle colonne.
    provider_payload    jsonb       NOT NULL DEFAULT '{}'::jsonb,

    topic               text,
    agenda              text,
    scheduled_start_at  timestamptz,
    duration_minutes    integer     CHECK (duration_minutes IS NULL OR duration_minutes BETWEEN 1 AND 1440),
    timezone            platform.timezone_name NOT NULL DEFAULT 'UTC',
    actual_start_at     timestamptz,
    actual_end_at       timestamptz,

    -- --- Données diffusables ---------------------------------------------
    join_url            platform.url,
    registration_url    platform.url,

    -- --- Secrets (D3) -----------------------------------------------------
    -- Chiffrés par l'API (AES-GCM, clé au coffre) AVANT écriture. Aucune
    -- fonction SQL ne déchiffre : la base ne détient pas la clé.
    passcode_encrypted  bytea,
    start_url_encrypted bytea,
    host_key_encrypted  bytea,
    host_email          platform.email,

    requires_registration boolean   NOT NULL DEFAULT true,
    max_participants    integer     CHECK (max_participants IS NULL OR max_participants > 0),

    status              live.meeting_status NOT NULL DEFAULT 'draft',
    sync_status         live.sync_status    NOT NULL DEFAULT 'pending',
    sync_attempts       smallint    NOT NULL DEFAULT 0,
    last_sync_error     text,
    last_synced_at      timestamptz,

    created_by          uuid        CONSTRAINT xmod_fk_meetings_creator
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    -- Une réunion autonome doit au moins porter un intitulé lisible.
    CONSTRAINT ck_meetings_context
        CHECK (session_id IS NOT NULL OR topic IS NOT NULL),
    -- Une réunion présente chez le fournisseur possède forcément son identifiant.
    CONSTRAINT ck_meetings_external_shape
        CHECK (status IN ('draft', 'failed', 'cancelled') OR external_id IS NOT NULL),
    CONSTRAINT ck_meetings_period
        CHECK (actual_end_at IS NULL OR actual_start_at IS NULL OR actual_end_at >= actual_start_at)
);

-- Un identifiant de réunion est unique par fournisseur : deux lignes ne peuvent
-- pas piloter la même réunion Zoom (double appel de l'edge function en v1).
CREATE UNIQUE INDEX ux_meetings_provider_external
    ON live.meetings (provider, external_id)
    WHERE external_id IS NOT NULL;

-- Une session n'a qu'une réunion vivante : la seconde serait invisible côté
-- public et enverrait les inscrits sur un lien mort.
CREATE UNIQUE INDEX ux_meetings_active_per_session
    ON live.meetings (session_id)
    WHERE session_id IS NOT NULL AND status NOT IN ('cancelled', 'failed');

CREATE INDEX ix_meetings_schedule ON live.meetings (scheduled_start_at DESC) WHERE status IN ('scheduled', 'started');
CREATE INDEX ix_meetings_event    ON live.meetings (event_id) WHERE event_id IS NOT NULL;
-- File « à pousser chez le fournisseur » : reste minuscule.
CREATE INDEX ix_meetings_to_sync  ON live.meetings (sync_status, created_at) WHERE sync_status IN ('pending', 'failed');

CREATE TRIGGER tg_meetings_updated_at
    BEFORE UPDATE ON live.meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_meetings_audit
    AFTER INSERT OR UPDATE OR DELETE ON live.meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE live.meetings IS
    'Réunion de visioconférence, décrite indépendamment du fournisseur (D1). Adossée à une session ou autonome.';
COMMENT ON COLUMN live.meetings.provider_payload IS
    'Réponse brute de l''API du fournisseur. Évite d''ajouter une colonne pour chaque champ propriétaire.';
COMMENT ON COLUMN live.meetings.start_url_encrypted IS
    'URL de démarrage hôte, CHIFFRÉE (D3). Elle vaut authentification animateur : ne jamais l''exposer, ni la journaliser.';
COMMENT ON COLUMN live.meetings.passcode_encrypted IS
    'Code d''accès, CHIFFRÉ (D3). Le code n''est transmis qu''aux inscrits, avec leur lien personnel.';
COMMENT ON COLUMN live.meetings.join_url IS
    'Lien de participation générique : donnée publique, diffusable. À distinguer strictement des colonnes chiffrées.';

-- Contrat d'exposition : ce que l'API a le droit de renvoyer au navigateur.
CREATE VIEW live.meetings_public AS
SELECT m.id, m.session_id, m.event_id, m.provider, m.topic,
       m.scheduled_start_at, m.duration_minutes, m.timezone,
       m.actual_start_at, m.actual_end_at,
       m.join_url, m.registration_url, m.requires_registration, m.status
FROM live.meetings m;

COMMENT ON VIEW live.meetings_public IS
    'Projection sans secret de live.meetings. Interdire tout SELECT * sur la table depuis les couches exposées.';

-- -----------------------------------------------------------------------------
-- 3. D2 — Participants et synchronisation fournisseur
--
-- Une ligne = « cette inscription doit exister chez le fournisseur ». Elle est
-- créée dans la transaction d'inscription, donc l'inscrit est acquis même si
-- Zoom est injoignable ; le lien personnel arrive ensuite, par email.
-- -----------------------------------------------------------------------------
CREATE TABLE live.meeting_participants (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    meeting_id          uuid        NOT NULL REFERENCES live.meetings(id) ON DELETE CASCADE,

    -- Inscription plateforme à l'origine de l'enregistrement. Nulle pour un
    -- panéliste ou un interprète, qui n'a pas d'inscription publique.
    registration_id     uuid        CONSTRAINT xmod_fk_meeting_participants_registration
                                    REFERENCES programme.registrations(id) ON DELETE CASCADE,
    person_id           uuid        CONSTRAINT xmod_fk_meeting_participants_person
                                    REFERENCES identity.people(id) ON DELETE SET NULL,

    role                live.participant_role NOT NULL DEFAULT 'attendee',
    display_name        text        NOT NULL,
    email               platform.email NOT NULL,

    -- --- Retour du fournisseur -------------------------------------------
    external_registrant_id text,
    personal_join_url   platform.url,

    -- --- État de la synchronisation (remplace fallback_payload de la v1) ---
    sync_status         live.sync_status NOT NULL DEFAULT 'pending',
    sync_attempts       smallint    NOT NULL DEFAULT 0,
    max_sync_attempts   smallint    NOT NULL DEFAULT 5,
    -- Charge utile exacte qui sera envoyée à l'API : rejouable à l'identique
    -- après une panne, sans reconstituer les données depuis plusieurs tables.
    request_payload     jsonb       NOT NULL DEFAULT '{}'::jsonb,
    last_error          text,
    last_attempt_at     timestamptz,
    next_retry_at       timestamptz,
    synced_at           timestamptz,
    -- Rattrapage manuel par un administrateur (bouton « relancer l'inscription »).
    recovered_at        timestamptz,
    recovered_by        uuid        CONSTRAINT xmod_fk_meeting_participants_recoverer
                                    REFERENCES identity.people(id) ON DELETE SET NULL,

    -- --- Présence effective ----------------------------------------------
    -- Premier clic sur « Rejoindre » (first-click-wins, cf. feature 007 v1),
    -- ou premier webhook de connexion : la valeur n'est jamais écrasée.
    joined_at           timestamptz,
    left_at             timestamptz,
    attendance_seconds  integer     CHECK (attendance_seconds IS NULL OR attendance_seconds >= 0),

    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_meeting_participants_synced_shape
        CHECK (sync_status <> 'synced' OR external_registrant_id IS NOT NULL)
);

-- Une inscription ne produit qu'un enregistrement fournisseur.
CREATE UNIQUE INDEX ux_meeting_participants_registration
    ON live.meeting_participants (registration_id)
    WHERE registration_id IS NOT NULL;
-- Un email ne peut être enregistré deux fois sur la même réunion : le
-- fournisseur le refuserait, autant l'empêcher en amont.
CREATE UNIQUE INDEX ux_meeting_participants_email
    ON live.meeting_participants (meeting_id, email);
CREATE UNIQUE INDEX ux_meeting_participants_registrant
    ON live.meeting_participants (meeting_id, external_registrant_id)
    WHERE external_registrant_id IS NOT NULL;

-- File de rattrapage : le back-office « inscriptions à resynchroniser » lit cet
-- index, jamais un balayage complet de la table.
CREATE INDEX ix_meeting_participants_to_sync
    ON live.meeting_participants ((COALESCE(next_retry_at, created_at)), id)
    WHERE sync_status IN ('pending', 'failed');
CREATE INDEX ix_meeting_participants_abandoned
    ON live.meeting_participants (meeting_id)
    WHERE sync_status = 'abandoned' AND recovered_at IS NULL;
CREATE INDEX ix_meeting_participants_meeting ON live.meeting_participants (meeting_id, role);

CREATE TRIGGER tg_meeting_participants_updated_at
    BEFORE UPDATE ON live.meeting_participants
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE live.meeting_participants IS
    'Correspondance inscription plateforme <-> enregistrement chez le fournisseur, avec son état de synchronisation (D2).';
COMMENT ON COLUMN live.meeting_participants.request_payload IS
    'Charge utile destinée à l''API du fournisseur. Successeur de activity_registrations.fallback_payload : ici, c''est la norme, pas un correctif.';
COMMENT ON COLUMN live.meeting_participants.joined_at IS
    'Premier accès effectif. Écrit une seule fois (first-click-wins) et protégé par trigger : sert au taux de participation réel.';

-- First-click-wins : ni un second clic, ni un webhook tardif ne réécrivent la
-- première connexion.
CREATE OR REPLACE FUNCTION live.tg_participant_first_join_wins()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.joined_at IS NOT NULL THEN
        NEW.joined_at := OLD.joined_at;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_meeting_participants_first_join
    BEFORE UPDATE OF joined_at ON live.meeting_participants
    FOR EACH ROW EXECUTE FUNCTION live.tg_participant_first_join_wins();

-- Dépôt du travail de synchronisation dans la file générique. La clé
-- d'idempotence est l'identifiant du participant : un redémarrage du worker ou
-- une reprise de transaction ne créent jamais deux enregistrements chez Zoom.
CREATE OR REPLACE FUNCTION live.tg_enqueue_participant_sync()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.sync_status <> 'pending' THEN
        RETURN NULL;
    END IF;

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts)
    VALUES (
        'live',
        'live.sync_meeting_participant',
        jsonb_build_object('participant_id', NEW.id, 'meeting_id', NEW.meeting_id),
        NEW.id::text,
        COALESCE(NEW.next_retry_at, now()),
        NEW.max_sync_attempts
    )
    ON CONFLICT DO NOTHING;

    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_meeting_participants_enqueue
    AFTER INSERT ON live.meeting_participants
    FOR EACH ROW EXECUTE FUNCTION live.tg_enqueue_participant_sync();

-- Rattrapage en masse : remet en file les inscriptions abandonnées (quota d'API
-- épuisé, incident fournisseur) une fois la cause corrigée.
CREATE OR REPLACE FUNCTION live.requeue_failed_participants(
    p_meeting_id uuid DEFAULT NULL,
    p_limit      integer DEFAULT 500
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    v_count integer;
    -- Marqueur de campagne de rattrapage. platform.jobs conserve la clé
    -- d'idempotence des travaux déjà terminés : réutiliser l'identifiant du
    -- participant empêcherait toute relance ultérieure.
    v_batch text := to_char(now(), 'YYYYMMDDHH24MISS');
BEGIN
    WITH target AS (
        SELECT id FROM live.meeting_participants
        WHERE sync_status IN ('failed', 'abandoned')
          AND (p_meeting_id IS NULL OR meeting_id = p_meeting_id)
        ORDER BY created_at
        LIMIT p_limit
        FOR UPDATE SKIP LOCKED
    )
    UPDATE live.meeting_participants p
    SET sync_status   = 'pending',
        sync_attempts = 0,
        next_retry_at = now(),
        recovered_at  = now(),
        recovered_by  = platform.current_actor_id()
    FROM target
    WHERE p.id = target.id;

    GET DIAGNOSTICS v_count = ROW_COUNT;

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
    SELECT 'live', 'live.sync_meeting_participant',
           jsonb_build_object('participant_id', p.id, 'meeting_id', p.meeting_id),
           p.id::text || ':' || v_batch, now()
    FROM live.meeting_participants p
    WHERE p.sync_status = 'pending'
      AND p.recovered_at >= now() - interval '1 second'
      AND (p_meeting_id IS NULL OR p.meeting_id = p_meeting_id)
    ON CONFLICT DO NOTHING;

    RETURN v_count;
END;
$$;

COMMENT ON FUNCTION live.requeue_failed_participants IS
    'Remet en file les enregistrements fournisseur en échec ou abandonnés, après correction de la cause (D2).';

-- -----------------------------------------------------------------------------
-- 4. D4 — Journal des webhooks entrants
--
-- Écrire d'abord, traiter ensuite. Le point d'entrée HTTP ne fait qu'insérer ici
-- et répondre 200 : le fournisseur ne réémet pas, et rien n'est perdu si le
-- traitement échoue. L'idempotence repose sur l'identifiant d'événement du
-- fournisseur, avec repli sur une empreinte de la charge utile.
-- -----------------------------------------------------------------------------
CREATE TABLE live.provider_webhook_events (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    provider            live.meeting_provider NOT NULL,
    -- Type d'événement tel qu'émis (ex. 'meeting.started', 'meeting.participant_joined').
    event_type          text        NOT NULL,
    -- Identifiant d'événement du fournisseur : clé d'idempotence naturelle.
    provider_event_id   text,
    -- Empreinte SHA-256 de la charge utile, calculée par l'API. Sert de clé
    -- d'idempotence de repli pour les fournisseurs qui n'émettent pas d'id.
    payload_hash        bytea,
    signature           text,
    signature_verified  boolean     NOT NULL DEFAULT false,

    -- Rattachement résolu au moment du traitement. `external_id` est conservé
    -- même si la réunion est inconnue : un webhook orphelin reste analysable.
    meeting_id          uuid        REFERENCES live.meetings(id) ON DELETE SET NULL,
    external_id         text,

    payload             jsonb       NOT NULL,
    headers             jsonb       NOT NULL DEFAULT '{}'::jsonb,

    status              live.webhook_status NOT NULL DEFAULT 'received',
    attempts            smallint    NOT NULL DEFAULT 0,
    last_error          text,
    processed_at        timestamptz,
    -- Rejeu : la nouvelle ligne pointe vers l'originale, l'historique est intact.
    replayed_from_id    uuid        REFERENCES live.provider_webhook_events(id) ON DELETE SET NULL,
    occurred_at         timestamptz,
    received_at         timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ux_webhook_events_provider_event
    ON live.provider_webhook_events (provider, provider_event_id)
    WHERE provider_event_id IS NOT NULL AND replayed_from_id IS NULL;
CREATE UNIQUE INDEX ux_webhook_events_payload_hash
    ON live.provider_webhook_events (provider, event_type, payload_hash)
    WHERE provider_event_id IS NULL AND payload_hash IS NOT NULL AND replayed_from_id IS NULL;

-- File de traitement : index partiel, donc quasi vide en régime permanent.
CREATE INDEX ix_webhook_events_pending
    ON live.provider_webhook_events (received_at, id)
    WHERE status IN ('received', 'failed');
CREATE INDEX ix_webhook_events_meeting
    ON live.provider_webhook_events (meeting_id, received_at DESC)
    WHERE meeting_id IS NOT NULL;

COMMENT ON TABLE live.provider_webhook_events IS
    'Journal idempotent et rejouable des webhooks de visioconférence (D4). Toute notification est écrite avant d''être traitée.';
COMMENT ON COLUMN live.provider_webhook_events.signature_verified IS
    'Résultat de la vérification de signature (en-tête x-zm-signature pour Zoom). Un webhook non vérifié est journalisé mais jamais appliqué.';

-- Rejeu d'un webhook : duplique la ligne en état `received` et laisse le worker
-- la reprendre. L'originale conserve sa trace d'échec.
CREATE OR REPLACE FUNCTION live.replay_webhook_event(p_event_id uuid)
RETURNS uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_new_id uuid;
BEGIN
    INSERT INTO live.provider_webhook_events (
        provider, event_type, provider_event_id, payload_hash, signature,
        signature_verified, meeting_id, external_id, payload, headers,
        status, replayed_from_id, occurred_at
    )
    SELECT provider, event_type, NULL, NULL, signature,
           signature_verified, meeting_id, external_id, payload, headers,
           'received', id, occurred_at
    FROM live.provider_webhook_events
    WHERE id = p_event_id
    RETURNING id INTO v_new_id;

    IF v_new_id IS NULL THEN
        RAISE EXCEPTION 'Webhook % introuvable.', p_event_id USING ERRCODE = 'no_data_found';
    END IF;

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key)
    VALUES ('live', 'live.process_webhook_event',
            jsonb_build_object('webhook_event_id', v_new_id), v_new_id::text)
    ON CONFLICT DO NOTHING;

    RETURN v_new_id;
END;
$$;

-- -----------------------------------------------------------------------------
-- 5. Diffusions en direct
--
-- L'affichage du lecteur YouTube embarqué sur la plateforme est une obligation
-- des COP climat. La v1 se contentait d'un `activities.youtube_link` en texte
-- libre, qui servait tantôt au direct, tantôt au replay, sans savoir lequel
-- était en cours ; l'identifiant de la vidéo était par ailleurs redécouvert par
-- scraping (get-youtube-live-id) sans jamais être mémorisé.
-- -----------------------------------------------------------------------------
CREATE TABLE live.streams (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    session_id          uuid        CONSTRAINT xmod_fk_streams_session
                                    REFERENCES programme.sessions(id) ON DELETE CASCADE,
    -- Diffusion de plateau, non rattachée à une session précise (chaîne de
    -- l'événement diffusant la journée entière).
    event_id            uuid        CONSTRAINT xmod_fk_streams_event
                                    REFERENCES event.events(id) ON DELETE CASCADE,

    provider            live.stream_provider NOT NULL DEFAULT 'youtube',
    kind                live.stream_kind NOT NULL DEFAULT 'live',
    -- Identifiant de la vidéo chez le diffuseur (videoId YouTube). C'est lui,
    -- et non une URL, qui permet de construire l'iframe d'intégration.
    embed_id            text,
    -- Chaîne ou compte diffuseur (ex. '@ifddoif') : point de départ de la
    -- détection automatique du direct en cours.
    channel_ref         text,
    watch_url           platform.url,
    -- Langue du flux : une session peut être diffusée en original et en
    -- interprétation simultanée.
    locale              text        REFERENCES reference.locales(code),
    is_primary          boolean     NOT NULL DEFAULT true,

    status              live.stream_status NOT NULL DEFAULT 'scheduled',
    scheduled_start_at  timestamptz,
    started_at          timestamptz,
    ended_at            timestamptz,

    -- Détection automatique (successeur de l'edge function get-youtube-live-id) :
    -- on trace ce qui a été trouvé et quand, pour ne pas rescraper en boucle.
    is_autodetected     boolean     NOT NULL DEFAULT false,
    detected_at         timestamptz,

    -- Replay : soit ce flux devient lui-même consultable en différé, soit une
    -- ligne de type `replay` pointe vers le direct dont elle est issue.
    replay_of_id        uuid        REFERENCES live.streams(id) ON DELETE SET NULL,
    replay_url          platform.url,
    replay_available_at timestamptz,
    -- Enregistrement archivé dans la médiathèque (FK posée en fin de fichier).
    recording_asset_id  uuid,

    peak_viewer_count   integer     CHECK (peak_viewer_count IS NULL OR peak_viewer_count >= 0),
    provider_payload    jsonb       NOT NULL DEFAULT '{}'::jsonb,

    created_by          uuid        CONSTRAINT xmod_fk_streams_creator
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_streams_context
        CHECK (num_nonnulls(session_id, event_id) >= 1),
    -- Un flux diffusable porte de quoi être affiché.
    CONSTRAINT ck_streams_playable
        CHECK (status IN ('scheduled', 'cancelled') OR embed_id IS NOT NULL OR watch_url IS NOT NULL),
    CONSTRAINT ck_streams_period
        CHECK (ended_at IS NULL OR started_at IS NULL OR ended_at >= started_at),
    CONSTRAINT ck_streams_replay_shape
        CHECK (kind = 'replay' OR replay_of_id IS NULL)
);

-- CONTRAINTE STRUCTURANTE : une session n'a jamais deux directs actifs. Sans
-- elle, le front devrait arbitrer entre plusieurs lecteurs — c'est exactement ce
-- qui produisait des pages affichant un ancien direct pendant le nouveau.
CREATE UNIQUE INDEX ux_streams_single_live_per_session
    ON live.streams (session_id)
    WHERE session_id IS NOT NULL AND kind = 'live' AND status = 'live' AND is_primary;

-- Un seul direct par langue : l'interprétation ne concurrence pas l'original.
CREATE UNIQUE INDEX ux_streams_single_live_per_locale
    ON live.streams (session_id, locale)
    WHERE session_id IS NOT NULL AND kind = 'live' AND status = 'live' AND locale IS NOT NULL;

-- RÈGLE MÉTIER : l'IFDD ne diffuse jamais deux activités en direct en même
-- temps. En amont, la planification ne fait que SIGNALER le problème :
-- `programme.detect_conflicts()` remonte les diffusions qui se chevauchent avec
-- la gravité « blocking », et `programme.publication_readiness()` refuse de
-- laisser publier le programme tant qu'elles subsistent — mais rien n'empêche
-- de les écrire, parce que l'équipe doit pouvoir réorganiser librement.
-- D'où ce verrou-ci, le seul qui soit dur : il couvre le moment du direct
-- lui-même — une session qui déborde, un flux lancé à la main pendant qu'un
-- autre tourne encore. Un canal ne porte qu'un direct à la fois.
ALTER TABLE live.streams
    ADD COLUMN broadcast_channel_id uuid CONSTRAINT xmod_fk_streams_broadcast_channel
        REFERENCES event.broadcast_channels(id) ON DELETE SET NULL;

CREATE UNIQUE INDEX ux_streams_single_live_per_channel
    ON live.streams (broadcast_channel_id)
    WHERE broadcast_channel_id IS NOT NULL AND kind = 'live' AND status = 'live' AND is_primary;

CREATE INDEX ix_streams_channel ON live.streams (broadcast_channel_id, status)
    WHERE broadcast_channel_id IS NOT NULL;

COMMENT ON COLUMN live.streams.broadcast_channel_id IS
    'Canal occupé par ce flux. Un seul direct principal actif par canal : c''est le verrou de la règle « pas deux directs simultanés ».';

CREATE UNIQUE INDEX ux_streams_provider_embed
    ON live.streams (provider, embed_id, kind)
    WHERE embed_id IS NOT NULL;

CREATE INDEX ix_streams_session ON live.streams (session_id, kind) WHERE session_id IS NOT NULL;
CREATE INDEX ix_streams_event   ON live.streams (event_id, status) WHERE event_id IS NOT NULL;
CREATE INDEX ix_streams_ongoing ON live.streams (started_at DESC) WHERE status = 'live';

CREATE TRIGGER tg_streams_updated_at
    BEFORE UPDATE ON live.streams
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE live.streams IS
    'Diffusion d''une session : direct ou replay, chez YouTube ou un autre diffuseur. Remplace activities.youtube_link.';
COMMENT ON COLUMN live.streams.embed_id IS
    'Identifiant de la vidéo chez le diffuseur (videoId YouTube). Sert à construire l''iframe : voir live.build_embed_url().';
COMMENT ON COLUMN live.streams.is_primary IS
    'Flux à embarquer par défaut sur la page publique. Un seul actif par session (index unique partiel).';

-- Construction de l'URL d'intégration : la règle vit en base, les clients ne
-- réinventent pas le format de chaque diffuseur.
CREATE OR REPLACE FUNCTION live.build_embed_url(p_provider live.stream_provider, p_embed_id text)
RETURNS text
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE
        WHEN p_embed_id IS NULL THEN NULL
        WHEN p_provider = 'youtube'     THEN 'https://www.youtube.com/embed/' || p_embed_id
        WHEN p_provider = 'vimeo'       THEN 'https://player.vimeo.com/video/' || p_embed_id
        WHEN p_provider = 'dailymotion' THEN 'https://www.dailymotion.com/embed/video/' || p_embed_id
        WHEN p_provider = 'facebook'    THEN 'https://www.facebook.com/plugins/video.php?href=' || p_embed_id
        ELSE NULL
    END;
$$;

-- Ce qui doit s'afficher maintenant, tous diffuseurs confondus.
CREATE VIEW live.current_streams AS
SELECT s.id, s.session_id, s.event_id, s.provider, s.locale, s.embed_id,
       COALESCE(live.build_embed_url(s.provider, s.embed_id), s.watch_url) AS embed_url,
       s.watch_url, s.started_at, s.is_primary
FROM live.streams s
WHERE s.kind = 'live' AND s.status = 'live';

-- Le passage en direct et la fin de diffusion sont des faits métier : les
-- notifications (« la session que vous suivez a commencé ») s'y abonnent au lieu
-- de scruter la table.
CREATE OR REPLACE FUNCTION live.tg_streams_emit_status()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status IS DISTINCT FROM OLD.status AND NEW.status IN ('live', 'ended') THEN
        PERFORM platform.emit_event(
            'live', 'stream', NEW.id,
            CASE WHEN NEW.status = 'live' THEN 'live.stream.started' ELSE 'live.stream.ended' END,
            jsonb_build_object(
                'session_id', NEW.session_id,
                'event_id',   NEW.event_id,
                'provider',   NEW.provider,
                'embed_id',   NEW.embed_id
            )
        );
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_streams_emit_status
    AFTER UPDATE OF status ON live.streams
    FOR EACH ROW EXECUTE FUNCTION live.tg_streams_emit_status();

-- -----------------------------------------------------------------------------
-- 6. D5 — Messages d'incident
--
-- Un incident s'affiche en bandeau devant les téléspectateurs. Sa portée est
-- désormais générique et sa durée de vie bornée : le bandeau disparaît seul.
-- -----------------------------------------------------------------------------
CREATE TABLE live.incidents (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    scope               live.incident_scope NOT NULL,
    event_id            uuid        CONSTRAINT xmod_fk_incidents_event
                                    REFERENCES event.events(id) ON DELETE CASCADE,
    event_day_id        uuid        CONSTRAINT xmod_fk_incidents_event_day
                                    REFERENCES event.event_days(id) ON DELETE CASCADE,
    session_id          uuid        CONSTRAINT xmod_fk_incidents_session
                                    REFERENCES programme.sessions(id) ON DELETE CASCADE,
    organization_id     uuid        CONSTRAINT xmod_fk_incidents_organization
                                    REFERENCES org.organizations(id) ON DELETE CASCADE,

    -- Nature de l'incident : vocabulaire ouvert (taxonomie `incident_kind`),
    -- l'IFDD peut en ajouter sans migration.
    incident_kind_code  text        NOT NULL DEFAULT 'technical_issue',
    severity            live.incident_severity NOT NULL DEFAULT 'warning',
    title               platform.i18n_text,
    message             platform.i18n_text NOT NULL,
    action_url          platform.url,
    -- Bandeau refermable par le visiteur, ou permanent tant qu'il est publié.
    is_dismissible      boolean     NOT NULL DEFAULT true,

    -- --- Fenêtre d'affichage ---------------------------------------------
    display_from        timestamptz NOT NULL DEFAULT now(),
    display_until       timestamptz,

    -- --- Publication tracée ----------------------------------------------
    published_at        timestamptz,
    published_by        uuid        CONSTRAINT xmod_fk_incidents_publisher
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    unpublished_at      timestamptz,
    unpublished_by      uuid        CONSTRAINT xmod_fk_incidents_unpublisher
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    unpublish_reason    text,

    created_by          uuid        CONSTRAINT xmod_fk_incidents_creator
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    -- La portée déclarée et la cible renseignée ne peuvent pas diverger.
    CONSTRAINT ck_incidents_scope_target CHECK (
        CASE scope
            WHEN 'global'       THEN num_nonnulls(event_id, event_day_id, session_id, organization_id) = 0
            WHEN 'event'        THEN event_id IS NOT NULL        AND num_nonnulls(event_day_id, session_id, organization_id) = 0
            WHEN 'event_day'    THEN event_day_id IS NOT NULL    AND num_nonnulls(session_id, organization_id) = 0
            WHEN 'session'      THEN session_id IS NOT NULL      AND num_nonnulls(event_day_id, organization_id) = 0
            WHEN 'organization' THEN organization_id IS NOT NULL AND num_nonnulls(event_day_id, session_id) = 0
        END
    ),
    CONSTRAINT ck_incidents_window
        CHECK (display_until IS NULL OR display_until > display_from),
    CONSTRAINT ck_incidents_unpublish_shape
        CHECK (unpublished_at IS NULL OR published_at IS NOT NULL)
);

-- Index de service : seuls les incidents publiés et non dépubliés sont lus par
-- le front, la borne temporelle étant appliquée à la requête (now() n'est pas
-- immuable, donc inutilisable dans un prédicat d'index).
CREATE INDEX ix_incidents_published
    ON live.incidents (scope, display_from DESC)
    WHERE published_at IS NOT NULL AND unpublished_at IS NULL;
CREATE INDEX ix_incidents_event   ON live.incidents (event_id) WHERE event_id IS NOT NULL;
CREATE INDEX ix_incidents_day     ON live.incidents (event_day_id) WHERE event_day_id IS NOT NULL;
CREATE INDEX ix_incidents_session ON live.incidents (session_id) WHERE session_id IS NOT NULL;
CREATE INDEX ix_incidents_org     ON live.incidents (organization_id) WHERE organization_id IS NOT NULL;

CREATE TRIGGER tg_incidents_updated_at
    BEFORE UPDATE ON live.incidents
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_incidents_audit
    AFTER INSERT OR UPDATE OR DELETE ON live.incidents
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE live.incidents IS
    'Message d''incident affiché aux téléspectateurs (D5) : portée générique, texte i18n, fenêtre d''affichage, publication tracée.';
COMMENT ON COLUMN live.incidents.scope IS
    'Cible du message. La contrainte ck_incidents_scope_target garantit la cohérence entre la portée et la colonne renseignée.';
COMMENT ON COLUMN live.incidents.display_until IS
    'Fin d''affichage. NULL = jusqu''à dépublication explicite. Renseignée, elle évite les bandeaux oubliés en ligne.';
COMMENT ON COLUMN live.incidents.incident_kind_code IS
    'reference.taxonomy_terms.code (taxonomie incident_kind) : panne technique, retard, annulation, changement de salle…';

CREATE OR REPLACE FUNCTION live.publish_incident(p_incident_id uuid)
RETURNS live.incidents
LANGUAGE plpgsql
AS $$
DECLARE
    v_row live.incidents;
BEGIN
    UPDATE live.incidents
    SET published_at     = COALESCE(published_at, now()),
        published_by     = COALESCE(published_by, platform.current_actor_id()),
        unpublished_at   = NULL,
        unpublished_by   = NULL,
        unpublish_reason = NULL
    WHERE id = p_incident_id
    RETURNING * INTO v_row;

    IF v_row.id IS NULL THEN
        RAISE EXCEPTION 'Incident % introuvable.', p_incident_id USING ERRCODE = 'no_data_found';
    END IF;

    PERFORM platform.emit_event('live', 'incident', v_row.id, 'live.incident.published',
        jsonb_build_object('scope', v_row.scope, 'severity', v_row.severity, 'message', v_row.message));
    RETURN v_row;
END;
$$;

CREATE OR REPLACE FUNCTION live.unpublish_incident(p_incident_id uuid, p_reason text DEFAULT NULL)
RETURNS live.incidents
LANGUAGE plpgsql
AS $$
DECLARE
    v_row live.incidents;
BEGIN
    UPDATE live.incidents
    SET unpublished_at   = now(),
        unpublished_by   = platform.current_actor_id(),
        unpublish_reason = p_reason
    WHERE id = p_incident_id AND published_at IS NOT NULL
    RETURNING * INTO v_row;

    IF v_row.id IS NULL THEN
        RAISE EXCEPTION 'Incident % introuvable ou jamais publié.', p_incident_id USING ERRCODE = 'no_data_found';
    END IF;

    PERFORM platform.emit_event('live', 'incident', v_row.id, 'live.incident.resolved',
        jsonb_build_object('reason', p_reason));
    RETURN v_row;
END;
$$;

-- Ce que le lecteur d'une session doit afficher à l'instant présent : les
-- incidents de la session, de sa journée, de son événement, de l'organisation
-- porteuse, et les messages globaux — en une seule requête.
--
-- Contrat d'interface avec le module Programmation : programme.sessions expose
-- (id, event_id, event_day_id, organization_id). Écrite en PL/pgSQL pour rester
-- créable indépendamment de l'ordre de chargement des fichiers.
CREATE OR REPLACE FUNCTION live.active_incidents(
    p_session_id uuid,
    p_at         timestamptz DEFAULT now()
)
RETURNS TABLE (
    incident_id    uuid,
    scope          live.incident_scope,
    severity       live.incident_severity,
    kind_code      text,
    title          platform.i18n_text,
    message        platform.i18n_text,
    action_url     platform.url,
    is_dismissible boolean,
    display_from   timestamptz,
    display_until  timestamptz
)
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    v_event_id uuid;
    v_day_id   uuid;
    v_org_id   uuid;
BEGIN
    SELECT s.event_id, s.event_day_id, s.organization_id
      INTO v_event_id, v_day_id, v_org_id
      FROM programme.sessions s
     WHERE s.id = p_session_id;

    RETURN QUERY
    SELECT i.id, i.scope, i.severity, i.incident_kind_code, i.title, i.message,
           i.action_url, i.is_dismissible, i.display_from, i.display_until
    FROM live.incidents i
    WHERE i.published_at IS NOT NULL
      AND i.unpublished_at IS NULL
      AND i.display_from <= p_at
      AND (i.display_until IS NULL OR i.display_until > p_at)
      AND (
             i.scope = 'global'
          OR (i.scope = 'session'      AND i.session_id      = p_session_id)
          OR (i.scope = 'event_day'    AND i.event_day_id    = v_day_id)
          OR (i.scope = 'event'        AND i.event_id        = v_event_id)
          OR (i.scope = 'organization' AND i.organization_id = v_org_id)
         )
    ORDER BY i.severity DESC, i.display_from DESC;
END;
$$;

COMMENT ON FUNCTION live.active_incidents(uuid, timestamptz) IS
    'Incidents à afficher maintenant pour une session, en remontant sa journée, son événement et son organisation porteuse.';

-- -----------------------------------------------------------------------------
-- 7. Intégration inter-modules
-- -----------------------------------------------------------------------------

-- La fusion d'organisations (org.merge_organizations) doit réaffecter les
-- incidents ciblant l'organisation absorbée.
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy) VALUES
    ('live', 'incidents', 'organization_id', 'reassign')
ON CONFLICT DO NOTHING;

-- Vocabulaire ouvert des natures d'incident (aucun ENUM : l'IFDD complète la
-- liste depuis le back-office).
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('incident_kind', '{"fr":"Natures d''incident","en":"Incident kinds"}',
     '{"fr":"Cause d''un message affiché aux téléspectateurs","en":"Cause of a message shown to viewers"}',
     false, false, false)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('incident_kind', 'technical_issue',   '{"fr":"Problème technique","en":"Technical issue"}', 10),
    ('incident_kind', 'connection_issue',  '{"fr":"Problème de connexion","en":"Connection issue"}', 20),
    ('incident_kind', 'delay',             '{"fr":"Retard","en":"Delay"}', 30),
    ('incident_kind', 'schedule_change',   '{"fr":"Changement d''horaire","en":"Schedule change"}', 40),
    ('incident_kind', 'room_change',       '{"fr":"Changement de salle","en":"Room change"}', 50),
    ('incident_kind', 'cancellation',      '{"fr":"Annulation","en":"Cancellation"}', 60),
    ('incident_kind', 'speaker_absence',   '{"fr":"Absence d''un intervenant","en":"Speaker absence"}', 70),
    ('incident_kind', 'information',       '{"fr":"Information","en":"Information"}', 80)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- Rattachement de l'enregistrement à la médiathèque. Posé conditionnellement :
-- le module Direct reste chargeable seul, sans le module média (extraction en
-- service autonome, base de test, jeu de reprise partiel).
DO $$
BEGIN
    IF to_regclass('media.assets') IS NOT NULL
       AND NOT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'xmod_fk_streams_recording_asset') THEN
        ALTER TABLE live.streams
            ADD CONSTRAINT xmod_fk_streams_recording_asset
            FOREIGN KEY (recording_asset_id) REFERENCES media.assets(id) ON DELETE SET NULL;
    END IF;
END
$$;

COMMENT ON COLUMN live.streams.recording_asset_id IS
    'media.assets.id du replay archivé. FK xmod_fk_streams_recording_asset posée dès que le module média est présent.';
