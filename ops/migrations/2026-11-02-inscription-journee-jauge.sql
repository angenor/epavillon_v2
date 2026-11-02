-- Inscription à une journée, suite (02/11). Rejouable. À jouer APRÈS
-- 2026-11-02-inscription-journee.sql.
-- Modèle : docs/database/075_programme_sessions.sql § 4 (tg_validate_registration)
-- et § 4 bis (composition) ; 110_engagement.sql § 6 (schedule_session_reminders).
--
-- Trois arbitrages du commanditaire, sans nouvelle colonne :
--   - une activité gardée d'une journée n'a ni jauge, ni clôture, ni formulaire
--     propres : elle suit sa journée (le direct est commun) ;
--   - les rappels d'une activité gardée ne partent qu'à moins d'un jour (1 h et
--     30 min avec la règle par défaut) ; ceux de 2 jours et 1 jour sont ceux de
--     la journée, mis en file par le worker ;
--   - une activité ajoutée à une journée l'est d'office aux inscrits de la
--     journée ; retirée, leurs lignes rattachées sont annulées, sans courriel.
-- Aucune donnée touchée : aucune inscription à une journée n'existait.
BEGIN;

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

CREATE OR REPLACE FUNCTION engagement.schedule_session_reminders(p_session_id uuid)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    v_starts_at timestamptz;
    v_event_id  uuid;
    v_rule      engagement.reminder_rules%ROWTYPE;
    v_created   integer := 0;
BEGIN
    SELECT s.starts_at, s.event_id INTO v_starts_at, v_event_id
    FROM programme.sessions s WHERE s.id = p_session_id;

    IF v_starts_at IS NULL THEN
        RETURN 0;   -- session inconnue ou pas encore datée : rien à programmer
    END IF;

    -- Règle applicable : celle de la session si elle existe, sinon celle de
    -- l'événement. Pas de cumul, pour que l'admin sache ce qui va partir.
    SELECT * INTO v_rule FROM engagement.reminder_rules r
    WHERE r.is_active AND (r.session_id = p_session_id OR r.event_id = v_event_id)
    ORDER BY (r.session_id IS NOT NULL) DESC
    LIMIT 1;

    IF v_rule.id IS NULL THEN
        RETURN 0;
    END IF;

    WITH recipients AS (
        SELECT reg.id AS registration_id, reg.person_id,
               (reg.track_registration_id IS NOT NULL) AS kept
        FROM programme.registrations reg
        WHERE reg.session_id = p_session_id
          -- Comparaison textuelle volontaire : le module programme peut faire
          -- évoluer son ENUM de statut sans casser cette fonction.
          AND reg.status::text NOT IN ('cancelled', 'declined', 'waitlisted')
    ),
    planned AS (
        SELECT r.registration_id,
               r.person_id,
               c.channel,
               o.offset_before,
               v_starts_at - o.offset_before AS scheduled_for
        FROM recipients r
        CROSS JOIN unnest(v_rule.offsets)  AS o(offset_before)
        CROSS JOIN unnest(v_rule.channels) AS c(channel)
        -- Un rappel dont l'heure est déjà passée n'a plus de sens : on ne
        -- réveille personne à 3 h du matin parce qu'un import a pris du retard.
        WHERE v_starts_at - o.offset_before > now()
          AND (NOT r.kept OR o.offset_before < interval '1 day')
    )
    INSERT INTO engagement.scheduled_reminders (
        rule_id, session_id, person_id, registration_id, channel, offset_before, scheduled_for
    )
    SELECT v_rule.id, p_session_id, p.person_id, p.registration_id,
           p.channel, p.offset_before, p.scheduled_for
    FROM planned p
    ON CONFLICT ON CONSTRAINT ux_scheduled_reminders_once DO NOTHING;

    GET DIAGNOSTICS v_created = ROW_COUNT;

    -- Mise en file : le job porte l'identifiant du rappel comme clé
    -- d'idempotence, seconde barrière contre le double envoi (cf. ux_jobs_idempotency).
    WITH enqueued AS (
        INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at)
        SELECT 'email',
               'engagement.send_reminder',
               jsonb_build_object('reminder_id', sr.id, 'session_id', sr.session_id,
                                  'person_id', sr.person_id, 'channel', sr.channel),
               sr.id::text,
               sr.scheduled_for
        FROM engagement.scheduled_reminders sr
        WHERE sr.session_id = p_session_id
          AND sr.status = 'pending'
          AND sr.job_id IS NULL
        ON CONFLICT DO NOTHING
        RETURNING id AS job_id, (payload ->> 'reminder_id')::uuid AS reminder_id
    )
    UPDATE engagement.scheduled_reminders sr
    SET job_id = e.job_id, status = 'queued'
    FROM enqueued e
    WHERE sr.id = e.reminder_id;

    PERFORM platform.emit_event(
        'engagement', 'session_reminders', p_session_id, 'engagement.reminders.scheduled',
        jsonb_build_object('rule_id', v_rule.id, 'created', v_created)
    );

    RETURN v_created;
END;
$$;

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

DROP TRIGGER IF EXISTS tg_session_tracks_follow_registrations ON programme.session_tracks;
CREATE TRIGGER tg_session_tracks_follow_registrations
    AFTER INSERT OR UPDATE OF track_id, session_id OR DELETE ON programme.session_tracks
    FOR EACH ROW EXECUTE FUNCTION programme.tg_session_tracks_follow_registrations();

COMMIT;
