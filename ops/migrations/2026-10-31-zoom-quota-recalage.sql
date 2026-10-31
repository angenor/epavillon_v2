-- Quota de Zoom : une réunion attend l'heure de reprise, un déplacement est
-- regroupé cinq minutes ; les notifications ignorées perdent leur contenu
-- (31/10). Rejouable.
-- Modèle : docs/database/080_live.sql (live.meetings.sync_due_at,
-- live.tg_enqueue_meeting_sync, tg_meetings_enqueue_sync,
-- COMMENT de live.provider_webhook_events.payload).
--
-- Additive pour les réunions : elles restent sans heure prévue et partent tout
-- de suite, comme avant. Seule donnée touchée : le contenu des notifications
-- déjà closes en `ignored`, vidé (type, numéro, heure, statut et empreinte
-- restent). À jouer en production AVANT de démarrer l'API et le worker qui
-- lisent `sync_due_at`.
BEGIN;

ALTER TABLE live.meetings ADD COLUMN IF NOT EXISTS sync_due_at timestamptz;

COMMENT ON COLUMN live.meetings.sync_due_at IS
    'Heure à laquelle la synchronisation part chez Zoom ; NULL : tout de suite. Zoom limite à 100 créations ou modifications de réunion par jour et par hôte (remise à zéro à 00:00 UTC). Un déplacement d''une réunion déjà créée la pose à cinq minutes, sans la repousser si elle est déjà dans le futur : déplacer plusieurs fois de suite ne compte qu''une modification. Un refus de quota avec Retry-After la pose à l''heure dite (ligne en failed), sans consommer d''essai. Remise à NULL par une synchronisation réussie, un échec ordinaire, la relance et le bouton « Envoyer à Zoom maintenant » du back-office.';

CREATE OR REPLACE FUNCTION live.tg_enqueue_meeting_sync()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_prevue boolean := COALESCE(NEW.sync_due_at > now(), false);
BEGIN
    IF NEW.sync_status = 'failed' AND NOT v_prevue THEN
        RETURN NULL;
    END IF;

    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, run_at, max_attempts)
    VALUES (
        'live',
        'live.sync_meeting',
        jsonb_build_object('meeting_id', NEW.id),
        CASE WHEN v_prevue
             THEN NEW.id::text || ':prevue:'
                  || to_char(NEW.sync_due_at AT TIME ZONE 'UTC', 'YYYYMMDDHH24MISSUS')
             ELSE NEW.id::text || ':' || to_char(clock_timestamp(), 'YYYYMMDDHH24MISSUS')
        END,
        CASE WHEN v_prevue THEN NEW.sync_due_at ELSE now() END,
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
    WHEN (NEW.sync_status IN ('pending', 'failed'))
    EXECUTE FUNCTION live.tg_enqueue_meeting_sync();

COMMENT ON COLUMN live.provider_webhook_events.payload IS
    'Charge utile reçue, telle quelle. Vidée (''{}'') quand la notification est close en ignored (31/10) : l''abonnement Zoom couvre tout le compte de l''IFDD, et une réunion étrangère à l''ePavillon n''a pas à laisser ici les noms et adresses de ses participants. Le type, external_id, l''heure de réception, le statut et payload_hash restent.';

UPDATE live.provider_webhook_events
   SET payload = '{}'::jsonb
 WHERE status = 'ignored' AND payload <> '{}'::jsonb;

COMMIT;
