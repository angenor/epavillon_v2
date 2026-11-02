-- Journal du back-office (02/11/2026). Rejouable.
-- Modèle : docs/database/010_platform.sql § 2 (index de l'audit), § 2 bis
-- (masquage), § 2 ter (journal d'exploitation), § 3 (index de l'outbox), § 4
-- (relance d'un travail) ; 030_identity.sql § 6 (permission) ;
-- 130_analytics.sql (indicateur `partitions_manquantes`).
--
-- Additive, aucune donnée touchée :
--   - platform.ops_log, partitionnée par mois (mois courant et trois suivants
--     amorcés), sa partition par défaut et sa purge à trois mois ;
--   - platform.audit_is_sensitive / audit_redact : valeurs sensibles masquées ;
--   - platform.retry_job : relance d'un travail mort ou en échec ;
--   - index sur audit_log.request_id et outbox_events.correlation_id ;
--   - permission identity.platform.journal (admin ; super_admin par déclencheur) ;
--   - v_operational_health surveille aussi les partitions de ops_log.
--
-- L'index de l'audit se pose sur chaque partition sous verrou d'écriture
-- (CONCURRENTLY est refusé sur une table partitionnée) : quelques secondes
-- tout au plus à la taille actuelle, à jouer hors d'un moment chargé.
-- Jouer AVANT de démarrer l'API et le worker, qui écrivent dans ops_log.
BEGIN;

CREATE INDEX IF NOT EXISTS ix_audit_log_request ON platform.audit_log (request_id) WHERE request_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS ix_outbox_correlation
    ON platform.outbox_events (correlation_id)
    WHERE correlation_id IS NOT NULL;

CREATE OR REPLACE FUNCTION platform.audit_is_sensitive(p_schema text, p_table text, p_field text)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT (p_schema = 'identity'
            AND p_table NOT IN ('roles', 'permissions', 'role_permissions', 'role_assignments'))
        OR p_field ~ '(^|_)(password|secret|token|otp|mfa)(_|$)'
        OR p_field ~ '_hash$';
$$;

COMMENT ON FUNCTION platform.audit_is_sensitive(text, text, text) IS
    'Vrai si la valeur de ce champ ne doit jamais sortir de l''audit : identité personnelle, secret, jeton, empreinte. Le nom du champ, lui, reste visible.';

-- Retire les clés sensibles d'un old_data / new_data. Le champ reste nommé dans
-- `changed_fields` : l'écran affiche « valeur masquée » pour un champ modifié
-- absent de l'objet rendu.
CREATE OR REPLACE FUNCTION platform.audit_redact(p_schema text, p_table text, p_data jsonb)
RETURNS jsonb
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE
        WHEN p_data IS NULL OR jsonb_typeof(p_data) <> 'object' THEN p_data
        ELSE COALESCE(
            (SELECT jsonb_object_agg(e.key, e.value)
             FROM jsonb_each(p_data) e
             WHERE NOT platform.audit_is_sensitive(p_schema, p_table, e.key)),
            '{}'::jsonb)
    END;
$$;

COMMENT ON FUNCTION platform.audit_redact(text, text, jsonb) IS
    'old_data ou new_data de l''audit sans les clés sensibles (platform.audit_is_sensitive). Toute lecture de l''audit hors historique d''entité passe par elle.';

CREATE TABLE IF NOT EXISTS platform.ops_log (
    id           uuid        NOT NULL DEFAULT platform.uuid_v7(),
    occurred_at  timestamptz NOT NULL DEFAULT now(),
    source       text        NOT NULL,
    kind         text        NOT NULL,
    level        text        NOT NULL,
    request_id   text,
    actor_id     uuid,                       -- identity.people.id (sans FK, volontairement)
    method       text,
    route        text,
    status       smallint,
    duration_ms  integer,
    error_code   text,                       -- code stable de l'erreur d'API (`proposal.not_found`…)
    task         text,                       -- travail en échec : platform.jobs.task
    job_id       uuid,                       -- platform.jobs.id, sans FK : la file est purgeable
    detail       text,
    PRIMARY KEY (occurred_at, id),
    CONSTRAINT ck_ops_log_source CHECK (source IN ('api', 'worker')),
    -- http_error : réponse 5xx ; http_refused : 4xx retenu (401, 403, 429, 404
    -- d'une personne connectée, 409 et 422 du back-office) ; http_slow : réponse
    -- réussie de plus d'une seconde ; job_failed : essai d'un travail en échec.
    CONSTRAINT ck_ops_log_kind CHECK (kind IN ('http_error', 'http_refused', 'http_slow', 'job_failed')),
    CONSTRAINT ck_ops_log_level CHECK (level IN ('error', 'warn', 'info')),
    CONSTRAINT ck_ops_log_shape CHECK (
        (kind = 'job_failed'
            AND task IS NOT NULL AND method IS NULL AND route IS NULL AND status IS NULL)
        OR (kind <> 'job_failed'
            AND method IS NOT NULL AND route IS NOT NULL AND status IS NOT NULL
            AND task IS NULL AND job_id IS NULL)
    ),
    CONSTRAINT ck_ops_log_status CHECK (
        status IS NULL
        OR (status BETWEEN 100 AND 599
            AND (kind <> 'http_error' OR status >= 500)
            AND (kind <> 'http_refused' OR status BETWEEN 400 AND 499))
    ),
    CONSTRAINT ck_ops_log_route CHECK (
        route IS NULL OR route = '*'
        OR (left(route, 1) = '/' AND strpos(route, '?') = 0 AND length(route) <= 300)
    ),
    CONSTRAINT ck_ops_log_duration CHECK (duration_ms IS NULL OR duration_ms >= 0),
    CONSTRAINT ck_ops_log_detail CHECK (
        detail IS NULL OR (octet_length(detail) <= 2048 AND kind IN ('http_error', 'job_failed'))
    ),
    CONSTRAINT ck_ops_log_lengths CHECK (
        length(request_id) <= 128 AND length(method) <= 10
        AND length(error_code) <= 100 AND length(task) <= 200
    )
) PARTITION BY RANGE (occurred_at);

CREATE INDEX IF NOT EXISTS ix_ops_log_request ON platform.ops_log (request_id) WHERE request_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_ops_log_kind    ON platform.ops_log (kind, occurred_at DESC);
CREATE INDEX IF NOT EXISTS ix_ops_log_actor   ON platform.ops_log (actor_id, occurred_at DESC) WHERE actor_id IS NOT NULL;

COMMENT ON TABLE platform.ops_log IS
    'Journal d''exploitation partitionné par mois : pannes, refus, pages lentes, travaux en échec. Écrit hors transaction métier, par lots ; purgé au-delà de trois mois.';
COMMENT ON COLUMN platform.ops_log.route IS
    'Modèle de route (`/admin/proposals/{id}`), jamais le chemin reçu ni la chaîne de requête ; `*` quand aucune route ne correspond.';
COMMENT ON COLUMN platform.ops_log.detail IS
    'Message de la panne ou dernière erreur du travail, tronqué à 2 Ko. Jamais de corps de requête, d''adresse électronique ni de jeton.';

CREATE TABLE IF NOT EXISTS platform.ops_log_default PARTITION OF platform.ops_log DEFAULT;

-- Rétention : supprime les partitions mensuelles entièrement antérieures au
-- début du mois courant moins `p_keep_months` mois, et les lignes de même âge
-- égarées dans la partition par défaut. Avec 3, le 2 novembre garde août,
-- septembre, octobre et novembre : toujours au moins trois mois pleins.
--
-- Propre à ops_log, volontairement : une purge générique par nom de table
-- n'est qu'à une faute de frappe d'une purge de l'audit, dont la rétention
-- n'est pas décidée.
CREATE OR REPLACE FUNCTION platform.purge_ops_log(p_keep_months int DEFAULT 3)
RETURNS text[]
LANGUAGE plpgsql
AS $$
DECLARE
    v_cutoff  timestamptz;
    v_dropped text[] := '{}';
    r         record;
BEGIN
    IF p_keep_months IS NULL OR p_keep_months < 1 THEN
        RAISE EXCEPTION 'rétention du journal d''exploitation : au moins un mois (reçu %)', p_keep_months
            USING ERRCODE = 'check_violation';
    END IF;

    v_cutoff := date_trunc('month', now()) - make_interval(months => p_keep_months);

    FOR r IN
        SELECT c.relname::text AS relname,
               to_date(right(c.relname, 6), 'YYYYMM') AS debut
        FROM pg_inherits i
        JOIN pg_class c     ON c.oid = i.inhrelid
        JOIN pg_class p     ON p.oid = i.inhparent
        JOIN pg_namespace n ON n.oid = p.relnamespace
        WHERE n.nspname = 'platform'
          AND p.relname = 'ops_log'
          AND c.relname ~ '^ops_log_[0-9]{6}$'
        ORDER BY c.relname
    LOOP
        IF r.debut + interval '1 month' <= v_cutoff THEN
            EXECUTE format('DROP TABLE platform.%I', r.relname);
            v_dropped := v_dropped || r.relname;
        END IF;
    END LOOP;

    DELETE FROM platform.ops_log_default WHERE occurred_at < v_cutoff;

    RETURN v_dropped;
END;
$$;

COMMENT ON FUNCTION platform.purge_ops_log(int) IS
    'Rétention du journal d''exploitation : supprime les partitions mensuelles de plus de p_keep_months mois (3 par défaut). Renvoie les partitions supprimées.';

-- Amorce des partitions du trimestre ; le worker prépare ensuite les mois à
-- venir (cf. v_operational_health, indicateur `partitions_manquantes`).
DO $$
DECLARE
    v_month date;
BEGIN
    FOR v_month IN
        SELECT generate_series(date_trunc('month', now()),
                               date_trunc('month', now()) + interval '3 months',
                               interval '1 month')::date
    LOOP
        PERFORM platform.ensure_month_partition('platform', 'ops_log', v_month);
    END LOOP;
END
$$;

CREATE OR REPLACE FUNCTION platform.retry_job(p_job_id uuid)
RETURNS boolean
LANGUAGE sql
AS $$
    WITH relance AS (
        UPDATE platform.jobs
        SET status       = 'queued',
            run_at       = now(),
            max_attempts = GREATEST(max_attempts, attempts + 1),
            locked_at    = NULL,
            locked_by    = NULL,
            completed_at = NULL
        WHERE id = p_job_id
          AND status IN ('failed', 'dead')
        RETURNING 1
    )
    SELECT EXISTS (SELECT 1 FROM relance);
$$;

COMMENT ON FUNCTION platform.retry_job(uuid) IS
    'Relance d''un travail mort ou en échec : un essai de plus, tout de suite, sans effacer le compteur ni la dernière erreur.';

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('identity.platform.journal', '{"fr":"Consulter le journal d''exploitation","en":"Read the operations log"}', 'identity')
ON CONFLICT (code) DO NOTHING;
INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin', 'identity.platform.journal')
ON CONFLICT DO NOTHING;

CREATE OR REPLACE VIEW analytics.v_operational_health AS
WITH indicateurs AS (
    -- Outbox : profondeur de la file non publiée.
    SELECT
        'outbox_non_publie'::text                                   AS code,
        'Événements de domaine non publiés'::text                   AS libelle,
        'platform'::text                                            AS domaine,
        count(*)::bigint                                            AS valeur,
        100::bigint                                                 AS seuil_attention,
        1000::bigint                                                AS seuil_critique,
        jsonb_build_object(
            'plus_ancien', min(o.occurred_at),
            'tentatives_max', max(o.attempts)
        )                                                           AS detail
    FROM platform.outbox_events o
    WHERE o.published_at IS NULL

    UNION ALL

    -- Outbox bloqué : des événements qui échouent en boucle. Un seul suffit à
    -- justifier un examen, c'est le symptôme d'un consommateur cassé.
    SELECT
        'outbox_en_echec', 'Événements d''outbox en échec répété', 'platform',
        count(*)::bigint, 1::bigint, 10::bigint,
        jsonb_build_object('derniere_erreur', max(o.last_error))
    FROM platform.outbox_events o
    WHERE o.published_at IS NULL AND o.attempts >= 3

    UNION ALL

    -- File morte : travaux abandonnés après épuisement des tentatives.
    SELECT
        'travaux_file_morte', 'Travaux en file morte', 'platform',
        count(*)::bigint, 1::bigint, 25::bigint,
        jsonb_build_object(
            'taches', COALESCE(jsonb_object_agg(j.task, j.n), '{}'::jsonb)
        )
    FROM (SELECT task, count(*) AS n FROM platform.jobs WHERE status = 'dead' GROUP BY task) j

    UNION ALL

    -- Travaux en retard d'exécution : la file se creuse plus vite qu'elle ne se vide.
    SELECT
        'travaux_en_retard', 'Travaux échus non pris en charge', 'platform',
        count(*)::bigint, 50::bigint, 500::bigint,
        jsonb_build_object('plus_ancien', min(j.run_at))
    FROM platform.jobs j
    WHERE j.status = 'queued' AND j.run_at <= now() - interval '5 minutes'

    UNION ALL

    -- Travaux réservés puis jamais relâchés : un worker tué en cours de route.
    SELECT
        'travaux_bloques', 'Travaux verrouillés depuis plus de 15 minutes', 'platform',
        count(*)::bigint, 1::bigint, 10::bigint,
        jsonb_build_object('plus_ancien', min(j.locked_at), 'workers', array_agg(DISTINCT j.locked_by))
    FROM platform.jobs j
    WHERE j.status = 'running' AND j.locked_at < now() - interval '15 minutes'

    UNION ALL

    -- Rappels en retard : un rappel « J-1 » envoyé après l'activité est pire
    -- qu'un rappel non envoyé.
    SELECT
        'rappels_en_retard', 'Rappels programmés non envoyés à l''heure', 'engagement',
        count(*)::bigint, 1::bigint, 50::bigint,
        jsonb_build_object('plus_ancien', min(sr.scheduled_for))
    FROM engagement.scheduled_reminders sr
    WHERE sr.status = 'pending' AND sr.scheduled_for < now() - interval '5 minutes'

    UNION ALL

    -- Délivrabilité : rebonds et plaintes sur sept jours. Au-delà, la réputation
    -- du domaine expéditeur se dégrade pour TOUS les envois, confirmations
    -- d'inscription comprises.
    SELECT
        'emails_rebond_7j', 'Courriels en rebond ou signalés (7 jours)', 'engagement',
        count(*)::bigint, 20::bigint, 100::bigint,
        jsonb_build_object(
            'rebonds_durs', count(*) FILTER (WHERE em.bounce_kind = 'hard'),
            'plaintes',     count(*) FILTER (WHERE em.status = 'complained')
        )
    FROM engagement.email_messages em
    WHERE em.created_at >= now() - interval '7 days'
      AND em.status IN ('bounced', 'complained')

    UNION ALL

    SELECT
        'emails_en_echec', 'Courriels en échec technique (7 jours)', 'engagement',
        count(*)::bigint, 5::bigint, 50::bigint,
        jsonb_build_object('derniere_erreur', max(em.last_error))
    FROM engagement.email_messages em
    WHERE em.created_at >= now() - interval '7 days' AND em.status = 'failed'

    UNION ALL

    SELECT
        'emails_en_attente', 'Courriels en file depuis plus de 15 minutes', 'engagement',
        count(*)::bigint, 20::bigint, 200::bigint,
        jsonb_build_object('plus_ancien', min(em.created_at))
    FROM engagement.email_messages em
    WHERE em.status = 'queued' AND em.created_at < now() - interval '15 minutes'

    UNION ALL

    -- Visioconférence : réunions non poussées chez le fournisseur.
    SELECT
        'visio_reunions_desynchronisees', 'Réunions non synchronisées chez le fournisseur', 'live',
        count(*)::bigint, 1::bigint, 5::bigint,
        jsonb_build_object('derniere_erreur', max(m.last_sync_error))
    FROM live.meetings m
    WHERE m.sync_status IN ('failed', 'abandoned')

    UNION ALL

    -- Inscriptions non enregistrées chez le fournisseur : l'inscrit est acquis
    -- côté plateforme mais n'a pas reçu son lien personnel.
    SELECT
        'visio_inscriptions_desynchronisees', 'Inscriptions visio à rattraper', 'live',
        count(*)::bigint, 5::bigint, 50::bigint,
        jsonb_build_object(
            'abandonnees', count(*) FILTER (WHERE mp.sync_status = 'abandoned'),
            'derniere_erreur', max(mp.last_error)
        )
    FROM live.meeting_participants mp
    WHERE mp.sync_status IN ('failed', 'abandoned') AND mp.recovered_at IS NULL

    UNION ALL

    SELECT
        'visio_webhooks_en_echec', 'Webhooks fournisseur non traités', 'live',
        count(*)::bigint, 5::bigint, 50::bigint,
        jsonb_build_object('plus_ancien', min(w.received_at))
    FROM live.provider_webhook_events w
    WHERE w.status = 'failed'

    UNION ALL

    -- Partitions du mois prochain : oubli silencieux jusqu'au 1er du mois, où
    -- toutes les écritures tombent dans la partition par défaut et la font
    -- gonfler. Une seule partition manquante justifie une intervention.
    SELECT
        'partitions_manquantes', 'Partitions du mois prochain non créées', 'platform',
        count(*)::bigint, 1::bigint, 1::bigint,
        jsonb_build_object('tables', COALESCE(array_agg(t.schema_name || '.' || t.table_name), '{}'))
    FROM (VALUES
            ('platform',    'audit_log'),
            ('platform',    'ops_log'),
            ('engagement',  'email_messages'),
            ('negotiation', 'channel_messages'),
            ('analytics',   'page_views')
         ) AS t(schema_name, table_name)
    WHERE NOT EXISTS (
        SELECT 1
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = t.schema_name
          AND c.relname = t.table_name || '_' || to_char(now() + interval '1 month', 'YYYYMM')
    )

    UNION ALL

    -- Fraîcheur des tableaux de bord : âge, en minutes, du dernier
    -- rafraîchissement complet réussi.
    SELECT
        'analytique_perimee', 'Minutes depuis le dernier rafraîchissement analytique', 'analytics',
        COALESCE(
            (EXTRACT(EPOCH FROM (now() - max(rl.finished_at))) / 60)::bigint,
            99999::bigint
        ),
        120::bigint, 1440::bigint,
        jsonb_build_object('dernier_succes', max(rl.finished_at))
    FROM analytics.refresh_log rl
    WHERE rl.succeeded
)
SELECT
    i.code,
    i.libelle,
    i.domaine,
    i.valeur,
    i.seuil_attention,
    i.seuil_critique,
    CASE
        WHEN i.valeur >= i.seuil_critique  THEN 'critique'
        WHEN i.valeur >= i.seuil_attention THEN 'attention'
        ELSE 'ok'
    END AS gravite,
    i.detail,
    now() AS mesure_le
FROM indicateurs i
ORDER BY
    CASE
        WHEN i.valeur >= i.seuil_critique  THEN 0
        WHEN i.valeur >= i.seuil_attention THEN 1
        ELSE 2
    END,
    i.code;

COMMENT ON VIEW analytics.v_operational_health IS
    'Santé opérationnelle temps réel : outbox, file de travaux, rappels, synchronisation visio, délivrabilité, partitions et fraîcheur analytique. Une ligne par indicateur, avec ses seuils.';

COMMIT;
