-- Réunions Zoom des activités du pavillon (17/10). Rejouable.
-- Modèle : docs/database/080_live.sql § 2 et § 3.
--
-- 1. live.meetings passée en `pending` dépose elle-même le travail de création
--    chez le fournisseur (`live.sync_meeting`), clé horodatée.
-- 2. live.meeting_participants.cancelled_at : l'inscription annulée se défait
--    chez le fournisseur, la ligne reste pour une réinscription.
-- 3. live.requeue_failed_participants() ne relance plus une inscription annulée.
BEGIN;

ALTER TABLE live.meeting_participants ADD COLUMN IF NOT EXISTS cancelled_at timestamptz;

COMMENT ON COLUMN live.meeting_participants.cancelled_at IS
    'Inscription annulée : l''enregistrement est annulé chez le fournisseur. Une réinscription remet la colonne à NULL et reprend la ligne (adresse unique par réunion).';

CREATE OR REPLACE FUNCTION live.tg_enqueue_meeting_sync()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, max_attempts)
    VALUES (
        'live',
        'live.sync_meeting',
        jsonb_build_object('meeting_id', NEW.id),
        NEW.id::text || ':' || to_char(clock_timestamp(), 'YYYYMMDDHH24MISSUS'),
        10
    )
    ON CONFLICT DO NOTHING;

    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_meetings_enqueue_sync ON live.meetings;
CREATE TRIGGER tg_meetings_enqueue_sync
    AFTER INSERT OR UPDATE OF sync_status ON live.meetings
    FOR EACH ROW
    WHEN (NEW.sync_status = 'pending')
    EXECUTE FUNCTION live.tg_enqueue_meeting_sync();

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
          AND cancelled_at IS NULL
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

COMMIT;
