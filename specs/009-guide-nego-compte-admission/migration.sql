-- =============================================================================
-- ePavillon v2 — migration 0b : compte et admission (Guide Négo)
--
-- Du schéma en service AUJOURD'HUI vers celui de docs/database/ après 0b.
--
-- POURQUOI CE FICHIER EXISTE
--   `docs/database/` n'est chargé qu'à la création du volume. Dès qu'il y a un
--   compte en production, recharger efface les données : on migre à la main,
--   selon le § 13 de docs/DEPLOIEMENT.md. 0b touche `identity.sessions`, table
--   en service — ce script est la prochaine application de cette méthode.
--
--   Il vaut AUSSI en local : la base de développement ne se détruit pas. On la
--   sauvegarde, on joue ce script, et SQLx compile contre elle.
--   **`down -v` n'est jamais la procédure.**
--
-- REJOUABLE SANS DÉGÂT
--   Chaque objet est créé sous condition. Le rejouer ne duplique rien, ne perd
--   rien, et ne fait échouer aucune étape déjà passée. C'est ce qui permet de
--   le répéter sur une copie de la production avant de le jouer pour de bon.
--
-- ORDRE
--   1. identity : le type de client et l'appareil sur les sessions
--   2. reference : la taxonomie des réseaux
--   3. negotiation : types, tables, contraintes, index, fonctions, triggers,
--      et les deux vues du back-office
--   4. platform : les réglages
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/ — la base modèle du harnais de test fait l'affaire.
--   Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. identity — la session dit d'où elle vient (ADR-001)
-- -----------------------------------------------------------------------------

DO $$
BEGIN
    CREATE TYPE identity.session_client AS ENUM ('web', 'app');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;

ALTER TABLE identity.sessions
    ADD COLUMN IF NOT EXISTS client_kind     identity.session_client NOT NULL DEFAULT 'web',
    ADD COLUMN IF NOT EXISTS device_id       text,
    ADD COLUMN IF NOT EXISTS device_label    text,
    ADD COLUMN IF NOT EXISTS device_platform text;

CREATE INDEX IF NOT EXISTS ix_sessions_client
    ON identity.sessions (client_kind, issued_at DESC);

COMMENT ON COLUMN identity.sessions.client_kind IS
    'D''où vient la session : "web" (le site) ou "app" (Guide Négo). Déclaré par le client à la connexion et RECOPIÉ À CHAQUE ROTATION — sans cela toute session de l''application redeviendrait "web" au premier renouvellement.';
COMMENT ON COLUMN identity.sessions.device_id IS
    'Identifiant d''installation opaque, engendré par le client. N''ACCORDE AUCUN DROIT et ne borne aucun compteur : il se forge, comme user_agent. Information d''affichage.';
COMMENT ON COLUMN identity.sessions.device_label IS
    'Ce que la personne lit dans la liste de ses appareils : « Android · Chrome ».';
COMMENT ON COLUMN identity.sessions.device_platform IS
    'Plateforme déclarée : android, ios, other.';

-- -----------------------------------------------------------------------------
-- 2. reference — les réseaux sont un vocabulaire ouvert (ADR-007)
--
-- Ni ENUM, ni colonne sur l'identité : personne n'a de case à cocher sur son
-- identité, et l'appartenance vient du code utilisé.
-- -----------------------------------------------------------------------------

-- is_system : le code du terme est lu par l'API (« women_negotiators »), il ne
-- doit pas se renommer librement depuis le back-office.
-- `description` est un platform.i18n_text, pas du texte : une chaîne simple
-- échouerait ici, et le semis de 020_reference.sql porte les deux langues.
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system)
VALUES ('negotiation_network',
        '{"fr":"Réseaux de négociation","en":"Negotiation networks"}'::jsonb,
        '{"fr":"Réseaux auxquels un code d''invitation peut donner l''appartenance","en":"Networks a invitation code may grant membership to"}'::jsonb,
        false, false, true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order)
VALUES ('negotiation_network', 'women_negotiators',
        '{"fr":"Réseau des négociatrices francophones","en":"Francophone women negotiators network"}'::jsonb,
        10)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- LE BLOC QUI SUIT EST LE MÊME TEXTE que le § 2 bis de
-- docs/database/100_negotiations.sql, à l'idempotence près : IF NOT EXISTS,
-- DROP TRIGGER IF EXISTS, types encapsulés. C'est ce qui rend la comparaison de
-- schémas du § 13 exploitable — un mot qui diffère dans un COMMENT ou dans une
-- expression de vue ressortirait comme un écart, et on perdrait le seul
-- contrôle dont on dispose.

-- -----------------------------------------------------------------------------
-- 3. Admission — comment on entre dans l'espace réservé
--
-- Le parcours : une personne reçoit un code d'invitation (souvent sur WhatsApp,
-- recopié à la main), le saisit dans Guide Négo, et obtient le rôle
-- `negotiator` avec sa portée — aussitôt, ou après l'approbation d'un
-- administrateur selon le réglage `negotiation.admission_mode`.
--
-- CE QUI ACCORDE LE DROIT reste `identity.role_assignments`, et lui seul. Les
-- tables ci-dessous sont l'HISTORIQUE (qui est entré avec quel code, qui a
-- demandé, qui a essayé) : aucune ne porte d'état d'accès, parce que deux
-- vérités divergent toujours un jour.
--
-- LA PORTÉE D'UN CODE est celle du rôle `negotiator`, dont les allowed_scopes
-- valent exactement `{global, negotiation_space}` : un code ouvre un espace
-- précis, ou Guide Négo en entier. Rien d'autre n'est offert au back-office.
-- -----------------------------------------------------------------------------

-- 3.1 — Les codes d'invitation
CREATE TABLE IF NOT EXISTS negotiation.invitation_codes (
    id                     uuid    PRIMARY KEY DEFAULT platform.uuid_v7(),
    code                   text    NOT NULL CHECK (code ~ '^[A-Z0-9-]{6,16}$'),
    -- Comparaison insensible à la casse et aux séparateurs : le code circule
    -- recopié à la main depuis WhatsApp, avec ou sans tiret.
    code_normalized        text    GENERATED ALWAYS AS
                                   (upper(regexp_replace(code, '[^A-Za-z0-9]', '', 'g'))) STORED,
    label                  text    NOT NULL,
    scope_type             identity.scope_type NOT NULL,
    space_id               uuid    REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    grants_network_term_id uuid    REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    max_uses               integer,                    -- NULL = sans limite
    used_count             integer NOT NULL DEFAULT 0, -- tenu par trigger, jamais écrit à la main
    valid_from             timestamptz NOT NULL DEFAULT now(),
    valid_until            timestamptz,
    revoked_at             timestamptz,
    revoked_by             uuid    CONSTRAINT xmod_fk_invitation_codes_revoker
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    revoked_reason         text,
    created_by             uuid    CONSTRAINT xmod_fk_invitation_codes_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at             timestamptz NOT NULL DEFAULT now(),
    updated_at             timestamptz NOT NULL DEFAULT now(),
    -- Les deux seules portées offertes, et les deux seules que le rôle
    -- `negotiator` autorise : un espace précis, ou Guide Négo en entier.
    CONSTRAINT ck_invitation_codes_scope CHECK (
        (scope_type = 'global'            AND space_id IS NULL) OR
        (scope_type = 'negotiation_space' AND space_id IS NOT NULL)),
    CONSTRAINT ck_invitation_codes_uses   CHECK (max_uses IS NULL OR max_uses > 0),
    -- Le quota se tient EN BASE. Sans ce CHECK, deux entrées simultanées sur le
    -- dernier usage passeraient toutes les deux : chacune lit `used_count = 119`
    -- avant que l'autre n'écrive. L'incrément du trigger prend le verrou de
    -- ligne, la seconde échoue ici, et l'API la traduit en « code épuisé ».
    CONSTRAINT ck_invitation_codes_quota  CHECK (max_uses IS NULL OR used_count <= max_uses),
    CONSTRAINT ck_invitation_codes_period CHECK (valid_until IS NULL OR valid_until > valid_from),
    -- Un auteur de révocation sans date serait une révocation qu'on ne peut pas
    -- dater — or l'écran de saisie doit dire « révoqué le 8 novembre ».
    CONSTRAINT ck_invitation_codes_revoked CHECK (revoked_by IS NULL OR revoked_at IS NOT NULL)
);

-- Unicité TOTALE, révoqués compris : retrouver un code révoqué est ce qui
-- permet de répondre « ce code a été révoqué le … » plutôt que « inconnu ».
CREATE UNIQUE INDEX IF NOT EXISTS ux_invitation_codes_normalized ON negotiation.invitation_codes (code_normalized);
CREATE INDEX IF NOT EXISTS ix_invitation_codes_space ON negotiation.invitation_codes (space_id, created_at DESC);

DROP TRIGGER IF EXISTS tg_invitation_codes_updated_at ON negotiation.invitation_codes;
CREATE TRIGGER tg_invitation_codes_updated_at BEFORE UPDATE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_invitation_codes_audit ON negotiation.invitation_codes;
CREATE TRIGGER tg_invitation_codes_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
DROP TRIGGER IF EXISTS tg_invitation_codes_check_network ON negotiation.invitation_codes;
CREATE TRIGGER tg_invitation_codes_check_network
    BEFORE INSERT OR UPDATE OF grants_network_term_id ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'grants_network_term_id', 'negotiation_network');

COMMENT ON TABLE negotiation.invitation_codes IS
    'Code d''invitation ouvrant l''espace réservé. N''a pas de colonne d''état : actif, révoqué, épuisé et terminé se dérivent (voir negotiation.v_invitation_codes).';
COMMENT ON COLUMN negotiation.invitation_codes.code_normalized IS
    'Forme de comparaison : majuscules, sans séparateur. « nego-024 », « NEGO 024 » et « Nego024 » désignent le même code.';
COMMENT ON COLUMN negotiation.invitation_codes.scope_type IS
    'Portée accordée par le code, reprise telle quelle dans role_assignments : negotiation_space (un espace) ou global (Guide Négo en entier).';
COMMENT ON COLUMN negotiation.invitation_codes.grants_network_term_id IS
    'Réseau dont le code donne l''appartenance (taxonomie negotiation_network). Aucun champ « genre » n''existe : l''appartenance vient du code, et de rien d''autre.';
COMMENT ON COLUMN negotiation.invitation_codes.used_count IS
    'Tenu par tg_invitation_code_uses_count. Jamais écrit par l''application : c''est l''incrément sous verrou de ligne qui borne le quota.';

-- 3.2 — Qui est entré avec quel code
--
-- Historique, pas droit : comme space_members, cette table n'accorde rien et ne
-- porte AUCUNE colonne de révocation. L'accès effectif se lit dans
-- identity.role_assignments, et le retrait s'y écrit.
CREATE TABLE IF NOT EXISTS negotiation.invitation_code_uses (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    code_id      uuid        NOT NULL REFERENCES negotiation.invitation_codes(id) ON DELETE CASCADE,
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_invitation_code_uses_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    session_id   uuid        CONSTRAINT xmod_fk_invitation_code_uses_session
                             REFERENCES identity.sessions(id) ON DELETE SET NULL,
    used_at      timestamptz NOT NULL DEFAULT now()
);

-- Deux appareils qui saisissent le même code pour la même personne ne produisent
-- qu'un usage : l'opération est idempotente.
CREATE UNIQUE INDEX IF NOT EXISTS ux_invitation_code_uses_person ON negotiation.invitation_code_uses (code_id, person_id);
CREATE INDEX IF NOT EXISTS ix_invitation_code_uses_person ON negotiation.invitation_code_uses (person_id, used_at DESC);

-- L'incrément prend le verrou EXCLUSIF de la ligne du code, et
-- ck_invitation_codes_quota refuse la seconde entrée.
--
-- ATTENTION — L'APPELANT DOIT VERROUILLER LA LIGNE DU CODE AVANT D'INSÉRER :
--     SELECT id FROM negotiation.invitation_codes WHERE id = :code FOR UPDATE;
-- Relevé en éprouvant quatre entrées simultanées, le 21/09. L'insertion d'un
-- usage prend d'abord un verrou PARTAGÉ sur la ligne du code, par la clé
-- étrangère ; l'incrément ci-dessous tente ensuite de le hausser en exclusif.
-- Deux transactions qui détiennent chacune le partagé s'attendent l'une
-- l'autre : PostgreSQL en tue une (40P01), et la personne reçoit une panne au
-- lieu du « code épuisé » qu'on lui promet. Verrouiller d'abord les sérialise
-- dans le bon ordre, sans jamais lire `used_count`.
CREATE OR REPLACE FUNCTION negotiation.tg_invitation_code_uses_count()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE negotiation.invitation_codes
           SET used_count = used_count + 1
         WHERE id = NEW.code_id;
    ELSIF TG_OP = 'DELETE' THEN
        UPDATE negotiation.invitation_codes
           SET used_count = greatest(used_count - 1, 0)
         WHERE id = OLD.code_id;
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_invitation_code_uses_count ON negotiation.invitation_code_uses;
CREATE TRIGGER tg_invitation_code_uses_count
    AFTER INSERT OR DELETE ON negotiation.invitation_code_uses
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_invitation_code_uses_count();

COMMENT ON TABLE negotiation.invitation_code_uses IS
    'Qui est entré avec quel code, et quand. Historique : n''accorde aucun droit et ne porte aucun état de révocation — dupliquer le RBAC, c''est se préparer à deux vérités.';

-- 3.3 — L'appartenance à un réseau
--
-- Elle n'ouvre aucun droit à ce stade : elle sert les chiffres des bailleurs et,
-- plus tard, les canaux réservés. Sa traçabilité passe par le code utilisé.
CREATE TABLE IF NOT EXISTS negotiation.network_memberships (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_network_memberships_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    network_term_id uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    source_code_id  uuid        REFERENCES negotiation.invitation_codes(id) ON DELETE SET NULL,
    joined_at       timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    CONSTRAINT ck_network_memberships_period CHECK (left_at IS NULL OR left_at >= joined_at)
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_network_memberships_active
    ON negotiation.network_memberships (person_id, network_term_id) WHERE left_at IS NULL;
CREATE INDEX IF NOT EXISTS ix_network_memberships_network
    ON negotiation.network_memberships (network_term_id, joined_at DESC) WHERE left_at IS NULL;

DROP TRIGGER IF EXISTS tg_network_memberships_audit ON negotiation.network_memberships;
CREATE TRIGGER tg_network_memberships_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.network_memberships
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
DROP TRIGGER IF EXISTS tg_network_memberships_check_network ON negotiation.network_memberships;
CREATE TRIGGER tg_network_memberships_check_network
    BEFORE INSERT OR UPDATE OF network_term_id ON negotiation.network_memberships
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'network_term_id', 'negotiation_network');

COMMENT ON TABLE negotiation.network_memberships IS
    'Appartenance à un réseau de négociation, venue du code d''invitation utilisé. N''ouvre aucun droit ; sert les chiffres et, plus tard, les canaux réservés.';
COMMENT ON COLUMN negotiation.network_memberships.source_code_id IS
    'Code par lequel l''appartenance est venue : c''est ce qui rend les chiffres du réseau vérifiables.';

-- 3.4 — Les demandes d'accès
--
-- `cancelled` = « annulée », le fait de la personne entrée par un code entre-temps.
-- Ce n'est PAS « révoquée », qui qualifie un accès retiré, jamais une demande.
DO $$
BEGIN
    CREATE TYPE negotiation.access_request_status AS ENUM ('pending', 'approved', 'rejected', 'cancelled');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;
CREATE TABLE IF NOT EXISTS negotiation.access_requests (
    id                 uuid    PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id          uuid    NOT NULL CONSTRAINT xmod_fk_access_requests_person
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    scope_type         identity.scope_type NOT NULL,
    space_id           uuid    REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    -- Mode « les deux » : le code a été reconnu, il n'a pas suffi à ouvrir.
    invitation_code_id uuid    REFERENCES negotiation.invitation_codes(id) ON DELETE SET NULL,
    message            text,
    status             negotiation.access_request_status NOT NULL DEFAULT 'pending',
    submitted_at       timestamptz NOT NULL DEFAULT now(),
    decided_at         timestamptz,
    decided_by         uuid    CONSTRAINT xmod_fk_access_requests_decider
                               REFERENCES identity.people(id) ON DELETE SET NULL,
    decision_reason    text,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_access_requests_scope CHECK (
        (scope_type = 'global'            AND space_id IS NULL) OR
        (scope_type = 'negotiation_space' AND space_id IS NOT NULL)),
    CONSTRAINT ck_access_requests_decision CHECK (
        (status = 'pending') = (decided_at IS NULL))
);

-- « Une seule demande en attente à la fois » se tient en base : deux appareils
-- qui l'envoient ensemble ne produisent qu'une ligne, et l'API traduit le
-- conflit plutôt que de le prévenir par un SELECT préalable.
CREATE UNIQUE INDEX IF NOT EXISTS ux_access_requests_pending_space
    ON negotiation.access_requests (person_id, space_id)
    WHERE status = 'pending' AND space_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ux_access_requests_pending_global
    ON negotiation.access_requests (person_id)
    WHERE status = 'pending' AND space_id IS NULL;
CREATE INDEX IF NOT EXISTS ix_access_requests_queue
    ON negotiation.access_requests (space_id, submitted_at) WHERE status = 'pending';

DROP TRIGGER IF EXISTS tg_access_requests_updated_at ON negotiation.access_requests;
CREATE TRIGGER tg_access_requests_updated_at BEFORE UPDATE ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_access_requests_audit ON negotiation.access_requests;
CREATE TRIGGER tg_access_requests_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

-- La machine à états : depuis `pending` on va vers approved, rejected ou
-- cancelled ; un état final ne se rouvre pas. Une nouvelle demande après un
-- refus est une NOUVELLE LIGNE, ce qui conserve l'historique des décisions.
CREATE OR REPLACE FUNCTION negotiation.tg_access_request_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status IS DISTINCT FROM OLD.status AND OLD.status <> 'pending' THEN
        RAISE EXCEPTION 'Demande d''accès % déjà tranchée (%) : elle ne peut plus passer à %',
            OLD.id, OLD.status, NEW.status
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_access_request_transition ON negotiation.access_requests;
CREATE TRIGGER tg_access_request_transition BEFORE UPDATE OF status ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_access_request_transition();

-- L'événement part DANS la transaction de la décision : le courriel ne peut pas
-- partir avant qu'elle soit enregistrée, et `negotiation` n'appelle jamais
-- `identity` directement.
CREATE OR REPLACE FUNCTION negotiation.tg_access_request_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_action text;
BEGIN
    IF TG_OP = 'INSERT' THEN
        v_action := 'submitted';
    ELSIF NEW.status IS DISTINCT FROM OLD.status AND NEW.status IN ('approved', 'rejected') THEN
        v_action := NEW.status;
    ELSE
        RETURN NULL;
    END IF;

    PERFORM platform.emit_event(
        'negotiation', 'access_request', NEW.id,
        'negotiation.access_request.' || v_action,
        jsonb_build_object('person_id', NEW.person_id, 'scope_type', NEW.scope_type,
                           'space_id', NEW.space_id, 'reason', NEW.decision_reason)
    );
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_access_request_event ON negotiation.access_requests;
CREATE TRIGGER tg_access_request_event AFTER INSERT OR UPDATE OF status ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_access_request_event();

COMMENT ON TABLE negotiation.access_requests IS
    'Demande d''accès à l''espace réservé, quand le mode d''admission exige une approbation. Un état final ne se rouvre pas : une nouvelle demande est une nouvelle ligne.';
COMMENT ON COLUMN negotiation.access_requests.status IS
    'cancelled se dit « annulée » — la personne est entrée par un code entre-temps. « Révoquée » qualifie un accès retiré, jamais une demande.';
COMMENT ON COLUMN negotiation.access_requests.invitation_code_id IS
    'Code reconnu mais insuffisant à ouvrir, en mode « code et approbation ». Sert à l''administrateur qui tranche.';

-- 3.5 — Les essais de code
--
-- Le compte se fait PAR PERSONNE, et par personne seulement. `device_id` vient
-- du corps de la requête : il se forge. Compter « par personne ET par appareil »
-- suffirait à changer d'identifiant pour remettre le compteur à zéro, et la
-- limite ne limiterait rien. L'appareil est gardé comme INFORMATION — il aide un
-- administrateur à lire une série d'échecs —, jamais comme borne.
--
-- Le code essayé n'est JAMAIS stocké, ni en clair ni en empreinte : un essai
-- raté peut être le vrai code d'un autre espace, et cette table est lisible par
-- un administrateur.
DO $$
BEGIN
    CREATE TYPE negotiation.invitation_attempt_outcome AS ENUM (
        'accepted', 'unknown', 'revoked', 'exhausted', 'expired', 'not_yet_valid', 'throttled');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;
CREATE TABLE IF NOT EXISTS negotiation.invitation_code_attempts (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_invitation_attempts_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    device_id    text,
    outcome      negotiation.invitation_attempt_outcome NOT NULL,
    attempted_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS ix_invitation_attempts_window
    ON negotiation.invitation_code_attempts (person_id, attempted_at DESC);

COMMENT ON TABLE negotiation.invitation_code_attempts IS
    'Essais de code, comptés par personne. Gardés 90 jours, purgés par une chaîne récurrente du worker : assez pour constater une série d''échecs, trop court pour devenir un journal de fréquentation.';
COMMENT ON COLUMN negotiation.invitation_code_attempts.device_id IS
    'Information, jamais borne : un client forge son identifiant d''appareil. La limite se compte par personne.';

-- Sa signature ne prend DÉLIBÉRÉMENT pas d'appareil : ce qui n'est pas passé en
-- argument ne peut pas être contourné.
CREATE OR REPLACE FUNCTION negotiation.invitation_attempts_recent(
    p_person_id uuid,
    p_window    interval
)
RETURNS integer
LANGUAGE sql
STABLE
AS $$
    SELECT count(*)::integer
      FROM negotiation.invitation_code_attempts a
     WHERE a.person_id = p_person_id
       -- `throttled` est exclu AVEC `accepted`, et pour une raison qui se
       -- mesure : un essai refusé par la limite est lui-même un essai non
       -- accepté. Le compter ferait repartir la fenêtre à chaque appui, et le
       -- quart d'heure annoncé à l'écran ne finirait jamais — la limite
       -- deviendrait un verrou définitif pour qui insiste. La ligne reste
       -- écrite : l'administrateur doit voir qu'on a continué de frapper.
       AND a.outcome NOT IN ('accepted', 'throttled')
       AND a.attempted_at > now() - p_window;
$$;

COMMENT ON FUNCTION negotiation.invitation_attempts_recent(uuid, interval) IS
    'Essais ayant CONSOMMÉ une tentative, sur la fenêtre donnée : ni les acceptés ni ceux que la limite a déjà refusés. Ne prend pas d''appareil : changer d''identifiant ne remet aucun compteur à zéro.';

-- 3.6 — Les deux vues du back-office
--
-- L'état affiché au back-office et celui qui refuse un code à l'application
-- sortent de LA MÊME expression : deux calculs séparés divergeraient le jour où
-- l'un des deux oublierait `valid_from`.
CREATE OR REPLACE VIEW negotiation.v_invitation_codes AS
SELECT c.*,
       CASE WHEN c.revoked_at IS NOT NULL                          THEN 'revoked'
            WHEN c.valid_until IS NOT NULL AND c.valid_until <= now() THEN 'expired'
            WHEN c.valid_from > now()                              THEN 'not_yet_valid'
            WHEN c.max_uses IS NOT NULL AND c.used_count >= c.max_uses THEN 'exhausted'
            ELSE 'active' END                          AS state,
       s.slug                                          AS space_slug,
       s.name                                          AS space_name,
       n.code                                          AS network_code,
       n.label                                         AS network_label
  FROM negotiation.invitation_codes c
  LEFT JOIN negotiation.spaces s           ON s.id = c.space_id
  LEFT JOIN reference.taxonomy_terms n     ON n.id = c.grants_network_term_id;

COMMENT ON VIEW negotiation.v_invitation_codes IS
    'Un écran, une requête. L''état (active, revoked, expired, not_yet_valid, exhausted) est dérivé ici et nulle part ailleurs.';

-- Dit, ligne par ligne, si la personne entrée par ce code a encore son accès ou
-- quand il lui a été retiré. L'attribution se retrouve par sa portée, celle-là
-- même que le code a accordée.
CREATE OR REPLACE VIEW negotiation.v_invitation_code_uses AS
SELECT u.id,
       u.code_id,
       u.person_id,
       u.session_id,
       u.used_at,
       p.display_name,
       p.primary_email        AS email,
       c.code,
       c.label                AS code_label,
       c.scope_type,
       c.space_id,
       ra.id                  AS role_assignment_id,
       ra.revoked_at          AS access_revoked_at,
       ra.revoked_reason      AS access_revoked_reason,
       (ra.id IS NOT NULL
        AND ra.revoked_at IS NULL
        AND (ra.valid_until IS NULL OR ra.valid_until > now())) AS access_active
  FROM negotiation.invitation_code_uses u
  JOIN negotiation.invitation_codes c ON c.id = u.code_id
  JOIN identity.people p              ON p.id = u.person_id
  LEFT JOIN LATERAL (
      SELECT r.id, r.revoked_at, r.revoked_reason, r.valid_until
        FROM identity.role_assignments r
       WHERE r.person_id  = u.person_id
         AND r.role_code  = 'negotiator'
         AND r.scope_type = c.scope_type
         AND r.scope_id IS NOT DISTINCT FROM c.space_id
       ORDER BY r.granted_at DESC
       LIMIT 1
  ) ra ON true;

COMMENT ON VIEW negotiation.v_invitation_code_uses IS
    'Usages d''un code avec l''état réel de l''accès, lu dans identity.role_assignments : la table des usages ne porte aucun état, c''est ici qu''il se joint.';

-- -----------------------------------------------------------------------------
-- 4. platform — les réglages (ADR-006)
--
-- Le mode d'admission est un RÉGLAGE et non un drapeau : platform.feature_flags
-- n'ouvre et ne ferme qu'en binaire, et il y a trois valeurs.
-- -----------------------------------------------------------------------------

INSERT INTO platform.settings (key, value, description, is_secret) VALUES
    ('negotiation.admission_mode', '"code"'::jsonb,
     'Comment on entre dans les modules réservés : "code", "approval" ou "code_and_approval". Modifiable depuis le back-office, sans redéploiement.',
     false),
    ('negotiation.invitation_attempts',
     '{"max": 5, "window_minutes": 15, "lock_minutes": 15}'::jsonb,
     'Limite des essais de code d''invitation, comptés par personne — jamais par appareil, qui se forge.',
     false)
ON CONFLICT (key) DO NOTHING;

COMMIT;


-- =============================================================================
-- APRÈS : le contrôle du § 13, étape 6
--
--   pg_dump --schema-only --no-owner --no-privileges "$MIGREE"  | sort > /tmp/migree.sql
--   pg_dump --schema-only --no-owner --no-privileges "$MODELE"  | sort > /tmp/modele.sql
--   diff /tmp/modele.sql /tmp/migree.sql
--
-- $MODELE : une base chargée depuis docs/database/ — la base modèle du harnais
-- de test (`epavillon_test_template_<empreinte>`) fait l'affaire, elle se
-- reconstruit seule dès que le SQL change.
--
-- Écart connu qui n'en est pas : engagement.email_messages_AAAAMM, partitions
-- créées à la volée par le worker.
-- =============================================================================
