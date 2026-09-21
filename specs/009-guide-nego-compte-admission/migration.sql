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
--   3. negotiation : types, tables, contraintes, index, fonctions, triggers
--   4. negotiation : les deux vues du back-office
--   5. platform : les réglages
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
INSERT INTO reference.taxonomies (code, label, description, is_system)
VALUES ('negotiation_network',
        '{"fr":"Réseaux de négociation","en":"Negotiation networks"}'::jsonb,
        'Réseaux auxquels un code d''invitation peut donner l''appartenance (ADR-007).',
        true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order)
VALUES ('negotiation_network', 'women_negotiators',
        '{"fr":"Réseau des négociatrices francophones","en":"Francophone women negotiators network"}'::jsonb,
        10)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 3. negotiation — types
-- -----------------------------------------------------------------------------

DO $$
BEGIN
    CREATE TYPE negotiation.access_request_status AS ENUM
        ('pending', 'approved', 'rejected', 'cancelled');
    -- 'cancelled' se dit « annulée » : le fait de la personne entrée par un code
    -- entre-temps. « Révoquée » qualifie un accès retiré, jamais une demande.
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;

DO $$
BEGIN
    CREATE TYPE negotiation.invitation_attempt_outcome AS ENUM
        ('accepted', 'unknown', 'revoked', 'exhausted', 'expired', 'not_yet_valid', 'throttled');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END
$$;

-- -----------------------------------------------------------------------------
-- 3.1 Les codes d'invitation
-- -----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS negotiation.invitation_codes (
    id                     uuid    PRIMARY KEY DEFAULT platform.uuid_v7(),
    code                   text    NOT NULL CHECK (code ~ '^[A-Z0-9-]{6,16}$'),
    code_normalized        text    GENERATED ALWAYS AS
                                   (upper(regexp_replace(code, '[^A-Za-z0-9]', '', 'g'))) STORED,
    label                  text    NOT NULL,
    scope_type             identity.scope_type NOT NULL,
    space_id               uuid    REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    grants_network_term_id uuid    REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    max_uses               integer,
    used_count             integer NOT NULL DEFAULT 0,
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
    CONSTRAINT ck_invitation_codes_scope CHECK (
        (scope_type = 'global'            AND space_id IS NULL) OR
        (scope_type = 'negotiation_space' AND space_id IS NOT NULL)),
    CONSTRAINT ck_invitation_codes_uses   CHECK (max_uses IS NULL OR max_uses > 0),
    -- Le quota se tient EN BASE : l'incrément du trigger prend le verrou de
    -- ligne, et la 121e entrée sur un code de 120 échoue ici, pas dans une
    -- vérification applicative que deux requêtes simultanées contourneraient.
    CONSTRAINT ck_invitation_codes_quota  CHECK (max_uses IS NULL OR used_count <= max_uses),
    CONSTRAINT ck_invitation_codes_period CHECK (valid_until IS NULL OR valid_until > valid_from),
    -- Un auteur de révocation sans date serait une révocation qu'on ne peut pas
    -- dater — or l'écran « 04c » doit dire « révoqué le 8 novembre ».
    CONSTRAINT ck_invitation_codes_revoked CHECK (revoked_by IS NULL OR revoked_at IS NOT NULL)
);

-- Unicité TOTALE, révoqués compris : l'écran « 04c » doit pouvoir dire
-- « révoqué le 8 novembre » plutôt que « code inconnu », qui enverrait la
-- personne chercher une faute de frappe.
CREATE UNIQUE INDEX IF NOT EXISTS ux_invitation_codes_normalized
    ON negotiation.invitation_codes (code_normalized);
CREATE INDEX IF NOT EXISTS ix_invitation_codes_space
    ON negotiation.invitation_codes (space_id, created_at DESC);

COMMENT ON TABLE negotiation.invitation_codes IS
    'Code partagé, diffusé à un groupe (WhatsApp, atelier). Sa portée est celle du rôle negotiator : un espace, ou global. Révoquer un code N''EN RETIRE AUCUN ACCÈS déjà accordé — c''est un second geste (ADR-006).';
COMMENT ON COLUMN negotiation.invitation_codes.code_normalized IS
    'Forme comparée : majuscules, sans séparateur. Le code se recopie à la main depuis un message.';
COMMENT ON COLUMN negotiation.invitation_codes.used_count IS
    'Tenu par tg_invitation_code_uses_count. Jamais écrit à la main.';
COMMENT ON COLUMN negotiation.invitation_codes.grants_network_term_id IS
    'Réseau que ce code fait rejoindre (taxonomie negotiation_network). NULL = code général. ADR-007 : c''est le réseau qui distingue, pas le genre.';

-- -----------------------------------------------------------------------------
-- 3.2 Qui est entré avec quel code
--
-- Historique, pas droit : comme space_members, cette table n'accorde rien.
-- L'accès effectif vit dans identity.role_assignments, et le retrait s'y écrit.
-- -----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS negotiation.invitation_code_uses (
    id         uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    code_id    uuid        NOT NULL REFERENCES negotiation.invitation_codes(id) ON DELETE CASCADE,
    person_id  uuid        NOT NULL CONSTRAINT xmod_fk_invitation_code_uses_person
                           REFERENCES identity.people(id) ON DELETE CASCADE,
    session_id uuid        CONSTRAINT xmod_fk_invitation_code_uses_session
                           REFERENCES identity.sessions(id) ON DELETE SET NULL,
    used_at    timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_invitation_code_uses_person
    ON negotiation.invitation_code_uses (code_id, person_id);
CREATE INDEX IF NOT EXISTS ix_invitation_code_uses_person
    ON negotiation.invitation_code_uses (person_id, used_at DESC);

COMMENT ON TABLE negotiation.invitation_code_uses IS
    'Qui est entré avec quel code. N''accorde aucun droit : l''autorisation passe par identity.has_permission. Sert à retirer en bloc les accès d''un code compromis.';

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

-- -----------------------------------------------------------------------------
-- 3.3 L'appartenance à un réseau (ADR-007)
-- -----------------------------------------------------------------------------

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

COMMENT ON TABLE negotiation.network_memberships IS
    'Appartenance à un réseau, portée par le code utilisé (ADR-007). N''ouvre aucun droit à l''étape 0b : elle sert le canal réservé et les chiffres des bailleurs. AUCUN champ « genre » n''existe ni n''est déduit.';

-- -----------------------------------------------------------------------------
-- 3.4 Les demandes d'accès (ADR-006)
-- -----------------------------------------------------------------------------

CREATE TABLE IF NOT EXISTS negotiation.access_requests (
    id                 uuid    PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id          uuid    NOT NULL CONSTRAINT xmod_fk_access_requests_person
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    scope_type         identity.scope_type NOT NULL,
    space_id           uuid    REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
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
    CONSTRAINT ck_access_requests_decision CHECK ((status = 'pending') = (decided_at IS NULL))
);

-- « Une seule demande en attente » est porté par la base : deux appareils qui
-- l'envoient ensemble ne produisent qu'une ligne, et l'API traduit le conflit.
CREATE UNIQUE INDEX IF NOT EXISTS ux_access_requests_pending_space
    ON negotiation.access_requests (person_id, space_id)
    WHERE status = 'pending' AND space_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ux_access_requests_pending_global
    ON negotiation.access_requests (person_id)
    WHERE status = 'pending' AND space_id IS NULL;
CREATE INDEX IF NOT EXISTS ix_access_requests_queue
    ON negotiation.access_requests (space_id, submitted_at) WHERE status = 'pending';

COMMENT ON TABLE negotiation.access_requests IS
    'Demande d''accès à trancher par un administrateur (ADR-006). Une seule en attente par personne et par portée. Un refus n''efface rien : une nouvelle demande est une nouvelle ligne.';

CREATE OR REPLACE FUNCTION negotiation.tg_access_request_transition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.status <> 'pending' AND NEW.status IS DISTINCT FROM OLD.status THEN
        RAISE EXCEPTION 'Cette demande d''accès a déjà été tranchée (% → %).', OLD.status, NEW.status
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_access_requests_transition ON negotiation.access_requests;
CREATE TRIGGER tg_access_requests_transition
    BEFORE UPDATE OF status ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_access_request_transition();

CREATE OR REPLACE FUNCTION negotiation.tg_access_request_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM platform.emit_event(
            'negotiation', 'access_request', NEW.id,
            'negotiation.access_request.submitted',
            jsonb_build_object('person_id', NEW.person_id, 'space_id', NEW.space_id,
                               'invitation_code_id', NEW.invitation_code_id));
    ELSIF NEW.status IS DISTINCT FROM OLD.status AND NEW.status IN ('approved', 'rejected') THEN
        PERFORM platform.emit_event(
            'negotiation', 'access_request', NEW.id,
            'negotiation.access_request.' || NEW.status::text,
            jsonb_build_object('person_id', NEW.person_id, 'space_id', NEW.space_id,
                               'decided_by', NEW.decided_by, 'reason', NEW.decision_reason));
    END IF;
    RETURN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_access_requests_event ON negotiation.access_requests;
CREATE TRIGGER tg_access_requests_event
    AFTER INSERT OR UPDATE OF status ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_access_request_event();

-- -----------------------------------------------------------------------------
-- 3.5 Les essais de code
--
-- LE COMPTE SE FAIT PAR PERSONNE. `device_id` vient du corps de la requête :
-- il se forge, et compter « par personne ET par appareil » laisserait changer
-- d'identifiant pour remettre le compteur à zéro. L'appareil n'est ici qu'une
-- information, utile à un administrateur qui regarde une série d'échecs.
--
-- Le code essayé n'est JAMAIS stocké, ni en clair ni en empreinte : un essai
-- raté peut être le vrai code d'un autre espace, et cette table se lit au
-- back-office.
--
-- DURÉE DE GARDE : 90 jours, purgés par une chaîne récurrente du worker. C'est
-- assez pour constater une série d'échecs et enquêter, trop court pour que la
-- table devienne un journal de fréquentation.
-- -----------------------------------------------------------------------------

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
    'Essais de code, pour en limiter le nombre. Comptés PAR PERSONNE — device_id se forge. Le code essayé n''est jamais stocké. Gardés 90 jours, purgés par le worker.';

CREATE OR REPLACE FUNCTION negotiation.invitation_attempts_recent(
    p_person_id uuid,
    p_window    interval
)
RETURNS integer
LANGUAGE sql
STABLE
AS $$
    SELECT count(*)::integer
      FROM negotiation.invitation_code_attempts
     WHERE person_id = p_person_id
       AND outcome <> 'accepted'
       AND attempted_at > now() - p_window;
$$;

COMMENT ON FUNCTION negotiation.invitation_attempts_recent(uuid, interval) IS
    'Essais non acceptés d''une personne sur une fenêtre glissante. Volontairement sans device_id : le compte est par personne.';

-- -----------------------------------------------------------------------------
-- 3.6 Déclencheurs partagés
-- -----------------------------------------------------------------------------

DROP TRIGGER IF EXISTS tg_invitation_codes_updated_at ON negotiation.invitation_codes;
CREATE TRIGGER tg_invitation_codes_updated_at BEFORE UPDATE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

DROP TRIGGER IF EXISTS tg_invitation_codes_audit ON negotiation.invitation_codes;
CREATE TRIGGER tg_invitation_codes_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

DROP TRIGGER IF EXISTS tg_invitation_codes_check_network ON negotiation.invitation_codes;
CREATE TRIGGER tg_invitation_codes_check_network
    BEFORE INSERT OR UPDATE OF grants_network_term_id ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'grants_network_term_id', 'negotiation_network');

DROP TRIGGER IF EXISTS tg_network_memberships_check_term ON negotiation.network_memberships;
CREATE TRIGGER tg_network_memberships_check_term
    BEFORE INSERT OR UPDATE OF network_term_id ON negotiation.network_memberships
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'network_term_id', 'negotiation_network');

DROP TRIGGER IF EXISTS tg_network_memberships_audit ON negotiation.network_memberships;
CREATE TRIGGER tg_network_memberships_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.network_memberships
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

DROP TRIGGER IF EXISTS tg_access_requests_updated_at ON negotiation.access_requests;
CREATE TRIGGER tg_access_requests_updated_at BEFORE UPDATE ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

DROP TRIGGER IF EXISTS tg_access_requests_audit ON negotiation.access_requests;
CREATE TRIGGER tg_access_requests_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

-- -----------------------------------------------------------------------------
-- 4. negotiation — les deux vues du back-office
--
-- L'état d'un code se CALCULE. Deux calculs séparés — un pour l'écran, un pour
-- le refus — divergeraient le jour où l'un des deux oublierait valid_from.
-- -----------------------------------------------------------------------------

CREATE OR REPLACE VIEW negotiation.v_invitation_codes AS
SELECT c.id,
       c.code,
       c.code_normalized,
       c.label,
       c.scope_type,
       c.space_id,
       s.name              AS space_name,
       c.grants_network_term_id,
       t.code              AS network_code,
       t.label             AS network_label,
       c.max_uses,
       c.used_count,
       c.valid_from,
       c.valid_until,
       c.revoked_at,
       c.revoked_by,
       c.revoked_reason,
       c.created_by,
       c.created_at,
       CASE
           WHEN c.revoked_at IS NOT NULL                              THEN 'revoked'
           WHEN c.valid_until IS NOT NULL AND c.valid_until <= now()  THEN 'expired'
           WHEN c.valid_from > now()                                  THEN 'not_yet_valid'
           WHEN c.max_uses IS NOT NULL AND c.used_count >= c.max_uses THEN 'exhausted'
           ELSE 'active'
       END AS state
  FROM negotiation.invitation_codes c
  LEFT JOIN negotiation.spaces s          ON s.id = c.space_id
  LEFT JOIN reference.taxonomy_terms t    ON t.id = c.grants_network_term_id;

COMMENT ON VIEW negotiation.v_invitation_codes IS
    'Un écran, une requête. L''état affiché au back-office et celui qui refuse un code sortent de cette même expression.';

CREATE OR REPLACE VIEW negotiation.v_invitation_code_uses AS
SELECT u.id,
       u.code_id,
       u.person_id,
       p.display_name,
       p.primary_email,
       u.used_at,
       ra.granted_at,
       ra.revoked_at        AS access_revoked_at,
       ra.revoked_by        AS access_revoked_by,
       (ra.id IS NOT NULL AND ra.revoked_at IS NULL
        AND (ra.valid_until IS NULL OR ra.valid_until > now())) AS access_active
  FROM negotiation.invitation_code_uses u
  JOIN negotiation.invitation_codes c ON c.id = u.code_id
  JOIN identity.people p              ON p.id = u.person_id
  LEFT JOIN LATERAL (
      SELECT r.id, r.granted_at, r.revoked_at, r.revoked_by, r.valid_until
        FROM identity.role_assignments r
       WHERE r.person_id = u.person_id
         AND r.role_code = 'negotiator'
         AND r.scope_type = c.scope_type
         AND r.scope_id IS NOT DISTINCT FROM c.space_id
       ORDER BY r.granted_at DESC
       LIMIT 1
  ) ra ON true;

COMMENT ON VIEW negotiation.v_invitation_code_uses IS
    'Qui est entré avec un code, et si son accès tient encore. Le RBAC fait foi : aucun état d''accès n''est dupliqué dans invitation_code_uses.';

-- -----------------------------------------------------------------------------
-- 5. platform — les réglages (ADR-006)
-- -----------------------------------------------------------------------------

INSERT INTO platform.settings (key, value, description, is_secret) VALUES
    ('negotiation.admission_mode', '"code"'::jsonb,
     'Comment on entre dans les modules réservés : "code", "approval" ou "code_and_approval" (ADR-006). Modifiable depuis le back-office, relu à chaque tentative, sans redéploiement.',
     false),
    ('negotiation.invitation_attempts',
     '{"max": 5, "window_minutes": 15, "lock_minutes": 15}'::jsonb,
     'Limite des essais de code d''invitation, comptés par personne.',
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
