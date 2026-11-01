-- Régie du jour (02/11). Rejouable.
-- Modèle : docs/database/075_programme_sessions.sql § 1 (colonnes et
-- tg_sessions_status_times) et § 9 ; 110_engagement.sql § 11 (type d'avis).
--
-- Additive, aucune donnée touchée :
--   - programme.sessions : instant, auteur, raison et note d'annulation ;
--     bornes réelles du direct ;
--   - programme.session_presence : présence des organisateurs ;
--   - programme.control_room_actions : journal de la régie, de quoi défaire ;
--   - taxonomie session_cancellation_reason ; type d'avis programme.session.restored.
-- Jouer AVANT de démarrer l'API et le worker, qui lisent ces colonnes.
BEGIN;

ALTER TABLE programme.sessions
    ADD COLUMN IF NOT EXISTS cancelled_at timestamptz,
    ADD COLUMN IF NOT EXISTS cancelled_by uuid,
    ADD COLUMN IF NOT EXISTS cancellation_reason_code text,
    ADD COLUMN IF NOT EXISTS cancellation_note text,
    ADD COLUMN IF NOT EXISTS live_started_at timestamptz,
    ADD COLUMN IF NOT EXISTS live_ended_at timestamptz;

ALTER TABLE programme.sessions DROP CONSTRAINT IF EXISTS xmod_fk_sessions_canceller;
ALTER TABLE programme.sessions ADD CONSTRAINT xmod_fk_sessions_canceller
    FOREIGN KEY (cancelled_by) REFERENCES identity.people(id) ON DELETE SET NULL;
ALTER TABLE programme.sessions DROP CONSTRAINT IF EXISTS ck_sessions_cancellation_note;
ALTER TABLE programme.sessions ADD CONSTRAINT ck_sessions_cancellation_note CHECK (
    cancellation_note IS NULL OR length(cancellation_note) <= 2000);

-- Les séances déjà annulées ou en direct gardent un instant plausible.
UPDATE programme.sessions SET cancelled_at = updated_at
 WHERE status = 'cancelled' AND cancelled_at IS NULL;
UPDATE programme.sessions SET live_started_at = greatest(starts_at, updated_at - interval '1 hour')
 WHERE status = 'live' AND live_started_at IS NULL;

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

DROP TRIGGER IF EXISTS tg_sessions_status_times ON programme.sessions;
CREATE TRIGGER tg_sessions_status_times
    BEFORE UPDATE OF status ON programme.sessions
    FOR EACH ROW
    WHEN (NEW.status IS DISTINCT FROM OLD.status)
    EXECUTE FUNCTION programme.tg_sessions_status_times();

CREATE TABLE IF NOT EXISTS programme.session_presence (
    session_id  uuid        PRIMARY KEY REFERENCES programme.sessions(id) ON DELETE CASCADE,
    is_present  boolean     NOT NULL,
    noted_at    timestamptz NOT NULL DEFAULT now(),
    noted_by    uuid        CONSTRAINT xmod_fk_session_presence_author
                            REFERENCES identity.people(id) ON DELETE SET NULL
);

COMMENT ON TABLE programme.session_presence IS
    'Présence des organisateurs d''une activité, notée en régie. Absence de ligne : inconnue.';

CREATE TABLE IF NOT EXISTS programme.control_room_actions (
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
    CONSTRAINT ck_control_room_actions_code CHECK (action_code IN (
        'presence_confirmed', 'session_cancelled', 'live_started', 'live_ended')),
    CONSTRAINT ck_control_room_actions_data CHECK (jsonb_typeof(data) = 'object'),
    CONSTRAINT ck_control_room_actions_undone CHECK (undone_at IS NOT NULL OR undone_by IS NULL)
);

CREATE INDEX IF NOT EXISTS ix_control_room_actions_event   ON programme.control_room_actions (event_id, acted_at DESC);
CREATE INDEX IF NOT EXISTS ix_control_room_actions_session ON programme.control_room_actions (session_id, acted_at DESC);

COMMENT ON TABLE programme.control_room_actions IS
    'Journal de la régie du jour : chaque geste (présence, annulation, direct) et de quoi le défaire. Défaire pose undone_at, la ligne reste.';
COMMENT ON COLUMN programme.control_room_actions.data IS
    'Ce qu''il faut pour défaire le geste : état d''avant, choix d''envoi des courriels. Aucune donnée personnelle.';

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

DROP TRIGGER IF EXISTS tg_control_room_actions_signal ON programme.control_room_actions;
CREATE TRIGGER tg_control_room_actions_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.control_room_actions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_control_room_signal();

DROP TRIGGER IF EXISTS tg_session_presence_signal ON programme.session_presence;
CREATE TRIGGER tg_session_presence_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_presence
    FOR EACH ROW EXECUTE FUNCTION programme.tg_control_room_signal();

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

INSERT INTO engagement.notification_types (code, module_code, label, default_channels, criticality, expected_variables) VALUES
    ('programme.session.restored', 'programme', '{"fr":"Session finalement maintenue","en":"Session going ahead after all"}', '{email}', 'critical', '{titre_session}')
ON CONFLICT (code) DO NOTHING;

COMMIT;
