-- Temps réel, pages publiques et poste de direct (31/10) : signaux posés par la base.
--   edition:<id>:public:live, :schedule   séances publiées, fils et leur page
--   edition:<id>:public:questions         questions du public
--   edition:<id>:public:incidents         messages d'incident (+ travail live.incident_window aux bornes)
--   edition:<id>:desk                     poste de direct (privé)
-- Rejouable, additive, aucune donnée touchée. Suppose 2026-10-31-temps-reel.sql
-- (`platform.signal`) et 2026-10-31-direct-poste.sql (`live.meeting_presences`) jouées.
-- Modèle : docs/database/060_events.sql § 4 ter, 075_programme_sessions.sql § 2 bis et § 5,
-- 080_live.sql § 3 bis et § 6.
--
-- Jouer AVANT le nouveau worker (il sert la file `live_incidents`) ; l'API n'en
-- dépend pas.
BEGIN;

-- 060_events.sql § 4 ter
-- Signal du programme public (temps réel, 31/10) : un fil publié, ou qui cesse
-- de l'être, et sa page (parties, moments, partenaires) changent ce que
-- montrent la programmation et la page du fil. `edition:<id>:public:schedule`,
-- sujet public relu par le pouls ; même sujet que les séances
-- (075_programme_sessions.sql, `tg_sessions_public_signal`).
CREATE OR REPLACE FUNCTION event.signal_public_track(p_track_id uuid)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_event uuid;
BEGIN
    SELECT event_id INTO v_event FROM event.programme_tracks
     WHERE id = p_track_id AND published_at IS NOT NULL;
    IF v_event IS NOT NULL THEN
        PERFORM platform.signal('edition:' || v_event || ':public:schedule');
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION event.tg_programme_tracks_public_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF (TG_OP <> 'INSERT' AND OLD.published_at IS NOT NULL)
       OR (TG_OP <> 'DELETE' AND NEW.published_at IS NOT NULL) THEN
        PERFORM platform.signal('edition:'
            || CASE WHEN TG_OP = 'DELETE' THEN OLD.event_id ELSE NEW.event_id END
            || ':public:schedule');
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_programme_tracks_public_signal ON event.programme_tracks;
CREATE TRIGGER tg_programme_tracks_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON event.programme_tracks
    FOR EACH ROW EXECUTE FUNCTION event.tg_programme_tracks_public_signal();

CREATE OR REPLACE FUNCTION event.tg_programme_track_page_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM event.signal_public_track(
        CASE WHEN TG_OP = 'DELETE' THEN OLD.track_id ELSE NEW.track_id END);
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_programme_track_parts_public_signal ON event.programme_track_parts;
CREATE TRIGGER tg_programme_track_parts_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON event.programme_track_parts
    FOR EACH ROW EXECUTE FUNCTION event.tg_programme_track_page_signal();

DROP TRIGGER IF EXISTS tg_programme_track_moments_public_signal ON event.programme_track_moments;
CREATE TRIGGER tg_programme_track_moments_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON event.programme_track_moments
    FOR EACH ROW EXECUTE FUNCTION event.tg_programme_track_page_signal();

DROP TRIGGER IF EXISTS tg_programme_track_partners_public_signal ON event.programme_track_partners;
CREATE TRIGGER tg_programme_track_partners_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON event.programme_track_partners
    FOR EACH ROW EXECUTE FUNCTION event.tg_programme_track_page_signal();

-- 075_programme_sessions.sql § 2 bis
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

DROP TRIGGER IF EXISTS tg_sessions_public_signal ON programme.sessions;
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

DROP TRIGGER IF EXISTS tg_session_tracks_public_signal ON programme.session_tracks;
CREATE TRIGGER tg_session_tracks_public_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_tracks
    FOR EACH ROW EXECUTE FUNCTION programme.tg_session_tracks_public_signal();

-- 075_programme_sessions.sql § 5
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

DROP TRIGGER IF EXISTS tg_session_questions_signal ON programme.session_questions;
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

DROP TRIGGER IF EXISTS tg_session_question_answers_signal ON programme.session_question_answers;
CREATE TRIGGER tg_session_question_answers_signal
    AFTER INSERT OR UPDATE OR DELETE ON programme.session_question_answers
    FOR EACH ROW EXECUTE FUNCTION programme.tg_session_question_answers_signal();

-- 080_live.sql § 3 bis
-- Signal du poste de direct (temps réel, 31/10) : `edition:<id>:desk`, sujet
-- PRIVÉ (`live.meeting.manage` sur l'édition, `live::topic_authorizers()`).
-- Entrée ou sortie de la salle, réunion créée ou en échec chez Zoom, inscription
-- acceptée, refusée ou abandonnée : tout ce que montre `LiveDeskMeeting`. En
-- base : le webhook, les travaux de synchronisation et les relances manuelles
-- écrivent ces trois tables. L'édition de la réunion, à défaut celle de sa séance.
CREATE OR REPLACE FUNCTION live.signal_desk(p_event_id uuid, p_session_id uuid)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_event uuid := p_event_id;
BEGIN
    IF v_event IS NULL AND p_session_id IS NOT NULL THEN
        SELECT event_id INTO v_event FROM programme.sessions WHERE id = p_session_id;
    END IF;
    IF v_event IS NOT NULL THEN
        PERFORM platform.signal('edition:' || v_event || ':desk');
    END IF;
END;
$$;

CREATE OR REPLACE FUNCTION live.tg_desk_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_meeting uuid;
    v_event   uuid;
    v_session uuid;
BEGIN
    IF TG_TABLE_NAME = 'meetings' THEN
        IF TG_OP = 'UPDATE'
           AND (OLD.status, OLD.sync_status, OLD.last_sync_error, OLD.actual_start_at,
                OLD.actual_end_at, OLD.external_uuid, OLD.session_id)
               IS NOT DISTINCT FROM
               (NEW.status, NEW.sync_status, NEW.last_sync_error, NEW.actual_start_at,
                NEW.actual_end_at, NEW.external_uuid, NEW.session_id) THEN
            RETURN NULL;
        END IF;
        IF TG_OP = 'DELETE' THEN
            PERFORM live.signal_desk(OLD.event_id, OLD.session_id);
        ELSE
            PERFORM live.signal_desk(NEW.event_id, NEW.session_id);
        END IF;
        RETURN NULL;
    END IF;

    -- IF imbriqués : `sync_status` n'existe que sur les inscriptions, et
    -- PL/pgSQL résout le champ dès qu'il évalue l'expression.
    IF TG_TABLE_NAME = 'meeting_participants' AND TG_OP = 'UPDATE' THEN
        IF OLD.sync_status = NEW.sync_status THEN
            RETURN NULL;
        END IF;
    END IF;
    v_meeting := CASE WHEN TG_OP = 'DELETE' THEN OLD.meeting_id ELSE NEW.meeting_id END;
    SELECT event_id, session_id INTO v_event, v_session FROM live.meetings WHERE id = v_meeting;
    IF FOUND THEN
        PERFORM live.signal_desk(v_event, v_session);
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_meetings_desk_signal ON live.meetings;
CREATE TRIGGER tg_meetings_desk_signal
    AFTER INSERT OR UPDATE OR DELETE ON live.meetings
    FOR EACH ROW EXECUTE FUNCTION live.tg_desk_signal();

DROP TRIGGER IF EXISTS tg_meeting_participants_desk_signal ON live.meeting_participants;
CREATE TRIGGER tg_meeting_participants_desk_signal
    AFTER INSERT OR UPDATE OR DELETE ON live.meeting_participants
    FOR EACH ROW EXECUTE FUNCTION live.tg_desk_signal();

DROP TRIGGER IF EXISTS tg_meeting_presences_desk_signal ON live.meeting_presences;
CREATE TRIGGER tg_meeting_presences_desk_signal
    AFTER INSERT OR UPDATE ON live.meeting_presences
    FOR EACH ROW EXECUTE FUNCTION live.tg_desk_signal();

-- 080_live.sql § 6
-- Signal des messages d'incident (temps réel, 31/10) :
-- `edition:<id>:public:incidents`, sujet public relu par le pouls, pour chaque
-- édition où le bandeau s'affiche. La cible se résout comme dans
-- `active_incidents()` : édition, journée, séance, organisation porteuse d'une
-- séance publiée, et pour un message global toutes les éditions (quelques
-- dizaines). En base : publier, corriger, retirer passent par plusieurs
-- fonctions et services, aucun n'oublie.
--
-- LA FENÊTRE D'AFFICHAGE N'ÉCRIT RIEN : un message programmé pour 14 h, ou qui
-- expire à 15 h, apparaît ou disparaît sans écriture. Le déclencheur dépose donc
-- un travail `live.incident_window` à chaque borne future ; le worker ne fait
-- que rejouer le signal (`live.signal_incident()`). File `live_incidents`, à
-- part de la file « live » qui n'est servie qu'avec Zoom configuré.
CREATE OR REPLACE FUNCTION live.signal_incident_editions(
    p_scope           live.incident_scope,
    p_event_id        uuid,
    p_event_day_id    uuid,
    p_session_id      uuid,
    p_organization_id uuid
)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_event uuid;
BEGIN
    FOR v_event IN
        SELECT p_event_id WHERE p_scope = 'event'
        UNION
        SELECT d.event_id FROM event.event_days d
         WHERE p_scope = 'event_day' AND d.id = p_event_day_id
        UNION
        SELECT s.event_id FROM programme.sessions s
         WHERE p_scope = 'session' AND s.id = p_session_id
        UNION
        SELECT s.event_id FROM programme.sessions s
         WHERE p_scope = 'organization' AND s.organization_id = p_organization_id
           AND s.published_at IS NOT NULL
        UNION
        SELECT e.id FROM event.events e WHERE p_scope = 'global'
    LOOP
        IF v_event IS NOT NULL THEN
            PERFORM platform.signal('edition:' || v_event || ':public:incidents');
        END IF;
    END LOOP;
END;
$$;

CREATE OR REPLACE FUNCTION live.signal_incident(p_incident_id uuid)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_row live.incidents;
BEGIN
    SELECT * INTO v_row FROM live.incidents
     WHERE id = p_incident_id AND published_at IS NOT NULL;
    IF FOUND THEN
        PERFORM live.signal_incident_editions(v_row.scope, v_row.event_id,
            v_row.event_day_id, v_row.session_id, v_row.organization_id);
    END IF;
END;
$$;

COMMENT ON FUNCTION live.signal_incident(uuid) IS
    'Signale `edition:<id>:public:incidents` aux éditions où le message publié s''affiche. Rejouée par le travail live.incident_window aux bornes de la fenêtre d''affichage.';

CREATE OR REPLACE FUNCTION live.tg_incidents_signal()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_borne timestamptz;
BEGIN
    IF TG_OP <> 'INSERT' AND OLD.published_at IS NOT NULL THEN
        PERFORM live.signal_incident_editions(OLD.scope, OLD.event_id,
            OLD.event_day_id, OLD.session_id, OLD.organization_id);
    END IF;
    IF TG_OP = 'DELETE' OR NEW.published_at IS NULL THEN
        RETURN NULL;
    END IF;

    PERFORM live.signal_incident_editions(NEW.scope, NEW.event_id,
        NEW.event_day_id, NEW.session_id, NEW.organization_id);
    IF NEW.unpublished_at IS NULL THEN
        FOREACH v_borne IN ARRAY ARRAY[NEW.display_from, NEW.display_until] LOOP
            IF v_borne IS NOT NULL AND v_borne > now() THEN
                INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
                VALUES ('live_incidents', 'live.incident_window',
                        jsonb_build_object('incident_id', NEW.id),
                        NEW.id::text || ':' || extract(epoch FROM v_borne)::bigint,
                        v_borne)
                ON CONFLICT DO NOTHING;
            END IF;
        END LOOP;
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_incidents_signal ON live.incidents;
CREATE TRIGGER tg_incidents_signal
    AFTER INSERT OR UPDATE OR DELETE ON live.incidents
    FOR EACH ROW EXECUTE FUNCTION live.tg_incidents_signal();

COMMIT;
