-- =============================================================================
-- ePavillon v2 — 030_identity.sql
-- Module Identité : personnes, comptes, authentification, RBAC scopé, RGPD.
--
-- Dépend de : 000_bootstrap.sql, 010_platform.sql, 020_reference.sql
--
-- DÉCISION STRUCTURANTE — séparer la PERSONNE du COMPTE
-- La v1 confondait les deux dans `public.users`, adossé à `auth.users` de
-- Supabase. Trois douleurs en ont découlé :
--   1. les inscriptions « invité » ont dû être greffées dans
--      `activity_registrations` avec six colonnes `guest_*` et deux CHECK
--      croisés, sans jamais pouvoir rattacher l'invité à son futur compte ;
--   2. les intervenants (`activity_speakers`) redupliquaient nom, prénom,
--      email et photo à chaque activité — le même expert apparaissait dix fois ;
--   3. l'anonymisation RGPD d'un compte détruisait des données de participation
--      qu'il fallait conserver à des fins statistiques.
--
-- En v2 :
--   identity.people   = une personne physique, identifiée par son email canonique.
--                       Existe sans compte (invité, intervenant saisi par un tiers).
--   identity.accounts = les moyens de se connecter rattachés à une personne.
--
-- Toute la plateforme (inscriptions, intervenants, rôles, messages) référence
-- `people`. La création d'un compte devient un simple enrichissement.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Personnes
-- -----------------------------------------------------------------------------
CREATE TYPE identity.person_status AS ENUM (
    'active',
    'suspended',   -- accès restreint temporairement, données conservées
    'blocked',     -- exclusion durable prononcée par un administrateur
    'anonymized'   -- droit à l'effacement exercé : identité purgée, agrégats conservés
);

CREATE TABLE identity.people (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    -- Email canonique : clé de rapprochement naturelle entre un invité et un
    -- compte créé plus tard. `citext` rend la comparaison insensible à la casse.
    primary_email      platform.email NOT NULL,
    email_verified_at  timestamptz,

    first_name         text        NOT NULL CHECK (length(btrim(first_name)) > 0),
    last_name          text        NOT NULL CHECK (length(btrim(last_name)) > 0),
    civility           text        CHECK (civility IN ('mme', 'm', 'dr', 'pr', 'other')),
    display_name       text        GENERATED ALWAYS AS (btrim(first_name || ' ' || last_name)) STORED,

    phone              text,
    job_title          text,
    biography          platform.i18n_text,
    country_id         uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    city               text,
    preferred_locale   text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    timezone           platform.timezone_name NOT NULL DEFAULT 'UTC',

    -- Organisation de rattachement principale. Cross-module assumé : nommage
    -- `xmod_fk_` pour être identifiable lors d'une extraction de service.
    primary_organization_id uuid,

    status             identity.person_status NOT NULL DEFAULT 'active',
    status_reason      text,
    status_changed_by  uuid        REFERENCES identity.people(id) ON DELETE SET NULL,
    status_changed_at  timestamptz,
    suspended_until    timestamptz,

    -- Visibilité dans l'annuaire / le réseautage.
    is_directory_visible boolean   NOT NULL DEFAULT true,

    -- Recherche plein texte (annuaire back-office et public).
    search_vector      tsvector    GENERATED ALWAYS AS (
        to_tsvector('french',
            coalesce(first_name, '') || ' ' ||
            coalesce(last_name, '')  || ' ' ||
            coalesce(job_title, ''))
    ) STORED,

    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_people_suspension_window
        CHECK (status <> 'suspended' OR suspended_until IS NOT NULL)
);

-- Unicité de l'email sur les personnes vivantes uniquement : une personne
-- anonymisée libère son adresse.
CREATE UNIQUE INDEX ux_people_primary_email
    ON identity.people (primary_email)
    WHERE status <> 'anonymized';

CREATE INDEX ix_people_search        ON identity.people USING gin (search_vector);
CREATE INDEX ix_people_organization  ON identity.people (primary_organization_id) WHERE primary_organization_id IS NOT NULL;
CREATE INDEX ix_people_country       ON identity.people (country_id);

CREATE TRIGGER tg_people_updated_at
    BEFORE UPDATE ON identity.people
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_people_audit
    AFTER INSERT OR UPDATE OR DELETE ON identity.people
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE identity.people IS
    'Personne physique connue de la plateforme, avec ou sans compte. Référencée par tous les modules.';

-- Adresses secondaires (email professionnel, adresse de contact d'un
-- intervenant). Sert aussi au rapprochement : une inscription arrivant sur une
-- adresse secondaire retrouve la bonne personne.
CREATE TABLE identity.person_emails (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id   uuid        NOT NULL REFERENCES identity.people(id) ON DELETE CASCADE,
    email       platform.email NOT NULL,
    label       text        NOT NULL DEFAULT 'secondary' CHECK (label IN ('secondary', 'professional', 'legacy')),
    verified_at timestamptz,
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_person_emails UNIQUE (email)
);

-- -----------------------------------------------------------------------------
-- 2. Comptes et authentification
--
-- L'auth quitte Supabase pour l'API Rust : mot de passe haché en Argon2id
-- (jamais stocké en clair ni réversible), plus fédération OAuth/OIDC pour les
-- comptes institutionnels. Une personne peut cumuler plusieurs fournisseurs.
-- -----------------------------------------------------------------------------
CREATE TYPE identity.auth_provider AS ENUM ('password', 'google', 'microsoft', 'linkedin', 'oidc');

CREATE TABLE identity.accounts (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id             uuid        NOT NULL REFERENCES identity.people(id) ON DELETE CASCADE,
    provider              identity.auth_provider NOT NULL,
    -- Identifiant chez le fournisseur (NULL pour 'password' : c'est l'email qui sert).
    provider_subject      text,
    password_hash         text,       -- Argon2id, uniquement si provider = 'password'
    password_changed_at   timestamptz,
    -- Second facteur (TOTP). Le secret est chiffré côté application avant écriture.
    mfa_secret_encrypted  bytea,
    mfa_enabled_at        timestamptz,
    mfa_recovery_codes    text[],     -- hachés
    last_login_at         timestamptz,
    failed_attempts       smallint    NOT NULL DEFAULT 0,
    locked_until          timestamptz,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_accounts_password_shape
        CHECK ((provider = 'password' AND password_hash IS NOT NULL AND provider_subject IS NULL)
            OR (provider <> 'password' AND provider_subject IS NOT NULL)),
    CONSTRAINT ux_accounts_provider_subject UNIQUE (provider, provider_subject)
);

-- Un seul compte mot de passe par personne.
CREATE UNIQUE INDEX ux_accounts_password_per_person
    ON identity.accounts (person_id)
    WHERE provider = 'password';

CREATE INDEX ix_accounts_person ON identity.accounts (person_id);

CREATE TRIGGER tg_accounts_updated_at
    BEFORE UPDATE ON identity.accounts
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON COLUMN identity.accounts.password_hash IS
    'Empreinte Argon2id calculée par l''API. Aucune fonction SQL ne doit vérifier de mot de passe.';

-- Sessions : jetons de rafraîchissement, révocables individuellement
-- (déconnexion d'un appareil) ou en masse (compromission).
CREATE TABLE identity.sessions (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id          uuid        NOT NULL REFERENCES identity.people(id) ON DELETE CASCADE,
    account_id         uuid        REFERENCES identity.accounts(id) ON DELETE SET NULL,
    -- Empreinte SHA-256 du jeton : un vol de la base ne donne aucun jeton utilisable.
    refresh_token_hash bytea       NOT NULL UNIQUE,
    user_agent         text,
    ip_address         inet,
    issued_at          timestamptz NOT NULL DEFAULT now(),
    expires_at         timestamptz NOT NULL,
    last_seen_at       timestamptz NOT NULL DEFAULT now(),
    revoked_at         timestamptz,
    revoked_reason     text
);

CREATE INDEX ix_sessions_person_active
    ON identity.sessions (person_id, expires_at DESC)
    WHERE revoked_at IS NULL;

-- Jetons à usage unique : vérification d'email, réinitialisation de mot de
-- passe, invitation, lien magique d'inscription rapide (cas PACO).
CREATE TYPE identity.token_purpose AS ENUM (
    'email_verification', 'password_reset', 'invitation', 'magic_link', 'speaker_confirmation'
);

CREATE TABLE identity.one_time_tokens (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id    uuid        REFERENCES identity.people(id) ON DELETE CASCADE,
    purpose      identity.token_purpose NOT NULL,
    token_hash   bytea       NOT NULL UNIQUE,
    payload      jsonb       NOT NULL DEFAULT '{}'::jsonb,
    expires_at   timestamptz NOT NULL,
    consumed_at  timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_one_time_tokens_cleanup ON identity.one_time_tokens (expires_at) WHERE consumed_at IS NULL;

-- -----------------------------------------------------------------------------
-- 3. RBAC scopé
--
-- La v1 posait un ENUM de huit rôles globaux : être « révisionniste » valait
-- pour toutes les COP passées et à venir, et un point focal d'organisation
-- n'existait pas. En v2, une attribution de rôle porte une PORTÉE :
--   global                      → administration de la plateforme
--   organization:<uuid>         → référent d'une organisation
--   event:<uuid>                → comité de sélection d'une COP donnée
--   negotiation_space:<uuid>    → accès à un espace de négociation
-- Les permissions sont des chaînes `module.ressource.action`, agrégées depuis
-- les rôles : le code applicatif teste une permission, jamais un nom de rôle.
-- -----------------------------------------------------------------------------
CREATE TABLE identity.permissions (
    code        text PRIMARY KEY CHECK (code ~ '^[a-z_]+\.[a-z_]+\.[a-z_]+$'),
    label       platform.i18n_text NOT NULL,
    module_code text NOT NULL REFERENCES platform.modules(code) ON DELETE CASCADE
);

CREATE TABLE identity.roles (
    code           text PRIMARY KEY CHECK (code ~ '^[a-z][a-z0-9_]*$'),
    label          platform.i18n_text NOT NULL,
    description    platform.i18n_text,
    -- Portées sur lesquelles ce rôle peut être attribué.
    allowed_scopes text[] NOT NULL DEFAULT '{global}',
    -- Rôle système : non supprimable, garantit qu'un administrateur subsiste.
    is_system      boolean NOT NULL DEFAULT false,
    created_at     timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE identity.role_permissions (
    role_code       text NOT NULL REFERENCES identity.roles(code) ON DELETE CASCADE,
    permission_code text NOT NULL REFERENCES identity.permissions(code) ON DELETE CASCADE,
    PRIMARY KEY (role_code, permission_code)
);

CREATE TYPE identity.scope_type AS ENUM ('global', 'organization', 'event', 'negotiation_space');

CREATE TABLE identity.role_assignments (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id    uuid        NOT NULL REFERENCES identity.people(id) ON DELETE CASCADE,
    role_code    text        NOT NULL REFERENCES identity.roles(code) ON DELETE CASCADE,
    scope_type   identity.scope_type NOT NULL DEFAULT 'global',
    -- NULL si scope_type = 'global'. Pas de FK : la cible vit dans un autre
    -- module et peut devenir un service distant.
    scope_id     uuid,
    granted_by   uuid        REFERENCES identity.people(id) ON DELETE SET NULL,
    granted_at   timestamptz NOT NULL DEFAULT now(),
    valid_from   timestamptz NOT NULL DEFAULT now(),
    valid_until  timestamptz,
    revoked_at   timestamptz,
    note         text,

    CONSTRAINT ck_role_assignment_scope
        CHECK ((scope_type = 'global' AND scope_id IS NULL)
            OR (scope_type <> 'global' AND scope_id IS NOT NULL)),
    CONSTRAINT ck_role_assignment_window
        CHECK (valid_until IS NULL OR valid_until > valid_from)
);

CREATE UNIQUE INDEX ux_role_assignments_active
    ON identity.role_assignments (person_id, role_code, scope_type, COALESCE(scope_id, '00000000-0000-0000-0000-000000000000'::uuid))
    WHERE revoked_at IS NULL;

CREATE INDEX ix_role_assignments_person ON identity.role_assignments (person_id) WHERE revoked_at IS NULL;
CREATE INDEX ix_role_assignments_scope  ON identity.role_assignments (scope_type, scope_id) WHERE revoked_at IS NULL;

CREATE TRIGGER tg_role_assignments_audit
    AFTER INSERT OR UPDATE OR DELETE ON identity.role_assignments
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

-- Un rôle ne peut être attribué que sur une portée qu'il autorise : attribuer
-- « référent d'organisation » globalement, ou « super administrateur » sur un
-- seul événement, n'a pas de sens et ouvrirait une faille silencieuse.
CREATE OR REPLACE FUNCTION identity.tg_check_role_scope()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_allowed text[];
BEGIN
    SELECT allowed_scopes INTO v_allowed FROM identity.roles WHERE code = NEW.role_code;

    IF NOT (NEW.scope_type::text = ANY (v_allowed)) THEN
        RAISE EXCEPTION 'Le rôle « % » ne peut pas être attribué sur la portée « % » (portées autorisées : %).',
            NEW.role_code, NEW.scope_type, array_to_string(v_allowed, ', ')
            USING ERRCODE = 'restrict_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_role_assignments_check_scope
    BEFORE INSERT OR UPDATE OF role_code, scope_type ON identity.role_assignments
    FOR EACH ROW EXECUTE FUNCTION identity.tg_check_role_scope();

-- Vérification d'une permission, portée comprise. Une attribution globale
-- couvre toutes les portées ; une attribution ciblée ne vaut que pour sa cible.
CREATE OR REPLACE FUNCTION identity.has_permission(
    p_person_id  uuid,
    p_permission text,
    p_scope_type identity.scope_type DEFAULT 'global',
    p_scope_id   uuid DEFAULT NULL
)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT EXISTS (
        SELECT 1
        FROM identity.role_assignments ra
        JOIN identity.role_permissions rp ON rp.role_code = ra.role_code
        WHERE ra.person_id = p_person_id
          AND rp.permission_code = p_permission
          AND ra.revoked_at IS NULL
          AND ra.valid_from <= now()
          AND (ra.valid_until IS NULL OR ra.valid_until > now())
          AND (
                ra.scope_type = 'global'
             OR (ra.scope_type = p_scope_type AND ra.scope_id IS NOT DISTINCT FROM p_scope_id)
          )
    );
$$;

COMMENT ON FUNCTION identity.has_permission IS
    'Test d''autorisation unique de la plateforme. Le code applicatif teste une permission, jamais un rôle.';

-- Périmètre d'administration d'une personne. Sert au filtrage systématique des
-- listes du back-office : un administrateur d'événement ne doit jamais voir,
-- même par accident d'URL, les propositions d'une autre édition.
--
-- La fonction agrège sans GROUP BY : elle renvoie TOUJOURS exactement une
-- ligne, y compris pour une personne dépourvue de toute attribution. Ses deux
-- colonnes sont donc renvoyées NON NULLES par construction, et le cas « aucun
-- droit » s'écrit `is_global = false` et `event_ids = '{}'`.
--
-- Une version antérieure laissait les agrégats retourner NULL sur zéro ligne.
-- Deux défauts en découlaient :
--   - un garde de back-office testant `NOT is_global` confondait « aucun
--     droit » et « administrateur d'un seul événement », et affichait une liste
--     vide là où il fallait un refus d'accès ;
--   - `event_id = ANY(NULL)` vaut NULL, donc le filtre canonique ne renvoyait
--     rien SANS erreur — un contrôle d'accès qui échoue en silence.
--
-- Le filtre canonique, côté API comme en SQL :
--     WHERE is_global OR event_id = ANY(event_ids)
-- Quand `is_global` vaut vrai, `event_ids` n'est plus signifiant : une
-- attribution globale couvre toutes les éditions, présentes et à venir.
CREATE OR REPLACE FUNCTION identity.administered_events(p_person_id uuid)
RETURNS TABLE (is_global boolean, event_ids uuid[])
LANGUAGE sql
STABLE
AS $$
    SELECT
        COALESCE(bool_or(ra.scope_type = 'global'), false),
        -- DISTINCT : cumuler `admin` et `programmer` sur la même édition ne doit
        -- pas la faire compter deux fois dans le périmètre.
        COALESCE(
            array_agg(DISTINCT ra.scope_id) FILTER (WHERE ra.scope_type = 'event'),
            '{}'::uuid[]
        )
    FROM identity.role_assignments ra
    JOIN identity.role_permissions rp ON rp.role_code = ra.role_code
    WHERE ra.person_id = p_person_id
      AND rp.permission_code = 'programme.proposal.read_all'
      AND ra.revoked_at IS NULL
      AND ra.valid_from <= now()
      AND (ra.valid_until IS NULL OR ra.valid_until > now());
$$;

COMMENT ON FUNCTION identity.administered_events IS
    'Périmètre d''administration : global, ou liste des événements confiés. Renvoie toujours une ligne, jamais de NULL — aucun droit s''écrit (false, {}). Filtre obligatoire de toutes les listes du back-office.';

-- Permissions effectives d'une personne : consommé par l'API pour composer le
-- jeton d'accès, et par le back-office pour afficher/masquer les entrées de menu.
CREATE OR REPLACE FUNCTION identity.effective_permissions(p_person_id uuid)
RETURNS TABLE (permission_code text, scope_type identity.scope_type, scope_id uuid)
LANGUAGE sql
STABLE
AS $$
    SELECT DISTINCT rp.permission_code, ra.scope_type, ra.scope_id
    FROM identity.role_assignments ra
    JOIN identity.role_permissions rp ON rp.role_code = ra.role_code
    WHERE ra.person_id = p_person_id
      AND ra.revoked_at IS NULL
      AND ra.valid_from <= now()
      AND (ra.valid_until IS NULL OR ra.valid_until > now());
$$;

-- -----------------------------------------------------------------------------
-- 4. Profil négociateur
--
-- Conservé comme extension de profil (et non comme rôle) : les désignations
-- annuelles, spécialisations et participations aux COP sont des données
-- métier, l'accès à l'espace de négociation reste géré par le RBAC.
-- -----------------------------------------------------------------------------
CREATE TABLE identity.negotiator_profiles (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id             uuid        NOT NULL REFERENCES identity.people(id) ON DELETE CASCADE,
    country_id            uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    is_unfccc_focal_point boolean     NOT NULL DEFAULT false,
    specializations       text[]      NOT NULL DEFAULT '{}',   -- codes de reference.taxonomy_terms
    notes                 text,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_negotiator_profiles_person UNIQUE (person_id)
);

CREATE TRIGGER tg_negotiator_profiles_updated_at
    BEFORE UPDATE ON identity.negotiator_profiles
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TABLE identity.negotiator_designations (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    profile_id     uuid        NOT NULL REFERENCES identity.negotiator_profiles(id) ON DELETE CASCADE,
    year           smallint    NOT NULL CHECK (year BETWEEN 2000 AND 2100),
    track_code     text        NOT NULL DEFAULT 'climate',
    designated_by  uuid        REFERENCES identity.people(id) ON DELETE SET NULL,
    designated_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_negotiator_designations UNIQUE (profile_id, year, track_code)
);

-- -----------------------------------------------------------------------------
-- 5. RGPD : consentements, export, effacement
--
-- Absent de la v1. Une plateforme portée par une organisation internationale
-- traitant des données de ressortissants européens et africains doit pouvoir
-- prouver le consentement et honorer les demandes en 30 jours.
-- -----------------------------------------------------------------------------
CREATE TABLE identity.consents (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id    uuid        NOT NULL REFERENCES identity.people(id) ON DELETE CASCADE,
    purpose      text        NOT NULL,   -- 'newsletter', 'directory_listing', 'photo_usage', 'analytics'
    is_granted   boolean     NOT NULL,
    policy_version text      NOT NULL,
    source       text,                   -- 'registration_form', 'profile_settings', 'import'
    ip_address   inet,
    recorded_at  timestamptz NOT NULL DEFAULT now()
);

-- Historique complet conservé (preuve) ; l'état courant se lit par cette vue.
CREATE OR REPLACE VIEW identity.current_consents AS
SELECT DISTINCT ON (person_id, purpose)
    person_id, purpose, is_granted, policy_version, recorded_at
FROM identity.consents
ORDER BY person_id, purpose, recorded_at DESC;

CREATE TYPE identity.privacy_request_type AS ENUM ('export', 'erasure', 'rectification');
CREATE TYPE identity.privacy_request_status AS ENUM ('received', 'in_progress', 'completed', 'rejected');

CREATE TABLE identity.privacy_requests (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id    uuid        NOT NULL REFERENCES identity.people(id) ON DELETE CASCADE,
    request_type identity.privacy_request_type NOT NULL,
    status       identity.privacy_request_status NOT NULL DEFAULT 'received',
    -- Échéance réglementaire : un mois à compter de la réception.
    -- Valeur par défaut plutôt que colonne générée : `timestamptz + interval`
    -- est STABLE (le résultat dépend du fuseau de session), donc interdit dans
    -- une expression GENERATED.
    due_at       timestamptz NOT NULL DEFAULT (now() + interval '30 days'),
    handled_by   uuid        REFERENCES identity.people(id) ON DELETE SET NULL,
    resolution   text,
    result_asset_id uuid,    -- media.assets.id de l'archive d'export
    created_at   timestamptz NOT NULL DEFAULT now(),
    completed_at timestamptz
);

CREATE INDEX ix_privacy_requests_open ON identity.privacy_requests (due_at) WHERE status IN ('received', 'in_progress');

-- Anonymisation : l'identité disparaît, les traces d'activité restent
-- exploitables statistiquement. Les compteurs d'inscriptions et les moyennes de
-- notation d'une COP passée ne doivent pas s'effondrer parce qu'une personne
-- exerce son droit à l'effacement.
CREATE OR REPLACE FUNCTION identity.anonymize_person(p_person_id uuid, p_reason text DEFAULT NULL)
RETURNS void
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = identity, platform, pg_temp
AS $$
DECLARE
    v_token text := left(replace(p_person_id::text, '-', ''), 12);
BEGIN
    UPDATE identity.people
    SET first_name           = 'Utilisateur',
        last_name            = 'anonymisé ' || v_token,
        primary_email        = ('anonymized+' || v_token || '@invalid.local')::platform.email,
        phone                = NULL,
        job_title            = NULL,
        biography            = NULL,
        city                 = NULL,
        is_directory_visible = false,
        status               = 'anonymized',
        status_reason        = COALESCE(p_reason, 'Demande d''effacement RGPD'),
        status_changed_at    = now(),
        status_changed_by    = platform.current_actor_id()
    WHERE id = p_person_id;

    DELETE FROM identity.person_emails WHERE person_id = p_person_id;
    DELETE FROM identity.accounts      WHERE person_id = p_person_id;
    UPDATE identity.sessions SET revoked_at = now(), revoked_reason = 'anonymization'
     WHERE person_id = p_person_id AND revoked_at IS NULL;

    PERFORM platform.emit_event(
        'identity', 'person', p_person_id, 'identity.person.anonymized',
        jsonb_build_object('reason', p_reason)
    );
END;
$$;

COMMENT ON FUNCTION identity.anonymize_person IS
    'Efface les données identifiantes en conservant l''identifiant technique et les agrégats de participation.';

-- -----------------------------------------------------------------------------
-- 6. Amorçage des rôles et permissions
-- -----------------------------------------------------------------------------
INSERT INTO identity.roles (code, label, description, allowed_scopes, is_system) VALUES
    ('super_admin',   '{"fr":"Super administrateur","en":"Super administrator"}', '{"fr":"Contrôle total de la plateforme. Seul rôle sans restriction de portée.","en":"Full platform control"}', '{global}', true),
    -- L'administrateur est attribuable GLOBALEMENT ou SUR UN SEUL ÉVÉNEMENT.
    -- C'est la réponse au cas PACO : la v1 avait dû créer une page
    -- d'administration séparée, développée dans l'urgence et en partie codée en
    -- dur, uniquement pour que le responsable du webinaire n'ait pas accès au
    -- reste de la plateforme. Ici, c'est une attribution de rôle : même
    -- back-office, même code, périmètre restreint à son événement et à ce qui
    -- en dépend (propositions, sessions, inscriptions, diffusions, incidents).
    ('admin',         '{"fr":"Administrateur","en":"Administrator"}',             '{"fr":"Administration courante, globale ou limitée à un événement","en":"Day-to-day administration, global or scoped to one event"}', '{global,event}', true),
    ('reviewer',      '{"fr":"Révisionniste","en":"Reviewer"}',                   '{"fr":"Évalue et note les propositions d''activité","en":"Assesses and scores activity proposals"}', '{global,event}', true),
    ('programmer',    '{"fr":"Programmateur","en":"Programme manager"}',          '{"fr":"Planifie les créneaux et publie la programmation","en":"Schedules slots and publishes the programme"}', '{global,event}', true),
    ('org_manager',   '{"fr":"Référent d''organisation","en":"Organization manager"}', '{"fr":"Gère les membres et les soumissions de son organisation","en":"Manages members and submissions"}', '{organization}', false),
    ('org_member',    '{"fr":"Membre d''organisation","en":"Organization member"}',    NULL, '{organization}', false),
    ('negotiator',    '{"fr":"Négociateur","en":"Negotiator"}',                   '{"fr":"Accède à l''espace de négociation","en":"Access to the negotiation space"}', '{global,negotiation_space}', true),
    ('trainer',       '{"fr":"Formateur","en":"Trainer"}',                        NULL, '{global}', false),
    ('editor',        '{"fr":"Éditeur","en":"Editor"}',                           '{"fr":"Modère les publications des organisations","en":"Moderates organization publications"}', '{global}', false),
    ('standard',      '{"fr":"Utilisateur","en":"Standard user"}',                NULL, '{global}', true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('identity.person.read',        '{"fr":"Consulter les utilisateurs","en":"Read users"}', 'identity'),
    ('identity.person.manage',      '{"fr":"Gérer les utilisateurs","en":"Manage users"}', 'identity'),
    ('identity.role.assign',        '{"fr":"Attribuer des rôles","en":"Assign roles"}', 'identity'),
    ('org.organization.read',       '{"fr":"Consulter les organisations","en":"Read organizations"}', 'org'),
    ('org.organization.manage',     '{"fr":"Gérer les organisations","en":"Manage organizations"}', 'org'),
    ('org.organization.merge',      '{"fr":"Fusionner des organisations","en":"Merge organizations"}', 'org'),
    ('event.event.manage',          '{"fr":"Gérer les événements","en":"Manage events"}', 'event'),
    ('event.call.manage',           '{"fr":"Gérer les appels à propositions","en":"Manage calls for proposals"}', 'event'),
    ('programme.proposal.submit',   '{"fr":"Soumettre une proposition","en":"Submit a proposal"}', 'programme'),
    ('programme.proposal.read_all', '{"fr":"Consulter toutes les propositions","en":"Read all proposals"}', 'programme'),
    ('programme.review.write',      '{"fr":"Noter et commenter une proposition","en":"Score and comment a proposal"}', 'programme'),
    ('programme.proposal.decide',   '{"fr":"Retenir ou rejeter une proposition","en":"Accept or reject a proposal"}', 'programme'),
    ('programme.session.schedule',  '{"fr":"Planifier les sessions","en":"Schedule sessions"}', 'programme'),
    ('programme.registration.manage','{"fr":"Gérer les inscriptions","en":"Manage registrations"}', 'programme'),
    ('live.meeting.manage',         '{"fr":"Gérer les réunions et diffusions","en":"Manage meetings and streams"}', 'live'),
    ('live.incident.publish',       '{"fr":"Publier un message d''incident","en":"Publish an incident message"}', 'live'),
    ('publication.article.write',   '{"fr":"Rédiger un article","en":"Write an article"}', 'publication'),
    ('publication.article.moderate','{"fr":"Modérer les articles","en":"Moderate articles"}', 'publication'),
    ('publication.quota.manage',    '{"fr":"Définir les quotas de publication","en":"Set publication quotas"}', 'publication'),
    ('negotiation.space.access',    '{"fr":"Accéder à l''espace de négociation","en":"Access the negotiation space"}', 'negotiation'),
    ('negotiation.content.manage',  '{"fr":"Gérer les contenus de négociation","en":"Manage negotiation content"}', 'negotiation'),
    ('engagement.campaign.send',    '{"fr":"Envoyer des campagnes","en":"Send campaigns"}', 'engagement'),
    ('tool.survey.manage',          '{"fr":"Gérer les sondages","en":"Manage surveys"}', 'tool'),
    ('analytics.dashboard.read',    '{"fr":"Consulter les tableaux de bord","en":"Read dashboards"}', 'analytics')
ON CONFLICT (code) DO NOTHING;

-- Le super administrateur détient toutes les permissions, y compris celles
-- ajoutées ultérieurement (voir le trigger qui suit).
INSERT INTO identity.role_permissions (role_code, permission_code)
SELECT 'super_admin', code FROM identity.permissions
ON CONFLICT DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin', 'identity.person.read'), ('admin', 'identity.person.manage'),
    ('admin', 'org.organization.read'), ('admin', 'org.organization.manage'), ('admin', 'org.organization.merge'),
    ('admin', 'event.event.manage'), ('admin', 'event.call.manage'),
    ('admin', 'programme.proposal.read_all'), ('admin', 'programme.proposal.decide'),
    ('admin', 'programme.session.schedule'), ('admin', 'programme.registration.manage'),
    ('admin', 'live.meeting.manage'), ('admin', 'live.incident.publish'),
    ('admin', 'publication.article.moderate'), ('admin', 'publication.quota.manage'),
    ('admin', 'engagement.campaign.send'), ('admin', 'analytics.dashboard.read'),
    ('reviewer', 'programme.proposal.read_all'), ('reviewer', 'programme.review.write'),
    ('reviewer', 'org.organization.read'), ('reviewer', 'analytics.dashboard.read'),
    ('programmer', 'programme.proposal.read_all'), ('programmer', 'programme.session.schedule'),
    ('programmer', 'live.meeting.manage'), ('programmer', 'live.incident.publish'),
    ('org_manager', 'programme.proposal.submit'), ('org_manager', 'publication.article.write'),
    ('org_member', 'programme.proposal.submit'),
    ('negotiator', 'negotiation.space.access'),
    ('editor', 'publication.article.moderate'),
    ('standard', 'org.organization.read')
ON CONFLICT DO NOTHING;

-- Toute permission créée plus tard est automatiquement rattachée au rôle
-- super_admin : impossible d'introduire une fonctionnalité que personne ne peut
-- administrer.
CREATE OR REPLACE FUNCTION identity.tg_grant_new_permission_to_super_admin()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO identity.role_permissions (role_code, permission_code)
    VALUES ('super_admin', NEW.code)
    ON CONFLICT DO NOTHING;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_permissions_grant_super_admin
    AFTER INSERT ON identity.permissions
    FOR EACH ROW EXECUTE FUNCTION identity.tg_grant_new_permission_to_super_admin();
