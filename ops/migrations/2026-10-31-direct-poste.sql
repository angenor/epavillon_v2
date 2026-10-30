-- Le direct s'allume, les présents dans la salle Zoom, les messages d'incident
-- d'une activité (31/10). Rejouable, additive, aucune donnée touchée.
-- Modèle : docs/database/080_live.sql, en-tête, § 2 bis, § 3 bis et § 6.
--
-- Jouer AVANT de démarrer la nouvelle API et le nouveau worker : leurs requêtes
-- nomment la table et les fonctions.
BEGIN;

-- § 2 bis. La séance suit sa réunion.
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

    PERFORM platform.emit_event(
        'live', 'meeting', v_meeting.id, 'live.meeting.started',
        jsonb_build_object(
            'session_id', v_meeting.session_id,
            'event_id',   v_meeting.event_id,
            'occurrence', v_meeting.external_uuid,
            'started_at', v_meeting.actual_start_at
        )
    );
    RETURN true;
END;
$$;

COMMENT ON FUNCTION live.announce_meeting_start(uuid, text) IS
    'Émet live.meeting.started si la réunion tourne et que sa séance commence dans moins d''un quart d''heure ; sinon (répétition) se reprogramme à début − 15 min. Faux si rien n''est émis.';

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
                'ended_at',   NEW.actual_end_at
            )
        );
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_meetings_follow_status ON live.meetings;
CREATE TRIGGER tg_meetings_follow_status
    AFTER UPDATE OF status, external_uuid ON live.meetings
    FOR EACH ROW EXECUTE FUNCTION live.tg_meetings_follow_status();

-- § 3 bis. Présence dans la salle.
CREATE TABLE IF NOT EXISTS live.meeting_presences (
    id               uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    meeting_id       uuid        NOT NULL REFERENCES live.meetings(id) ON DELETE CASCADE,
    occurrence       text        NOT NULL DEFAULT '',
    participant_key  text        NOT NULL,
    last_joined_at   timestamptz,
    last_left_at     timestamptz,
    updated_at       timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_meeting_presences_participant UNIQUE (meeting_id, occurrence, participant_key)
);

COMMENT ON TABLE live.meeting_presences IS
    'Connexions à une réunion vues par les notifications Zoom, par occurrence. Sert au compte des présents du poste de direct.';

CREATE OR REPLACE VIEW live.meeting_room_counts AS
SELECT m.id AS meeting_id,
       count(p.id) FILTER (WHERE p.last_joined_at IS NOT NULL
                             AND (p.last_left_at IS NULL OR p.last_joined_at > p.last_left_at)) AS present_count,
       count(p.id) FILTER (WHERE p.last_joined_at IS NOT NULL) AS joined_count
FROM live.meetings m
LEFT JOIN live.meeting_presences p
       ON p.meeting_id = m.id AND p.occurrence = COALESCE(m.external_uuid, '')
GROUP BY m.id;

COMMENT ON VIEW live.meeting_room_counts IS
    'Par réunion, dans son occurrence en cours : personnes dans la salle (present_count) et passées par la salle (joined_count). Ne vaut que pour une réunion démarrée.';

-- § 6. Les messages d'incident d'une activité ou d'une journée spéciale.
CREATE OR REPLACE FUNCTION live.active_incidents_for_sessions(
    p_event_id    uuid,
    p_session_ids uuid[],
    p_at          timestamptz DEFAULT now()
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
    display_until  timestamptz,
    target_id      uuid,
    target_label   text
)
LANGUAGE sql
STABLE
AS $$
    WITH cibles AS (
        SELECT s.id, s.event_day_id, s.organization_id
          FROM programme.sessions s
         WHERE s.event_id = p_event_id
           AND s.id = ANY (p_session_ids)
           AND s.published_at IS NOT NULL
    )
    SELECT x.*
      FROM live.active_incidents_for_event(p_event_id, p_at) x
     WHERE x.scope IN ('global', 'event')
        OR (x.scope = 'session'      AND x.target_id IN (SELECT id FROM cibles))
        OR (x.scope = 'event_day'    AND x.target_id IN (SELECT event_day_id FROM cibles))
        OR (x.scope = 'organization' AND x.target_id IN (SELECT organization_id FROM cibles))
     ORDER BY x.severity DESC, x.display_from DESC;
$$;

COMMENT ON FUNCTION live.active_incidents_for_sessions(uuid, uuid[], timestamptz) IS
    'Incidents actifs concernant des activités publiées d''une édition : globaux, de l''édition, de leurs journées, d''elles-mêmes, de leurs organisations porteuses. Page d''une activité et d''une journée spéciale.';

COMMIT;
