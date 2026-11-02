-- =============================================================================
-- ePavillon v2 — 010_platform.sql
-- Noyau technique : journal d'audit, outbox transactionnel, file de travaux,
-- réglages, drapeaux de fonctionnalités, registre des modules.
--
-- Dépend de : 000_bootstrap.sql
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Registre des modules
--
-- Rend explicite, en base, la cartographie « module -> schéma -> état ».
-- Sert de source de vérité au démarrage de l'API : un module désactivé n'est pas
-- monté dans le routeur Actix, et un module marqué `external` bascule vers un
-- client HTTP au lieu de l'accès direct au schéma.
-- -----------------------------------------------------------------------------
CREATE TYPE platform.module_deployment AS ENUM (
    'embedded',   -- servi par le monolithe (défaut)
    'external',   -- extrait : appelé via HTTP/gRPC, schéma en lecture seule
    'disabled'    -- non monté
);

CREATE TABLE platform.modules (
    code              text PRIMARY KEY,
    schema_name       text        NOT NULL UNIQUE,
    display_name      platform.i18n_text NOT NULL,
    deployment        platform.module_deployment NOT NULL DEFAULT 'embedded',
    base_url          platform.url,
    depends_on        text[]      NOT NULL DEFAULT '{}',
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_modules_external_needs_url
        CHECK (deployment <> 'external' OR base_url IS NOT NULL)
);

CREATE TRIGGER tg_modules_updated_at
    BEFORE UPDATE ON platform.modules
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE platform.modules IS
    'Cartographie des modules et de leur mode de déploiement. Lue au démarrage de l''API pour monter les routes.';

-- -----------------------------------------------------------------------------
-- 2. Journal d'audit
--
-- Partitionné par mois : la table la plus volumineuse de la base ne doit jamais
-- pénaliser les écritures métier, et la purge réglementaire se fait par DROP
-- PARTITION plutôt que par DELETE massif.
--
-- L'audit ne porte aucune FK : il doit survivre à la suppression des entités
-- auditées et à l'extraction d'un module.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.audit_log (
    id             uuid        NOT NULL DEFAULT platform.uuid_v7(),
    occurred_at    timestamptz NOT NULL DEFAULT now(),
    actor_id       uuid,                       -- identity.people.id (sans FK, volontairement)
    actor_label    text,                       -- dénormalisé : reste lisible après anonymisation RGPD
    request_id     text,
    entity_schema  text        NOT NULL,
    entity_table   text        NOT NULL,
    entity_id      uuid,
    action         text        NOT NULL CHECK (action IN ('insert', 'update', 'delete')),
    changed_fields text[],
    old_data       jsonb,
    new_data       jsonb,
    PRIMARY KEY (occurred_at, id)
) PARTITION BY RANGE (occurred_at);

CREATE INDEX ix_audit_log_entity ON platform.audit_log (entity_schema, entity_table, entity_id, occurred_at DESC);
CREATE INDEX ix_audit_log_actor  ON platform.audit_log (actor_id, occurred_at DESC) WHERE actor_id IS NOT NULL;
-- Recherche « Code de la demande » du journal du back-office (02/11/2026) : tout
-- ce qu'une même requête a écrit, sans parcourir chaque partition.
CREATE INDEX ix_audit_log_request ON platform.audit_log (request_id) WHERE request_id IS NOT NULL;

COMMENT ON TABLE platform.audit_log IS
    'Journal d''audit partitionné par mois. Alimenté par platform.tg_audit(), sans clé étrangère pour rester indépendant.';

-- Partition « fourre-tout » : garantit qu'aucune écriture ne peut échouer si le
-- worker de maintenance a pris du retard.
CREATE TABLE platform.audit_log_default PARTITION OF platform.audit_log DEFAULT;

-- Création idempotente d'une partition mensuelle (appelée par le worker).
CREATE OR REPLACE FUNCTION platform.ensure_month_partition(
    p_parent_schema text,
    p_parent_table  text,
    p_month         date
)
RETURNS text
LANGUAGE plpgsql
AS $$
DECLARE
    v_start   date := date_trunc('month', p_month)::date;
    v_end     date := (date_trunc('month', p_month) + interval '1 month')::date;
    v_name    text := format('%s_%s', p_parent_table, to_char(v_start, 'YYYYMM'));
    v_default text;
    v_keycol  text;
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname = p_parent_schema AND c.relname = v_name
    ) THEN
        RETURN v_name;
    END IF;

    -- La partition « fourre-tout » peut déjà contenir des lignes du mois visé :
    -- PostgreSQL refuse alors la création de la partition définitive. On la
    -- détache le temps de créer la nouvelle partition, on y déplace les lignes
    -- concernées, puis on la rattache.
    SELECT c.relname INTO v_default
    FROM pg_class c
    JOIN pg_inherits i ON i.inhrelid = c.oid
    JOIN pg_class p    ON p.oid = i.inhparent
    JOIN pg_namespace n ON n.oid = p.relnamespace
    WHERE n.nspname = p_parent_schema
      AND p.relname = p_parent_table
      AND pg_get_expr(c.relpartbound, c.oid) = 'DEFAULT';

    IF v_default IS NULL THEN
        EXECUTE format(
            'CREATE TABLE %I.%I PARTITION OF %I.%I FOR VALUES FROM (%L) TO (%L)',
            p_parent_schema, v_name, p_parent_schema, p_parent_table, v_start, v_end
        );
        RETURN v_name;
    END IF;

    SELECT a.attname INTO v_keycol
    FROM pg_partitioned_table pt
    JOIN pg_class p     ON p.oid = pt.partrelid
    JOIN pg_namespace n ON n.oid = p.relnamespace
    JOIN pg_attribute a ON a.attrelid = p.oid AND a.attnum = pt.partattrs[0]
    WHERE n.nspname = p_parent_schema AND p.relname = p_parent_table;

    EXECUTE format('ALTER TABLE %I.%I DETACH PARTITION %I.%I',
                   p_parent_schema, p_parent_table, p_parent_schema, v_default);

    EXECUTE format(
        'CREATE TABLE %I.%I PARTITION OF %I.%I FOR VALUES FROM (%L) TO (%L)',
        p_parent_schema, v_name, p_parent_schema, p_parent_table, v_start, v_end
    );

    EXECUTE format(
        'WITH deplacees AS (DELETE FROM %I.%I WHERE %I >= %L AND %I < %L RETURNING *)
         INSERT INTO %I.%I SELECT * FROM deplacees',
        p_parent_schema, v_default, v_keycol, v_start, v_keycol, v_end,
        p_parent_schema, p_parent_table
    );

    EXECUTE format('ALTER TABLE %I.%I ATTACH PARTITION %I.%I DEFAULT',
                   p_parent_schema, p_parent_table, p_parent_schema, v_default);

    RETURN v_name;
END;
$$;

COMMENT ON FUNCTION platform.ensure_month_partition(text, text, date) IS
    'Crée si nécessaire la partition mensuelle d''une table partitionnée par RANGE sur un timestamptz.';

-- Trigger d'audit générique. À attacher explicitement, table par table, aux
-- seules entités dont la traçabilité a une valeur métier ou réglementaire :
--   CREATE TRIGGER tg_audit AFTER INSERT OR UPDATE OR DELETE ON <table>
--     FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE OR REPLACE FUNCTION platform.tg_audit()
RETURNS trigger
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = platform, pg_temp
AS $$
DECLARE
    v_old      jsonb;
    v_new      jsonb;
    v_changed  text[];
    v_entity   uuid;
    v_actor    uuid;
    v_label    text;
BEGIN
    IF TG_OP = 'INSERT' THEN
        v_new := to_jsonb(NEW);
    ELSIF TG_OP = 'UPDATE' THEN
        v_old := to_jsonb(OLD);
        v_new := to_jsonb(NEW);
        SELECT array_agg(key ORDER BY key) INTO v_changed
        FROM jsonb_each(v_new)
        WHERE v_new -> key IS DISTINCT FROM v_old -> key;
        -- Aucune modification effective : ne pas polluer le journal.
        IF v_changed IS NULL THEN
            RETURN NULL;
        END IF;
    ELSE
        v_old := to_jsonb(OLD);
    END IF;

    -- La clé de l'entité est `id`, sauf quand le déclencheur en nomme une autre :
    -- une table clé par la ligne qu'elle prolonge (document_renditions).
    v_entity := COALESCE(v_new ->> COALESCE(TG_ARGV[0], 'id'),
                         v_old ->> COALESCE(TG_ARGV[0], 'id'))::uuid;
    v_actor  := platform.current_actor_id();

    -- `actor_label` est dénormalisée pour rester lisible APRÈS anonymisation
    -- RGPD, et l'écriture est le seul instant où le nom existe encore : le lire
    -- plus tard par jointure rendrait « Utilisateur anonymisé » pour toutes les
    -- décisions passées d'une personne qui a exercé son droit à l'effacement.
    -- Le coût est une lecture par clé primaire sur une écriture déjà auditée.
    IF v_actor IS NOT NULL THEN
        SELECT p.display_name INTO v_label FROM identity.people p WHERE p.id = v_actor;
    END IF;

    INSERT INTO platform.audit_log (
        actor_id, actor_label, request_id, entity_schema, entity_table, entity_id,
        action, changed_fields, old_data, new_data
    )
    VALUES (
        v_actor,
        v_label,
        platform.current_request_id(),
        TG_TABLE_SCHEMA,
        TG_TABLE_NAME,
        v_entity,
        lower(TG_OP),
        v_changed,
        v_old,
        v_new
    );

    RETURN NULL; -- trigger AFTER : la valeur de retour est ignorée
END;
$$;

-- Historique lisible d'une entité : dépile `changed_fields` en une ligne par
-- champ modifié, avec ancienne et nouvelle valeur.
--
-- La v1 maintenait une table dédiée `activity_modifications` alimentée à la
-- main par le code applicatif : elle ne couvrait que les activités, et
-- seulement les écritures qui passaient par le bon chemin. Ici, l'historique
-- est un SOUS-PRODUIT de l'audit : toute table portant le trigger
-- `platform.tg_audit()` obtient gratuitement son historique complet, y compris
-- pour les corrections faites en console.
--
-- Le filtre par entité s'appuie sur ix_audit_log_entity ; la fenêtre temporelle
-- permet en plus l'élagage des partitions.
CREATE OR REPLACE FUNCTION platform.entity_history(
    p_schema         text,
    p_table          text,
    p_entity_id      uuid,
    p_exclude_fields text[] DEFAULT ARRAY['updated_at', 'search_vector', 'view_count'],
    p_since          timestamptz DEFAULT NULL
)
RETURNS TABLE (
    occurred_at timestamptz,
    actor_id    uuid,
    actor_label text,
    action      text,
    field       text,
    old_value   jsonb,
    new_value   jsonb
)
LANGUAGE sql
STABLE
AS $$
    SELECT a.occurred_at,
           a.actor_id,
           a.actor_label,
           a.action,
           f.field,
           a.old_data -> f.field,
           a.new_data -> f.field
    FROM platform.audit_log a
    LEFT JOIN LATERAL unnest(COALESCE(a.changed_fields, '{}')) AS f(field) ON true
    WHERE a.entity_schema = p_schema
      AND a.entity_table = p_table
      AND a.entity_id = p_entity_id
      AND (p_since IS NULL OR a.occurred_at >= p_since)
      AND (f.field IS NULL OR NOT (f.field = ANY (p_exclude_fields)))
    ORDER BY a.occurred_at DESC, f.field;
$$;

COMMENT ON FUNCTION platform.entity_history IS
    'Historique des modifications d''une entité, champ par champ. Remplace la table activity_modifications de la v1.';

-- -----------------------------------------------------------------------------
-- 2 bis. Masquage des valeurs sensibles de l'audit (02/11/2026)
--
-- Le journal du back-office (`/admin/journal`, onglet « Modifications ») montre
-- l'audit de toute la plateforme. Il nomme chaque champ modifié, mais ne rend
-- jamais la valeur d'un champ sensible : données personnelles de l'identité
-- (personnes, adresses, comptes, sessions, jetons, consentements) et, partout,
-- tout ce qui ressemble à un secret ou à une empreinte.
--
-- La règle vit ici, pas dans l'API : un second écran qui lirait l'audit sans
-- elle exposerait ce que le premier cache. Les tables de gouvernance de
-- l'identité (rôles, permissions, attributions) restent lisibles : savoir qui
-- a reçu quel rôle est précisément ce qu'on vient chercher dans l'audit.
-- -----------------------------------------------------------------------------
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

-- -----------------------------------------------------------------------------
-- 2 ter. Journal d'exploitation (02/11/2026)
--
-- Ce que l'audit ne voit pas : les demandes qui ÉCHOUENT ou TRAÎNENT. L'audit
-- naît d'un déclencheur sur une écriture validée ; une panne, un refus, une
-- page lente, un travail en échec n'écrivent rien, et jusqu'ici on ne les
-- trouvait que dans les journaux texte du serveur, sans écran pour les lire.
--
-- Alimentée HORS transaction métier, par lots : l'API pose ses lignes dans un
-- canal mémoire borné qu'une tâche vide toutes les deux secondes (canal plein :
-- ligne perdue et comptée) ; le worker y écrit ses travaux en échec. Une ligne
-- perdue vaut mieux qu'une requête ralentie ou refusée pour avoir été journalisée.
--
-- `request_id` (l'en-tête X-Request-Id) relie une ligne à l'audit
-- (`audit_log.request_id`) et à l'outbox (`outbox_events.correlation_id`) de la
-- même demande : c'est la recherche « Code de la demande » du back-office.
--
-- Ce qui n'y entre JAMAIS : corps de requête, chaîne de requête, en-têtes
-- Authorization et Cookie, mot de passe, adresse IP. `route` est le MODÈLE de
-- route (`/admin/proposals/{id}`), jamais le chemin reçu, qui peut porter un
-- jeton (`/auth/activate/<jeton>`) ; `*` quand aucune route ne correspond.
-- `detail` (2 Ko au plus) n'existe que pour une panne, sans adresse ni jeton.
--
-- Partitionnée par mois comme l'audit, sans clé étrangère, avec la même
-- partition « fourre-tout ». À la différence de l'audit, elle est PURGÉE : trois
-- mois suffisent à diagnostiquer, et elle grossit avec le trafic, pas avec
-- l'activité métier (platform.purge_ops_log, appelée par le worker).
-- -----------------------------------------------------------------------------
CREATE TABLE platform.ops_log (
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

CREATE INDEX ix_ops_log_request ON platform.ops_log (request_id) WHERE request_id IS NOT NULL;
CREATE INDEX ix_ops_log_kind    ON platform.ops_log (kind, occurred_at DESC);
CREATE INDEX ix_ops_log_actor   ON platform.ops_log (actor_id, occurred_at DESC) WHERE actor_id IS NOT NULL;

COMMENT ON TABLE platform.ops_log IS
    'Journal d''exploitation partitionné par mois : pannes, refus, pages lentes, travaux en échec. Écrit hors transaction métier, par lots ; purgé au-delà de trois mois.';
COMMENT ON COLUMN platform.ops_log.route IS
    'Modèle de route (`/admin/proposals/{id}`), jamais le chemin reçu ni la chaîne de requête ; `*` quand aucune route ne correspond.';
COMMENT ON COLUMN platform.ops_log.detail IS
    'Message de la panne ou dernière erreur du travail, tronqué à 2 Ko. Jamais de corps de requête, d''adresse électronique ni de jeton.';

CREATE TABLE platform.ops_log_default PARTITION OF platform.ops_log DEFAULT;

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

-- -----------------------------------------------------------------------------
-- 3. Outbox transactionnel
--
-- Pièce maîtresse du « microservice ready ». Un changement d'état métier et
-- l'événement qui l'annonce sont écrits dans la MÊME transaction : impossible
-- d'avoir un email envoyé pour une activité non validée, ou une validation sans
-- notification. Un relais lit les lignes non publiées (FOR UPDATE SKIP LOCKED)
-- et les remet aux consommateurs — aujourd'hui des handlers in-process, demain
-- un bus (NATS/Kafka) sans changer une ligne du code métier.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.outbox_events (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    aggregate_schema  text        NOT NULL,
    aggregate_type    text        NOT NULL,
    aggregate_id      uuid        NOT NULL,
    event_type        text        NOT NULL,     -- ex. 'programme.proposal.approved'
    event_version     smallint    NOT NULL DEFAULT 1,
    payload           jsonb       NOT NULL,
    metadata          jsonb       NOT NULL DEFAULT '{}'::jsonb,
    correlation_id    text,
    occurred_at       timestamptz NOT NULL DEFAULT now(),
    available_at      timestamptz NOT NULL DEFAULT now(),
    published_at      timestamptz,
    attempts          smallint    NOT NULL DEFAULT 0,
    last_error        text,
    CONSTRAINT ck_outbox_event_type_format CHECK (event_type ~ '^[a-z_]+\.[a-z_]+\.[a-z_]+$')
);

-- Index partiel : la file « à publier » reste minuscule même avec des millions
-- d'événements historisés.
CREATE INDEX ix_outbox_pending
    ON platform.outbox_events (available_at, id)
    WHERE published_at IS NULL;

CREATE INDEX ix_outbox_aggregate
    ON platform.outbox_events (aggregate_schema, aggregate_type, aggregate_id, occurred_at DESC);

-- Les événements d'une même demande, pour le journal du back-office (02/11/2026).
CREATE INDEX ix_outbox_correlation
    ON platform.outbox_events (correlation_id)
    WHERE correlation_id IS NOT NULL;

COMMENT ON TABLE platform.outbox_events IS
    'Outbox transactionnel : événements de domaine écrits avec le changement d''état. Relayés par le worker.';

-- Publication d'un événement depuis une fonction ou un trigger métier.
CREATE OR REPLACE FUNCTION platform.emit_event(
    p_aggregate_schema text,
    p_aggregate_type   text,
    p_aggregate_id     uuid,
    p_event_type       text,
    p_payload          jsonb DEFAULT '{}'::jsonb,
    p_metadata         jsonb DEFAULT '{}'::jsonb
)
RETURNS uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_id uuid;
BEGIN
    INSERT INTO platform.outbox_events (
        aggregate_schema, aggregate_type, aggregate_id, event_type,
        payload, metadata, correlation_id
    )
    VALUES (
        p_aggregate_schema, p_aggregate_type, p_aggregate_id, p_event_type,
        p_payload,
        p_metadata || jsonb_build_object('actor_id', platform.current_actor_id()),
        platform.current_request_id()
    )
    RETURNING id INTO v_id;

    -- Réveil immédiat du relais : évite d'attendre le prochain cycle de scrutation.
    PERFORM pg_notify('platform_outbox', v_id::text);
    RETURN v_id;
END;
$$;

-- Idempotence côté consommateur : un même événement rejoué (redémarrage du
-- worker, redistribution du bus) ne doit produire ses effets qu'une fois.
CREATE TABLE platform.inbox_events (
    consumer     text        NOT NULL,
    event_id     uuid        NOT NULL,
    processed_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (consumer, event_id)
);

COMMENT ON TABLE platform.inbox_events IS
    'Garde-fou d''idempotence : trace (consommateur, événement) déjà traité.';

-- -----------------------------------------------------------------------------
-- 3 bis. Signal d'écran (temps réel, 31/10/2026)
--
-- Annonce « tel sujet a changé » aux pages ouvertes, qui relisent alors la route
-- REST qu'elles appellent déjà. Le signal ne porte JAMAIS la donnée :
-- l'autorisation de la donnée reste celle de sa route.
--
-- Pourquoi pas l'outbox : l'outbox porte des faits métier, durables, rejoués à
-- des consommateurs idempotents ; un message interne au comité n'y émet rien, et
-- un signal par lot de dix courriels encombrerait une table faite pour être
-- relue. Le signal est éphémère : perdu si personne n'écoute, rattrapé par la
-- relecture complète que la page fait à chaque reconnexion.
--
-- `pg_notify` est transactionnel : livré AU COMMIT de la transaction appelante,
-- jamais pour une transaction annulée. On l'appelle donc dans la transaction du
-- changement, comme `emit_event()`. Deux appels identiques dans une transaction
-- n'en livrent qu'un (PostgreSQL dédoublonne), ce qui convient.
--
-- Écouté par l'API (`LISTEN platform_signal`, un concentrateur en mémoire) :
-- voir `backend/crates/api/src/realtime/` et
-- `docs/progression/decisions/2026-10-31-temps-reel.md`.
--
-- Sujet : nom en minuscules puis segments séparés par « : »
-- (`proposal:<uuid>`, `event:<uuid>:mailings`, `edition:<uuid>:public:live`).
-- Minuscules seulement : un uuid sort en minuscules de PostgreSQL comme de
-- Rust, et deux casses pour un sujet feraient deux sujets.
-- Charge utile : objet JSON facultatif, petit (`kind`, `audience`). L'API lit
-- `audience` pour ne jamais annoncer au déposant un échange interne au comité.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION platform.signal(p_topic text, p_payload jsonb DEFAULT NULL)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_message text;
BEGIN
    IF p_topic IS NULL
       OR length(p_topic) > 200
       OR p_topic !~ '^[a-z][a-z_]*(:[a-z0-9_-]+)+$' THEN
        RAISE EXCEPTION 'sujet de signal invalide : %', coalesce(p_topic, 'NULL')
            USING ERRCODE = 'check_violation';
    END IF;

    IF p_payload IS NOT NULL AND jsonb_typeof(p_payload) <> 'object' THEN
        RAISE EXCEPTION 'charge d''un signal : objet JSON attendu'
            USING ERRCODE = 'check_violation';
    END IF;

    v_message := jsonb_build_object('topic', p_topic, 'payload', p_payload)::text;

    -- `pg_notify` refuse au-delà de 8000 octets ; la marge laisse un message
    -- d'erreur lisible plutôt que celui du serveur.
    IF octet_length(v_message) > 7500 THEN
        RAISE EXCEPTION 'signal trop volumineux (% octets) : un signal annonce, il ne transporte pas',
            octet_length(v_message)
            USING ERRCODE = 'check_violation';
    END IF;

    PERFORM pg_notify('platform_signal', v_message);
END;
$$;

COMMENT ON FUNCTION platform.signal(text, jsonb) IS
    'Signal d''écran : annonce qu''un sujet a changé (pg_notify sur platform_signal, livré au commit). Ne porte jamais la donnée ; les pages relisent leur route REST.';

-- -----------------------------------------------------------------------------
-- 4. File de travaux
--
-- PostgreSQL sert de courtier en phase 1 (SKIP LOCKED) : pas de Redis, pas de
-- RabbitMQ à exploiter tant que la charge ne le justifie pas. La table porte
-- déjà tout ce qu'il faut pour migrer vers un vrai broker (clé d'unicité,
-- planification, backoff, file morte).
-- -----------------------------------------------------------------------------
CREATE TYPE platform.job_status AS ENUM ('queued', 'running', 'succeeded', 'failed', 'dead', 'cancelled');

CREATE TABLE platform.jobs (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    queue           text        NOT NULL DEFAULT 'default',
    task            text        NOT NULL,          -- ex. 'engagement.send_reminder'
    payload         jsonb       NOT NULL DEFAULT '{}'::jsonb,
    -- Clé d'unicité métier : empêche structurellement le double envoi d'un même
    -- rappel ou la double création d'une réunion Zoom.
    idempotency_key text,
    status          platform.job_status NOT NULL DEFAULT 'queued',
    priority        smallint    NOT NULL DEFAULT 100,
    run_at          timestamptz NOT NULL DEFAULT now(),
    attempts        smallint    NOT NULL DEFAULT 0,
    max_attempts    smallint    NOT NULL DEFAULT 5,
    locked_at       timestamptz,
    locked_by       text,
    last_error      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    completed_at    timestamptz
);

CREATE UNIQUE INDEX ux_jobs_idempotency
    ON platform.jobs (task, idempotency_key)
    WHERE idempotency_key IS NOT NULL AND status <> 'cancelled';

CREATE INDEX ix_jobs_ready
    ON platform.jobs (queue, priority, run_at, id)
    WHERE status = 'queued';

CREATE INDEX ix_jobs_stuck
    ON platform.jobs (locked_at)
    WHERE status = 'running';

COMMENT ON TABLE platform.jobs IS
    'File de travaux différés (rappels email, synchronisation Zoom, variantes d''images, rafraîchissement analytique).';

-- Réservation atomique d'un lot de travaux par un worker.
CREATE OR REPLACE FUNCTION platform.claim_jobs(
    p_queue     text,
    p_worker_id text,
    p_limit     int DEFAULT 10
)
RETURNS SETOF platform.jobs
LANGUAGE sql
AS $$
    WITH claimed AS (
        SELECT id
        FROM platform.jobs
        WHERE queue = p_queue
          AND status = 'queued'
          AND run_at <= now()
        ORDER BY priority, run_at, id
        LIMIT p_limit
        FOR UPDATE SKIP LOCKED
    )
    UPDATE platform.jobs j
    SET status    = 'running',
        locked_at = now(),
        locked_by = p_worker_id,
        attempts  = j.attempts + 1
    FROM claimed
    WHERE j.id = claimed.id
    RETURNING j.*;
$$;

-- Échec d'un travail : replanification avec backoff exponentiel plafonné,
-- puis passage en file morte au-delà du nombre d'essais autorisé.
CREATE OR REPLACE FUNCTION platform.fail_job(p_job_id uuid, p_error text)
RETURNS void
LANGUAGE sql
AS $$
    UPDATE platform.jobs
    SET status     = CASE WHEN attempts >= max_attempts THEN 'dead'::platform.job_status
                          ELSE 'queued'::platform.job_status END,
        run_at     = now() + (least(power(2, attempts), 3600) || ' seconds')::interval,
        last_error = p_error,
        locked_at  = NULL,
        locked_by  = NULL
    WHERE id = p_job_id;
$$;

-- Relance manuelle depuis le journal du back-office (02/11/2026) : un travail
-- mort ou en échec repart aussitôt pour UN essai de plus. On relève le plafond
-- plutôt que de remettre le compteur à zéro : le nombre d'essais et la dernière
-- erreur restent lisibles, et un nouvel échec le renvoie en file morte au lieu
-- de lui rendre cinq essais. Faux si le travail n'est ni mort ni en échec.
--
-- Un travail mort dont la charge a été effacée (kernel `jobs::redact_dead`)
-- échouera de nouveau : la relance ne reconstitue pas ce qui a été effacé.
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

-- -----------------------------------------------------------------------------
-- 5. Réglages et drapeaux de fonctionnalités
--
-- La v1 codait en dur les journées thématiques et les campagnes dans le routeur
-- Vue (`/programmations/2025/journee-jeunesse`). Ici, tout ce qui varie d'une
-- édition à l'autre se pilote en base, sans redéploiement.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.settings (
    key         text        PRIMARY KEY,
    value       jsonb       NOT NULL,
    description text,
    is_secret   boolean     NOT NULL DEFAULT false,
    updated_by  uuid,
    updated_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_settings_updated_at
    BEFORE UPDATE ON platform.settings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON COLUMN platform.settings.is_secret IS
    'Si vrai, la valeur n''est jamais exposée par l''API ; elle référence un secret du coffre (ex. identifiants Zoom).';

CREATE TABLE platform.feature_flags (
    key             text        PRIMARY KEY,
    description     text        NOT NULL,
    is_enabled      boolean     NOT NULL DEFAULT false,
    -- Déploiement progressif : pourcentage d'utilisateurs, ou liste explicite.
    rollout_percent smallint    NOT NULL DEFAULT 0 CHECK (rollout_percent BETWEEN 0 AND 100),
    enabled_for     uuid[]      NOT NULL DEFAULT '{}',
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_feature_flags_updated_at
    BEFORE UPDATE ON platform.feature_flags
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE OR REPLACE FUNCTION platform.is_feature_enabled(p_key text, p_person_id uuid DEFAULT NULL)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(
        (
            SELECT f.is_enabled
               AND (
                   f.rollout_percent = 100
                   OR (p_person_id IS NOT NULL AND p_person_id = ANY (f.enabled_for))
                   OR (p_person_id IS NOT NULL
                       AND ('x' || substr(md5(f.key || p_person_id::text), 1, 4))::bit(16)::int % 100 < f.rollout_percent)
               )
            FROM platform.feature_flags f
            WHERE f.key = p_key
        ),
        false
    );
$$;

-- -----------------------------------------------------------------------------
-- 6. Suivi des migrations de schéma
--
-- La v1 accumulait des fichiers SQL appliqués à la main, sans savoir lesquels
-- l'avaient été. Le migrateur (sqlx migrate / refinery côté Rust) écrit ici.
-- -----------------------------------------------------------------------------
CREATE TABLE platform.schema_migrations (
    version     text        PRIMARY KEY,
    description text        NOT NULL,
    checksum    text        NOT NULL,
    applied_at  timestamptz NOT NULL DEFAULT now(),
    applied_by  text        NOT NULL DEFAULT current_user,
    duration_ms integer
);

-- -----------------------------------------------------------------------------
-- 7. Amorçage du registre des modules
-- -----------------------------------------------------------------------------
INSERT INTO platform.modules (code, schema_name, display_name, depends_on) VALUES
    ('identity',    'identity',    '{"fr":"Identité","en":"Identity"}',            '{}'),
    ('org',         'org',         '{"fr":"Organisations","en":"Organizations"}',  '{identity}'),
    ('event',       'event',       '{"fr":"Événements","en":"Events"}',            '{org,identity}'),
    ('programme',   'programme',   '{"fr":"Programmation","en":"Programme"}',      '{event,org,identity}'),
    ('live',        'live',        '{"fr":"Direct","en":"Live"}',                  '{programme}'),
    ('publication', 'publication', '{"fr":"Publications","en":"Publications"}',    '{org,identity}'),
    ('negotiation', 'negotiation', '{"fr":"Négociations","en":"Negotiations"}',    '{identity}'),
    ('engagement',  'engagement',  '{"fr":"Engagement","en":"Engagement"}',        '{identity}'),
    ('media',       'media',       '{"fr":"Médiathèque","en":"Media"}',            '{identity}'),
    ('tool',        'tool',        '{"fr":"Outils","en":"Tools"}',                 '{identity}'),
    ('analytics',   'analytics',   '{"fr":"Analytique","en":"Analytics"}',         '{}')
ON CONFLICT (code) DO NOTHING;
