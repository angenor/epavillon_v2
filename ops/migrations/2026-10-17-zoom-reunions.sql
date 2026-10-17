-- Réunions Zoom des activités du pavillon (17/10). Rejouable.
-- Modèle : docs/database/080_live.sql § 2, § 3 et § 4.
--
-- 1. live.meetings passée en `pending` dépose elle-même le travail de création
--    chez le fournisseur (`live.sync_meeting`), clé horodatée.
-- 2. live.meeting_participants.cancelled_at : l'inscription annulée se défait
--    chez le fournisseur, la ligne reste pour une réinscription.
-- 3. live.meeting_participants.max_sync_attempts passe à dix par défaut.
-- 4. live.requeue_failed_participants() : ne relance plus une inscription
--    annulée, et prend un troisième paramètre (filet de sécurité périodique).
-- 5. ux_webhook_events_payload_hash ne retient que les notifications signées.
BEGIN;

ALTER TABLE live.meeting_participants ADD COLUMN IF NOT EXISTS cancelled_at timestamptz;

COMMENT ON COLUMN live.meeting_participants.cancelled_at IS
    'Inscription annulée : l''enregistrement est annulé chez le fournisseur. Une réinscription remet la colonne à NULL et reprend la ligne (adresse unique par réunion).';

ALTER TABLE live.meeting_participants ALTER COLUMN max_sync_attempts SET DEFAULT 10;
UPDATE live.meeting_participants SET max_sync_attempts = 10 WHERE max_sync_attempts = 5;

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

DROP FUNCTION IF EXISTS live.requeue_failed_participants(uuid, integer);
CREATE OR REPLACE FUNCTION live.requeue_failed_participants(
    p_meeting_id        uuid    DEFAULT NULL,
    p_limit             integer DEFAULT 500,
    -- Faux : filet de sécurité périodique (17/10). Seules les lignes `failed`
    -- dont l'échéance est passée repartent ; un refus définitif de Zoom
    -- (`abandoned`) n'est pas relancé toutes les quinze minutes.
    p_include_abandoned boolean DEFAULT true
)
RETURNS integer
LANGUAGE plpgsql
AS $$
DECLARE
    v_count integer;
    -- Marqueur de campagne de rattrapage. platform.jobs conserve la clé
    -- d'idempotence des travaux déjà terminés : réutiliser l'identifiant du
    -- participant empêcherait toute relance ultérieure. À la microseconde :
    -- deux relances dans la même seconde ne se marchent pas dessus.
    v_batch text := to_char(clock_timestamp(), 'YYYYMMDDHH24MISSUS');
BEGIN
    WITH target AS (
        SELECT id FROM live.meeting_participants
        WHERE cancelled_at IS NULL
          AND (p_meeting_id IS NULL OR meeting_id = p_meeting_id)
          AND (sync_status = 'failed' AND (p_include_abandoned OR next_retry_at IS NULL OR next_retry_at <= now())
               OR sync_status = 'abandoned' AND p_include_abandoned)
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

DROP INDEX IF EXISTS live.ux_webhook_events_payload_hash;
CREATE UNIQUE INDEX ux_webhook_events_payload_hash
    ON live.provider_webhook_events (provider, event_type, payload_hash)
    WHERE provider_event_id IS NULL AND payload_hash IS NOT NULL AND replayed_from_id IS NULL
      AND signature_verified;

COMMENT ON COLUMN live.provider_webhook_events.signature_verified IS
    'Résultat de la vérification de signature (en-tête x-zm-signature pour Zoom). Un webhook non vérifié est journalisé sans son contenu (type et horodatage seulement) et jamais appliqué.';

COMMIT;
