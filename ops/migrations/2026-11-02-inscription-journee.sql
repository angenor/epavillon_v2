-- Inscription à une journée (02/11). Rejouable.
-- Modèle : docs/database/075_programme_sessions.sql § 4 (émission des
-- inscriptions), § 4 bis et § 8 ; 080_live.sql (en-tête, live.meetings,
-- live.meetings_public, § 2 bis, live.meeting_participants) ; 110_engagement.sql
-- § 11 (type d'avis).
--
-- Une journée spéciale, et l'atelier préparatoire, se suivent avec un seul
-- direct ; on s'y inscrit, avec ou sans compte, en gardant les activités qui
-- intéressent. Additive, aucune donnée touchée :
--   - programme.is_day_track_kind(), programme.day_track_of() ;
--   - programme.track_registrations : l'inscription à la journée, ses réponses,
--     l'empreinte du jeton de l'invité, sa première entrée dans le direct ;
--   - programme.registrations.track_registration_id : les activités gardées,
--     contrôlées par tg_registrations_check_track ; l'événement d'inscription
--     porte désormais sa journée et part aussi quand on rattache une ligne ;
--   - live.meetings.track_id (exclusif de session_id), une réunion vivante par
--     journée ; live.meetings_public la montre ; les annonces de démarrage et de
--     fin la portent ;
--   - live.meeting_participants.track_registration_id (exclusif de
--     registration_id), une place par inscription à la journée ;
--   - type d'avis programme.track_registration.confirmed.
-- Aucune inscription ni réunion n'existait sur les activités des journées le
-- 02/11 : rien à reprendre.
-- Jouer AVANT de démarrer l'API et le worker de la phase 2, qui lisent ces
-- colonnes. Sans eux, rien ne change : aucune ligne n'est écrite.
BEGIN;

DO $$
BEGIN
    IF to_regclass('event.programme_tracks') IS NULL
       OR to_regclass('programme.session_tracks') IS NULL
       OR to_regclass('programme.registrations') IS NULL
       OR to_regclass('live.meetings') IS NULL
       OR to_regclass('live.meeting_participants') IS NULL THEN
        RAISE EXCEPTION 'Modèle incomplet : programmation, direct ou journées spéciales absents.';
    END IF;
END
$$;

-- --- Programmation : quelle journée ----------------------------------------

CREATE OR REPLACE FUNCTION programme.is_day_track_kind(p_kind event.track_kind)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT p_kind IN ('special_day', 'side_programme');
$$;

COMMENT ON FUNCTION programme.is_day_track_kind(event.track_kind) IS
    'Vrai pour une journée spéciale ou un programme parallèle (atelier préparatoire) : un seul direct pour la journée, une inscription à la journée.';

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

-- --- Programmation : l'inscription à la journée -----------------------------

CREATE TABLE IF NOT EXISTS programme.track_registrations (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    track_id        uuid        NOT NULL CONSTRAINT xmod_fk_track_registrations_track
                                REFERENCES event.programme_tracks(id) ON DELETE CASCADE,
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_track_registrations_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    organization_id uuid        CONSTRAINT xmod_fk_track_registrations_organization
                                REFERENCES org.organizations(id) ON DELETE SET NULL,
    status          programme.registration_status NOT NULL DEFAULT 'registered',
    answers         jsonb       NOT NULL DEFAULT '{}'::jsonb,
    locale          text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    manage_token_hash bytea,
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

CREATE UNIQUE INDEX IF NOT EXISTS ux_track_registrations_person_track
    ON programme.track_registrations (track_id, person_id)
    WHERE status <> 'cancelled';
CREATE UNIQUE INDEX IF NOT EXISTS ux_track_registrations_manage_token
    ON programme.track_registrations (manage_token_hash)
    WHERE manage_token_hash IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_track_registrations_track  ON programme.track_registrations (track_id, status);
CREATE INDEX IF NOT EXISTS ix_track_registrations_person ON programme.track_registrations (person_id, created_at DESC);

DROP TRIGGER IF EXISTS tg_track_registrations_updated_at ON programme.track_registrations;
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

DROP TRIGGER IF EXISTS tg_track_registrations_check ON programme.track_registrations;
CREATE TRIGGER tg_track_registrations_check
    BEFORE INSERT OR UPDATE OF track_id ON programme.track_registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_track_registration();

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

DROP TRIGGER IF EXISTS tg_track_registrations_events ON programme.track_registrations;
CREATE TRIGGER tg_track_registrations_events
    AFTER INSERT OR UPDATE OF status ON programme.track_registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_track_registrations_emit_events();

INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy, dedupe_on) VALUES
    ('programme', 'track_registrations', 'organization_id', 'reassign', '{}')
ON CONFLICT DO NOTHING;

-- --- Programmation : les activités gardées ---------------------------------

ALTER TABLE programme.registrations
    ADD COLUMN IF NOT EXISTS track_registration_id uuid;

ALTER TABLE programme.registrations DROP CONSTRAINT IF EXISTS fk_registrations_track_registration;
ALTER TABLE programme.registrations ADD CONSTRAINT fk_registrations_track_registration
    FOREIGN KEY (track_registration_id) REFERENCES programme.track_registrations(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS ix_registrations_track_registration
    ON programme.registrations (track_registration_id)
    WHERE track_registration_id IS NOT NULL;

COMMENT ON COLUMN programme.registrations.track_registration_id IS
    'Inscription à la journée dont cette activité est gardée ; NULL pour une inscription ordinaire. Une telle ligne n''a pas de place Zoom propre : le direct est celui de la journée.';

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

DROP TRIGGER IF EXISTS tg_registrations_check_track ON programme.registrations;
CREATE TRIGGER tg_registrations_check_track
    BEFORE INSERT OR UPDATE OF track_registration_id, session_id, person_id ON programme.registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_registration_track();

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

DROP TRIGGER IF EXISTS tg_registrations_events ON programme.registrations;
CREATE TRIGGER tg_registrations_events
    AFTER INSERT OR UPDATE OF status, track_registration_id ON programme.registrations
    FOR EACH ROW EXECUTE FUNCTION programme.tg_registrations_emit_events();

-- --- Direct : la réunion de la journée --------------------------------------

ALTER TABLE live.meetings
    ADD COLUMN IF NOT EXISTS track_id uuid;

ALTER TABLE live.meetings DROP CONSTRAINT IF EXISTS xmod_fk_meetings_track;
ALTER TABLE live.meetings ADD CONSTRAINT xmod_fk_meetings_track
    FOREIGN KEY (track_id) REFERENCES event.programme_tracks(id) ON DELETE CASCADE;
ALTER TABLE live.meetings DROP CONSTRAINT IF EXISTS ck_meetings_target;
ALTER TABLE live.meetings ADD CONSTRAINT ck_meetings_target
    CHECK (session_id IS NULL OR track_id IS NULL);

CREATE UNIQUE INDEX IF NOT EXISTS ux_meetings_active_per_track
    ON live.meetings (track_id)
    WHERE track_id IS NOT NULL AND status NOT IN ('cancelled', 'failed');

COMMENT ON COLUMN live.meetings.track_id IS
    'Journée (fil special_day ou side_programme) dont c''est l''unique réunion ; ses activités n''en ont pas. Exclusive de session_id (ck_meetings_target).';

-- Une colonne en queue : CREATE OR REPLACE VIEW l'accepte.
CREATE OR REPLACE VIEW live.meetings_public AS
SELECT m.id, m.session_id, m.event_id, m.provider, m.topic,
       m.scheduled_start_at, m.duration_minutes, m.timezone,
       m.actual_start_at, m.actual_end_at,
       m.join_url, m.registration_url, m.requires_registration, m.status,
       m.track_id
FROM live.meetings m;

CREATE OR REPLACE FUNCTION live.announce_meeting_start(
    p_meeting_id uuid,
    p_occurrence text DEFAULT NULL
)
RETURNS boolean
LANGUAGE plpgsql
AS $$
DECLARE
    v_meeting live.meetings%ROWTYPE;
    v_opens   timestamptz;
BEGIN
    SELECT * INTO v_meeting FROM live.meetings WHERE id = p_meeting_id;
    IF NOT FOUND OR v_meeting.status <> 'started'
       OR (p_occurrence IS NOT NULL AND v_meeting.external_uuid IS DISTINCT FROM p_occurrence) THEN
        RETURN false;
    END IF;

    SELECT s.starts_at - interval '15 minutes' INTO v_opens
      FROM programme.sessions s WHERE s.id = v_meeting.session_id;

    IF v_opens IS NOT NULL AND now() < v_opens THEN
        INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
        VALUES ('live', 'live.announce_meeting_start',
                jsonb_build_object('meeting_id', v_meeting.id, 'occurrence', v_meeting.external_uuid),
                v_meeting.id::text || ':' || COALESCE(v_meeting.external_uuid, '')
                    || ':' || extract(epoch FROM v_opens)::bigint,
                v_opens)
        ON CONFLICT DO NOTHING;
        RETURN false;
    END IF;

    -- `track_id` (02/11) : la réunion d'une journée, dont `session_id` est nul.
    PERFORM platform.emit_event(
        'live', 'meeting', v_meeting.id, 'live.meeting.started',
        jsonb_build_object(
            'session_id', v_meeting.session_id,
            'event_id',   v_meeting.event_id,
            'occurrence', v_meeting.external_uuid,
            'started_at', v_meeting.actual_start_at,
            'track_id',   v_meeting.track_id
        )
    );
    RETURN true;
END;
$$;

CREATE OR REPLACE FUNCTION live.tg_meetings_follow_status()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = 'started'
       AND (OLD.status IS DISTINCT FROM 'started' OR NEW.external_uuid IS DISTINCT FROM OLD.external_uuid) THEN
        PERFORM live.announce_meeting_start(NEW.id);
    ELSIF NEW.status = 'ended' AND OLD.status IS DISTINCT FROM 'ended' THEN
        PERFORM platform.emit_event(
            'live', 'meeting', NEW.id, 'live.meeting.ended',
            jsonb_build_object(
                'session_id', NEW.session_id,
                'event_id',   NEW.event_id,
                'occurrence', NEW.external_uuid,
                'ended_at',   NEW.actual_end_at,
                'track_id',   NEW.track_id
            )
        );
    END IF;
    RETURN NULL;
END;
$$;

-- --- Direct : la place de l'inscrit à la journée ----------------------------

ALTER TABLE live.meeting_participants
    ADD COLUMN IF NOT EXISTS track_registration_id uuid;

ALTER TABLE live.meeting_participants
    DROP CONSTRAINT IF EXISTS xmod_fk_meeting_participants_track_registration;
ALTER TABLE live.meeting_participants ADD CONSTRAINT xmod_fk_meeting_participants_track_registration
    FOREIGN KEY (track_registration_id) REFERENCES programme.track_registrations(id) ON DELETE CASCADE;
ALTER TABLE live.meeting_participants DROP CONSTRAINT IF EXISTS ck_meeting_participants_origin;
ALTER TABLE live.meeting_participants ADD CONSTRAINT ck_meeting_participants_origin
    CHECK (registration_id IS NULL OR track_registration_id IS NULL);

CREATE UNIQUE INDEX IF NOT EXISTS ux_meeting_participants_track_registration
    ON live.meeting_participants (track_registration_id)
    WHERE track_registration_id IS NOT NULL;

COMMENT ON COLUMN live.meeting_participants.track_registration_id IS
    'Inscription à la journée qui a donné cette place, dans la réunion de la journée. Exclusive de registration_id (ck_meeting_participants_origin).';

-- --- Engagement : l'avis de la journée --------------------------------------

INSERT INTO engagement.notification_types (code, module_code, label, default_channels, criticality, expected_variables) VALUES
    ('programme.track_registration.confirmed', 'programme', '{"fr":"Inscription à la journée confirmée","en":"Day registration confirmed"}', '{in_app,email}', 'important', '{prenom,titre_journee,date_journee,activites_gardees,lien_direct,lien_gestion}')
ON CONFLICT (code) DO NOTHING;

COMMIT;
