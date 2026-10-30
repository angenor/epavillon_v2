-- Envoi de courriels en échec (31/10). Rejouable.
-- Modèle : docs/database/110_engagement.sql § 12 (valeur `failed` de
-- engagement.mailing_status, colonnes failed_at, failure_reason,
-- relaunch_count, relaunched_at, engagement.fail_mailing(),
-- engagement.fail_stalled_mailings()).
--
-- Corrige l'envoi resté « en cours » pour toujours quand le serveur de courriel
-- est injoignable une minute (travail d'envoi mort, envoi `queued`). En fin de
-- migration, les envois DÉJÀ bloqués (en cours, sans travail vivant) passent en
-- échec avec leur motif : l'équipe peut les relancer depuis le back-office.
-- Un envoi qui progresse a un travail en file ou en cours : il n'est pas touché.
--
-- À jouer avant de démarrer l'API et le worker qui écrivent `failed`.

-- Hors transaction : une valeur ajoutée à une énumération ne s'emploie pas
-- dans la transaction qui l'ajoute.
ALTER TYPE engagement.mailing_status ADD VALUE IF NOT EXISTS 'failed';

BEGIN;

ALTER TABLE engagement.mailings
    ADD COLUMN IF NOT EXISTS failed_at      timestamptz,
    ADD COLUMN IF NOT EXISTS failure_reason text,
    ADD COLUMN IF NOT EXISTS relaunch_count smallint NOT NULL DEFAULT 0,
    ADD COLUMN IF NOT EXISTS relaunched_at  timestamptz;

ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS ck_mailings_failed;
ALTER TABLE engagement.mailings ADD CONSTRAINT ck_mailings_failed CHECK (
    status <> 'failed' OR (failed_at IS NOT NULL AND length(btrim(failure_reason)) > 0));
ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS ck_mailings_relaunch;
ALTER TABLE engagement.mailings ADD CONSTRAINT ck_mailings_relaunch
    CHECK ((relaunch_count = 0) = (relaunched_at IS NULL));

CREATE OR REPLACE FUNCTION engagement.fail_mailing(p_mailing_id uuid, p_reason text)
RETURNS boolean
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE engagement.mailings
       SET status = 'failed', failed_at = now(), failure_reason = p_reason
     WHERE id = p_mailing_id AND status = 'queued';
    IF NOT FOUND THEN
        RETURN false;
    END IF;
    UPDATE engagement.mailing_recipients
       SET status = 'failed', last_error = 'envoi interrompu : départ non confirmé'
     WHERE mailing_id = p_mailing_id AND status = 'sending';
    RETURN true;
END;
$$;

COMMENT ON FUNCTION engagement.fail_mailing IS
    'Passe un envoi en cours à l''échec avec son motif ; les destinataires en file attendent une relance.';

CREATE OR REPLACE FUNCTION engagement.fail_stalled_mailings(p_event_id uuid DEFAULT NULL)
RETURNS SETOF uuid
LANGUAGE plpgsql
AS $$
DECLARE
    r record;
BEGIN
    FOR r IN
        SELECT m.id,
               (SELECT j.last_error FROM platform.jobs j
                 WHERE j.task = 'engagement.send_mailing'
                   AND j.idempotency_key LIKE 'mailing:' || m.id || ':%'
                   AND j.status = 'dead'
                 ORDER BY j.created_at DESC
                 LIMIT 1) AS derniere_erreur
          FROM engagement.mailings m
         WHERE m.status = 'queued'
           AND (p_event_id IS NULL OR m.event_id = p_event_id)
           AND NOT EXISTS (SELECT 1 FROM platform.jobs j
                            WHERE j.task = 'engagement.send_mailing'
                              AND j.idempotency_key LIKE 'mailing:' || m.id || ':%'
                              AND j.status IN ('queued', 'running'))
           FOR UPDATE OF m SKIP LOCKED
    LOOP
        IF EXISTS (SELECT 1 FROM engagement.mailing_recipients q
                    WHERE q.mailing_id = r.id AND q.status IN ('queued', 'sending')) THEN
            PERFORM engagement.fail_mailing(r.id, CASE
                WHEN r.derniere_erreur ILIKE '%injoignable%' THEN
                    'Le serveur de courriel est resté injoignable : l''envoi s''est arrêté après plusieurs tentatives.'
                WHEN r.derniere_erreur IS NOT NULL THEN
                    'L''envoi s''est interrompu et ne reprenait plus. Dernière erreur : '
                    || left(r.derniere_erreur, 300)
                ELSE 'L''envoi s''est interrompu et ne reprenait plus.' END);
        ELSE
            UPDATE engagement.mailings SET status = 'sent', completed_at = now()
             WHERE id = r.id AND status = 'queued';
        END IF;
        RETURN NEXT r.id;
    END LOOP;
END;
$$;

COMMENT ON FUNCTION engagement.fail_stalled_mailings IS
    'Filet : passe à l''échec (ou clôt) les envois en cours qu''aucun travail vivant ne sert plus. Rend les envois touchés.';

-- Les envois déjà bloqués : leur nombre s'affiche à l'exécution.
SELECT count(*) AS envois_debloques FROM engagement.fail_stalled_mailings();

COMMIT;
