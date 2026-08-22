-- =============================================================================
-- ePavillon v2 — 110_engagement.sql
-- Module Engagement : notifications in-app, emails transactionnels et leur
-- délivrabilité, rappels programmés, commentaires et réactions, messagerie
-- directe, mise en relation, infolettres.
--
-- Dépend de : 000_bootstrap.sql, 010_platform.sql, 020_reference.sql,
--             030_identity.sql, 040_organizations.sql, event, programme, media.
--
-- RÈGLE CARDINALE — CE MODULE N'ENVOIE RIEN.
-- Il enregistre des INTENTIONS (« cette personne doit recevoir ceci, à cette
-- heure, sur ce canal ») et des TRACES (« voici ce qui est parti, et ce que le
-- fournisseur en a dit »). L'exécution réelle — appel SMTP/API, rendu du
-- modèle, gestion du backoff — appartient au worker, qui consomme
-- `platform.jobs` et `platform.outbox_events`. Aucune fonction de ce fichier ne
-- doit ouvrir de connexion sortante : c'est ce qui permet de rejouer, d'auditer
-- et, le jour venu, d'extraire l'expédition dans un service dédié sans toucher
-- au schéma.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Catalogue des types de notification
--
-- DÉFAUT V1 CORRIGÉ — `notification_type` était un ENUM fermé de 8 valeurs
-- ('activity_validation', 'connection_request', ...). Chaque fonctionnalité
-- nouvelle imposait un `ALTER TYPE ... ADD VALUE`, donc une migration, un
-- déploiement, et un ENUM qui ne pouvait plus être réduit (PostgreSQL ne sait
-- pas retirer une valeur d'ENUM). En v2, le type de notification est une CLÉ
-- TEXTE déclarée en table : ajouter une notification devient un INSERT, la
-- désactiver un UPDATE, et le catalogue porte lui-même son modèle d'email, ses
-- canaux par défaut et sa criticité.
--
-- Le CANAL, lui, reste un ENUM : il énumère des capacités techniques de la
-- plateforme (trois implémentations d'expédition), pas des cas d'usage métier.
-- Il ne bouge qu'avec le code du worker.
-- -----------------------------------------------------------------------------
CREATE TYPE engagement.notification_channel AS ENUM ('in_app', 'email', 'push');

CREATE TYPE engagement.notification_criticality AS ENUM (
    'critical',   -- sécurité, RGPD, annulation de session : NON désactivable par l'utilisateur
    'important',  -- décision sur une proposition, confirmation d'inscription
    'normal',     -- rappels, réponses à un commentaire
    'low'         -- suggestions, animation éditoriale
);

CREATE TABLE engagement.notification_types (
    code               text PRIMARY KEY
                       CHECK (code ~ '^[a-z_]+\.[a-z_]+\.[a-z_]+$'),   -- même grammaire que outbox_events.event_type
    module_code        text        NOT NULL REFERENCES platform.modules(code) ON DELETE CASCADE,
    label              platform.i18n_text NOT NULL,
    description        platform.i18n_text,
    default_channels   engagement.notification_channel[] NOT NULL DEFAULT '{in_app}',
    criticality        engagement.notification_criticality NOT NULL DEFAULT 'normal',
    -- Modèle d'email associé (FK ajoutée après la création de message_templates).
    default_template_id uuid,
    -- Variables que l'émetteur s'engage à fournir : contrat vérifiable en test.
    expected_variables text[]      NOT NULL DEFAULT '{}',
    is_active          boolean     NOT NULL DEFAULT true,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_notification_types_channels CHECK (array_length(default_channels, 1) >= 1)
);

CREATE TRIGGER tg_notification_types_updated_at
    BEFORE UPDATE ON engagement.notification_types
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE engagement.notification_types IS
    'Catalogue ouvert des types de notification. Remplace l''ENUM fermé de la v1 : une notification nouvelle est un INSERT, pas une migration.';
COMMENT ON COLUMN engagement.notification_types.criticality IS
    'Une notification « critical » ignore les préférences utilisateur : sécurité du compte, annulation de session, obligations légales.';

-- -----------------------------------------------------------------------------
-- 2. Notifications in-app
-- -----------------------------------------------------------------------------
CREATE TABLE engagement.notifications (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id      uuid        NOT NULL CONSTRAINT xmod_fk_notifications_person
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    type_code      text        NOT NULL REFERENCES engagement.notification_types(code) ON DELETE RESTRICT,

    -- Deux modes d'alimentation : texte figé au moment de l'événement (i18n), ou
    -- variables rendues à l'affichage via le modèle du type. Le second garde les
    -- notifications traduisibles a posteriori et allège fortement le stockage.
    title          platform.i18n_text,
    body           platform.i18n_text,
    variables      jsonb       NOT NULL DEFAULT '{}'::jsonb,

    -- Lien de rebond : chemin relatif Nuxt, jamais une URL absolue (les domaines
    -- de préproduction et de production ne doivent pas fuiter dans les données).
    link_path      text        CHECK (link_path IS NULL OR link_path ~ '^/'),

    -- Sujet visé, en lecture seule : permet « tout marquer comme lu sur cette
    -- session » sans connaître le module émetteur.
    subject_schema text,
    subject_table  text,
    subject_id     uuid,

    -- Regroupement d'affichage : « 3 nouveaux commentaires sur votre activité »
    -- au lieu de trois lignes. Le worker incrémente `group_count` sur la
    -- notification non lue portant la même clé plutôt que d'en créer une autre.
    group_key      text,
    group_count    integer     NOT NULL DEFAULT 1 CHECK (group_count >= 1),

    read_at        timestamptz,
    archived_at    timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_notifications_content
        CHECK (title IS NOT NULL OR variables <> '{}'::jsonb),
    CONSTRAINT ck_notifications_subject
        CHECK (num_nonnulls(subject_schema, subject_table, subject_id) IN (0, 3))
);

-- Index partiel : la requête la plus fréquente de l'application (badge « non
-- lues ») ne parcourt que la fraction vivante de la table.
CREATE INDEX ix_notifications_unread
    ON engagement.notifications (person_id, created_at DESC)
    WHERE read_at IS NULL AND archived_at IS NULL;

CREATE INDEX ix_notifications_person   ON engagement.notifications (person_id, created_at DESC);
CREATE INDEX ix_notifications_subject  ON engagement.notifications (subject_schema, subject_table, subject_id)
    WHERE subject_id IS NOT NULL;
CREATE UNIQUE INDEX ux_notifications_group
    ON engagement.notifications (person_id, group_key)
    WHERE group_key IS NOT NULL AND read_at IS NULL;

COMMENT ON TABLE engagement.notifications IS
    'Notifications in-app. Écrites dans la transaction métier ; l''éventuel doublage par email passe par platform.jobs.';

-- -----------------------------------------------------------------------------
-- 3. Préférences fines par type ET par canal
--
-- DÉFAUT V1 CORRIGÉ — la v1 stockait `users.notification_preferences jsonb` en
-- vrac : structure non contrainte, non requêtable (« qui accepte les rappels
-- email ? » imposait un scan complet avec extraction JSON), et impossible à
-- faire évoluer sans risque de clés orphelines. Ici, une ligne = un arbitrage
-- explicite, indexable et joignable.
-- Absence de ligne = on retombe sur `default_channels` du type : l'utilisateur
-- n'a rien à configurer pour que la plateforme se comporte correctement.
-- -----------------------------------------------------------------------------
CREATE TABLE engagement.notification_preferences (
    person_id  uuid NOT NULL CONSTRAINT xmod_fk_notification_preferences_person
               REFERENCES identity.people(id) ON DELETE CASCADE,
    type_code  text NOT NULL REFERENCES engagement.notification_types(code) ON DELETE CASCADE,
    channel    engagement.notification_channel NOT NULL,
    is_enabled boolean     NOT NULL DEFAULT true,
    updated_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, type_code, channel)
);

CREATE TRIGGER tg_notification_preferences_updated_at
    BEFORE UPDATE ON engagement.notification_preferences
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE engagement.notification_preferences IS
    'Arbitrage utilisateur par type de notification et par canal. Remplace le jsonb informe de la v1.';

-- Décision unique consultée par le worker avant toute expédition.
-- Fonction TOTALE : elle répond toujours vrai ou faux, jamais NULL — un type
-- inconnu (faute de frappe, module désinstallé) vaut refus, sans quoi un
-- `WHERE NOT is_channel_enabled(...)` laisserait passer la ligne silencieusement.
CREATE OR REPLACE FUNCTION engagement.is_channel_enabled(
    p_person_id uuid,
    p_type_code text,
    p_channel   engagement.notification_channel
)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(
        (
            SELECT CASE
                WHEN nt.criticality = 'critical' THEN true    -- non désactivable
                WHEN NOT nt.is_active            THEN false
                ELSE COALESCE(np.is_enabled, p_channel = ANY (nt.default_channels))
            END
            FROM engagement.notification_types nt
            LEFT JOIN engagement.notification_preferences np
                   ON np.type_code = nt.code
                  AND np.person_id = p_person_id
                  AND np.channel   = p_channel
            WHERE nt.code = p_type_code
        ),
        false   -- type inconnu : on n'invente pas d'envoi
    );
$$;

COMMENT ON FUNCTION engagement.is_channel_enabled IS
    'Autorise ou non un canal pour une personne et un type. Repli sur les canaux par défaut du type ; les types critiques passent toujours.';

-- -----------------------------------------------------------------------------
-- 4. Modèles de messages versionnés et multilingues
--
-- DÉFAUT V1 CORRIGÉ — les gabarits HTML vivaient en dur dans les edge functions
-- (send-email, registration-email, send-activity-notification...) : corriger une
-- faute d'orthographe exigeait un redéploiement Deno, la version anglaise
-- n'existait pas, et deux fonctions divergeaient sur le même courriel.
-- En v2 le contenu vit en base, en `i18n_text`, versionné : on publie une
-- révision sans redéployer et on peut revenir à la précédente.
-- -----------------------------------------------------------------------------
CREATE TABLE engagement.message_templates (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    key                 platform.slug NOT NULL UNIQUE,     -- 'registration-confirmed', 'session-reminder'
    label               platform.i18n_text NOT NULL,
    -- Type de notification servi par ce modèle (facultatif : les campagnes
    -- d'infolettre n'en dépendent pas).
    type_code           text        REFERENCES engagement.notification_types(code) ON DELETE SET NULL,
    -- Version actuellement servie ; NULL tant qu'aucune révision n'est publiée.
    current_version     smallint,
    is_active           boolean     NOT NULL DEFAULT true,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE engagement.template_versions (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    template_id    uuid        NOT NULL REFERENCES engagement.message_templates(id) ON DELETE CASCADE,
    version        smallint    NOT NULL CHECK (version >= 1),
    subject        platform.i18n_text NOT NULL,
    body_html      platform.i18n_text NOT NULL,
    body_text      platform.i18n_text,          -- repli texte brut, exigé par les bons clients mail
    -- Variables attendues par le gabarit ({{prenom}}, {{titre_session}}...).
    -- Le worker refuse le rendu si une variable manque : mieux vaut un job en
    -- échec visible qu'un email « Bonjour  , » envoyé à 2 000 personnes.
    variables      text[]      NOT NULL DEFAULT '{}',
    published_at   timestamptz,
    created_by     uuid        CONSTRAINT xmod_fk_template_versions_author
                               REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_template_versions UNIQUE (template_id, version)
);

CREATE INDEX ix_template_versions_published
    ON engagement.template_versions (template_id, version DESC)
    WHERE published_at IS NOT NULL;

ALTER TABLE engagement.notification_types
    ADD CONSTRAINT fk_notification_types_template
    FOREIGN KEY (default_template_id) REFERENCES engagement.message_templates(id) ON DELETE SET NULL;

CREATE TRIGGER tg_message_templates_updated_at
    BEFORE UPDATE ON engagement.message_templates
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_message_templates_audit
    AFTER INSERT OR UPDATE OR DELETE ON engagement.message_templates
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE engagement.message_templates IS
    'Modèles d''emails et de notifications, sortis du code des edge functions v1. Une entrée = un courriel fonctionnel.';
COMMENT ON TABLE engagement.template_versions IS
    'Révisions successives d''un modèle. La publication est un UPDATE de message_templates.current_version, réversible.';

-- -----------------------------------------------------------------------------
-- 5. Emails : journal d'expédition et délivrabilité
--
-- DÉFAUT V1 CORRIGÉ — `email_history` se contentait de trois états
-- ('sent', 'failed', 'pending') posés par l'appelant. Personne ne savait si un
-- message avait été REMIS, s'il avait rebondi, ni si le destinataire s'était
-- plaint. Résultat classique : on continue d'écrire à des adresses mortes, le
-- taux de rebond grimpe, et le domaine expéditeur finit en dossier indésirable
-- pour tout le monde — y compris pour les confirmations d'inscription.
--
-- Partitionnée par mois comme platform.audit_log : volumétrie de log, purge par
-- DROP PARTITION, et aucune FK (la trace doit survivre à la suppression d'une
-- personne, d'un modèle ou d'une session).
-- -----------------------------------------------------------------------------
CREATE TYPE engagement.email_status AS ENUM (
    'queued',      -- intention enregistrée, job créé
    'sent',        -- accepté par le fournisseur
    'delivered',   -- remise confirmée par webhook
    'bounced',     -- rebond (dur ou souple, cf. bounce_kind)
    'complained',  -- signalé comme indésirable par le destinataire
    'failed'       -- échec technique après épuisement des tentatives
);

CREATE TABLE engagement.email_messages (
    id                  uuid        NOT NULL DEFAULT platform.uuid_v7(),
    created_at          timestamptz NOT NULL DEFAULT now(),

    to_email            platform.email NOT NULL,
    to_person_id        uuid,                    -- sans FK : trace conservée après anonymisation
    locale              text        NOT NULL DEFAULT 'fr',

    type_code           text,                    -- engagement.notification_types.code, sans FK
    template_id         uuid,
    template_version    smallint,
    subject             text        NOT NULL,    -- sujet réellement rendu, figé
    variables           jsonb       NOT NULL DEFAULT '{}'::jsonb,

    -- Rattachements sans FK, pour l'exploitation et le support.
    reminder_id         uuid,
    campaign_id         uuid,
    job_id              uuid,

    status              engagement.email_status NOT NULL DEFAULT 'queued',
    provider            text,                    -- 'laravel_relay', 'smtp', 'ses'...
    provider_message_id text,                    -- corrélation des webhooks de délivrabilité
    bounce_kind         text        CHECK (bounce_kind IN ('hard', 'soft', 'block')),
    attempts            smallint    NOT NULL DEFAULT 0,
    max_attempts        smallint    NOT NULL DEFAULT 5,
    last_error          text,

    sent_at             timestamptz,
    delivered_at        timestamptz,
    failed_at           timestamptz,
    updated_at          timestamptz NOT NULL DEFAULT now(),

    PRIMARY KEY (created_at, id)
) PARTITION BY RANGE (created_at);

CREATE TABLE engagement.email_messages_default PARTITION OF engagement.email_messages DEFAULT;

-- File de réémission : minuscule en permanence, même avec des millions de lignes
-- historisées. C'est l'index que consulte le worker à chaque cycle.
CREATE INDEX ix_email_messages_retryable
    ON engagement.email_messages (created_at, id)
    WHERE status IN ('queued', 'failed');

CREATE INDEX ix_email_messages_recipient ON engagement.email_messages (to_email, created_at DESC);
CREATE INDEX ix_email_messages_person    ON engagement.email_messages (to_person_id, created_at DESC)
    WHERE to_person_id IS NOT NULL;
CREATE UNIQUE INDEX ux_email_messages_provider
    ON engagement.email_messages (created_at, provider, provider_message_id)
    WHERE provider_message_id IS NOT NULL;

CREATE TRIGGER tg_email_messages_updated_at
    BEFORE UPDATE ON engagement.email_messages
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE engagement.email_messages IS
    'Journal d''expédition partitionné par mois, avec suivi de délivrabilité alimenté par les webhooks du fournisseur.';

-- Amorce des partitions du trimestre courant ; le worker de maintenance
-- appellera ensuite platform.ensure_month_partition() mois par mois.
DO $$
DECLARE
    v_month date;
BEGIN
    FOR v_month IN
        SELECT generate_series(date_trunc('month', now()),
                               date_trunc('month', now()) + interval '2 months',
                               interval '1 month')::date
    LOOP
        PERFORM platform.ensure_month_partition('engagement', 'email_messages', v_month);
    END LOOP;
END
$$;

-- Liste de suppression : la garde-fou de la réputation d'envoi. Une adresse qui
-- rebondit durement ou qui s'est désabonnée n'est PLUS JAMAIS sollicitée, quel
-- que soit le module émetteur. Cette table se consulte avant toute mise en file.
CREATE TYPE engagement.suppression_reason AS ENUM (
    'hard_bounce', 'complaint', 'unsubscribe', 'invalid_address', 'manual'
);

CREATE TABLE engagement.email_suppressions (
    email        platform.email PRIMARY KEY,
    reason       engagement.suppression_reason NOT NULL,
    detail       text,
    -- NULL = suppression définitive. Une valeur permet de lever une suppression
    -- temporaire (boîte pleine) sans intervention manuelle.
    expires_at   timestamptz,
    suppressed_at timestamptz NOT NULL DEFAULT now(),
    suppressed_by uuid        CONSTRAINT xmod_fk_email_suppressions_actor
                              REFERENCES identity.people(id) ON DELETE SET NULL
);

CREATE INDEX ix_email_suppressions_expiry ON engagement.email_suppressions (expires_at)
    WHERE expires_at IS NOT NULL;

COMMENT ON TABLE engagement.email_suppressions IS
    'Adresses interdites d''envoi (rebond dur, plainte, désabonnement). À interroger AVANT toute mise en file : protège la délivrabilité du domaine.';

CREATE OR REPLACE FUNCTION engagement.is_email_suppressed(p_email platform.email)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1 FROM engagement.email_suppressions s
        WHERE s.email = p_email
          AND (s.expires_at IS NULL OR s.expires_at > now())
    );
$$;

-- -----------------------------------------------------------------------------
-- 6. Rappels programmés
--
-- EXIGENCE MÉTIER — « après inscription à une activité, selon ce qu'a programmé
-- l'admin pour l'événement, on peut recevoir des emails de rappel (2 jours,
-- 1 jour, 1 heure, 30 minutes, cumulés) avant l'activité. »
--
-- DÉFAUT V1 CORRIGÉ — les rappels étaient déclenchés par du code applicatif
-- dispersé (edge functions + appels côté client), sans état persistant : un
-- worker relancé, un webhook rejoué ou deux onglets ouverts pouvaient produire
-- deux fois le même email, et personne ne pouvait répondre à « ce participant
-- a-t-il reçu son rappel de 1 h ? ».
--
-- En v2, deux tables :
--   reminder_rules      = la POLITIQUE, paramétrée par l'admin (offsets cumulés)
--   scheduled_reminders = la MATÉRIALISATION, une ligne par destinataire × offset
-- La clé unique (session, personne, canal, offset) rend le double envoi
-- structurellement impossible : ce n'est pas une convention de code, c'est une
-- contrainte de base.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION engagement.are_offsets_valid(p_offsets interval[])
RETURNS boolean
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT p_offsets IS NOT NULL
       AND array_length(p_offsets, 1) BETWEEN 1 AND 8
       AND NOT EXISTS (SELECT 1 FROM unnest(p_offsets) AS o WHERE o <= interval '0');
$$;

CREATE TABLE engagement.reminder_rules (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- Portée : soit tout un événement (COP entière), soit une session précise.
    -- Une règle de session prend le pas sur les règles de son événement.
    event_id    uuid        CONSTRAINT xmod_fk_reminder_rules_event
                            REFERENCES event.events(id) ON DELETE CASCADE,
    session_id  uuid        CONSTRAINT xmod_fk_reminder_rules_session
                            REFERENCES programme.sessions(id) ON DELETE CASCADE,

    -- Décalages CUMULÉS avant le début : '{2 days, 1 day, 1 hour, 30 minutes}'.
    offsets     interval[]  NOT NULL DEFAULT '{2 days, 1 day, 1 hour, 30 minutes}',
    channels    engagement.notification_channel[] NOT NULL DEFAULT '{email}',
    type_code   text        NOT NULL DEFAULT 'programme.session.reminder'
                            REFERENCES engagement.notification_types(code) ON DELETE RESTRICT,
    template_id uuid        REFERENCES engagement.message_templates(id) ON DELETE SET NULL,
    is_active   boolean     NOT NULL DEFAULT true,
    created_by  uuid        CONSTRAINT xmod_fk_reminder_rules_author
                            REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at  timestamptz NOT NULL DEFAULT now(),
    updated_at  timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_reminder_rules_scope
        CHECK (num_nonnulls(event_id, session_id) = 1),
    CONSTRAINT ck_reminder_rules_offsets
        CHECK (engagement.are_offsets_valid(offsets)),
    CONSTRAINT ck_reminder_rules_channels
        CHECK (array_length(channels, 1) >= 1)
);

CREATE UNIQUE INDEX ux_reminder_rules_event   ON engagement.reminder_rules (event_id)   WHERE event_id IS NOT NULL;
CREATE UNIQUE INDEX ux_reminder_rules_session ON engagement.reminder_rules (session_id) WHERE session_id IS NOT NULL;

CREATE TRIGGER tg_reminder_rules_updated_at
    BEFORE UPDATE ON engagement.reminder_rules
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE engagement.reminder_rules IS
    'Politique de rappel paramétrée par l''administrateur : décalages cumulés avant le début, canaux, modèle. Une règle de session écrase celle de son événement.';

CREATE TYPE engagement.reminder_status AS ENUM ('pending', 'queued', 'sent', 'skipped', 'cancelled');

CREATE TABLE engagement.scheduled_reminders (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    rule_id       uuid        REFERENCES engagement.reminder_rules(id) ON DELETE SET NULL,
    session_id    uuid        NOT NULL CONSTRAINT xmod_fk_scheduled_reminders_session
                              REFERENCES programme.sessions(id) ON DELETE CASCADE,
    person_id     uuid        NOT NULL CONSTRAINT xmod_fk_scheduled_reminders_person
                              REFERENCES identity.people(id) ON DELETE CASCADE,
    registration_id uuid      CONSTRAINT xmod_fk_scheduled_reminders_registration
                              REFERENCES programme.registrations(id) ON DELETE CASCADE,

    channel       engagement.notification_channel NOT NULL DEFAULT 'email',
    offset_before interval    NOT NULL CHECK (offset_before > interval '0'),
    scheduled_for timestamptz NOT NULL,

    status        engagement.reminder_status NOT NULL DEFAULT 'pending',
    job_id        uuid        REFERENCES platform.jobs(id) ON DELETE SET NULL,
    sent_at       timestamptz,
    skip_reason   text,       -- 'suppressed', 'channel_disabled', 'session_cancelled'
    created_at    timestamptz NOT NULL DEFAULT now(),

    -- LA contrainte anti-doublon : un destinataire ne peut recevoir qu'une seule
    -- fois le rappel « J-1 » d'une session donnée, sur un canal donné, quel que
    -- soit le nombre de règles, de rejeux ou de reprogrammations.
    CONSTRAINT ux_scheduled_reminders_once
        UNIQUE (session_id, person_id, channel, offset_before)
);

-- Échéancier du worker : seules les lignes encore à traiter sont indexées.
CREATE INDEX ix_scheduled_reminders_due
    ON engagement.scheduled_reminders (scheduled_for)
    WHERE status = 'pending';

CREATE INDEX ix_scheduled_reminders_person  ON engagement.scheduled_reminders (person_id, scheduled_for);
CREATE INDEX ix_scheduled_reminders_session ON engagement.scheduled_reminders (session_id, status);

COMMENT ON TABLE engagement.scheduled_reminders IS
    'Matérialisation des rappels : une ligne par destinataire et par décalage. La clé unique interdit structurellement le double envoi.';

-- Matérialisation idempotente des rappels d'une session.
--
-- Appelée par l'abonné outbox sur 'programme.registration.confirmed' et sur
-- 'programme.session.scheduled' / '.rescheduled'. Peut être rejouée sans
-- précaution : ON CONFLICT DO NOTHING sur la clé unique.
--
-- Contrat attendu du module programme (aligné sur 060_programme.sql) :
--   programme.sessions(id, event_id, starts_at)
--   programme.registrations(id, session_id, person_id, status)
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
        SELECT reg.id AS registration_id, reg.person_id
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

COMMENT ON FUNCTION engagement.schedule_session_reminders(uuid) IS
    'Matérialise les rappels d''une session d''après la règle applicable. Idempotente : rejouable après un incident sans risque de doublon.';

-- Calendrier des rappels d'une session, AGRÉGÉ PAR (décalage, canal).
--
-- POURQUOI UNE FONCTION ET NON DEUX REQUÊTES. Cet agrégat a DEUX lecteurs : la
-- lecture par séance du module Engagement, et la composition de l'espace
-- organisation servie par le module Programmation. Écrites séparément, les deux
-- agrégations divergeraient au premier ajustement de la règle de consolidation
-- — et la divergence serait SILENCIEUSE : un nombre de destinataires faux
-- ressemble à un nombre juste. C'est le raisonnement exact de
-- media.attached_image(), que trois modules appellent déjà.
--
-- CE QU'ELLE NE REND JAMAIS : ni identifiant de personne, ni nom, ni adresse.
-- L'organisation qui anime une séance a droit au NOMBRE de destinataires, pas à
-- leur identité — et cette garantie est portée par la SIGNATURE de la fonction,
-- pas par la discipline de ses appelants.
--
-- L'ÉTAT CONSOLIDÉ EST CELUI DE LA LIGNE LA MOINS AVANCÉE, dans cet ordre :
-- une seule ligne encore à traiter suffit à dire « en attente », puis « en
-- file », puis « parti ». « Parti » ne doit pas se dire tant qu'une personne
-- attend encore son courriel. Un groupe entièrement écarté ou annulé prend
-- l'état majoritaire des deux, et porte alors son motif dominant — seul cas où
-- le motif sort, sans quoi une ligne écartée dans un groupe encore vivant
-- ferait croire à un incident.
--
-- Le décalage sort EN MINUTES : `'1 day'` et `'24 hours'` sont le même
-- intervalle pour la base et deux chaînes différentes pour l'écran qui les
-- regrouperait par leur texte.
CREATE OR REPLACE FUNCTION engagement.session_reminder_schedule(p_session_id uuid)
RETURNS TABLE (
    offset_minutes  integer,
    channel         text,
    scheduled_for   timestamptz,
    status          text,
    recipient_count bigint,
    skip_reason     text,
    sent_at         timestamptz
)
LANGUAGE sql
STABLE
AS $$
    WITH groupes AS (
        SELECT sr.offset_before,
               sr.channel                                          AS canal,
               min(sr.scheduled_for)                               AS scheduled_for,
               count(*)                                            AS recipient_count,
               max(sr.sent_at)                                     AS sent_at,
               count(*) FILTER (WHERE sr.status = 'pending')       AS n_pending,
               count(*) FILTER (WHERE sr.status = 'queued')        AS n_queued,
               count(*) FILTER (WHERE sr.status = 'sent')          AS n_sent,
               count(*) FILTER (WHERE sr.status = 'skipped')       AS n_skipped,
               count(*) FILTER (WHERE sr.status = 'cancelled')     AS n_cancelled
        FROM engagement.scheduled_reminders sr
        WHERE sr.session_id = p_session_id
        GROUP BY sr.offset_before, sr.channel
    ),
    consolides AS (
        SELECT g.*,
               CASE
                   WHEN g.n_pending > 0            THEN 'pending'
                   WHEN g.n_queued  > 0            THEN 'queued'
                   WHEN g.n_sent    > 0            THEN 'sent'
                   WHEN g.n_skipped >= g.n_cancelled THEN 'skipped'
                   ELSE 'cancelled'
               END AS etat
        FROM groupes g
    )
    SELECT (extract(epoch FROM c.offset_before) / 60)::integer,
           c.canal::text,
           c.scheduled_for,
           c.etat,
           c.recipient_count,
           CASE WHEN c.etat IN ('skipped', 'cancelled') THEN (
               SELECT m.skip_reason
               FROM engagement.scheduled_reminders m
               WHERE m.session_id    = p_session_id
                 AND m.offset_before = c.offset_before
                 AND m.channel       = c.canal
                 AND m.skip_reason IS NOT NULL
               GROUP BY m.skip_reason
               ORDER BY count(*) DESC, m.skip_reason
               LIMIT 1
           ) END,
           c.sent_at
    FROM consolides c
    ORDER BY c.offset_before DESC, c.canal;
$$;

COMMENT ON FUNCTION engagement.session_reminder_schedule(uuid) IS
    'Calendrier des rappels d''une session, agrégé par (décalage, canal) : instant d''envoi, état consolidé, NOMBRE de destinataires. Ne rend jamais d''identité. Écrite une fois pour ses deux lecteurs — engagement et programme.';

-- -----------------------------------------------------------------------------
-- 7. Commentaires et réactions
--
-- DÉFAUT V1 CORRIGÉ — `comments(context_type comment_context_type, context_id)`
-- pointait n'importe où sans la moindre intégrité : un commentaire pouvait
-- survivre à la suppression de sa cible, viser un identifiant inexistant, ou
-- désigner un type de contexte que l'application ne savait plus afficher.
--
-- Le lien polymorphe est CONSERVÉ — commenter doit rester transversal (article,
-- session, événement) sans créer une table par cible — mais il est encadré :
--   1. le couple (subject_schema, subject_table) référence une TABLE BLANCHE ;
--   2. un trigger vérifie l'existence réelle de la ligne visée ;
--   3. les réponses en fil sont garanties par une FK composite : une réponse ne
--      peut pas changer de sujet en cours de route.
-- -----------------------------------------------------------------------------
CREATE TABLE engagement.commentable_subjects (
    subject_schema      text    NOT NULL,
    subject_table       text    NOT NULL,
    label               platform.i18n_text NOT NULL,
    allows_comments     boolean NOT NULL DEFAULT true,
    allows_reactions    boolean NOT NULL DEFAULT true,
    requires_moderation boolean NOT NULL DEFAULT false,   -- a priori (true) ou a posteriori (false)
    PRIMARY KEY (subject_schema, subject_table)
);

COMMENT ON TABLE engagement.commentable_subjects IS
    'Table blanche des cibles ouvertes au commentaire et à la réaction. Rend le polymorphisme déclaratif au lieu d''implicite.';

-- Refuse d'inscrire une cible qui n'existe pas : une faute de frappe dans la
-- table blanche ne doit pas se découvrir en production.
CREATE OR REPLACE FUNCTION engagement.tg_check_subject_table_exists()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF to_regclass(format('%I.%I', NEW.subject_schema, NEW.subject_table)) IS NULL THEN
        RAISE EXCEPTION 'Table cible inexistante : %.%', NEW.subject_schema, NEW.subject_table
            USING ERRCODE = 'undefined_table';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_commentable_subjects_exists
    BEFORE INSERT OR UPDATE ON engagement.commentable_subjects
    FOR EACH ROW EXECUTE FUNCTION engagement.tg_check_subject_table_exists();

-- Vérifie l'existence de la LIGNE visée. Partagé par comments et reactions.
CREATE OR REPLACE FUNCTION engagement.tg_validate_subject_row()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_exists boolean;
BEGIN
    EXECUTE format('SELECT EXISTS (SELECT 1 FROM %I.%I WHERE id = $1)',
                   NEW.subject_schema, NEW.subject_table)
    INTO v_exists USING NEW.subject_id;

    IF NOT v_exists THEN
        RAISE EXCEPTION 'Sujet introuvable : %.% (id=%)',
            NEW.subject_schema, NEW.subject_table, NEW.subject_id
            USING ERRCODE = 'foreign_key_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TYPE engagement.comment_status AS ENUM ('published', 'pending_review', 'rejected', 'hidden');

CREATE TABLE engagement.comments (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    subject_schema text        NOT NULL,
    subject_table  text        NOT NULL,
    subject_id     uuid        NOT NULL,
    author_id      uuid        NOT NULL CONSTRAINT xmod_fk_comments_author
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    parent_id      uuid,
    depth          smallint    NOT NULL DEFAULT 0,
    body           text        NOT NULL CHECK (length(btrim(body)) BETWEEN 1 AND 5000),

    status         engagement.comment_status NOT NULL DEFAULT 'published',
    moderated_by   uuid        CONSTRAINT xmod_fk_comments_moderator
                               REFERENCES identity.people(id) ON DELETE SET NULL,
    moderated_at   timestamptz,
    moderation_note text,
    reply_count    integer     NOT NULL DEFAULT 0 CHECK (reply_count >= 0),

    -- Suppression douce : le fil reste lisible, l'auteur peut se rétracter.
    deleted_at     timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT fk_comments_subject
        FOREIGN KEY (subject_schema, subject_table)
        REFERENCES engagement.commentable_subjects (subject_schema, subject_table) ON DELETE RESTRICT,
    -- Un fil à un seul niveau de réponse : suffisant pour un débat de session,
    -- et cela évite les arbres profonds impossibles à afficher sur mobile.
    CONSTRAINT ck_comments_depth
        CHECK ((parent_id IS NULL AND depth = 0) OR (parent_id IS NOT NULL AND depth = 1)),
    CONSTRAINT ux_comments_thread_key UNIQUE (id, subject_schema, subject_table, subject_id)
);

-- FK composite : la réponse hérite obligatoirement du sujet de son parent.
ALTER TABLE engagement.comments
    ADD CONSTRAINT fk_comments_parent
    FOREIGN KEY (parent_id, subject_schema, subject_table, subject_id)
    REFERENCES engagement.comments (id, subject_schema, subject_table, subject_id) ON DELETE CASCADE;

CREATE INDEX ix_comments_subject
    ON engagement.comments (subject_schema, subject_table, subject_id, created_at DESC)
    WHERE deleted_at IS NULL AND status = 'published';
CREATE INDEX ix_comments_author  ON engagement.comments (author_id, created_at DESC);
CREATE INDEX ix_comments_parent  ON engagement.comments (parent_id) WHERE parent_id IS NOT NULL;
CREATE INDEX ix_comments_pending ON engagement.comments (created_at) WHERE status = 'pending_review';

CREATE TRIGGER tg_comments_validate_subject
    BEFORE INSERT OR UPDATE OF subject_schema, subject_table, subject_id ON engagement.comments
    FOR EACH ROW EXECUTE FUNCTION engagement.tg_validate_subject_row();

CREATE TRIGGER tg_comments_updated_at
    BEFORE UPDATE ON engagement.comments
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE engagement.comments IS
    'Commentaires publics polymorphes, encadrés par une table blanche, un trigger d''existence et une FK composite de fil.';

-- Réaction générique et légère : une personne, une cible, un type de réaction.
-- Le type reste une chaîne libre contrainte (et non un ENUM) pour ajouter un
-- émoji sans migration — même raisonnement que pour les types de notification.
CREATE TABLE engagement.reactions (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    subject_schema text        NOT NULL,
    subject_table  text        NOT NULL,
    subject_id     uuid        NOT NULL,
    person_id      uuid        NOT NULL CONSTRAINT xmod_fk_reactions_person
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    kind           text        NOT NULL DEFAULT 'like'
                               CHECK (kind ~ '^[a-z_]{2,24}$'),
    created_at     timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT fk_reactions_subject
        FOREIGN KEY (subject_schema, subject_table)
        REFERENCES engagement.commentable_subjects (subject_schema, subject_table) ON DELETE RESTRICT,
    CONSTRAINT ux_reactions_once
        UNIQUE (subject_schema, subject_table, subject_id, person_id, kind)
);

CREATE INDEX ix_reactions_subject ON engagement.reactions (subject_schema, subject_table, subject_id, kind);

CREATE TRIGGER tg_reactions_validate_subject
    BEFORE INSERT OR UPDATE ON engagement.reactions
    FOR EACH ROW EXECUTE FUNCTION engagement.tg_validate_subject_row();

-- -----------------------------------------------------------------------------
-- 8. Messagerie directe
--
-- La v1 juxtaposait `messages` (1-1) et `group_messages` + `message_groups`
-- (groupe) : deux modèles, deux requêtes, deux composants d'interface, et aucun
-- moyen de savoir jusqu'où un participant avait lu. En v2, un seul modèle
-- conversationnel ; le 1-1 n'est qu'une conversation à deux participants.
-- -----------------------------------------------------------------------------
CREATE TYPE engagement.conversation_kind AS ENUM ('direct', 'group');

CREATE TABLE engagement.conversations (
    id         uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    kind       engagement.conversation_kind NOT NULL DEFAULT 'direct',
    title      text,
    -- Clé canonique du 1-1 : 'uuid_le_plus_petit:uuid_le_plus_grand'. Empêche la
    -- création de deux conversations parallèles entre les deux mêmes personnes.
    direct_key text,
    created_by uuid        CONSTRAINT xmod_fk_conversations_creator
                           REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    last_message_at timestamptz,

    CONSTRAINT ck_conversations_shape
        CHECK ((kind = 'direct' AND direct_key IS NOT NULL AND title IS NULL)
            OR (kind = 'group'  AND direct_key IS NULL AND title IS NOT NULL))
);

CREATE UNIQUE INDEX ux_conversations_direct ON engagement.conversations (direct_key)
    WHERE direct_key IS NOT NULL;

CREATE TABLE engagement.conversation_participants (
    conversation_id uuid NOT NULL REFERENCES engagement.conversations(id) ON DELETE CASCADE,
    person_id       uuid NOT NULL CONSTRAINT xmod_fk_conversation_participants_person
                    REFERENCES identity.people(id) ON DELETE CASCADE,
    is_admin        boolean     NOT NULL DEFAULT false,
    is_muted        boolean     NOT NULL DEFAULT false,
    -- Curseur de lecture : un seul horodatage par participant remplace un
    -- booléen `is_read` par message (la v1 en écrivait un par ligne).
    last_read_at    timestamptz,
    joined_at       timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    PRIMARY KEY (conversation_id, person_id)
);

CREATE INDEX ix_conversation_participants_person
    ON engagement.conversation_participants (person_id) WHERE left_at IS NULL;

CREATE TABLE engagement.direct_messages (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    conversation_id uuid        NOT NULL REFERENCES engagement.conversations(id) ON DELETE CASCADE,
    sender_id       uuid        NOT NULL CONSTRAINT xmod_fk_direct_messages_sender
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    body            text        NOT NULL CHECK (length(btrim(body)) BETWEEN 1 AND 10000),
    attachment_id   uuid        CONSTRAINT xmod_fk_direct_messages_attachment
                                REFERENCES media.assets(id) ON DELETE SET NULL,
    edited_at       timestamptz,
    deleted_at      timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now()
);

-- Pagination inversée d'un fil : l'index sert aussi au calcul des non-lus par
-- comparaison avec `last_read_at`.
CREATE INDEX ix_direct_messages_conversation
    ON engagement.direct_messages (conversation_id, created_at DESC)
    WHERE deleted_at IS NULL;

COMMENT ON TABLE engagement.conversations IS
    'Conversation 1-1 ou de groupe. Modèle unique : la v1 dupliquait messages et group_messages.';
COMMENT ON COLUMN engagement.conversation_participants.last_read_at IS
    'Curseur de lecture du participant : le nombre de non-lus se déduit sans écrire une ligne par message lu.';

-- -----------------------------------------------------------------------------
-- 9. Mise en relation et blocages
-- -----------------------------------------------------------------------------
CREATE TYPE engagement.connection_status AS ENUM ('pending', 'accepted', 'declined', 'cancelled', 'expired');

CREATE TABLE engagement.connection_requests (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    requester_id uuid        NOT NULL CONSTRAINT xmod_fk_connection_requests_requester
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    recipient_id uuid        NOT NULL CONSTRAINT xmod_fk_connection_requests_recipient
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    message      text        CHECK (message IS NULL OR length(message) <= 1000),
    status       engagement.connection_status NOT NULL DEFAULT 'pending',
    responded_at timestamptz,
    expires_at   timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_connection_requests_distinct CHECK (requester_id <> recipient_id),
    CONSTRAINT ck_connection_requests_response
        CHECK (status = 'pending' OR responded_at IS NOT NULL)
);

-- Une seule demande en attente dans un sens donné...
CREATE UNIQUE INDEX ux_connection_requests_pending
    ON engagement.connection_requests (requester_id, recipient_id)
    WHERE status = 'pending';

-- ...et une seule relation acceptée par PAIRE, quel que soit l'initiateur :
-- la v1 autorisait deux lignes 'accepted' croisées pour la même relation.
CREATE UNIQUE INDEX ux_connection_requests_accepted
    ON engagement.connection_requests (
        least(requester_id, recipient_id), greatest(requester_id, recipient_id)
    )
    WHERE status = 'accepted';

CREATE INDEX ix_connection_requests_recipient
    ON engagement.connection_requests (recipient_id, created_at DESC) WHERE status = 'pending';

CREATE TRIGGER tg_connection_requests_updated_at
    BEFORE UPDATE ON engagement.connection_requests
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TABLE engagement.blocks (
    blocker_id uuid        NOT NULL CONSTRAINT xmod_fk_blocks_blocker
                           REFERENCES identity.people(id) ON DELETE CASCADE,
    blocked_id uuid        NOT NULL CONSTRAINT xmod_fk_blocks_blocked
                           REFERENCES identity.people(id) ON DELETE CASCADE,
    reason     text,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (blocker_id, blocked_id),
    CONSTRAINT ck_blocks_distinct CHECK (blocker_id <> blocked_id)
);

CREATE INDEX ix_blocks_blocked ON engagement.blocks (blocked_id);

COMMENT ON TABLE engagement.blocks IS
    'Blocages entre membres. À consulter avant toute mise en relation, tout message direct et toute notification sociale.';

-- -----------------------------------------------------------------------------
-- 10. Infolettres — HORS PÉRIMÈTRE PHASE 1
--
-- Structure posée dès maintenant pour que les consentements RGPD
-- (identity.consents, purpose = 'newsletter') et la liste de suppression aient
-- une destination cohérente le jour du démarrage. Aucun code applicatif de la
-- phase 1 ne lit ni n'écrit ces tables ; l'envoi de masse passera par le même
-- couple platform.jobs / engagement.email_messages que le transactionnel.
-- -----------------------------------------------------------------------------
CREATE TABLE engagement.newsletter_lists (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    key         platform.slug NOT NULL UNIQUE,
    label       platform.i18n_text NOT NULL,
    description platform.i18n_text,
    is_active   boolean     NOT NULL DEFAULT true,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE engagement.newsletter_subscriptions (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    list_id        uuid        NOT NULL REFERENCES engagement.newsletter_lists(id) ON DELETE CASCADE,
    -- Un abonné peut être une personne connue OU une simple adresse collectée.
    person_id      uuid        CONSTRAINT xmod_fk_newsletter_subscriptions_person
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    email          platform.email NOT NULL,
    locale         text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    -- Jeton de désabonnement en un clic (exigence RGPD et bonne pratique d'envoi).
    unsubscribe_token bytea    NOT NULL DEFAULT gen_random_bytes(16),
    subscribed_at  timestamptz NOT NULL DEFAULT now(),
    unsubscribed_at timestamptz,
    CONSTRAINT ux_newsletter_subscriptions UNIQUE (list_id, email)
);

CREATE INDEX ix_newsletter_subscriptions_active
    ON engagement.newsletter_subscriptions (list_id) WHERE unsubscribed_at IS NULL;

CREATE TYPE engagement.campaign_status AS ENUM ('draft', 'scheduled', 'sending', 'sent', 'cancelled');

CREATE TABLE engagement.newsletter_campaigns (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    list_id      uuid        REFERENCES engagement.newsletter_lists(id) ON DELETE SET NULL,
    template_id  uuid        REFERENCES engagement.message_templates(id) ON DELETE SET NULL,
    title        text        NOT NULL,
    subject      platform.i18n_text NOT NULL,
    body_html    platform.i18n_text NOT NULL,
    -- Ciblage déclaratif (pays, organisation, événement...) évalué au lancement,
    -- au lieu du couple target_type/target_id sans intégrité de la v1.
    audience_filter jsonb    NOT NULL DEFAULT '{}'::jsonb,
    status       engagement.campaign_status NOT NULL DEFAULT 'draft',
    scheduled_at timestamptz,
    sent_at      timestamptz,
    recipient_count integer,
    created_by   uuid        CONSTRAINT xmod_fk_newsletter_campaigns_author
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at   timestamptz NOT NULL DEFAULT now(),
    updated_at   timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_newsletter_campaigns_updated_at
    BEFORE UPDATE ON engagement.newsletter_campaigns
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE engagement.newsletter_campaigns IS
    'Campagnes d''infolettre. HORS PÉRIMÈTRE PHASE 1 : structure posée, aucun traitement applicatif associé.';

-- -----------------------------------------------------------------------------
-- 11. Amorçage
-- -----------------------------------------------------------------------------
INSERT INTO engagement.notification_types (code, module_code, label, default_channels, criticality, expected_variables) VALUES
    ('identity.account.security',       'identity',    '{"fr":"Alerte de sécurité du compte","en":"Account security alert"}',   '{in_app,email}', 'critical',  '{prenom}'),
    ('org.membership.requested',        'org',         '{"fr":"Demande d''adhésion","en":"Membership request"}',                '{in_app,email}', 'important', '{organisation,demandeur}'),
    ('org.membership.approved',         'org',         '{"fr":"Adhésion validée","en":"Membership approved"}',                  '{in_app,email}', 'important', '{organisation}'),
    ('programme.proposal.submitted',    'programme',   '{"fr":"Proposition reçue","en":"Proposal received"}',                   '{in_app,email}', 'important', '{titre_activite}'),
    ('programme.proposal.decided',      'programme',   '{"fr":"Décision sur votre proposition","en":"Proposal decision"}',      '{in_app,email}', 'important', '{titre_activite,decision}'),
    ('programme.revision.requested',    'programme',   '{"fr":"Révision demandée","en":"Revision requested"}',                  '{in_app,email}', 'important', '{titre_activite,commentaire}'),
    ('programme.registration.confirmed','programme',   '{"fr":"Inscription confirmée","en":"Registration confirmed"}',         '{in_app,email}', 'important', '{prenom,titre_session,date_session,lien_participation}'),
    ('programme.session.reminder',      'programme',   '{"fr":"Rappel de session","en":"Session reminder"}',                    '{email}',        'normal',    '{prenom,titre_session,date_session,delai,lien_participation}'),
    ('programme.session.cancelled',     'programme',   '{"fr":"Session annulée","en":"Session cancelled"}',                     '{in_app,email}', 'critical',  '{titre_session,motif}'),
    ('programme.session.rescheduled',   'programme',   '{"fr":"Session reprogrammée","en":"Session rescheduled"}',              '{in_app,email}', 'critical',  '{titre_session,nouvelle_date}'),
    ('live.session.started',            'live',        '{"fr":"La session commence","en":"Session starting"}',                  '{in_app,push}',  'normal',    '{titre_session,lien_participation}'),
    ('publication.article.moderated',   'publication', '{"fr":"Article modéré","en":"Article moderated"}',                      '{in_app,email}', 'important', '{titre_article,decision}'),
    ('negotiation.document.published',  'negotiation', '{"fr":"Nouveau document de négociation","en":"New negotiation document"}', '{in_app}',    'normal',    '{titre_document}'),
    ('engagement.connection.requested', 'engagement',  '{"fr":"Demande de mise en relation","en":"Connection request"}',        '{in_app}',       'normal',    '{demandeur}'),
    ('engagement.connection.accepted',  'engagement',  '{"fr":"Mise en relation acceptée","en":"Connection accepted"}',         '{in_app}',       'normal',    '{destinataire}'),
    ('engagement.message.received',     'engagement',  '{"fr":"Nouveau message","en":"New message"}',                           '{in_app}',       'normal',    '{expediteur}'),
    ('engagement.comment.replied',      'engagement',  '{"fr":"Réponse à votre commentaire","en":"Reply to your comment"}',     '{in_app}',       'low',       '{auteur}'),
    ('engagement.announcement.general', 'engagement',  '{"fr":"Annonce de la plateforme","en":"Platform announcement"}',        '{in_app,email}', 'low',       '{}')
ON CONFLICT (code) DO NOTHING;

-- Table blanche des cibles commentables. Filtrée par to_regclass afin que ce
-- fichier reste exécutable quel que soit l'ordre d'arrivée des modules.
INSERT INTO engagement.commentable_subjects (subject_schema, subject_table, label, requires_moderation)
SELECT s.subject_schema, s.subject_table, s.label::platform.i18n_text, s.requires_moderation
FROM (VALUES
    ('event',       'events',   '{"fr":"Événement","en":"Event"}',   false),
    ('programme',   'sessions', '{"fr":"Session","en":"Session"}',    false),
    ('publication', 'articles', '{"fr":"Article","en":"Article"}',    true)
) AS s(subject_schema, subject_table, label, requires_moderation)
WHERE to_regclass(format('%I.%I', s.subject_schema, s.subject_table)) IS NOT NULL
ON CONFLICT DO NOTHING;

-- Permissions propres au module (rattachées automatiquement à super_admin par
-- le trigger d'identity ; on les ouvre explicitement à l'administrateur).
INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('engagement.comment.moderate',   '{"fr":"Modérer les commentaires","en":"Moderate comments"}', 'engagement'),
    ('engagement.reminder.manage',    '{"fr":"Paramétrer les rappels","en":"Manage reminder rules"}', 'engagement'),
    ('engagement.template.manage',    '{"fr":"Gérer les modèles de messages","en":"Manage message templates"}', 'engagement'),
    ('engagement.notification.broadcast', '{"fr":"Diffuser une annonce","en":"Broadcast an announcement"}', 'engagement')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin',      'engagement.comment.moderate'),
    ('admin',      'engagement.reminder.manage'),
    ('admin',      'engagement.template.manage'),
    ('admin',      'engagement.notification.broadcast'),
    ('programmer', 'engagement.reminder.manage'),
    ('editor',     'engagement.comment.moderate')
ON CONFLICT DO NOTHING;
