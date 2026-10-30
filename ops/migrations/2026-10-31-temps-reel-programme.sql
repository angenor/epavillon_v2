-- Temps réel, module Programme (31/10) : signaux d'écran posés par la base.
--   proposal:<id>:status, organization:<id>, event:<id>:proposals  (état d'un dossier)
--   event:<id>:planner                                            (grille du planificateur)
-- Rejouable, additive, aucune donnée touchée. Suppose 2026-10-31-temps-reel.sql
-- (`platform.signal`) déjà jouée.
-- Modèle : docs/database/070_programme_proposals.sql § 3 et 075_programme_sessions.sql § 2.
BEGIN;

CREATE OR REPLACE FUNCTION programme.tg_proposals_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM platform.signal('proposal:' || NEW.id || ':status');
    PERFORM platform.signal('organization:' || NEW.organization_id);
    PERFORM platform.signal('event:' || NEW.event_id || ':proposals');
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_proposals_signal ON programme.proposals;
CREATE TRIGGER tg_proposals_signal
    AFTER INSERT OR UPDATE OF status, deleted_at ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_proposals_signal();

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

DROP TRIGGER IF EXISTS tg_sessions_planner_signal ON programme.sessions;
CREATE TRIGGER tg_sessions_planner_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_planner_signal();

DROP TRIGGER IF EXISTS tg_session_tracks_planner_signal ON programme.session_tracks;
CREATE TRIGGER tg_session_tracks_planner_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_tracks
    FOR EACH ROW EXECUTE FUNCTION programme.tg_planner_signal();

COMMIT;
