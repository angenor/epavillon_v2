-- =============================================================================
-- ePavillon v2 — 100_negotiations.sql
-- Module Négociations : espaces réservés aux négociateurs francophones,
-- réunions unifiées, documents d'aide, canaux d'échange temps réel.
--
-- Dépend de : 000_bootstrap, 010_platform, 020_reference, 030_identity,
--             040_organizations, ainsi que des modules `event`, `media`,
--             `live` et `programme` (FK inter-modules nommées xmod_fk_*).
--
-- CADRAGE — « Espace où on publie : sessions de négociation ; documents d'aide
-- (lien ou fichier uploadé) ; réunions Francophonie (session avec lien zoom) ;
-- groupes d'échange temps réel comme WhatsApp, par thématique et parfois par
-- promotion, créés dynamiquement ; outils pour aider les négociateurs (agent IA
-- + RAG). Cet espace est réservé aux personnes ayant le rôle négociateur. »
--
-- QUATRE DÉFAUTS DE LA V1 CORRIGÉS ICI
--
--   D1. LA FILIÈRE ÉTAIT UN ENUM FERMÉ
--       `session_category_v2 ENUM('climate','biodiversity','desertification')`
--       était répété sur quatre tables. Ouvrir un espace « Traité plastique » ou
--       « Océans » imposait un ALTER TYPE, un redéploiement et la reprise des
--       filtres du frontend. En v2 la brique de base devient l'ESPACE
--       (negotiation.spaces), rattaché à une filière par un terme de la
--       taxonomie `negotiation_track`. Créer un espace = une ligne, zéro DDL.
--
--   D2. DEUX TABLES JUMELLES POUR LE MÊME CONCEPT
--       `negotiation_sessions` et `francophonie_meetings` portaient les mêmes
--       colonnes (titre, description, dates, lieu, catégorie, zoom, créateur),
--       avec deux tables d'inscription identiques. Toute évolution devait être
--       écrite deux fois, et « mes inscriptions » exigeait un UNION. En v2 :
--       UNE table `negotiation.meetings` discriminée par `kind`, UNE table
--       `negotiation.meeting_registrations`.
--
--   D3. FICHIER ET LIEN CONFONDUS
--       `negotiation_documents.file_url TEXT NOT NULL` accueillait aussi bien
--       une URL de stockage qu'un lien externe : impossible de savoir ce qu'on
--       devait purger, sauvegarder ou indexer. En v2, XOR explicite entre
--       `asset_id` (objet stocké, module media) et `external_url` (lien tiers).
--
--   D4. LES GROUPES D'ÉCHANGE N'ÉTAIENT PAS UNE MESSAGERIE
--       `message_groups` / `group_messages` : ni canal thématique, ni fil de
--       discussion, ni accusé de lecture, ni modération, ni maîtrise de la
--       volumétrie. En v2, canaux typés (thématique / promotion), fils de
--       réponses, `last_read_at` par membre, modération douce, et table de
--       messages PARTITIONNÉE PAR MOIS comme platform.audit_log.
--
-- AUTORISATION — aucun système de droits parallèle n'est défini ici.
-- L'accès à un espace se teste par
--   identity.has_permission(person_id, 'negotiation.space.access',
--                           'negotiation_space', space_id)
-- La valeur `negotiation_space` du type identity.scope_type fait des lignes de
-- `negotiation.spaces` des CIBLES DE PORTÉE RBAC : `role_assignments.scope_id`
-- pointe sur `spaces.id` (sans FK, la portée traversant les modules).
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 0. Garde-fou taxonomique
--
-- Un CHECK ne peut pas interroger une autre table : ce trigger générique vérifie
-- qu'un terme référencé relève bien de la taxonomie attendue.
-- Usage : EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('colonne', 'taxonomie')
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION negotiation.tg_check_term_taxonomy()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_term uuid := (to_jsonb(NEW) ->> TG_ARGV[0])::uuid;
BEGIN
    IF v_term IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM reference.taxonomy_terms t
        WHERE t.id = v_term AND t.taxonomy_code = TG_ARGV[1]
    ) THEN
        RAISE EXCEPTION 'Le terme % ne relève pas de la taxonomie « % » (colonne %.%)',
            v_term, TG_ARGV[1], TG_TABLE_NAME, TG_ARGV[0]
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

-- -----------------------------------------------------------------------------
-- 1. Espaces de négociation  (correction D1)
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.space_visibility AS ENUM (
    'private',  -- ni listé ni consultable hors membres
    'listed',   -- listé publiquement, contenus réservés aux membres
    'public'    -- contenus non restreints consultables sans adhésion
);

CREATE TABLE negotiation.spaces (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    slug               platform.slug NOT NULL,
    name               platform.i18n_text NOT NULL,
    tagline            platform.i18n_text,
    description        platform.i18n_text,
    -- Filière de rattachement : terme de la taxonomie `negotiation_track`
    -- (climate, biodiversity, desertification, et tout code ajouté ensuite).
    track_term_id      uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    cover_asset_id     uuid        CONSTRAINT xmod_fk_spaces_cover
                                   REFERENCES media.assets(id) ON DELETE SET NULL,
    visibility         negotiation.space_visibility NOT NULL DEFAULT 'listed',
    -- Une adhésion peut-elle être demandée depuis l'espace public ?
    is_membership_open boolean     NOT NULL DEFAULT false,
    opened_at          timestamptz NOT NULL DEFAULT now(),
    archived_at        timestamptz,        -- espace clos : lecture seule
    created_by         uuid        CONSTRAINT xmod_fk_spaces_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_spaces_slug UNIQUE (slug)
);

CREATE INDEX ix_spaces_track  ON negotiation.spaces (track_term_id);
CREATE INDEX ix_spaces_active ON negotiation.spaces (visibility, opened_at DESC) WHERE archived_at IS NULL;

CREATE TRIGGER tg_spaces_updated_at BEFORE UPDATE ON negotiation.spaces
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_spaces_check_track BEFORE INSERT OR UPDATE OF track_term_id ON negotiation.spaces
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('track_term_id', 'negotiation_track');

COMMENT ON TABLE negotiation.spaces IS
    'Espace thématique réservé aux négociateurs. Remplace l''ENUM de filière de la v1 et sert de portée RBAC (identity.scope_type = negotiation_space).';
COMMENT ON COLUMN negotiation.spaces.track_term_id IS
    'Filière (taxonomie negotiation_track). Un nouvel espace « Traité plastique » ne demande qu''un terme, jamais de migration.';

-- -----------------------------------------------------------------------------
-- 2. Appartenance à un espace
--
-- ARTICULATION AVEC LE RBAC — cette table décrit l'ANNUAIRE de l'espace (qui en
-- fait partie, à quel titre, depuis quand, pour quelle délégation). Elle ne
-- donne AUCUN droit par elle-même : l'autorisation reste portée par
-- `identity.role_assignments` (rôle `negotiator` ou `space_lead`, portée
-- `negotiation_space` + scope_id = spaces.id). L'API crée les deux dans la même
-- transaction ; en cas de divergence, le RBAC fait foi.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.space_member_role AS ENUM ('lead', 'facilitator', 'negotiator', 'observer');

CREATE TABLE negotiation.space_members (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    space_id              uuid        NOT NULL REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    person_id             uuid        NOT NULL CONSTRAINT xmod_fk_space_members_person
                                      REFERENCES identity.people(id) ON DELETE CASCADE,
    -- Profil négociateur associé : désignations annuelles et spécialisations
    -- restent dans le module identity, on ne les recopie pas.
    negotiator_profile_id uuid        CONSTRAINT xmod_fk_space_members_profile
                                      REFERENCES identity.negotiator_profiles(id) ON DELETE SET NULL,
    role                  negotiation.space_member_role NOT NULL DEFAULT 'negotiator',
    -- Délégation représentée dans l'espace (peut différer du pays de résidence).
    country_id            uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    organization_id       uuid        CONSTRAINT xmod_fk_space_members_organization
                                      REFERENCES org.organizations(id) ON DELETE SET NULL,
    cohort_label          text,       -- promotion de formation, ex. « Promotion 2026 »
    admitted_at           timestamptz NOT NULL DEFAULT now(),
    admitted_by           uuid        CONSTRAINT xmod_fk_space_members_admitter
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    left_at               timestamptz,
    CONSTRAINT ck_space_members_period CHECK (left_at IS NULL OR left_at >= admitted_at)
);

CREATE UNIQUE INDEX ux_space_members_active
    ON negotiation.space_members (space_id, person_id) WHERE left_at IS NULL;
CREATE INDEX ix_space_members_person ON negotiation.space_members (person_id) WHERE left_at IS NULL;
CREATE INDEX ix_space_members_cohort ON negotiation.space_members (space_id, cohort_label) WHERE cohort_label IS NOT NULL;

CREATE TRIGGER tg_space_members_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.space_members
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.space_members IS
    'Annuaire d''un espace. N''accorde aucun droit : l''autorisation passe par identity.has_permission(..., ''negotiation_space'', space_id).';


-- -----------------------------------------------------------------------------
-- 2 bis. Admission — comment on entre dans l'espace réservé
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

-- 2 bis.1 — Les codes d'invitation
CREATE TABLE negotiation.invitation_codes (
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
CREATE UNIQUE INDEX ux_invitation_codes_normalized ON negotiation.invitation_codes (code_normalized);
CREATE INDEX ix_invitation_codes_space ON negotiation.invitation_codes (space_id, created_at DESC);

CREATE TRIGGER tg_invitation_codes_updated_at BEFORE UPDATE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_invitation_codes_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.invitation_codes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
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

-- 2 bis.2 — Qui est entré avec quel code
--
-- Historique, pas droit : comme space_members, cette table n'accorde rien et ne
-- porte AUCUNE colonne de révocation. L'accès effectif se lit dans
-- identity.role_assignments, et le retrait s'y écrit.
CREATE TABLE negotiation.invitation_code_uses (
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
CREATE UNIQUE INDEX ux_invitation_code_uses_person ON negotiation.invitation_code_uses (code_id, person_id);
CREATE INDEX ix_invitation_code_uses_person ON negotiation.invitation_code_uses (person_id, used_at DESC);

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

CREATE TRIGGER tg_invitation_code_uses_count
    AFTER INSERT OR DELETE ON negotiation.invitation_code_uses
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_invitation_code_uses_count();

COMMENT ON TABLE negotiation.invitation_code_uses IS
    'Qui est entré avec quel code, et quand. Historique : n''accorde aucun droit et ne porte aucun état de révocation — dupliquer le RBAC, c''est se préparer à deux vérités.';

-- 2 bis.3 — L'appartenance à un réseau
--
-- Elle n'ouvre aucun droit à ce stade : elle sert les chiffres des bailleurs et,
-- plus tard, les canaux réservés. Sa traçabilité passe par le code utilisé.
CREATE TABLE negotiation.network_memberships (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_network_memberships_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    network_term_id uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    source_code_id  uuid        REFERENCES negotiation.invitation_codes(id) ON DELETE SET NULL,
    joined_at       timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    CONSTRAINT ck_network_memberships_period CHECK (left_at IS NULL OR left_at >= joined_at)
);

CREATE UNIQUE INDEX ux_network_memberships_active
    ON negotiation.network_memberships (person_id, network_term_id) WHERE left_at IS NULL;
CREATE INDEX ix_network_memberships_network
    ON negotiation.network_memberships (network_term_id, joined_at DESC) WHERE left_at IS NULL;

CREATE TRIGGER tg_network_memberships_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.network_memberships
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_network_memberships_check_network
    BEFORE INSERT OR UPDATE OF network_term_id ON negotiation.network_memberships
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'network_term_id', 'negotiation_network');

COMMENT ON TABLE negotiation.network_memberships IS
    'Appartenance à un réseau de négociation, venue du code d''invitation utilisé. N''ouvre aucun droit ; sert les chiffres et, plus tard, les canaux réservés.';
COMMENT ON COLUMN negotiation.network_memberships.source_code_id IS
    'Code par lequel l''appartenance est venue : c''est ce qui rend les chiffres du réseau vérifiables.';

-- 2 bis.3 bis — Les thématiques suivies
--
-- Une préférence de lecture, choisie et défaite librement depuis l'application :
-- elle n'ouvre aucun droit, elle commande ce qu'on montre en premier et ce dont
-- on avertit. Ce n'est PAS une compétence attestée — celle-là vit dans
-- identity.negotiator_profiles, renseignée par un administrateur.
--
-- Même patron que network_memberships : un suivi se ferme (left_at), il ne se
-- supprime pas, et l'index unique ne porte que sur le suivi vivant.
CREATE TABLE negotiation.theme_subscriptions (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_theme_subscriptions_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    theme_term_id   uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    followed_at     timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    notify_changes  boolean     NOT NULL DEFAULT false,
    CONSTRAINT ck_theme_subscriptions_period CHECK (left_at IS NULL OR left_at >= followed_at)
);

CREATE UNIQUE INDEX ux_theme_subscriptions_active
    ON negotiation.theme_subscriptions (person_id, theme_term_id) WHERE left_at IS NULL;
CREATE INDEX ix_theme_subscriptions_theme
    ON negotiation.theme_subscriptions (theme_term_id, followed_at DESC) WHERE left_at IS NULL;

CREATE TRIGGER tg_theme_subscriptions_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.theme_subscriptions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_theme_subscriptions_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.theme_subscriptions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

COMMENT ON TABLE negotiation.theme_subscriptions IS
    'Thématiques de négociation qu''une personne choisit de suivre. N''ouvre aucun droit : commande ce qu''on lui montre en premier et ce dont on l''avertit. Choix révocable, jamais une compétence attestée — celle-là vit dans identity.negotiator_profiles.';
COMMENT ON COLUMN negotiation.theme_subscriptions.left_at IS
    'Un suivi se ferme, il ne se supprime pas : ce qu''une personne a suivi pendant une COP reste lisible, et l''index unique ne porte que sur le suivi vivant.';
COMMENT ON COLUMN negotiation.theme_subscriptions.notify_changes IS
    'Prévenir des changements des sessions de la thématique (Guide Négo, étape 3b). Éteint par défaut : suivre une thématique ordonne ce qu''on montre, pas ce dont on avertit. Quitter la thématique l''éteint avec elle.';

-- 2 bis.4 — Les demandes d'accès
--
-- `cancelled` = « annulée », le fait de la personne entrée par un code entre-temps.
-- Ce n'est PAS « révoquée », qui qualifie un accès retiré, jamais une demande.
CREATE TYPE negotiation.access_request_status AS ENUM ('pending', 'approved', 'rejected', 'cancelled');

CREATE TABLE negotiation.access_requests (
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
CREATE UNIQUE INDEX ux_access_requests_pending_space
    ON negotiation.access_requests (person_id, space_id)
    WHERE status = 'pending' AND space_id IS NOT NULL;
CREATE UNIQUE INDEX ux_access_requests_pending_global
    ON negotiation.access_requests (person_id)
    WHERE status = 'pending' AND space_id IS NULL;
CREATE INDEX ix_access_requests_queue
    ON negotiation.access_requests (space_id, submitted_at) WHERE status = 'pending';

CREATE TRIGGER tg_access_requests_updated_at BEFORE UPDATE ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
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

CREATE TRIGGER tg_access_request_event AFTER INSERT OR UPDATE OF status ON negotiation.access_requests
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_access_request_event();

COMMENT ON TABLE negotiation.access_requests IS
    'Demande d''accès à l''espace réservé, quand le mode d''admission exige une approbation. Un état final ne se rouvre pas : une nouvelle demande est une nouvelle ligne.';
COMMENT ON COLUMN negotiation.access_requests.status IS
    'cancelled se dit « annulée » — la personne est entrée par un code entre-temps. « Révoquée » qualifie un accès retiré, jamais une demande.';
COMMENT ON COLUMN negotiation.access_requests.invitation_code_id IS
    'Code reconnu mais insuffisant à ouvrir, en mode « code et approbation ». Sert à l''administrateur qui tranche.';

-- 2 bis.5 — Les essais de code
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
CREATE TYPE negotiation.invitation_attempt_outcome AS ENUM (
    'accepted', 'unknown', 'revoked', 'exhausted', 'expired', 'not_yet_valid', 'throttled');

CREATE TABLE negotiation.invitation_code_attempts (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_invitation_attempts_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    device_id    text,
    outcome      negotiation.invitation_attempt_outcome NOT NULL,
    attempted_at timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_invitation_attempts_window
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

-- 2 bis.6 — Les deux vues du back-office
--
-- L'état affiché au back-office et celui qui refuse un code à l'application
-- sortent de LA MÊME expression : deux calculs séparés divergeraient le jour où
-- l'un des deux oublierait `valid_from`.
CREATE VIEW negotiation.v_invitation_codes AS
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
CREATE VIEW negotiation.v_invitation_code_uses AS
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
-- 3. Réunions unifiées  (correction D2)
--
-- Une seule table remplace `negotiation_sessions` + `francophonie_meetings`.
-- `kind` porte la distinction éditoriale, `format` la modalité de participation.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.meeting_kind AS ENUM (
    'negotiation_session',       -- session officielle de la convention (COP, SB, pré-session)
    'francophone_consultation',  -- concertation francophone (ex-francophonie_meetings)
    'preparatory_workshop',      -- atelier préparatoire
    'field_training',            -- atelier de formation terrain
    'innovation'                 -- rencontre innovation
);
CREATE TYPE negotiation.meeting_format AS ENUM ('onsite', 'online', 'hybrid');
CREATE TYPE negotiation.meeting_status AS ENUM ('draft', 'scheduled', 'ongoing', 'completed', 'cancelled');

-- 3.0 — Les points de l'ordre du jour d'une COP (Guide Négo, étape 3a)
--
-- La source officielle ne les publie pas à part : l'import les extrait du titre
-- des sessions (« SBI 12 (a) … - Informal consultation ») et les crée à la
-- première lecture qui les cite. Leur rattachement à une thématique est un
-- travail de l'IFDD, au back-office : un point n'est donc jamais supprimé par
-- l'import, même quand plus aucune session ne le cite.
CREATE TABLE negotiation.agenda_items (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id        uuid        NOT NULL CONSTRAINT xmod_fk_agenda_items_event
                                REFERENCES event.events(id) ON DELETE RESTRICT,
    code            text        NOT NULL,
    title           text        NOT NULL,
    theme_term_id   uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    theme_set_by    uuid        CONSTRAINT xmod_fk_agenda_items_theme_setter
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    theme_set_at    timestamptz,
    first_read_at   timestamptz NOT NULL,
    CONSTRAINT ux_agenda_items_code UNIQUE (event_id, code)
);

CREATE INDEX ix_agenda_items_theme ON negotiation.agenda_items (theme_term_id) WHERE theme_term_id IS NOT NULL;

CREATE TRIGGER tg_agenda_items_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.agenda_items
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_agenda_items_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.agenda_items
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

COMMENT ON TABLE negotiation.agenda_items IS
    'Points de l''ordre du jour d''une COP, extraits du titre des sessions par l''import. Jamais supprimés par lui : leur thématique est posée à la main par l''IFDD.';
COMMENT ON COLUMN negotiation.agenda_items.code IS
    'Code tel que lu dans le titre de la session : « SBI 12 (a) », « CMA 8 ». Unique par édition.';
COMMENT ON COLUMN negotiation.agenda_items.title IS
    'Intitulé anglais, extrait du titre de la première session qui cite le point.';
COMMENT ON COLUMN negotiation.agenda_items.theme_term_id IS
    'Thématique (negotiation_theme), posée au back-office. Les sessions du point en héritent : c''est ce qui les montre en premier à qui suit la thématique.';

CREATE TABLE negotiation.meetings (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    space_id              uuid        NOT NULL REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    kind                  negotiation.meeting_kind NOT NULL,
    slug                  platform.slug NOT NULL,
    title                 platform.i18n_text NOT NULL,
    description           platform.i18n_text,
    start_at              timestamptz NOT NULL,
    -- Nulle seulement pour une session importée : la source en publie sans fin
    -- annoncée (ck_meetings_imported_end).
    end_at                timestamptz,
    timezone              platform.timezone_name NOT NULL DEFAULT 'UTC',
    format                negotiation.meeting_format NOT NULL DEFAULT 'online',
    venue_label           text,
    city                  text,
    country_id            uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    -- Visioconférence pilotée par la plateforme (création Zoom, présence, replay).
    live_meeting_id       uuid        CONSTRAINT xmod_fk_negotiation_meetings_live
                                      REFERENCES live.meetings(id) ON DELETE SET NULL,
    -- Lien de connexion tiers, quand la réunion n'est pas hébergée par l'IFDD.
    external_url          platform.url,
    -- Rattachement facultatif à une édition (COP) du module event.
    event_id              uuid        CONSTRAINT xmod_fk_negotiation_meetings_event
                                      REFERENCES event.events(id) ON DELETE SET NULL,
    organizer_org_id      uuid        CONSTRAINT xmod_fk_negotiation_meetings_organizer
                                      REFERENCES org.organizations(id) ON DELETE SET NULL,
    is_ifdd_organized     boolean     NOT NULL DEFAULT true,
    host_person_id        uuid        CONSTRAINT xmod_fk_negotiation_meetings_host
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    capacity              integer     CHECK (capacity IS NULL OR capacity > 0),
    registration_opens_at timestamptz,
    registration_closes_at timestamptz,
    registered_count      integer     NOT NULL DEFAULT 0,
    status                negotiation.meeting_status NOT NULL DEFAULT 'draft',
    cancellation_reason   text,
    created_by            uuid        CONSTRAINT xmod_fk_negotiation_meetings_creator
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),
    -- Session officielle importée (Guide Négo, étape 3a) : toutes nulles pour
    -- une réunion saisie à la main, les cinq premières exigées dès que
    -- `source_key` est posé (ck_meetings_source_complete).
    source_key            text,
    source_url            platform.url,
    title_original        text,
    first_read_at         timestamptz,
    last_read_at          timestamptz,
    meeting_type_term_id  uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    group_term_id         uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    agenda_item_id        uuid        REFERENCES negotiation.agenda_items(id) ON DELETE RESTRICT,
    is_open_access        boolean,
    absent_reads          smallint    NOT NULL DEFAULT 0,
    cancelled_at          timestamptz,
    -- Réunion de la Francophonie (Guide Négo, étape 4) : saisie par l'IFDD,
    -- de kind preparatory_workshop ou francophone_consultation.
    francophone_type_term_id uuid     REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    access_audience       platform.i18n_text,
    pavilion_session_id   uuid        CONSTRAINT xmod_fk_negotiation_meetings_pavilion_session
                                      REFERENCES programme.sessions(id) ON DELETE SET NULL,
    requires_registration boolean     NOT NULL DEFAULT true,
    waitlist_enabled      boolean     NOT NULL DEFAULT true,

    CONSTRAINT ux_meetings_slug UNIQUE (space_id, slug),
    CONSTRAINT ck_meetings_period CHECK (end_at IS NULL OR end_at > start_at),
    -- Une réunion saisie à la main a toujours une fin : l'étape 4 n'hérite pas
    -- de la tolérance faite à la source.
    CONSTRAINT ck_meetings_imported_end CHECK (source_key IS NOT NULL OR end_at IS NOT NULL),
    CONSTRAINT ck_meetings_source_complete
        CHECK (source_key IS NULL OR (event_id IS NOT NULL AND source_url IS NOT NULL
            AND title_original IS NOT NULL AND first_read_at IS NOT NULL AND last_read_at IS NOT NULL)),
    CONSTRAINT ck_meetings_registration_window
        CHECK (registration_opens_at IS NULL OR registration_closes_at IS NULL
            OR registration_closes_at > registration_opens_at),
    -- Une réunion publiée à distance doit offrir un moyen de s'y connecter,
    -- et une réunion sur site doit indiquer où elle se tient.
    CONSTRAINT ck_meetings_online_access
        CHECK (status = 'draft' OR format = 'onsite'
            OR live_meeting_id IS NOT NULL OR external_url IS NOT NULL),
    -- Une session importée peut n'avoir pas de salle à la source : on n'en
    -- invente pas.
    CONSTRAINT ck_meetings_onsite_venue
        CHECK (status = 'draft' OR format = 'online' OR venue_label IS NOT NULL OR source_key IS NOT NULL),
    CONSTRAINT ck_meetings_cancellation
        CHECK (status <> 'cancelled' OR cancellation_reason IS NOT NULL),
    -- L'import écrit un code, jamais un libellé : « CANCELLED » à la source,
    -- « POSTPONED » à la source, ou disparue de deux lectures de suite.
    CONSTRAINT ck_meetings_import_cancellation
        CHECK (source_key IS NULL OR cancellation_reason IS NULL
            OR cancellation_reason IN ('source', 'postponed', 'removed')),
    -- Réunions de la Francophonie : ces quatre règles ne visent que les
    -- réunions SAISIES de ces deux natures. Une session importée n'est jamais
    -- touchée. La correspondance fine terme ↔ kind lit le code du terme : elle
    -- vit dans tg_meetings_check_francophone_kind, qui lève sous le nom
    -- ck_meetings_francophone_kind ; le CHECK du même nom n'en garde que la
    -- partie qui se lit dans la ligne (un terme sur ces deux natures seulement).
    CONSTRAINT ck_meetings_francophone_type
        CHECK (source_key IS NOT NULL OR status = 'draft'
            OR kind NOT IN ('preparatory_workshop', 'francophone_consultation')
            OR francophone_type_term_id IS NOT NULL),
    -- `event_id` est posé ON DELETE SET NULL : pour ces réunions, supprimer
    -- l'édition est donc refusé par ce CHECK, et c'est voulu.
    CONSTRAINT ck_meetings_francophone_event
        CHECK (source_key IS NOT NULL
            OR kind NOT IN ('preparatory_workshop', 'francophone_consultation')
            OR event_id IS NOT NULL),
    CONSTRAINT ck_meetings_francophone_kind
        CHECK (francophone_type_term_id IS NULL
            OR (source_key IS NULL AND kind IN ('preparatory_workshop', 'francophone_consultation'))),
    CONSTRAINT ck_meetings_access_audience
        CHECK (source_key IS NOT NULL
            OR kind NOT IN ('preparatory_workshop', 'francophone_consultation')
            OR (is_open_access IS NOT NULL AND (is_open_access OR access_audience IS NOT NULL))),
    -- Une même salle Zoom ne peut pas héberger deux réunions qui se chevauchent.
    --
    -- Ce blocage est volontaire et ne contredit pas la règle du module
    -- `programme`, où les chevauchements de créneaux sont signalés et jamais
    -- refusés. La différence tient à la nature de l'obstacle : là-bas, une
    -- collision d'horaires appelle un arbitrage humain, que l'équipe résout en
    -- déplaçant ses blocs ; ici, c'est une impossibilité du fournisseur — une
    -- réunion Zoom donnée n'accueille pas deux sessions à la fois, et l'écrire
    -- en base ne produirait qu'un lien de connexion inutilisable.
    CONSTRAINT ex_meetings_live_room_overlap EXCLUDE USING gist (
        live_meeting_id WITH =,
        tstzrange(start_at, end_at, '[)') WITH &&
    ) WHERE (live_meeting_id IS NOT NULL AND status <> 'cancelled')
);

CREATE INDEX ix_meetings_agenda   ON negotiation.meetings (space_id, start_at DESC)
    WHERE status IN ('scheduled', 'ongoing');
CREATE INDEX ix_meetings_kind     ON negotiation.meetings (kind, start_at DESC);
CREATE INDEX ix_meetings_upcoming ON negotiation.meetings (start_at) WHERE status = 'scheduled';
CREATE UNIQUE INDEX ux_meetings_source ON negotiation.meetings (event_id, source_key)
    WHERE source_key IS NOT NULL;
CREATE INDEX ix_meetings_event_day ON negotiation.meetings (event_id, start_at)
    WHERE kind = 'negotiation_session';
CREATE INDEX ix_meetings_francophonie ON negotiation.meetings (event_id, start_at)
    WHERE kind IN ('preparatory_workshop', 'francophone_consultation') AND status <> 'draft';

CREATE TRIGGER tg_meetings_updated_at BEFORE UPDATE ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_meetings_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_meetings_check_type
    BEFORE INSERT OR UPDATE OF meeting_type_term_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'meeting_type_term_id', 'negotiation_meeting_type');
CREATE TRIGGER tg_meetings_check_group
    BEFORE INSERT OR UPDATE OF group_term_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'group_term_id', 'negotiation_group');
CREATE TRIGGER tg_meetings_check_francophone_type
    BEFORE INSERT OR UPDATE OF francophone_type_term_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'francophone_type_term_id', 'francophone_meeting_type');

-- La nature dit le parcours : un atelier préparatoire est de kind
-- preparatory_workshop, une concertation de kind francophone_consultation
-- (specs/016-guide-nego-reunions, R9 bis). Un CHECK ne lit pas le code du
-- terme : le refus porte le nom de contrainte ck_meetings_francophone_kind,
-- que l'API traduit comme les autres.
CREATE OR REPLACE FUNCTION negotiation.tg_check_meeting_francophone_kind()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_code text;
BEGIN
    IF NEW.francophone_type_term_id IS NULL THEN
        RETURN NEW;
    END IF;
    SELECT t.code INTO v_code
      FROM reference.taxonomy_terms t
     WHERE t.id = NEW.francophone_type_term_id AND t.taxonomy_code = 'francophone_meeting_type';
    -- Un terme d'une autre taxonomie est refusé par tg_meetings_check_francophone_type.
    IF v_code IS NOT NULL AND NEW.kind IS DISTINCT FROM (CASE v_code
            WHEN 'preparatory_workshop' THEN 'preparatory_workshop'
            ELSE 'francophone_consultation' END)::negotiation.meeting_kind THEN
        RAISE EXCEPTION 'La nature « % » ne correspond pas au type de réunion « % ».', v_code, NEW.kind
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_meetings_francophone_kind';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_meetings_check_francophone_kind
    BEFORE INSERT OR UPDATE OF francophone_type_term_id, kind ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_meeting_francophone_kind();

-- Le lien au Pavillon ne vaut que pour une activité de la même édition (R7).
-- Une lecture de `programme`, jamais une écriture.
CREATE OR REPLACE FUNCTION negotiation.tg_check_meeting_pavilion_edition()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.pavilion_session_id IS NOT NULL AND NOT EXISTS (
        SELECT 1 FROM programme.sessions s
         WHERE s.id = NEW.pavilion_session_id AND s.event_id = NEW.event_id
    ) THEN
        RAISE EXCEPTION 'L''activité du Pavillon % n''appartient pas à l''édition de la réunion.',
            NEW.pavilion_session_id
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_meetings_pavilion_edition';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_meetings_check_pavilion_edition
    BEFORE INSERT OR UPDATE OF pavilion_session_id, event_id ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_meeting_pavilion_edition();

COMMENT ON TABLE negotiation.meetings IS
    'Événements d''un espace de négociation : sessions officielles, concertations francophones, ateliers, formations, innovation. Unifie les deux tables jumelles de la v1.';
COMMENT ON COLUMN negotiation.meetings.kind IS
    'ENUM fermé assumé : chaque valeur engage un parcours applicatif distinct (contrairement aux filières, ouvertes par taxonomie).';
COMMENT ON COLUMN negotiation.meetings.end_at IS
    'Nulle seulement pour une session importée sans fin annoncée à la source ; toute autre réunion en a une (ck_meetings_imported_end).';
COMMENT ON COLUMN negotiation.meetings.source_key IS
    'Identifiant de la session à la source officielle ; nul pour une réunion saisie à la main. Unique par édition (ux_meetings_source).';
COMMENT ON COLUMN negotiation.meetings.source_url IS
    'La fiche de la session à la source : « Voir l''original ».';
COMMENT ON COLUMN negotiation.meetings.title_original IS
    'Titre anglais tel que lu, préfixe « CANCELLED » ou « POSTPONED » retiré : il FAIT FOI. `title` en reçoit la copie sous "fr" et "en" — le domaine i18n_text exige "fr", et aucun texte humain ne le traduit ; la traduction automatique vit dans title_translations.';
COMMENT ON COLUMN negotiation.meetings.last_read_at IS
    'Dernière lecture réussie où la session figurait à la source — exacte seulement quand la ligne est écrite pour une autre raison (apparition, écart, reparution, première absence, groupe re-résolu) : une session lue sans écart n''est pas réécrite, sinon l''audit de meetings gagnerait une ligne par session et par lecture. Tant que absent_reads = 0, l''heure de lecture servie est official_imports.last_success_at ; au-delà, c''est cette colonne.';
COMMENT ON COLUMN negotiation.meetings.meeting_type_term_id IS
    'Type de réunion (negotiation_meeting_type), résolu à l''import par les metadata du vocabulaire.';
COMMENT ON COLUMN negotiation.meetings.group_term_id IS
    'Groupe de négociation (negotiation_group) d''une coordination, re-résolu à chaque lecture ; nul si le titre n''en nomme aucun.';
COMMENT ON COLUMN negotiation.meetings.agenda_item_id IS
    'Point de l''ordre du jour ; nul = « Hors ordre du jour officiel ». La session hérite de sa thématique.';
COMMENT ON COLUMN negotiation.meetings.is_open_access IS
    'Ouverte (vrai) ou à accès limité (faux) : selon la source pour une session importée, selon l''IFDD pour une réunion saisie — requise pour une réunion de la Francophonie (ck_meetings_access_audience). Une information, jamais une règle de lecture ni d''inscription.';
COMMENT ON COLUMN negotiation.meetings.absent_reads IS
    'Lectures réussies de suite où la session manquait à la source. À 2, elle passe cancelled (motif removed) ; reparue, le compteur retombe à 0.';
COMMENT ON COLUMN negotiation.meetings.cancelled_at IS
    'Heure du constat de l''annulation par l''import.';
COMMENT ON COLUMN negotiation.meetings.francophone_type_term_id IS
    'Nature d''une réunion de la Francophonie (francophone_meeting_type) ; requise hors brouillon (ck_meetings_francophone_type), accordée au kind (ck_meetings_francophone_kind). Nulle pour toute autre réunion.';
COMMENT ON COLUMN negotiation.meetings.access_audience IS
    'Public d''une réunion à accès limité, dit à la personne : « ministres et chefs de délégation ». Requis quand is_open_access est faux sur une réunion de la Francophonie.';
COMMENT ON COLUMN negotiation.meetings.pavilion_session_id IS
    'Activité du Pavillon liée, de la même édition (tg_meetings_check_pavilion_edition). Une clé, rien d''autre : l''activité supprimée, le lien tombe.';
COMMENT ON COLUMN negotiation.meetings.requires_registration IS
    'Faux : ni inscription, ni capacité, ni liste d''attente ; le lien de connexion est servi à toute personne admise.';
COMMENT ON COLUMN negotiation.meetings.waitlist_enabled IS
    'Vrai : une réunion complète met en liste d''attente. Faux : elle refuse (meeting_full).';
COMMENT ON COLUMN negotiation.meetings.registered_count IS
    'Inscrites (registered), tenu par tg_sync_registered_count en fin de ligne. Pour l''affichage seulement : la jauge se calcule par count(*) sous verrou, jamais sur ce compteur. Chaque inscription réécrit la réunion : son updated_at ne garde pas une édition au back-office.';

-- Publication d'un événement de domaine à l'annulation : le module engagement
-- prévient les inscrits sans que ce module connaisse l'email.
CREATE OR REPLACE FUNCTION negotiation.tg_meeting_status_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    -- Une session importée n'émet rien : ses « inscrits » n'existent pas, et
    -- l'avis de changement viendra des signalements (étape 3b).
    IF NEW.source_key IS NOT NULL THEN
        RETURN NULL;
    END IF;
    IF NEW.status IS DISTINCT FROM OLD.status AND NEW.status IN ('scheduled', 'cancelled') THEN
        PERFORM platform.emit_event(
            'negotiation', 'meeting', NEW.id,
            'negotiation.meeting.' || CASE WHEN NEW.status = 'cancelled' THEN 'cancelled' ELSE 'published' END,
            jsonb_build_object('space_id', NEW.space_id, 'kind', NEW.kind,
                               'start_at', NEW.start_at, 'reason', NEW.cancellation_reason)
        );
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_meetings_status_event AFTER UPDATE OF status ON negotiation.meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_meeting_status_event();

-- -----------------------------------------------------------------------------
-- 4. Inscriptions — une seule table pour tous les types de réunion
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.registration_status AS ENUM ('registered', 'waitlisted', 'cancelled');
CREATE TYPE negotiation.attendance_state AS ENUM ('unknown', 'present', 'partial', 'absent');

CREATE TABLE negotiation.meeting_registrations (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    meeting_id         uuid        NOT NULL REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    -- Pas de colonnes `guest_*` comme en v1 : une personne existe toujours dans
    -- identity.people, avec ou sans compte.
    person_id          uuid        NOT NULL CONSTRAINT xmod_fk_meeting_registrations_person
                                   REFERENCES identity.people(id) ON DELETE CASCADE,
    status             negotiation.registration_status NOT NULL DEFAULT 'registered',
    registered_at      timestamptz NOT NULL DEFAULT now(),
    cancelled_at       timestamptz,
    attendance         negotiation.attendance_state NOT NULL DEFAULT 'unknown',
    attendance_minutes integer     CHECK (attendance_minutes IS NULL OR attendance_minutes >= 0),
    first_joined_at    timestamptz,
    note               text,
    waitlist_position  integer,
    client_ref         uuid,
    -- Se réinscrire est un UPDATE de la même ligne : l'unicité reste totale.
    CONSTRAINT ux_meeting_registrations UNIQUE (meeting_id, person_id),
    CONSTRAINT ux_meeting_registrations_client_ref UNIQUE (person_id, client_ref),
    CONSTRAINT ck_meeting_registrations_cancel
        CHECK ((status = 'cancelled') = (cancelled_at IS NOT NULL)),
    CONSTRAINT ck_meeting_registrations_waitlist
        CHECK ((status = 'waitlisted') = (waitlist_position IS NOT NULL))
);

CREATE INDEX ix_meeting_registrations_person ON negotiation.meeting_registrations (person_id, registered_at DESC);
CREATE INDEX ix_meeting_registrations_active ON negotiation.meeting_registrations (meeting_id) WHERE status = 'registered';

CREATE TRIGGER tg_meeting_registrations_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.meeting_registrations
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN negotiation.meeting_registrations.waitlist_position IS
    'Rang dans la liste d''attente, à partir de 1 ; posé si et seulement si la ligne est waitlisted (ck_meeting_registrations_waitlist). Recompacté par promote_meeting_waitlist.';
COMMENT ON COLUMN negotiation.meeting_registrations.client_ref IS
    'Référence du geste posée par le téléphone, neuve à chaque inscription ou réinscription : un envoi rejoué retrouve la ligne au lieu d''écrire deux fois (ux_meeting_registrations_client_ref).';

-- La jauge, tenue en base (specs/016-guide-nego-reunions, R4).
--
-- Les contrôles ne portent que sur une ENTRÉE : un INSERT, ou cancelled →
-- registered|waitlisted (réinscription). Se désinscrire n'est jamais contrôlé ;
-- waitlisted → registered (promotion) non plus : une place obtenue après la
-- fermeture des inscriptions reste obtenue.
--
-- La réunion est verrouillée (FOR UPDATE) et les places prises se comptent
-- par count(*) : registered_count n'est tenu qu'en fin de ligne par un trigger
-- AFTER, deux inscriptions simultanées y liraient la même valeur.
--
-- Les refus portent un nom de contrainte stable, que l'API traduit :
--   meeting_unavailable  réunion non publiée, annulée ou commencée
--   meeting_closed       inscription non demandée, ou hors de la fenêtre
--   meeting_full         complet, sans liste d'attente
CREATE OR REPLACE FUNCTION negotiation.tg_validate_meeting_registration()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_meeting negotiation.meetings%ROWTYPE;
    v_taken   integer;
BEGIN
    IF NEW.status = 'cancelled' THEN
        NEW.cancelled_at := COALESCE(NEW.cancelled_at, now());
        NEW.waitlist_position := NULL;
        RETURN NEW;
    END IF;

    IF TG_OP = 'UPDATE' AND OLD.status <> 'cancelled' THEN
        IF NEW.status = 'registered' THEN
            NEW.waitlist_position := NULL;
        END IF;
        RETURN NEW;
    END IF;

    SELECT * INTO v_meeting FROM negotiation.meetings WHERE id = NEW.meeting_id FOR UPDATE;

    IF v_meeting.status IS DISTINCT FROM 'scheduled' OR now() >= v_meeting.start_at THEN
        RAISE EXCEPTION 'meeting_unavailable : cette réunion n''accepte pas d''inscription (non publiée, annulée ou déjà commencée).'
            USING ERRCODE = 'restrict_violation', CONSTRAINT = 'meeting_unavailable';
    END IF;

    IF NOT v_meeting.requires_registration
       OR (v_meeting.registration_opens_at IS NOT NULL AND now() < v_meeting.registration_opens_at)
       OR (v_meeting.registration_closes_at IS NOT NULL AND now() >= v_meeting.registration_closes_at) THEN
        RAISE EXCEPTION 'meeting_closed : les inscriptions à cette réunion ne sont pas ouvertes.'
            USING ERRCODE = 'restrict_violation', CONSTRAINT = 'meeting_closed';
    END IF;

    SELECT count(*) INTO v_taken
      FROM negotiation.meeting_registrations r
     WHERE r.meeting_id = NEW.meeting_id AND r.status = 'registered' AND r.id <> NEW.id;

    -- Une liste d'attente non vide passe avant la nouvelle venue, même si une
    -- place s'est libérée sans promotion encore.
    IF (v_meeting.capacity IS NOT NULL AND v_taken >= v_meeting.capacity)
       OR EXISTS (SELECT 1 FROM negotiation.meeting_registrations r
                   WHERE r.meeting_id = NEW.meeting_id AND r.status = 'waitlisted' AND r.id <> NEW.id) THEN
        IF NOT v_meeting.waitlist_enabled THEN
            RAISE EXCEPTION 'meeting_full : cette réunion est complète.'
                USING ERRCODE = 'restrict_violation', CONSTRAINT = 'meeting_full';
        END IF;
        NEW.status := 'waitlisted';
        SELECT COALESCE(max(r.waitlist_position), 0) + 1 INTO NEW.waitlist_position
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = NEW.meeting_id AND r.status = 'waitlisted';
    ELSE
        NEW.status := 'registered';
        NEW.waitlist_position := NULL;
    END IF;

    NEW.cancelled_at := NULL;
    IF TG_OP = 'UPDATE' THEN
        NEW.registered_at := now();
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_meeting_registrations_validate
    BEFORE INSERT OR UPDATE OF status ON negotiation.meeting_registrations
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_validate_meeting_registration();

-- Appelée à chaque désinscription et quand l'administration relève ou retire
-- la capacité, dans la même transaction. Le paramètre `p_count` du patron de
-- programme disparaît : les places libres se calculent ici, sous le verrou.
CREATE OR REPLACE FUNCTION negotiation.promote_meeting_waitlist(p_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_capacity integer;
    v_free     integer;
BEGIN
    SELECT m.capacity INTO v_capacity FROM negotiation.meetings m WHERE m.id = p_meeting_id FOR UPDATE;
    IF NOT FOUND THEN
        RETURN;
    END IF;

    IF v_capacity IS NOT NULL THEN
        SELECT greatest(v_capacity - count(*), 0) INTO v_free
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = p_meeting_id AND r.status = 'registered';
    END IF;

    -- LIMIT NULL : sans capacité, toute la liste passe.
    RETURN QUERY
    WITH suivantes AS (
        SELECT r.id
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = p_meeting_id AND r.status = 'waitlisted'
         ORDER BY r.waitlist_position, r.registered_at
         LIMIT v_free
    )
    UPDATE negotiation.meeting_registrations r
       SET status = 'registered', waitlist_position = NULL
      FROM suivantes s
     WHERE r.id = s.id
    RETURNING r.person_id;

    WITH rangs AS (
        SELECT r.id, row_number() OVER (ORDER BY r.waitlist_position, r.registered_at)::integer AS rang
          FROM negotiation.meeting_registrations r
         WHERE r.meeting_id = p_meeting_id AND r.status = 'waitlisted'
    )
    UPDATE negotiation.meeting_registrations r
       SET waitlist_position = g.rang
      FROM rangs g
     WHERE r.id = g.id AND r.waitlist_position <> g.rang;
END;
$$;

COMMENT ON FUNCTION negotiation.promote_meeting_waitlist(uuid) IS
    'Promeut la liste d''attente dans l''ordre, autant que de places libres (capacity − count(registered) sous verrou ; toute la liste sans capacité), recompacte les rangs et rend les personnes promues — à prévenir.';

-- Qui prévenir d'un changement d'une réunion de la Francophonie : lue par
-- negotiation seul, comme change_recipients (R8).
CREATE OR REPLACE FUNCTION negotiation.meeting_audience(p_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE sql
STABLE
AS $$
    SELECT r.person_id
      FROM negotiation.meeting_registrations r
     WHERE r.meeting_id = p_meeting_id AND r.status IN ('registered', 'waitlisted');
$$;

COMMENT ON FUNCTION negotiation.meeting_audience(uuid) IS
    'Personnes à prévenir d''une annulation ou d''un changement de réunion : inscrites et en liste d''attente, désinscrites exclues.';

-- Compteur dénormalisé maintenu par la base : la liste des réunions n'a plus à
-- déclencher un COUNT(*) par ligne.
CREATE OR REPLACE FUNCTION negotiation.tg_sync_registered_count()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_meeting_id uuid;
BEGIN
    -- NEW n'est pas affecté sur un DELETE : la branche est explicite.
    IF TG_OP = 'DELETE' THEN
        v_meeting_id := OLD.meeting_id;
    ELSE
        v_meeting_id := NEW.meeting_id;
    END IF;

    UPDATE negotiation.meetings m
    SET registered_count = (
        SELECT count(*) FROM negotiation.meeting_registrations r
        WHERE r.meeting_id = m.id AND r.status = 'registered'
    )
    WHERE m.id = v_meeting_id;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_meeting_registrations_count
    AFTER INSERT OR UPDATE OF status OR DELETE ON negotiation.meeting_registrations
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_sync_registered_count();

-- -----------------------------------------------------------------------------
-- 4 bis. L'import de la source officielle — Guide Négo, étape 3a
--
-- Les sessions de négociation d'une COP ne se saisissent pas : un travail lit le
-- calendrier de conférence de la CCNUCC, compare à ce qu'il a déjà lu et
-- n'écrit que les écarts (specs/014-guide-nego-sessions-agenda, R2, R5, R7).
-- La source fait foi ; ce qui en vient porte son origine et son heure de
-- lecture, dans negotiation.meetings.
-- -----------------------------------------------------------------------------

-- L'import d'une édition : son interrupteur, son lecteur et son état de santé.
CREATE TABLE negotiation.official_imports (
    id                      uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id                uuid        NOT NULL CONSTRAINT xmod_fk_official_imports_event
                                        REFERENCES event.events(id) ON DELETE RESTRICT,
    is_enabled              boolean     NOT NULL DEFAULT false,
    reader                  text        NOT NULL DEFAULT 'archive',
    archive_name            text,
    archive_first_day       date,
    live_url                platform.url,
    time_correction_minutes smallint    NOT NULL DEFAULT 60,
    official_programme_url  platform.url NOT NULL,
    interval_seconds        integer     NOT NULL DEFAULT 300,
    missed_threshold        smallint    NOT NULL DEFAULT 3,
    missed_reads            smallint    NOT NULL DEFAULT 0,
    last_success_at         timestamptz,
    last_attempt_at         timestamptz,
    last_error              text,
    last_change_count       integer,
    failing_since           timestamptz,
    updated_by              uuid        CONSTRAINT xmod_fk_official_imports_updater
                                        REFERENCES identity.people(id) ON DELETE SET NULL,
    updated_at              timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_official_imports_event UNIQUE (event_id),
    CONSTRAINT ck_official_imports_reader    CHECK (reader IN ('archive', 'live')),
    CONSTRAINT ck_official_imports_interval  CHECK (interval_seconds >= 60),
    CONSTRAINT ck_official_imports_threshold CHECK (missed_threshold >= 1),
    CONSTRAINT ck_official_imports_missed    CHECK (missed_reads >= 0)
);

CREATE TRIGGER tg_official_imports_updated_at BEFORE UPDATE ON negotiation.official_imports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_official_imports_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.official_imports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.official_imports IS
    'L''import des sessions officielles d''une édition : interrupteur, lecteur, cadence, et santé de la source. Une ligne par édition.';
COMMENT ON COLUMN negotiation.official_imports.reader IS
    'archive = un jeu archivé embarqué (recette, et tant que l''accès à la source manque) ; live = le JSON du calendrier de conférence, par HTTP.';
COMMENT ON COLUMN negotiation.official_imports.archive_name IS
    'Le fichier du jeu archivé que lit le lecteur archive.';
COMMENT ON COLUMN negotiation.official_imports.archive_first_day IS
    'Pose le premier jour de l''archive sur ce jour, heure murale gardée, dans le fuseau de l''édition : l''archive se rejoue sur une autre COP, ou sur aujourd''hui pour la recette.';
COMMENT ON COLUMN negotiation.official_imports.time_correction_minutes IS
    'Minutes ajoutées aux heures naïves de la source avant de les lire dans le fuseau de l''édition : la CCNUCC publie l''heure locale moins une heure (constaté sur la COP29 et la COP30).';
COMMENT ON COLUMN negotiation.official_imports.official_programme_url IS
    'Le renvoi « Programme officiel de la CCNUCC », offert quand l''affichage est coupé.';
COMMENT ON COLUMN negotiation.official_imports.missed_threshold IS
    'Lectures manquées d''affilée au-delà desquelles l''affichage se coupe (negotiation.import_is_serving).';
COMMENT ON COLUMN negotiation.official_imports.missed_reads IS
    'Lectures manquées d''affilée : source injoignable, délai dépassé, contenu illisible, ou aucune réunion retenue. Retombe à zéro à la première réussite.';
COMMENT ON COLUMN negotiation.official_imports.failing_since IS
    'Heure de la première lecture manquée de la série en cours : « la source n''a pas répondu depuis 06:40 ». Nulle dès qu''une lecture réussit.';
COMMENT ON COLUMN negotiation.official_imports.last_change_count IS
    'Sessions touchées par la dernière lecture réussie.';

-- Le journal des lectures, purgé au-delà de 30 jours par le travail d'import.
CREATE TABLE negotiation.import_runs (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    import_id       uuid        NOT NULL REFERENCES negotiation.official_imports(id) ON DELETE CASCADE,
    started_at      timestamptz NOT NULL DEFAULT now(),
    finished_at     timestamptz,
    outcome         text        NOT NULL,
    error           text,
    session_count   integer,
    change_count    integer,
    is_manual       boolean     NOT NULL DEFAULT false,
    CONSTRAINT ck_import_runs_outcome CHECK (outcome IN ('success', 'failure'))
);

CREATE INDEX ix_import_runs_import ON negotiation.import_runs (import_id, started_at DESC);

COMMENT ON TABLE negotiation.import_runs IS
    'Journal des lectures de la source officielle, une ligne par lecture. Purgé au-delà de 30 jours par le travail d''import ; pas d''audit, c''est déjà un journal.';
COMMENT ON COLUMN negotiation.import_runs.change_count IS
    'Sessions touchées par la lecture — apparues, changées, absentes, reparues —, et non lignes de meeting_changes.';
COMMENT ON COLUMN negotiation.import_runs.is_manual IS
    'Lecture demandée par « Lire maintenant » : elle ne replanifie pas la chaîne.';

-- L'historique des écarts constatés entre deux lectures.
CREATE TABLE negotiation.meeting_changes (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    meeting_id      uuid        NOT NULL REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    field           text        NOT NULL,
    old_value       jsonb,
    new_value       jsonb,
    detected_at     timestamptz NOT NULL,
    import_run_id   uuid        REFERENCES negotiation.import_runs(id) ON DELETE SET NULL,
    CONSTRAINT ck_meeting_changes_field
        CHECK (field IN ('start', 'end', 'venue', 'title', 'type', 'access', 'agenda_item', 'status'))
);

CREATE INDEX ix_meeting_changes_meeting ON negotiation.meeting_changes (meeting_id, detected_at DESC);

COMMENT ON TABLE negotiation.meeting_changes IS
    'Écarts constatés par l''import, champ par champ. « Déplacée » s''en déduit (un changement start, end ou venue) : aucun état stocké ne se désynchronise de l''heure. Pas d''audit, c''est déjà un journal.';
COMMENT ON COLUMN negotiation.meeting_changes.field IS
    'Liste close du code de l''import, pas un vocabulaire : un champ de plus est une évolution du comparateur.';
COMMENT ON COLUMN negotiation.meeting_changes.detected_at IS
    'Heure de la lecture qui a vu l''écart.';
COMMENT ON COLUMN negotiation.meeting_changes.import_run_id IS
    'La lecture qui a vu l''écart. Mise à nul quand le journal est purgé : l''écart, lui, reste.';

-- Une traduction par titre anglais, partagée par les sessions de même titre.
CREATE TABLE negotiation.title_translations (
    source_text     text        PRIMARY KEY,
    text_fr         text        NOT NULL,
    model           text        NOT NULL,
    translated_at   timestamptz NOT NULL DEFAULT now()
);

COMMENT ON TABLE negotiation.title_translations IS
    'Traduction automatique des titres de sessions officielles, une par titre anglais. ÉCART AU PRINCIPE XII, décidé par le commanditaire le 25/09 : publiée SANS relecture humaine, bornée aux titres de sessions officielles (une COP en compte des centaines par jour), toujours affichée sous le titre anglais — qui fait foi — et marquée « Traduction automatique ». Pas d''audit : c''est le travail qui écrit, et le modèle est noté.';
COMMENT ON COLUMN negotiation.title_translations.source_text IS
    'Le titre anglais exact, tel que gardé dans meetings.title_original : un titre changé à la source appelle une nouvelle traduction.';
COMMENT ON COLUMN negotiation.title_translations.model IS
    'Le modèle d''OpenRouter qui l''a produite (réglage ai.drafting_model au moment de la traduction).';

-- La règle de coupure, en un seul endroit : la route publique et le back-office
-- la lisent, aucun code ne la réécrit.
CREATE OR REPLACE FUNCTION negotiation.import_is_serving(p_event_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT coalesce((
        SELECT i.is_enabled
           AND i.last_success_at IS NOT NULL
           AND i.missed_reads < i.missed_threshold
           AND now() - i.last_success_at
               < make_interval(secs => i.missed_threshold * i.interval_seconds + 60)
          FROM negotiation.official_imports i
         WHERE i.event_id = p_event_id
    ), false);
$$;

COMMENT ON FUNCTION negotiation.import_is_serving(uuid) IS
    'Vrai si les sessions importées de l''édition peuvent s''afficher : import allumé, au moins une lecture réussie, moins de missed_threshold lectures manquées d''affilée, ET dernière réussite plus récente que missed_threshold × interval_seconds + 60 s — un worker arrêté ne manque aucune lecture, et ne doit pas laisser servir une liste périmée. Faux sans ligne d''import.';

-- -----------------------------------------------------------------------------
-- 4 ter. « Mon groupe » et « Mon agenda » — Guide Négo, étape 3a
-- -----------------------------------------------------------------------------

-- Les groupes de négociation suivis : même patron que theme_subscriptions.
-- Aucun groupe suivi : toutes les coordinations passent le filtre.
CREATE TABLE negotiation.group_subscriptions (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_group_subscriptions_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    group_term_id   uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    followed_at     timestamptz NOT NULL DEFAULT now(),
    left_at         timestamptz,
    CONSTRAINT ck_group_subscriptions_period CHECK (left_at IS NULL OR left_at >= followed_at)
);

CREATE UNIQUE INDEX ux_group_subscriptions_active
    ON negotiation.group_subscriptions (person_id, group_term_id) WHERE left_at IS NULL;
CREATE INDEX ix_group_subscriptions_group
    ON negotiation.group_subscriptions (group_term_id, followed_at DESC) WHERE left_at IS NULL;

CREATE TRIGGER tg_group_subscriptions_audit
    AFTER INSERT OR UPDATE OR DELETE ON negotiation.group_subscriptions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_group_subscriptions_check_group
    BEFORE INSERT OR UPDATE OF group_term_id ON negotiation.group_subscriptions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'group_term_id', 'negotiation_group');

COMMENT ON TABLE negotiation.group_subscriptions IS
    'Groupes de négociation qu''une personne suit (« Mon groupe »). N''ouvre aucun droit : commande les coordinations qu''on lui montre. Même patron que theme_subscriptions.';
COMMENT ON COLUMN negotiation.group_subscriptions.left_at IS
    'Un suivi se ferme, il ne se supprime pas ; l''index unique ne porte que sur le suivi vivant.';

-- « Mon agenda » : les sessions officielles qu'une personne garde. Pas une
-- inscription (meeting_registrations) : suivre une session officielle n'engage
-- aucune place, sans capacité ni liste d'attente. Aucune contrainte de
-- chevauchement : deux sessions à la même heure se gardent (règle n° 2).
CREATE TABLE negotiation.agenda_entries (
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_agenda_entries_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    meeting_id      uuid        NOT NULL REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    remind_before   interval,
    added_at        timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, meeting_id),
    CONSTRAINT ck_agenda_entries_remind
        CHECK (remind_before IS NULL OR remind_before = interval '15 minutes')
);

CREATE INDEX ix_agenda_entries_meeting ON negotiation.agenda_entries (meeting_id);

CREATE TRIGGER tg_agenda_entries_updated_at BEFORE UPDATE ON negotiation.agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_agenda_entries_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('meeting_id');

COMMENT ON TABLE negotiation.agenda_entries IS
    'Sessions officielles gardées dans « Mon agenda ». Le retrait supprime la ligne. Pas une inscription : aucune place engagée, aucun refus pour chevauchement.';
COMMENT ON COLUMN negotiation.agenda_entries.remind_before IS
    'Rappel dans l''application ouverte : nul = aucun, sinon 15 minutes, seule valeur offerte. Sans effet sur une session annulée.';

-- -----------------------------------------------------------------------------
-- 4 quater. Signalements et réunions non annoncées — Guide Négo, étape 3b
--
-- Un signalement ne touche JAMAIS la session officielle (ADR-010) : il vit dans
-- sa table et se pose par-dessus, une fois validé ET publié. `published_at` est
-- la seule porte du public : l'encart, la réunion non annoncée, « Validé » chez
-- l'autrice, les avis et les courriels n'existent qu'une fois posé. Entre la
-- validation et la publication, trente secondes où l'annulation gagne toujours
-- (specs/015-guide-nego-signalements, R3).
--
-- Aucun déclencheur d'outbox sur ces tables : les événements naissent du travail
-- de publication et du refus, qui calculent les destinataires par
-- change_recipients() et network_recipients() ci-dessous (R8).
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.report_status AS ENUM ('submitted', 'validated', 'rejected');

CREATE TABLE negotiation.session_reports (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    author_id           uuid        NOT NULL CONSTRAINT xmod_fk_session_reports_author
                                    REFERENCES identity.people(id) ON DELETE CASCADE,
    client_ref          uuid        NOT NULL,
    event_id            uuid        NOT NULL CONSTRAINT xmod_fk_session_reports_event
                                    REFERENCES event.events(id) ON DELETE RESTRICT,
    meeting_id          uuid        REFERENCES negotiation.meetings(id) ON DELETE CASCADE,
    reason              text        NOT NULL,
    proposed_start      timestamptz,
    proposed_venue      text,
    what                text,
    proposed_day        date,
    theme_term_id       uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    detail              text,
    status              negotiation.report_status NOT NULL DEFAULT 'submitted',
    submitted_at        timestamptz NOT NULL DEFAULT now(),
    decided_by          uuid        CONSTRAINT xmod_fk_session_reports_decider
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    decided_at          timestamptz,
    reject_reason       text,
    reject_detail       text,
    source_snapshot     jsonb,
    published_at        timestamptz,
    withdrawn_at        timestamptz,
    withdrawal          text,
    network_meeting_id  uuid,
    CONSTRAINT ux_session_reports_client_ref UNIQUE (author_id, client_ref),
    CONSTRAINT ck_session_reports_reason
        CHECK (reason IN ('cancelled', 'time', 'venue', 'other', 'unannounced')),
    CONSTRAINT ck_session_reports_target
        CHECK ((meeting_id IS NULL) = (reason = 'unannounced')),
    CONSTRAINT ck_session_reports_unannounced
        CHECK (CASE WHEN reason = 'unannounced'
                    THEN what IS NOT NULL AND proposed_day IS NOT NULL
                    ELSE what IS NULL AND proposed_day IS NULL AND theme_term_id IS NULL
                         AND network_meeting_id IS NULL END),
    CONSTRAINT ck_session_reports_detail CHECK (char_length(detail) <= 600),
    CONSTRAINT ck_session_reports_reject_reason
        CHECK (reject_reason IN ('source_maintains', 'already_known', 'not_precise')),
    CONSTRAINT ck_session_reports_reject_detail CHECK (char_length(reject_detail) <= 600),
    CONSTRAINT ck_session_reports_decision
        CHECK (CASE status
                    WHEN 'submitted' THEN decided_by IS NULL AND decided_at IS NULL
                         AND source_snapshot IS NULL AND published_at IS NULL
                    ELSE decided_at IS NOT NULL END
               AND (status = 'rejected') = (reject_reason IS NOT NULL)
               AND (reject_detail IS NULL OR reject_reason IS NOT NULL)
               AND (published_at IS NULL OR status = 'validated')),
    CONSTRAINT ck_session_reports_withdrawal
        CHECK ((withdrawal IN ('caught_up', 'admin') AND withdrawn_at IS NOT NULL AND status = 'validated')
               OR (withdrawal IS NULL AND withdrawn_at IS NULL))
);

CREATE UNIQUE INDEX ux_session_reports_pending ON negotiation.session_reports (author_id, meeting_id)
    WHERE status = 'submitted' AND meeting_id IS NOT NULL;
CREATE INDEX ix_session_reports_queue ON negotiation.session_reports (submitted_at)
    WHERE status = 'submitted';
CREATE INDEX ix_session_reports_meeting ON negotiation.session_reports (meeting_id)
    WHERE published_at IS NOT NULL AND withdrawn_at IS NULL;

CREATE TRIGGER tg_session_reports_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.session_reports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_session_reports_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.session_reports
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

COMMENT ON TABLE negotiation.session_reports IS
    'Signalements du réseau sur les sessions officielles, et réunions non annoncées (motif unannounced, sans session). Ne modifie jamais la session : se pose par-dessus, une fois validé et publié (ADR-010). Aucun déclencheur d''outbox : les événements naissent du travail de publication et du refus.';
COMMENT ON COLUMN negotiation.session_reports.client_ref IS
    'Référence posée par le téléphone à la création : un envoi rejoué par la file retrouve la ligne au lieu d''en créer une seconde (ux_session_reports_client_ref).';
COMMENT ON COLUMN negotiation.session_reports.event_id IS
    'La COP du signalement, posée aussi pour une réunion non annoncée, qui n''a pas de session.';
COMMENT ON COLUMN negotiation.session_reports.meeting_id IS
    'Session officielle signalée ; nulle si et seulement si le motif est unannounced (ck_session_reports_target).';
COMMENT ON COLUMN negotiation.session_reports.reason IS
    'Motif, code stable : cancelled (annulée), time (heure changée), venue (salle changée), other, unannounced (réunion non annoncée).';
COMMENT ON COLUMN negotiation.session_reports.proposed_start IS
    'Nouvelle heure de début (motif time), ou début d''une réunion non annoncée — nul = sans heure.';
COMMENT ON COLUMN negotiation.session_reports.proposed_venue IS
    'Nouvelle salle (motif venue), ou « Où » d''une réunion non annoncée.';
COMMENT ON COLUMN negotiation.session_reports.what IS
    '« Quoi » d''une réunion non annoncée ; requis pour ce motif, nul pour les autres (ck_session_reports_unannounced).';
COMMENT ON COLUMN negotiation.session_reports.proposed_day IS
    'Jour de la liste où la réunion non annoncée s''affiche, dans le fuseau de la COP ; requis pour ce motif.';
COMMENT ON COLUMN negotiation.session_reports.theme_term_id IS
    'Thématique (negotiation_theme) d''une réunion non annoncée ; une session signalée tient la sienne de son point de l''ordre du jour.';
COMMENT ON COLUMN negotiation.session_reports.detail IS
    'Précision libre de l''autrice, 600 caractères au plus. Jamais affichée avec son nom.';
COMMENT ON COLUMN negotiation.session_reports.status IS
    'submitted → validated | rejected ; validated → submitted tant que published_at est nul (annulation). « Validé » ne se montre à l''autrice qu''une fois publié.';
COMMENT ON COLUMN negotiation.session_reports.decided_by IS
    'Administratrice ou administrateur qui a validé ou refusé ; remis à nul par l''annulation.';
COMMENT ON COLUMN negotiation.session_reports.reject_reason IS
    'Motif du refus, code stable : source_maintains (la source maintient), already_known (déjà connu), not_precise (pas assez précis). Posé si et seulement si refusé.';
COMMENT ON COLUMN negotiation.session_reports.source_snapshot IS
    'Ce que disait la source officielle au moment de la décision (ADR-010) : en validant, l''administration voit la source à cet instant.';
COMMENT ON COLUMN negotiation.session_reports.published_at IS
    'Seule porte du public, posée par le travail de publication trente secondes après la validation, jamais avant. Tant qu''elle est nulle, rien n''est montré ni envoyé, et l''annulation gagne.';
COMMENT ON COLUMN negotiation.session_reports.withdrawn_at IS
    'Retrait de l''affichage : la source a rattrapé le signalement (caught_up), ou l''administration l''a retiré (admin). La fin de session n''écrit rien, elle se calcule.';
COMMENT ON COLUMN negotiation.session_reports.network_meeting_id IS
    'Réunion non annoncée née de ce signalement, posée par le travail de publication.';

-- La réunion non annoncée n'est PAS une ligne de meetings : chaque requête de 3a
-- filtre les sessions importées par source_key, et un oubli ferait passer une
-- réunion du réseau pour officielle. Deux tables, deux sens (R2).
CREATE TABLE negotiation.network_meetings (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    event_id        uuid        NOT NULL CONSTRAINT xmod_fk_network_meetings_event
                                REFERENCES event.events(id) ON DELETE RESTRICT,
    report_id       uuid        NOT NULL REFERENCES negotiation.session_reports(id) ON DELETE CASCADE,
    title           text        NOT NULL,
    venue           text,
    start_at        timestamptz,
    day             date        NOT NULL,
    theme_term_id   uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    validated_at    timestamptz NOT NULL,
    withdrawn_at    timestamptz,
    CONSTRAINT ux_network_meetings_report UNIQUE (report_id)
);

CREATE INDEX ix_network_meetings_event_day ON negotiation.network_meetings (event_id, day)
    WHERE withdrawn_at IS NULL;

CREATE TRIGGER tg_network_meetings_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.network_meetings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();
CREATE TRIGGER tg_network_meetings_check_theme
    BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.network_meetings
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy(
        'theme_term_id', 'negotiation_theme');

ALTER TABLE negotiation.session_reports ADD CONSTRAINT fk_session_reports_network_meeting
    FOREIGN KEY (network_meeting_id) REFERENCES negotiation.network_meetings(id) ON DELETE SET NULL;

COMMENT ON TABLE negotiation.network_meetings IS
    'Réunions non annoncées, signalées par le réseau et validées. Créées par le travail de publication, jamais avant. Aucune colonne de source : ce n''est pas une session officielle, et elle ne se montre jamais comme telle.';
COMMENT ON COLUMN negotiation.network_meetings.title IS
    '« Quoi », tel que signalé.';
COMMENT ON COLUMN negotiation.network_meetings.start_at IS
    'Heure de début ; nulle = sans heure.';
COMMENT ON COLUMN negotiation.network_meetings.day IS
    'Jour de la liste, dans le fuseau de la COP. La réunion quitte l''affichage à la fin de ce jour, sans rien écrire.';
COMMENT ON COLUMN negotiation.network_meetings.withdrawn_at IS
    'Retrait par l''administration.';

-- « Mon agenda » pour une réunion non annoncée : table sœur d'agenda_entries,
-- que 3a garde intacte.
CREATE TABLE negotiation.network_agenda_entries (
    person_id           uuid        NOT NULL CONSTRAINT xmod_fk_network_agenda_entries_person
                                    REFERENCES identity.people(id) ON DELETE CASCADE,
    network_meeting_id  uuid        NOT NULL REFERENCES negotiation.network_meetings(id) ON DELETE CASCADE,
    remind_before       interval,
    added_at            timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, network_meeting_id),
    CONSTRAINT ck_network_agenda_entries_remind
        CHECK (remind_before IS NULL OR remind_before = interval '15 minutes')
);

CREATE INDEX ix_network_agenda_entries_meeting ON negotiation.network_agenda_entries (network_meeting_id);

CREATE TRIGGER tg_network_agenda_entries_updated_at BEFORE UPDATE ON negotiation.network_agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_network_agenda_entries_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.network_agenda_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('network_meeting_id');

COMMENT ON TABLE negotiation.network_agenda_entries IS
    'Réunions non annoncées gardées dans « Mon agenda ». Même règle qu''agenda_entries : le retrait supprime la ligne, aucune place engagée.';
COMMENT ON COLUMN negotiation.network_agenda_entries.remind_before IS
    'Rappel dans l''application ouverte : nul = aucun, sinon 15 minutes, seule valeur offerte.';

-- Qui prévenir : une seule règle, lue par negotiation seul — l'import, le
-- travail de publication, le travail de courriel. engagement reçoit la liste
-- dans l'événement et ne lit rien ici (R8).
CREATE OR REPLACE FUNCTION negotiation.change_recipients(p_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE sql
STABLE
AS $$
    SELECT ae.person_id
      FROM negotiation.agenda_entries ae
     WHERE ae.meeting_id = p_meeting_id
    UNION
    SELECT ts.person_id
      FROM negotiation.meetings m
      JOIN negotiation.agenda_items ai ON ai.id = m.agenda_item_id
      JOIN negotiation.theme_subscriptions ts
        ON ts.theme_term_id = ai.theme_term_id AND ts.left_at IS NULL AND ts.notify_changes
     WHERE m.id = p_meeting_id;
$$;

COMMENT ON FUNCTION negotiation.change_recipients(uuid) IS
    'Personnes à prévenir d''un changement de session : celles qui l''ont dans « Mon agenda », et celles qui suivent sa thématique (celle de son point de l''ordre du jour) avec notify_changes. Sans doublon.';

CREATE OR REPLACE FUNCTION negotiation.network_recipients(p_network_meeting_id uuid)
RETURNS SETOF uuid
LANGUAGE sql
STABLE
AS $$
    SELECT na.person_id
      FROM negotiation.network_agenda_entries na
     WHERE na.network_meeting_id = p_network_meeting_id
    UNION
    SELECT ts.person_id
      FROM negotiation.network_meetings nm
      JOIN negotiation.theme_subscriptions ts
        ON ts.theme_term_id = nm.theme_term_id AND ts.left_at IS NULL AND ts.notify_changes
     WHERE nm.id = p_network_meeting_id;
$$;

COMMENT ON FUNCTION negotiation.network_recipients(uuid) IS
    'Personnes à prévenir d''une réunion non annoncée : celles qui l''ont dans « Mon agenda », et celles qui suivent sa thématique avec notify_changes. Sans doublon.';

-- -----------------------------------------------------------------------------
-- 5. Documents d'aide à la négociation  (correction D3)
-- -----------------------------------------------------------------------------
CREATE TABLE negotiation.documents (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- NULL = document transversal, visible depuis tous les espaces.
    space_id              uuid        REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    slug                  platform.slug NOT NULL,
    title                 platform.i18n_text NOT NULL,
    summary               platform.i18n_text,
    -- Type via la taxonomie `document_type` (guide, note technique, rapport...).
    document_type_term_id uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    track_term_id         uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    locale_code           text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    version               text        NOT NULL DEFAULT '1',
    -- Chaînage des versions : on ne remplace jamais un document cité ailleurs.
    supersedes_id         uuid        REFERENCES negotiation.documents(id) ON DELETE SET NULL,

    -- SOURCE : soit un objet stocké (Garage/S3 via le module media), soit un
    -- lien externe. Jamais les deux ; exactement un dès la publication — un
    -- brouillon naît sans source, et son PDF se dépose ensuite (Guide Négo R5).
    asset_id              uuid        CONSTRAINT xmod_fk_documents_asset
                                      REFERENCES media.assets(id) ON DELETE RESTRICT,
    external_url          platform.url,
    -- Qui a produit le document, qu'il soit fichier ou lien : « Secrétariat de
    -- la CCNUCC », « OIF/IFDD ».
    publisher             text,
    cover_asset_id        uuid        CONSTRAINT xmod_fk_documents_cover
                                      REFERENCES media.assets(id) ON DELETE SET NULL,

    published_at          timestamptz,
    is_restricted         boolean     NOT NULL DEFAULT true,   -- réservé aux membres de l'espace
    download_count        integer     NOT NULL DEFAULT 0,
    -- Éligibilité et état d'indexation pour le RAG de l'assistant négociateur :
    -- les embeddings (pgvector) vivent dans le module `tool`, ce module ne
    -- publie que le signal d'indexation. Faux par défaut : seul ce qu'un humain
    -- a choisi est lu par l'assistant (ADR-011 de Guide Négo).
    is_rag_eligible       boolean     NOT NULL DEFAULT false,
    rag_indexed_at        timestamptz,
    migrated_from_v1      boolean     NOT NULL DEFAULT false,
    uploaded_by           uuid        CONSTRAINT xmod_fk_documents_uploader
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    search_vector         tsvector    GENERATED ALWAYS AS (
        to_tsvector('french',
            coalesce(title ->> 'fr', '')   || ' ' ||
            coalesce(title ->> 'en', '')   || ' ' ||
            coalesce(summary ->> 'fr', '') || ' ' ||
            coalesce(publisher, ''))
    ) STORED,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),

    -- Guide Négo, étape 1. Ces trois colonnes closent la table : elles sont
    -- arrivées par ALTER sur une base en service.
    -- La COP que le document concerne, au plus une.
    event_id              uuid        CONSTRAINT xmod_fk_documents_event
                                      REFERENCES event.events(id) ON DELETE RESTRICT,
    -- La date du document lui-même, distincte de sa publication sur la plateforme.
    issued_on             date,
    -- Posée au retrait, effacée à la republication : distingue un document
    -- retiré d'un brouillon jamais publié.
    unpublished_at        timestamptz,

    CONSTRAINT ck_documents_source_at_most_one CHECK (num_nonnulls(asset_id, external_url) <= 1),
    CONSTRAINT ck_documents_published_has_source
        CHECK (published_at IS NULL OR num_nonnulls(asset_id, external_url) = 1),
    CONSTRAINT ck_documents_not_self_superseding CHECK (supersedes_id IS DISTINCT FROM id)
);

-- Unicité du couple (espace, slug, version) — COALESCE pour traiter les
-- documents transversaux (space_id NULL) comme un espace virtuel unique.
CREATE UNIQUE INDEX ux_documents_slug
    ON negotiation.documents (COALESCE(space_id, '00000000-0000-0000-0000-000000000000'::uuid), slug, version);
CREATE INDEX ix_documents_published ON negotiation.documents (space_id, published_at DESC)
    WHERE published_at IS NOT NULL;
CREATE INDEX ix_documents_type      ON negotiation.documents (document_type_term_id, published_at DESC);
CREATE INDEX ix_documents_track     ON negotiation.documents (track_term_id) WHERE track_term_id IS NOT NULL;
CREATE INDEX ix_documents_search    ON negotiation.documents USING gin (search_vector);
CREATE INDEX ix_documents_rag_queue ON negotiation.documents (created_at)
    WHERE is_rag_eligible AND rag_indexed_at IS NULL AND published_at IS NOT NULL;
CREATE INDEX ix_documents_event     ON negotiation.documents (event_id) WHERE event_id IS NOT NULL;
-- Un document n'a qu'un successeur : sans quoi « le document à jour » serait
-- deux documents.
CREATE UNIQUE INDEX ux_documents_supersedes ON negotiation.documents (supersedes_id)
    WHERE supersedes_id IS NOT NULL;

CREATE TRIGGER tg_documents_updated_at BEFORE UPDATE ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_documents_check_type BEFORE INSERT OR UPDATE OF document_type_term_id ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('document_type_term_id', 'document_type');
CREATE TRIGGER tg_documents_check_track BEFORE INSERT OR UPDATE OF track_term_id ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('track_term_id', 'negotiation_track');
CREATE TRIGGER tg_documents_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

-- Un remplacement ne boucle jamais : si A remplace B qui remplace A, « le
-- document à jour » n'existe plus. La chaîne se remonte depuis le document
-- remplacé ; y retrouver le document écrit est une boucle.
CREATE OR REPLACE FUNCTION negotiation.tg_documents_no_supersede_cycle()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.supersedes_id IS NOT NULL AND EXISTS (
        WITH RECURSIVE chaine(id) AS (
            SELECT NEW.supersedes_id
            UNION
            SELECT d.supersedes_id
              FROM negotiation.documents d
              JOIN chaine c ON c.id = d.id
             WHERE d.supersedes_id IS NOT NULL
        )
        SELECT 1 FROM chaine WHERE id = NEW.id
    ) THEN
        RAISE EXCEPTION 'Le document % remplacerait, même de loin, un document qui le remplace', NEW.id
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_documents_supersede_cycle';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_documents_no_supersede_cycle
    BEFORE INSERT OR UPDATE OF supersedes_id ON negotiation.documents
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_documents_no_supersede_cycle();

COMMENT ON TABLE negotiation.documents IS
    'Documents d''aide : fichier stocké OU lien externe, jamais les deux, exactement un une fois publié. Versionnés, typés par taxonomie, indexés plein texte. Thématiques par reference.entity_terms (negotiation_theme).';
COMMENT ON CONSTRAINT ck_documents_source_at_most_one ON negotiation.documents IS
    'Jamais un fichier ET un lien. Corrige la v1 où `file_url` accueillait indifféremment une URL de stockage et un lien tiers, rendant purge et indexation impossibles à automatiser.';
COMMENT ON CONSTRAINT ck_documents_published_has_source ON negotiation.documents IS
    'Un brouillon peut naître sans source — son PDF se dépose avec lui pour propriétaire ; un document publié en a exactement une.';
COMMENT ON COLUMN negotiation.documents.event_id IS
    'La COP que le document concerne, au plus une. NULL : il ne se rattache à aucune édition.';
COMMENT ON COLUMN negotiation.documents.unpublished_at IS
    'Date du retrait. published_at NULL et unpublished_at posé : dépublié ; les deux NULL : brouillon.';
COMMENT ON FUNCTION negotiation.tg_documents_no_supersede_cycle() IS
    'Refuse un remplacement qui bouclerait (ck_documents_supersede_cycle) : le bout publié de la chaîne doit exister.';

-- Incrément du compteur de téléchargements. Les statistiques fines (qui, quand,
-- depuis où) relèvent du module analytics, pas de cette colonne.
CREATE OR REPLACE FUNCTION negotiation.register_document_download(p_document_id uuid)
RETURNS void
LANGUAGE sql
AS $$
    UPDATE negotiation.documents SET download_count = download_count + 1 WHERE id = p_document_id;
$$;

-- Favoris — remplace `user_favorite_documents` (clé de substitution inutile,
-- l'unicité (personne, document) EST la clé).
CREATE TABLE negotiation.document_bookmarks (
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_document_bookmarks_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    document_id uuid        NOT NULL REFERENCES negotiation.documents(id) ON DELETE CASCADE,
    note        text,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, document_id)
);

CREATE INDEX ix_document_bookmarks_document ON negotiation.document_bookmarks (document_id);
CREATE INDEX ix_document_bookmarks_recent   ON negotiation.document_bookmarks (person_id, created_at DESC);

-- -----------------------------------------------------------------------------
-- 5 bis. La forme lisible d'un document — Guide Négo, étape 1
--
-- Le lecteur de Guide Négo recompose le texte du PDF publié : l'extraction le
-- découpe en pages de blocs typés (grammaire close, sans HTML), repère le
-- sommaire, et rend une image de chaque page, déposée dans le bucket privé.
-- Tout vit ici, page par page : la recherche dans le texte, l'ancre des notes
-- de correction, et un seul corps servi au téléphone
-- (specs/011-guide-nego-documents, R3, R6, R7).
-- -----------------------------------------------------------------------------

CREATE TYPE negotiation.rendition_status AS ENUM ('pending', 'extracting', 'ready', 'failed');

COMMENT ON TYPE negotiation.rendition_status IS
    'pending → extracting → ready | failed. Relancer l''extraction ramène à pending.';

CREATE TABLE negotiation.document_renditions (
    document_id     uuid        PRIMARY KEY REFERENCES negotiation.documents(id) ON DELETE CASCADE,
    -- Le fichier extrait : un autre fichier donne une autre extraction.
    asset_id        uuid        NOT NULL CONSTRAINT xmod_fk_document_renditions_asset
                                REFERENCES media.assets(id) ON DELETE RESTRICT,
    status          negotiation.rendition_status NOT NULL DEFAULT 'pending',
    -- Pages du document, pas du fichier.
    page_count      integer     CHECK (page_count IS NULL OR page_count > 0),
    -- Sommaire, en grammaire close : titre, niveau, page, enfants.
    outline         jsonb,
    -- Le verdict : le texte se recompose-t-il ?
    is_reflowable   boolean,
    -- Les indicateurs du verdict : pages avec texte, blocs d'origine, termes…
    quality         jsonb,
    -- Proposer « Texte agrandi » au téléphone. NULL : suit le verdict
    -- (is_reflowable) ; vrai ou faux : le choix de l'administratrice, qui se
    -- change sans republier. La règle vit dans document_reading_modes().
    large_text_choice boolean,
    -- Poids de la copie que garde le téléphone : le PDF et la forme lisible
    -- servie (ADR-022). C'est la taille annoncée.
    reading_bytes   bigint      CHECK (reading_bytes IS NULL OR reading_bytes >= 0),
    -- Outil et version, pour savoir quoi réextraire le jour où ils changent.
    extractor       text,
    failure_reason  text,
    attempts        integer     NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    extracted_at    timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    -- La dernière demande d'extraction : seul son travail écrit ici. Une
    -- relance en pose une nouvelle, et le travail d'une demande antérieure ne
    -- peut plus conclure par-dessus — ni laisser publier avant la relance.
    request_id      uuid        NOT NULL DEFAULT platform.uuid_v7(),

    CONSTRAINT ck_document_renditions_ready  CHECK (status <> 'ready' OR page_count > 0),
    CONSTRAINT ck_document_renditions_failed CHECK (status <> 'failed' OR failure_reason IS NOT NULL)
);

CREATE INDEX ix_document_renditions_asset ON negotiation.document_renditions (asset_id);

CREATE TRIGGER tg_document_renditions_updated_at BEFORE UPDATE ON negotiation.document_renditions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Audité, bien que le worker y écrive surtout : le choix « Texte agrandi » de
-- l'administratrice vit ici, et c'est une décision éditoriale.
CREATE TRIGGER tg_document_renditions_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.document_renditions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('document_id');

COMMENT ON TABLE negotiation.document_renditions IS
    'L''extraction d''un document fichier : son état, son verdict, son sommaire, et le choix « Texte agrandi ». Une ligne par document.';
COMMENT ON COLUMN negotiation.document_renditions.large_text_choice IS
    'Proposer « Texte agrandi » au téléphone. NULL = suit le verdict de l''extraction (is_reflowable) ; vrai ou faux = choix de l''administratrice, qui se change sans republier (arbitré le 24/09). Une relance d''extraction le garde. Lu par negotiation.document_reading_modes().';
COMMENT ON COLUMN negotiation.document_renditions.reading_bytes IS
    'Octets du PDF + octets du JSON de lecture : la copie gardée depuis l''étape 1b (ADR-022). Calculé à la fin de l''extraction ; c''est la taille annoncée sur la fiche.';

CREATE TABLE negotiation.document_pages (
    document_id       uuid     NOT NULL REFERENCES negotiation.documents(id) ON DELETE CASCADE,
    -- De 1 au nombre de pages : clé de la progression et des notes.
    page_index        integer  NOT NULL CHECK (page_index > 0),
    -- L'étiquette imprimée, « 59 ».
    label             text     NOT NULL,
    -- Les blocs de la page, en grammaire close : la matière de « Texte agrandi ».
    blocks            jsonb    NOT NULL DEFAULT '[]',
    -- La concaténation du texte des blocs : rien ne se cherche qui ne s'affiche pas.
    plain_text        text     NOT NULL DEFAULT '',
    -- Sans accents : sinon « progres » ne trouve pas « progrès », les deux mots
    -- n'ayant pas la même racine. La requête s'écrit de même.
    search_vector     tsvector GENERATED ALWAYS AS (
        to_tsvector('french', platform.immutable_unaccent(plain_text))
    ) STORED,
    -- L'image de la page, dans le bucket privé : l'aperçu du back-office.
    image_key         text,
    image_bytes       integer  CHECK (image_bytes IS NULL OR image_bytes > 0),
    -- Un tableau ou une figure : « Texte agrandi » y renvoie à la page d'origine.
    has_origin_block  boolean  NOT NULL DEFAULT false,
    PRIMARY KEY (document_id, page_index)
);

CREATE INDEX ix_document_pages_search ON negotiation.document_pages USING gin (search_vector);

COMMENT ON TABLE negotiation.document_pages IS
    'La forme lisible d''un document, page par page. Une nouvelle extraction remplace toutes les lignes du document, dans une transaction.';
COMMENT ON COLUMN negotiation.document_pages.image_key IS
    'Image de la page dans le bucket privé : aperçu du back-office — jamais servi au téléphone depuis l''étape 1b (ADR-022).';
COMMENT ON COLUMN negotiation.document_pages.image_bytes IS
    'Poids de l''image de la page : aperçu du back-office — jamais servi au téléphone depuis l''étape 1b.';
COMMENT ON COLUMN negotiation.document_pages.has_origin_block IS
    'La page porte un tableau ou une figure : « Texte agrandi » y renvoie à la page d''origine, et l''aperçu du back-office la montre. Aperçu du back-office — jamais d''image servie au téléphone depuis l''étape 1b.';

-- Ce que le téléphone peut offrir d'un document : la recherche et le sommaire
-- s'il a du texte, « Texte agrandi » si ce texte est proposé. Écrite une fois,
-- lue par la liste, la lecture et l'aperçu : la règle ne se recopie pas.
CREATE OR REPLACE FUNCTION negotiation.document_reading_modes(p_document_id uuid)
RETURNS TABLE (has_text boolean, large_text boolean)
LANGUAGE sql
STABLE
SECURITY INVOKER
AS $$
    WITH texte AS (
        SELECT EXISTS (
            SELECT 1 FROM negotiation.document_pages p
             WHERE p.document_id = p_document_id AND p.plain_text <> ''
        ) AS present
    )
    SELECT t.present,
           t.present AND coalesce(r.large_text_choice, r.is_reflowable, false)
      FROM texte t
      LEFT JOIN negotiation.document_renditions r ON r.document_id = p_document_id;
$$;

COMMENT ON FUNCTION negotiation.document_reading_modes(uuid) IS
    'has_text : une page au moins a du texte (recherche, sommaire). large_text : has_text ET coalesce(large_text_choice, is_reflowable, false) — « Texte agrandi » offert. Toujours une ligne, fausse pour un document sans extraction.';

-- La note d'un expert sur un passage dépassé. Elle se pose PAR-DESSUS le texte,
-- sans jamais le modifier, et ne se supprime pas : un retrait se date.
CREATE TABLE negotiation.correction_notes (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- Seul un brouillon se supprime ; ses notes partent avec lui.
    document_id   uuid        NOT NULL REFERENCES negotiation.documents(id) ON DELETE CASCADE,
    page_index    integer     NOT NULL,
    -- L'extrait cité du passage visé ; NULL : la note vaut pour la page.
    passage       text,
    body          platform.i18n_text NOT NULL,
    author_id     uuid        NOT NULL CONSTRAINT xmod_fk_correction_notes_author
                              REFERENCES identity.people(id) ON DELETE RESTRICT,
    created_at    timestamptz NOT NULL DEFAULT now(),
    withdrawn_at  timestamptz,
    withdrawn_by  uuid        CONSTRAINT xmod_fk_correction_notes_withdrawer
                              REFERENCES identity.people(id) ON DELETE RESTRICT,

    CONSTRAINT ck_correction_notes_withdrawal CHECK ((withdrawn_at IS NULL) = (withdrawn_by IS NULL))
);

CREATE INDEX ix_correction_notes_live ON negotiation.correction_notes (document_id, page_index)
    WHERE withdrawn_at IS NULL;

-- La page visée existe : une note sans ancre ne se lirait nulle part.
CREATE OR REPLACE FUNCTION negotiation.tg_correction_notes_page_exists()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM negotiation.document_pages p
         WHERE p.document_id = NEW.document_id AND p.page_index = NEW.page_index
    ) THEN
        RAISE EXCEPTION 'La page % du document % n''existe pas', NEW.page_index, NEW.document_id
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_correction_notes_page_exists';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_correction_notes_page_exists
    BEFORE INSERT OR UPDATE OF document_id, page_index ON negotiation.correction_notes
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_correction_notes_page_exists();

CREATE TRIGGER tg_correction_notes_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.correction_notes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.correction_notes IS
    'Notes de correction d''un expert, posées sur une page sans modifier le texte. Jamais supprimées : un retrait se date (withdrawn_at, withdrawn_by).';
COMMENT ON COLUMN negotiation.correction_notes.passage IS
    'L''extrait cité du passage visé, retrouvé dans le texte à l''affichage ; NULL, ou introuvable : la note se pose en tête de page.';

-- Le PDF d'un document et le fichier extrait sont la donnée, pas une
-- illustration : sans cette déclaration, le média les tiendrait pour orphelins.
INSERT INTO media.asset_references (ref_schema, ref_table, ref_column) VALUES
    ('negotiation', 'documents',           'asset_id'),
    ('negotiation', 'documents',           'cover_asset_id'),
    ('negotiation', 'document_renditions', 'asset_id')
ON CONFLICT DO NOTHING;

-- -----------------------------------------------------------------------------
-- 6. Canaux d'échange temps réel  (correction D4)
--
-- Créés dynamiquement par les animateurs : par thématique, par promotion de
-- formation, ou en groupe de travail éphémère.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.channel_kind AS ENUM (
    'thematic',      -- adossé à une thématique / filière
    'cohort',        -- promotion ou cohorte de formation
    'working_group', -- groupe de travail restreint et temporaire
    'announcement',  -- diffusion descendante, écriture réservée
    'support'        -- entraide et questions aux animateurs
);

CREATE TABLE negotiation.channels (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    space_id        uuid        NOT NULL REFERENCES negotiation.spaces(id) ON DELETE CASCADE,
    kind            negotiation.channel_kind NOT NULL DEFAULT 'thematic',
    slug            platform.slug NOT NULL,
    name            platform.i18n_text NOT NULL,
    topic           text,       -- sujet courant, modifiable par les modérateurs
    track_term_id   uuid        REFERENCES reference.taxonomy_terms(id) ON DELETE SET NULL,
    cohort_label    text,       -- ex. « Promotion 2026 »
    cohort_year     smallint    CHECK (cohort_year IS NULL OR cohort_year BETWEEN 2000 AND 2100),
    is_private      boolean     NOT NULL DEFAULT true,
    is_read_only    boolean     NOT NULL DEFAULT false,
    -- Compteurs dénormalisés : la liste des canaux se rend sans agrégat sur la
    -- table partitionnée des messages.
    message_count   bigint      NOT NULL DEFAULT 0,
    last_message_at timestamptz,
    archived_at     timestamptz,
    created_by      uuid        CONSTRAINT xmod_fk_channels_creator
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_channels_slug UNIQUE (space_id, slug),
    CONSTRAINT ck_channels_cohort CHECK (kind <> 'cohort' OR cohort_label IS NOT NULL),
    CONSTRAINT ck_channels_announcement_read_only CHECK (kind <> 'announcement' OR is_read_only)
);

-- Index partiel : la barre latérale ne liste que les canaux vivants.
CREATE INDEX ix_channels_active
    ON negotiation.channels (space_id, last_message_at DESC NULLS LAST)
    WHERE archived_at IS NULL;
CREATE INDEX ix_channels_cohort ON negotiation.channels (cohort_year, cohort_label)
    WHERE kind = 'cohort' AND archived_at IS NULL;

CREATE TRIGGER tg_channels_updated_at BEFORE UPDATE ON negotiation.channels
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE negotiation.channels IS
    'Canal d''échange temps réel créé dynamiquement, par thématique ou par promotion. Remplace `message_groups` de la v1.';

CREATE TYPE negotiation.channel_member_role AS ENUM ('owner', 'moderator', 'member');
CREATE TYPE negotiation.notification_level AS ENUM ('all', 'mentions', 'none');

CREATE TABLE negotiation.channel_members (
    channel_id           uuid        NOT NULL REFERENCES negotiation.channels(id) ON DELETE CASCADE,
    person_id            uuid        NOT NULL CONSTRAINT xmod_fk_channel_members_person
                                     REFERENCES identity.people(id) ON DELETE CASCADE,
    role                 negotiation.channel_member_role NOT NULL DEFAULT 'member',
    joined_at            timestamptz NOT NULL DEFAULT now(),
    -- Accusés de lecture : borne à partir de laquelle les messages sont non lus.
    last_read_at         timestamptz,
    last_read_message_id uuid,       -- sans FK : cible partitionnée (voir §7)
    is_muted             boolean     NOT NULL DEFAULT false,
    muted_until          timestamptz,
    notification_level   negotiation.notification_level NOT NULL DEFAULT 'all',
    left_at              timestamptz,
    PRIMARY KEY (channel_id, person_id)
);

CREATE INDEX ix_channel_members_person ON negotiation.channel_members (person_id) WHERE left_at IS NULL;

COMMENT ON COLUMN negotiation.channel_members.last_read_at IS
    'Horodatage du dernier message lu. Base du badge « non lus », absent de la v1.';

-- -----------------------------------------------------------------------------
-- 7. Messages — table partitionnée par mois
--
-- Table à plus forte volumétrie du module : mêmes règles que platform.audit_log.
--   * PRIMARY KEY (created_at, id) — la clé primaire d'une table partitionnée
--     DOIT contenir la colonne de partitionnement ;
--   * partition DEFAULT pour qu'aucune écriture ne puisse échouer si le worker
--     de maintenance a pris du retard ;
--   * AUCUNE FK ENTRANTE : elle imposerait de référencer le couple
--     (created_at, id) et interdirait DETACH/DROP des partitions anciennes.
--     Les références vers un message (réponse, accusé de lecture, réaction) sont
--     donc de simples colonnes uuid, accompagnées de l'horodatage du message
--     cible pour permettre l'élagage de partitions lors des jointures.
-- -----------------------------------------------------------------------------
CREATE TYPE negotiation.moderation_state AS ENUM ('visible', 'flagged', 'hidden', 'removed');

CREATE TABLE negotiation.channel_messages (
    id                  uuid        NOT NULL DEFAULT platform.uuid_v7(),
    created_at          timestamptz NOT NULL DEFAULT now(),
    channel_id          uuid        NOT NULL REFERENCES negotiation.channels(id) ON DELETE CASCADE,
    sender_person_id    uuid        NOT NULL CONSTRAINT xmod_fk_channel_messages_sender
                                    REFERENCES identity.people(id) ON DELETE CASCADE,
    body                text,
    -- Fil de discussion : réponse à un message. Couple (id, created_at) recopié
    -- pour cibler directement la bonne partition lors des mises à jour.
    parent_message_id   uuid,
    parent_created_at   timestamptz,
    thread_root_id      uuid,
    reply_count         integer     NOT NULL DEFAULT 0,
    attachment_asset_id uuid        CONSTRAINT xmod_fk_channel_messages_attachment
                                    REFERENCES media.assets(id) ON DELETE SET NULL,
    mentioned_person_ids uuid[]     NOT NULL DEFAULT '{}',
    -- Référence d'envoi optimiste côté client (déduplication des rejeux).
    client_reference    text,
    edited_at           timestamptz,
    deleted_at          timestamptz,           -- suppression douce
    deleted_by          uuid,
    moderation_state    negotiation.moderation_state NOT NULL DEFAULT 'visible',
    moderated_by        uuid,
    moderation_reason   text,
    PRIMARY KEY (created_at, id),
    CONSTRAINT ck_channel_messages_content
        CHECK (num_nonnulls(nullif(btrim(coalesce(body, '')), ''), attachment_asset_id) >= 1),
    CONSTRAINT ck_channel_messages_parent_shape
        CHECK ((parent_message_id IS NULL) = (parent_created_at IS NULL)),
    CONSTRAINT ck_channel_messages_not_self_parent CHECK (parent_message_id IS DISTINCT FROM id)
) PARTITION BY RANGE (created_at);

-- Fil de lecture d'un canal (index partiel : les messages retirés ne sont
-- jamais rendus, ils n'ont pas à peser dans l'index).
CREATE INDEX ix_channel_messages_timeline
    ON negotiation.channel_messages (channel_id, created_at DESC)
    WHERE deleted_at IS NULL AND moderation_state = 'visible';
CREATE INDEX ix_channel_messages_thread
    ON negotiation.channel_messages (thread_root_id, created_at)
    WHERE thread_root_id IS NOT NULL;
CREATE INDEX ix_channel_messages_sender
    ON negotiation.channel_messages (sender_person_id, created_at DESC);
CREATE INDEX ix_channel_messages_moderation
    ON negotiation.channel_messages (channel_id, created_at DESC)
    WHERE moderation_state = 'flagged';
CREATE INDEX ix_channel_messages_mentions
    ON negotiation.channel_messages USING gin (mentioned_person_ids);

COMMENT ON TABLE negotiation.channel_messages IS
    'Messages des canaux, partitionnés par mois. Purge et archivage par DROP/DETACH PARTITION, jamais par DELETE massif.';
COMMENT ON COLUMN negotiation.channel_messages.client_reference IS
    'Aucune contrainte d''unicité possible ici : elle devrait inclure created_at (clé de partitionnement) et perdrait tout effet. Le rejeu est filtré par l''API sur une fenêtre courte.';

CREATE TABLE negotiation.channel_messages_default PARTITION OF negotiation.channel_messages DEFAULT;

-- Partitions du mois courant et des trois suivants ; le worker de maintenance
-- appelle ensuite platform.ensure_month_partition() en continu.
DO $$
DECLARE
    v_month date;
BEGIN
    FOR v_month IN
        SELECT generate_series(date_trunc('month', now()),
                               date_trunc('month', now()) + interval '3 months',
                               interval '1 month')::date
    LOOP
        PERFORM platform.ensure_month_partition('negotiation', 'channel_messages', v_month);
    END LOOP;
END
$$;

-- Diffusion temps réel + compteurs. Un trigger FOR EACH ROW posé sur la table
-- partitionnée est propagé à toutes ses partitions (PostgreSQL 13+).
CREATE OR REPLACE FUNCTION negotiation.tg_channel_message_fanout()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE negotiation.channels
    SET message_count   = message_count + 1,
        last_message_at = greatest(coalesce(last_message_at, NEW.created_at), NEW.created_at)
    WHERE id = NEW.channel_id;

    IF NEW.parent_message_id IS NOT NULL THEN
        UPDATE negotiation.channel_messages
        SET reply_count = reply_count + 1
        WHERE id = NEW.parent_message_id
          AND created_at = NEW.parent_created_at;   -- élagage de partition
    END IF;

    -- Le serveur WebSocket écoute ce canal LISTEN et relaie aux membres
    -- connectés : aucune scrutation périodique côté client.
    PERFORM pg_notify('negotiation_channel_message', jsonb_build_object(
        'channel_id', NEW.channel_id, 'message_id', NEW.id,
        'sender_id', NEW.sender_person_id, 'created_at', NEW.created_at,
        'parent_message_id', NEW.parent_message_id
    )::text);
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_channel_messages_fanout AFTER INSERT ON negotiation.channel_messages
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_channel_message_fanout();

-- Réactions emoji. Volontairement SANS clé étrangère vers channel_messages :
-- une FK entrante vers une table partitionnée fige les partitions (impossible
-- de les détacher ou de les supprimer). `message_created_at` est recopié pour
-- retrouver la partition du message et purger les réactions en même temps que
-- lui. L'intégrité est assurée applicativement, le coût d'une orpheline étant
-- nul (une réaction sans message ne s'affiche jamais).
CREATE TABLE negotiation.channel_message_reactions (
    message_id         uuid        NOT NULL,
    message_created_at timestamptz NOT NULL,
    channel_id         uuid        NOT NULL REFERENCES negotiation.channels(id) ON DELETE CASCADE,
    person_id          uuid        NOT NULL CONSTRAINT xmod_fk_channel_message_reactions_person
                                   REFERENCES identity.people(id) ON DELETE CASCADE,
    emoji              text        NOT NULL CHECK (length(emoji) BETWEEN 1 AND 16),
    created_at         timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (message_id, person_id, emoji)
);

CREATE INDEX ix_channel_message_reactions_channel
    ON negotiation.channel_message_reactions (channel_id, message_created_at DESC);

-- -----------------------------------------------------------------------------
-- 8. Messages non lus
-- -----------------------------------------------------------------------------
-- Compteur de non-lus par canal pour une personne : une seule requête alimente
-- toute la barre latérale. La fenêtre `p_since` borne le balayage aux partitions
-- récentes — un membre absent six mois n'entraîne pas la lecture de l'historique
-- complet.
CREATE OR REPLACE FUNCTION negotiation.unread_message_counts(
    p_person_id uuid,
    p_since     interval DEFAULT '180 days'
)
RETURNS TABLE (
    channel_id      uuid,
    channel_name    platform.i18n_text,
    unread_count    bigint,
    mention_count   bigint,
    last_message_at timestamptz,
    is_muted        boolean
)
LANGUAGE sql
STABLE
AS $$
    SELECT c.id,
           c.name,
           count(m.id),
           count(m.id) FILTER (WHERE p_person_id = ANY (m.mentioned_person_ids)),
           c.last_message_at,
           cm.is_muted AND (cm.muted_until IS NULL OR cm.muted_until > now())
    FROM negotiation.channel_members cm
    JOIN negotiation.channels c ON c.id = cm.channel_id
    LEFT JOIN negotiation.channel_messages m
           ON m.channel_id = c.id
          AND m.created_at > greatest(coalesce(cm.last_read_at, cm.joined_at), now() - p_since)
          AND m.sender_person_id <> p_person_id
          AND m.deleted_at IS NULL
          AND m.moderation_state = 'visible'
    WHERE cm.person_id = p_person_id
      AND cm.left_at IS NULL
      AND c.archived_at IS NULL
    GROUP BY c.id, c.name, c.last_message_at, cm.is_muted, cm.muted_until;
$$;

COMMENT ON FUNCTION negotiation.unread_message_counts(uuid, interval) IS
    'Badges « non lus » et « mentions » par canal pour une personne, calculés depuis son last_read_at.';

-- Marquage d'un canal comme lu jusqu'au dernier message reçu.
CREATE OR REPLACE FUNCTION negotiation.mark_channel_read(
    p_channel_id uuid,
    p_person_id  uuid,
    p_read_at    timestamptz DEFAULT now()
)
RETURNS void
LANGUAGE sql
AS $$
    UPDATE negotiation.channel_members
    SET last_read_at = greatest(coalesce(last_read_at, '-infinity'::timestamptz), p_read_at)
    WHERE channel_id = p_channel_id AND person_id = p_person_id;
$$;

-- -----------------------------------------------------------------------------
-- 9. Amorçage — permissions complémentaires et espaces d'origine
-- -----------------------------------------------------------------------------
INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('negotiation.space.manage',    '{"fr":"Administrer un espace de négociation","en":"Manage a negotiation space"}', 'negotiation'),
    ('negotiation.meeting.manage',  '{"fr":"Gérer les réunions de négociation","en":"Manage negotiation meetings"}',   'negotiation'),
    ('negotiation.document.publish','{"fr":"Publier un document d''aide","en":"Publish a support document"}',          'negotiation'),
    ('negotiation.channel.moderate','{"fr":"Modérer les canaux d''échange","en":"Moderate discussion channels"}',      'negotiation'),
    ('negotiation.correction.post', '{"fr":"Poser une note de correction","en":"Post a correction note"}',            'negotiation'),
    ('negotiation.correction.withdraw','{"fr":"Retirer une note de correction","en":"Withdraw a correction note"}',   'negotiation'),
    ('negotiation.knowledge.publish','{"fr":"Publier la FAQ, le parcours et le lexique","en":"Publish the FAQ, pathway and glossary"}', 'negotiation'),
    ('negotiation.knowledge.review', '{"fr":"Vérifier le savoir et traiter la file des experts","en":"Verify knowledge and handle the expert queue"}', 'negotiation'),
    ('negotiation.report.validate', '{"fr":"Valider les signalements du réseau","en":"Validate network reports"}',    'negotiation')
ON CONFLICT (code) DO NOTHING;

-- Rôle attribuable à la portée `negotiation_space` : anime un espace donné sans
-- rien pouvoir faire sur les autres.
INSERT INTO identity.roles (code, label, description, allowed_scopes, is_system) VALUES
    ('space_lead',
     '{"fr":"Animateur d''espace","en":"Space lead"}',
     '{"fr":"Anime un espace de négociation : réunions, documents, canaux","en":"Runs a negotiation space: meetings, documents, channels"}',
     '{negotiation_space}', false)
ON CONFLICT (code) DO NOTHING;

-- L'EXPERT corrige le fond, pas la forme : il pose et retire les notes de
-- correction de Guide Négo (étape 1), et ni ne publie ni ne modifie un
-- document. Portée globale : une note vaut pour tous les lecteurs. L'étape 2 lui
-- donne `negotiation.knowledge.review` — dater la vérification d'une réponse,
-- traiter les signalements, les questions et les termes proposés — et, comme à
-- `admin`, `negotiation.knowledge.publish`. Les étapes 7 (valider une réponse de
-- l'assistant) et 8 (relire un quiz) y ajouteront les leurs. `admin` ne reçoit
-- pas `review` : dater une vérification engage un expert.
INSERT INTO identity.roles (code, label, description, allowed_scopes, is_system) VALUES
    ('expert',
     '{"fr":"Expert","en":"Expert"}',
     '{"fr":"Corrige le fond des contenus de Guide Négo : notes de correction sur les documents, vérification de la FAQ, questions et termes proposés","en":"Corrects the substance of Guide Négo content: correction notes on documents, FAQ verification, questions and proposed terms"}',
     '{global}', true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('space_lead', 'negotiation.space.access'),
    ('space_lead', 'negotiation.content.manage'),
    ('space_lead', 'negotiation.meeting.manage'),
    ('space_lead', 'negotiation.document.publish'),
    ('space_lead', 'negotiation.channel.moderate'),
    ('admin',      'negotiation.space.manage'),
    ('admin',      'negotiation.meeting.manage'),
    ('admin',      'negotiation.document.publish'),
    ('admin',      'negotiation.channel.moderate'),
    ('admin',      'negotiation.report.validate'),
    ('trainer',    'negotiation.space.access'),
    ('expert',     'negotiation.correction.post'),
    ('expert',     'negotiation.correction.withdraw'),
    ('admin',      'negotiation.knowledge.publish'),
    ('expert',     'negotiation.knowledge.publish'),
    ('expert',     'negotiation.knowledge.review')
ON CONFLICT DO NOTHING;

-- Les trois valeurs de l'ex-ENUM `session_category_v2` deviennent trois lignes :
-- la reprise de données v1 est une simple correspondance, et le quatrième espace
-- s'ajoutera par un INSERT.
INSERT INTO negotiation.spaces (slug, name, description, track_term_id, visibility, is_membership_open)
SELECT v.slug, v.name::jsonb::platform.i18n_text, v.description::jsonb::platform.i18n_text,
       t.id, 'listed'::negotiation.space_visibility, false
FROM (VALUES
    ('climat',
     '{"fr":"Espace Climat (CCNUCC)","en":"Climate space (UNFCCC)"}',
     '{"fr":"Négociations climatiques : COP, organes subsidiaires et concertations francophones.","en":"Climate negotiations: COP, subsidiary bodies and Francophone consultations."}',
     'climate'),
    ('biodiversite',
     '{"fr":"Espace Biodiversité (CDB)","en":"Biodiversity space (CBD)"}',
     '{"fr":"Négociations sur la diversité biologique et suivi du cadre mondial.","en":"Biological diversity negotiations and global framework follow-up."}',
     'biodiversity'),
    ('desertification',
     '{"fr":"Espace Désertification (CNULCD)","en":"Desertification space (UNCCD)"}',
     '{"fr":"Négociations sur la lutte contre la désertification et la dégradation des terres.","en":"Negotiations on desertification and land degradation."}',
     'desertification')
) AS v(slug, name, description, track_code)
JOIN reference.taxonomy_terms t
  ON t.taxonomy_code = 'negotiation_track' AND t.code = v.track_code
ON CONFLICT (slug) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 10. Savoir — FAQ, parcours « Ma première COP » et lexique (Guide Négo, étape 2)
--
-- Le savoir se lit sans réseau : le téléphone garde tout le publié, relu par
-- différence (specs/013-guide-nego-faq-lexique, R2). Trois conséquences tenues
-- ici, en base :
--   - une entrée publiée une fois ne se SUPPRIME jamais, elle se dépublie :
--     sinon le téléphone ne saurait pas qu'elle a disparu ;
--   - une source, une question liée ou un terme lié qui change avance
--     `updated_at` de son entrée : sinon la différence ne la renverrait pas ;
--   - les vocabulaires (`faq_section`, `glossary_family`, `negotiation_theme`)
--     se référencent par identifiant, gardés par tg_check_term_taxonomy.
--
-- L'auteur d'un retour, d'un signalement, d'une question ou d'une proposition
-- est gardé (plafond, une voix, courriel de la réponse), mais aucune route du
-- back-office ne le rend (R9).
-- -----------------------------------------------------------------------------

CREATE TYPE negotiation.knowledge_status AS ENUM ('draft', 'published', 'to_review');
CREATE TYPE negotiation.question_status  AS ENUM ('pending', 'answered', 'added_to_faq');
CREATE TYPE negotiation.proposal_status  AS ENUM ('pending', 'accepted', 'rejected');
CREATE TYPE negotiation.faq_report_status AS ENUM ('open', 'closed');

COMMENT ON TYPE negotiation.knowledge_status IS
    'draft → published (la FAQ exige une vérification datée) ; published ⇄ to_review ; published | to_review → draft (dépublier). to_review reste servi au téléphone, avec sa mention (tranché le 25/09).';
COMMENT ON TYPE negotiation.question_status IS
    'pending → answered → added_to_faq (promotion, seulement avec le consentement de l''auteure).';
COMMENT ON TYPE negotiation.proposal_status IS
    'pending → accepted (une entrée du lexique en naît, en brouillon) | rejected (motif requis).';
COMMENT ON TYPE negotiation.faq_report_status IS
    'open → closed, avec l''issue (revised, confirmed, dismissed), l''expert et la date.';

-- Posée au premier passage visible, jamais effacée : c'est elle qui interdit la
-- suppression.
CREATE OR REPLACE FUNCTION negotiation.tg_knowledge_first_published()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.first_published_at IS NOT NULL THEN
        NEW.first_published_at := OLD.first_published_at;
    ELSIF NEW.status IN ('published', 'to_review') AND NEW.first_published_at IS NULL THEN
        NEW.first_published_at := now();
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION negotiation.tg_knowledge_undeletable()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF OLD.first_published_at IS NOT NULL THEN
        RAISE EXCEPTION 'L''entrée % a été publiée : elle se dépublie, elle ne se supprime pas', OLD.id
            USING ERRCODE = 'check_violation', CONSTRAINT = 'ck_' || TG_TABLE_NAME || '_undeletable';
    END IF;
    RETURN OLD;
END;
$$;

-- Arguments par paires (table parente, colonne qui la désigne) : l'ancien et le
-- nouveau parent sont touchés, pour qu'un déplacement fasse partir les deux.
CREATE OR REPLACE FUNCTION negotiation.tg_touch_knowledge_parent()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_old jsonb := CASE WHEN TG_OP <> 'INSERT' THEN to_jsonb(OLD) END;
    v_new jsonb := CASE WHEN TG_OP <> 'DELETE' THEN to_jsonb(NEW) END;
    i     integer := 0;
BEGIN
    WHILE i < TG_NARGS LOOP
        EXECUTE format('UPDATE negotiation.%I SET updated_at = now() WHERE id = ANY ($1)', TG_ARGV[i])
            USING ARRAY[(v_old ->> TG_ARGV[i + 1])::uuid, (v_new ->> TG_ARGV[i + 1])::uuid];
        i := i + 2;
    END LOOP;
    RETURN NULL;
END;
$$;

COMMENT ON FUNCTION negotiation.tg_knowledge_first_published() IS
    'Pose first_published_at au premier passage à published (ou to_review) et ne l''efface jamais.';
COMMENT ON FUNCTION negotiation.tg_knowledge_undeletable() IS
    'Refuse la suppression d''une entrée déjà publiée (ck_<table>_undeletable, check_violation) : elle se dépublie.';
COMMENT ON FUNCTION negotiation.tg_touch_knowledge_parent() IS
    'Avance updated_at de l''entrée parente quand une ligne enfant change. Arguments : paires (table, colonne).';

-- 10.1 — La FAQ
CREATE TABLE negotiation.faq_entries (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    section_term_id     uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    question            platform.i18n_text NOT NULL,
    -- NULL tant que la réponse n'est pas rédigée : une question peut attendre.
    answer              platform.i18n_text,
    status              negotiation.knowledge_status NOT NULL DEFAULT 'draft',
    verified_on         date,
    verified_by         uuid        CONSTRAINT xmod_fk_faq_entries_verifier
                                    REFERENCES identity.people(id) ON DELETE RESTRICT,
    editorial_rank      smallint    CHECK (editorial_rank IS NULL OR editorial_rank > 0),
    -- La question d'expert dont l'entrée est née ; la clé suit expert_questions.
    origin_question_id  uuid,
    first_published_at  timestamptz,
    question_norm       text        GENERATED ALWAYS AS (platform.normalize_label(question ->> 'fr')) STORED,
    created_by          uuid        CONSTRAINT xmod_fk_faq_entries_author
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_faq_entries_verified CHECK (status = 'draft' OR verified_on IS NOT NULL),
    CONSTRAINT ck_faq_entries_verification_pair CHECK ((verified_on IS NULL) = (verified_by IS NULL)),
    CONSTRAINT ck_faq_entries_answer CHECK (status = 'draft' OR answer IS NOT NULL)
);

CREATE INDEX ix_faq_entries_section    ON negotiation.faq_entries (section_term_id, status);
CREATE INDEX ix_faq_entries_updated    ON negotiation.faq_entries (updated_at);
CREATE INDEX ix_faq_entries_question_trgm ON negotiation.faq_entries USING gin (question_norm gin_trgm_ops);

CREATE TRIGGER tg_faq_entries_updated_at BEFORE UPDATE ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_faq_entries_check_section BEFORE INSERT OR UPDATE OF section_term_id ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('section_term_id', 'faq_section');
CREATE TRIGGER tg_faq_entries_first_published BEFORE INSERT OR UPDATE ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_first_published();
CREATE TRIGGER tg_faq_entries_undeletable BEFORE DELETE ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_undeletable();
CREATE TRIGGER tg_faq_entries_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.faq_entries IS
    'Entrées de la FAQ de Guide Négo. Publiée ou « À revoir », une entrée porte une vérification datée et signée par un expert ; publiée une fois, elle ne se supprime plus.';
COMMENT ON COLUMN negotiation.faq_entries.verified_on IS
    '« Vérifié le » : requis dès published ou to_review (ck_faq_entries_verified), posé avec verified_by.';
COMMENT ON COLUMN negotiation.faq_entries.editorial_rank IS
    'Ordre des « plus lues » tant qu''aucune lecture n''est comptée. NULL : hors classement.';
COMMENT ON COLUMN negotiation.faq_entries.first_published_at IS
    'Premier passage à published, jamais effacé. Posé : l''entrée ne se supprime plus (ck_faq_entries_undeletable).';
COMMENT ON COLUMN negotiation.faq_entries.question_norm IS
    'Question française normalisée (platform.normalize_label) : recherche par trigrammes du back-office.';

CREATE TABLE negotiation.faq_related (
    entry_id    uuid     NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    related_id  uuid     NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    sort_order  smallint NOT NULL DEFAULT 0,
    PRIMARY KEY (entry_id, related_id),
    CONSTRAINT ck_faq_related_not_self CHECK (entry_id <> related_id)
);

CREATE INDEX ix_faq_related_related ON negotiation.faq_related (related_id);

CREATE TRIGGER tg_faq_related_touch_parent AFTER INSERT OR UPDATE OR DELETE ON negotiation.faq_related
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_touch_knowledge_parent('faq_entries', 'entry_id');
CREATE TRIGGER tg_faq_related_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.faq_related
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('entry_id');

COMMENT ON TABLE negotiation.faq_related IS
    'Questions liées d''une entrée de FAQ, dans l''ordre choisi. Tous les liens sont servis, publiés ou non : le téléphone filtre sur ce qu''il a.';

-- 10.2 — Le lexique
CREATE TABLE negotiation.glossary_entries (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    slug                text        NOT NULL,
    family_term_id      uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    term                text        NOT NULL,
    acronym             text,
    variants            text[]      NOT NULL DEFAULT '{}',
    translation         platform.i18n_text NOT NULL,
    definition          platform.i18n_text NOT NULL,
    heard_in_room       text,
    status              negotiation.knowledge_status NOT NULL DEFAULT 'draft',
    first_published_at  timestamptz,
    term_norm           text        GENERATED ALWAYS AS (platform.normalize_label(term)) STORED,
    acronym_norm        text        GENERATED ALWAYS AS (platform.normalize_label(acronym)) STORED,
    -- Tenu par tg_glossary_slug : une colonne générée ne parcourt pas un tableau.
    variants_norm       text[]      NOT NULL DEFAULT '{}',
    created_by          uuid        CONSTRAINT xmod_fk_glossary_entries_author
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_glossary_entries_slug UNIQUE (slug),
    CONSTRAINT ck_glossary_entries_slug CHECK (slug ~ '^[a-z0-9]+(-[a-z0-9]+)*$'),
    CONSTRAINT ck_glossary_entries_term CHECK (term_norm IS NOT NULL)
);

-- Unique : deux entrées qui se normalisent pareil rendraient la résolution
-- d'un texte ambiguë.
CREATE UNIQUE INDEX ux_glossary_entries_term_norm ON negotiation.glossary_entries (term_norm);
CREATE INDEX ix_glossary_entries_term_trgm ON negotiation.glossary_entries USING gin (term_norm gin_trgm_ops);
CREATE INDEX ix_glossary_entries_acronym   ON negotiation.glossary_entries (acronym_norm) WHERE acronym_norm IS NOT NULL;
CREATE INDEX ix_glossary_entries_variants  ON negotiation.glossary_entries USING gin (variants_norm);
CREATE INDEX ix_glossary_entries_family    ON negotiation.glossary_entries (family_term_id, status);
CREATE INDEX ix_glossary_entries_updated   ON negotiation.glossary_entries (updated_at);

-- Le slug se pose une fois, depuis le terme anglais, et ne se recalcule jamais :
-- c'est l'adresse d'une entrée (/guide-nego/lexique/contact-group).
CREATE OR REPLACE FUNCTION negotiation.tg_glossary_slug()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        NEW.slug := OLD.slug;
    ELSIF NEW.slug IS NULL OR NEW.slug = '' THEN
        NEW.slug := platform.slugify(NEW.term);
    END IF;
    NEW.variants_norm := ARRAY(
        SELECT DISTINCT n
          FROM unnest(NEW.variants) AS v(texte), platform.normalize_label(v.texte) AS n
         WHERE n IS NOT NULL
         ORDER BY n);
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_glossary_entries_slug BEFORE INSERT OR UPDATE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_glossary_slug();
CREATE TRIGGER tg_glossary_entries_updated_at BEFORE UPDATE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_glossary_entries_check_family BEFORE INSERT OR UPDATE OF family_term_id ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('family_term_id', 'glossary_family');
CREATE TRIGGER tg_glossary_entries_first_published BEFORE INSERT OR UPDATE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_first_published();
CREATE TRIGGER tg_glossary_entries_undeletable BEFORE DELETE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_undeletable();
CREATE TRIGGER tg_glossary_entries_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.glossary_entries IS
    'Lexique de Guide Négo : le terme anglais entendu en salle, sa traduction, sa définition. Publiée une fois, une entrée ne se supprime plus.';
COMMENT ON COLUMN negotiation.glossary_entries.slug IS
    'Désignation stable, posée à l''insertion par platform.slugify(term) et jamais recalculée, même si le terme est corrigé : c''est l''adresse de l''entrée.';
COMMENT ON COLUMN negotiation.glossary_entries.variants IS
    'Écritures admises du terme (« bracketed », « brackets ») : la résolution d''un texte les reconnaît.';
COMMENT ON COLUMN negotiation.glossary_entries.heard_in_room IS
    'La phrase entendue en salle, en anglais : « The contact group will reconvene at 3 p.m. »';
COMMENT ON COLUMN negotiation.glossary_entries.variants_norm IS
    'Variantes normalisées (platform.normalize_label), tenues par tg_glossary_slug.';
COMMENT ON COLUMN negotiation.glossary_entries.first_published_at IS
    'Premier passage à published, jamais effacé. Posé : l''entrée ne se supprime plus (ck_glossary_entries_undeletable).';

CREATE TABLE negotiation.glossary_related (
    entry_id    uuid     NOT NULL REFERENCES negotiation.glossary_entries(id) ON DELETE CASCADE,
    related_id  uuid     NOT NULL REFERENCES negotiation.glossary_entries(id) ON DELETE CASCADE,
    sort_order  smallint NOT NULL DEFAULT 0,
    PRIMARY KEY (entry_id, related_id),
    CONSTRAINT ck_glossary_related_not_self CHECK (entry_id <> related_id)
);

CREATE INDEX ix_glossary_related_related ON negotiation.glossary_related (related_id);

CREATE TRIGGER tg_glossary_related_touch_parent AFTER INSERT OR UPDATE OR DELETE ON negotiation.glossary_related
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_touch_knowledge_parent('glossary_entries', 'entry_id');
CREATE TRIGGER tg_glossary_related_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.glossary_related
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('entry_id');

COMMENT ON TABLE negotiation.glossary_related IS
    'Termes liés d''une entrée du lexique, dans l''ordre choisi.';

-- 10.3 — Les sources d'une entrée
CREATE TABLE negotiation.knowledge_sources (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    faq_entry_id        uuid        REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    glossary_entry_id   uuid        REFERENCES negotiation.glossary_entries(id) ON DELETE CASCADE,
    -- Un document de la bibliothèque, ou une référence extérieure.
    document_id         uuid        REFERENCES negotiation.documents(id) ON DELETE RESTRICT,
    external_title      text,
    external_url        platform.url,
    section_label       text,
    page_from           smallint,
    page_to             smallint,
    quote               text,
    sort_order          smallint    NOT NULL DEFAULT 0,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_knowledge_sources_owner CHECK (num_nonnulls(faq_entry_id, glossary_entry_id) = 1),
    CONSTRAINT ck_knowledge_sources_target CHECK (
        (document_id IS NOT NULL AND external_title IS NULL AND external_url IS NULL) OR
        (document_id IS NULL AND external_title IS NOT NULL AND btrim(external_title) <> '')),
    CONSTRAINT ck_knowledge_sources_pages CHECK (
        (page_from IS NULL OR page_from > 0) AND
        (page_to IS NULL OR (page_from IS NOT NULL AND page_to >= page_from)))
);

CREATE INDEX ix_knowledge_sources_faq      ON negotiation.knowledge_sources (faq_entry_id, sort_order)
    WHERE faq_entry_id IS NOT NULL;
CREATE INDEX ix_knowledge_sources_glossary ON negotiation.knowledge_sources (glossary_entry_id, sort_order)
    WHERE glossary_entry_id IS NOT NULL;
CREATE INDEX ix_knowledge_sources_document ON negotiation.knowledge_sources (document_id)
    WHERE document_id IS NOT NULL;

CREATE TRIGGER tg_knowledge_sources_updated_at BEFORE UPDATE ON negotiation.knowledge_sources
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_knowledge_sources_touch_parent AFTER INSERT OR UPDATE OR DELETE ON negotiation.knowledge_sources
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_touch_knowledge_parent(
        'faq_entries', 'faq_entry_id', 'glossary_entries', 'glossary_entry_id');
CREATE TRIGGER tg_knowledge_sources_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.knowledge_sources
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.knowledge_sources IS
    'Sources d''une entrée de FAQ ou du lexique, exactement une des deux : un document de la bibliothèque, ou une référence extérieure (titre, lien facultatif).';
COMMENT ON COLUMN negotiation.knowledge_sources.section_label IS
    'La partie citée : « annexe A.3 », « Le contexte de négociation ».';
COMMENT ON COLUMN negotiation.knowledge_sources.page_from IS
    'Première page citée : pour un document de la bibliothèque, l''index de page du lecteur (de 1 au nombre de pages) ; pour une référence extérieure, la page imprimée.';

-- 10.4 — Lectures, retours et signalements de la FAQ
CREATE TABLE negotiation.faq_reads (
    entry_id  uuid    NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    day       date    NOT NULL,
    count     integer NOT NULL DEFAULT 1 CHECK (count > 0),
    PRIMARY KEY (entry_id, day)
);

CREATE INDEX ix_faq_reads_day ON negotiation.faq_reads (day);

COMMENT ON TABLE negotiation.faq_reads IS
    'Lectures d''une entrée de FAQ, comptées par jour, sans auteur ni appareil : la matière des « plus lues » sur trente jours. Purgées au-delà de 90 jours.';

CREATE TABLE negotiation.faq_feedback (
    entry_id        uuid        NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_faq_feedback_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    helpful         boolean     NOT NULL,
    missing_reason  text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (entry_id, person_id),
    CONSTRAINT ck_faq_feedback_reason CHECK (
        missing_reason IS NULL OR
        (NOT helpful AND missing_reason IN ('too_vague', 'off_topic', 'outdated')))
);

CREATE INDEX ix_faq_feedback_person ON negotiation.faq_feedback (person_id);

CREATE TRIGGER tg_faq_feedback_updated_at BEFORE UPDATE ON negotiation.faq_feedback
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE negotiation.faq_feedback IS
    '« Cette réponse vous a-t-elle aidée ? » : une voix par personne et par entrée, la dernière gardée. La personne n''est jamais rendue par une route.';
COMMENT ON COLUMN negotiation.faq_feedback.missing_reason IS
    'Après « Non » : too_vague, off_topic ou outdated — outdated ouvre aussi un signalement (from_feedback).';

CREATE TABLE negotiation.faq_reports (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    entry_id        uuid        NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    reporter_id     uuid        NOT NULL CONSTRAINT xmod_fk_faq_reports_reporter
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    client_ref      uuid        NOT NULL,
    reasons         text[]      NOT NULL DEFAULT '{}',
    from_feedback   boolean     NOT NULL DEFAULT false,
    details         text,
    status          negotiation.faq_report_status NOT NULL DEFAULT 'open',
    outcome         text,
    handled_by      uuid        CONSTRAINT xmod_fk_faq_reports_handler
                                REFERENCES identity.people(id) ON DELETE RESTRICT,
    handled_at      timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_faq_reports_client_ref UNIQUE (reporter_id, client_ref),
    CONSTRAINT ck_faq_reports_reasons CHECK (
        reasons <@ ARRAY['rule_changed', 'wrong', 'source_mismatch']
        AND (from_feedback OR cardinality(reasons) > 0)),
    CONSTRAINT ck_faq_reports_details CHECK (details IS NULL OR char_length(details) <= 600),
    CONSTRAINT ck_faq_reports_outcome CHECK (outcome IS NULL OR outcome IN ('revised', 'confirmed', 'dismissed')),
    CONSTRAINT ck_faq_reports_closed CHECK (
        (status = 'closed') = (outcome IS NOT NULL) AND
        (status = 'closed') = (handled_by IS NOT NULL) AND
        (status = 'closed') = (handled_at IS NOT NULL))
);

-- « Dépassée ou fausse » depuis le retour ne compte qu'une fois par personne.
CREATE UNIQUE INDEX ux_faq_reports_from_feedback ON negotiation.faq_reports (entry_id, reporter_id)
    WHERE from_feedback;
CREATE INDEX ix_faq_reports_queue    ON negotiation.faq_reports (created_at) WHERE status = 'open';
CREATE INDEX ix_faq_reports_entry    ON negotiation.faq_reports (entry_id) WHERE status = 'open';
CREATE INDEX ix_faq_reports_reporter ON negotiation.faq_reports (reporter_id, created_at DESC);

CREATE TRIGGER tg_faq_reports_updated_at BEFORE UPDATE ON negotiation.faq_reports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_faq_reports_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.faq_reports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.faq_reports IS
    '« Dépassé ou faux » sur une entrée de FAQ, traité dans la file des experts. L''auteur sert le plafond et l''unicité ; il n''est jamais rendu (R9).';
COMMENT ON COLUMN negotiation.faq_reports.client_ref IS
    'Référence posée par le téléphone : un envoi rejoué par la file hors connexion ne crée pas de seconde ligne.';
COMMENT ON COLUMN negotiation.faq_reports.reasons IS
    'Parmi rule_changed, wrong, source_mismatch ; au moins un, sauf un signalement venu du retour « Dépassée ou fausse » (from_feedback).';

-- 10.5 — Les questions aux experts
CREATE TABLE negotiation.expert_questions (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    asker_id        uuid        NOT NULL CONSTRAINT xmod_fk_expert_questions_asker
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    client_ref      uuid        NOT NULL,
    theme_term_id   uuid        NOT NULL REFERENCES reference.taxonomy_terms(id) ON DELETE RESTRICT,
    body            text        NOT NULL,
    consent_to_faq  boolean     NOT NULL,
    status          negotiation.question_status NOT NULL DEFAULT 'pending',
    answer          text,
    answered_by     uuid        CONSTRAINT xmod_fk_expert_questions_answerer
                                REFERENCES identity.people(id) ON DELETE RESTRICT,
    answered_at     timestamptz,
    -- L'entrée de FAQ née de la question, à sa promotion.
    faq_entry_id    uuid        REFERENCES negotiation.faq_entries(id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_expert_questions_client_ref UNIQUE (asker_id, client_ref),
    CONSTRAINT ck_expert_questions_body CHECK (char_length(btrim(body)) BETWEEN 1 AND 600),
    CONSTRAINT ck_expert_questions_answer CHECK (
        status = 'pending' OR
        (answer IS NOT NULL AND btrim(answer) <> '' AND answered_by IS NOT NULL AND answered_at IS NOT NULL)),
    CONSTRAINT ck_expert_questions_promotion CHECK (
        consent_to_faq OR (status <> 'added_to_faq' AND faq_entry_id IS NULL))
);

CREATE INDEX ix_expert_questions_queue ON negotiation.expert_questions (created_at) WHERE status = 'pending';
CREATE INDEX ix_expert_questions_asker ON negotiation.expert_questions (asker_id, created_at DESC);

ALTER TABLE negotiation.faq_entries ADD CONSTRAINT faq_entries_origin_question_id_fkey
    FOREIGN KEY (origin_question_id) REFERENCES negotiation.expert_questions(id) ON DELETE SET NULL;

CREATE TRIGGER tg_expert_questions_updated_at BEFORE UPDATE ON negotiation.expert_questions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_expert_questions_check_theme BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.expert_questions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('theme_term_id', 'negotiation_theme');
CREATE TRIGGER tg_expert_questions_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.expert_questions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

-- Pour l'audit : le courriel de la réponse est un travail mis en file par le
-- service, dans la même transaction (R11).
CREATE OR REPLACE FUNCTION negotiation.tg_expert_question_event()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = 'answered' AND OLD.status = 'pending' THEN
        PERFORM platform.emit_event(
            'negotiation', 'expert_question', NEW.id,
            'negotiation.expert_question.answered',
            jsonb_build_object('answered_by', NEW.answered_by, 'theme_term_id', NEW.theme_term_id)
        );
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_expert_question_event AFTER UPDATE OF status ON negotiation.expert_questions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_expert_question_event();

COMMENT ON TABLE negotiation.expert_questions IS
    'Questions posées aux experts depuis Guide Négo. L''auteure est gardée pour le courriel de la réponse et « Mes questions » ; aucune route du back-office ne la rend (R9).';
COMMENT ON COLUMN negotiation.expert_questions.consent_to_faq IS
    '« Ma question, anonymisée, pourra rejoindre la FAQ » — cochée par défaut. Sans elle, pas de promotion (ck_expert_questions_promotion).';
COMMENT ON COLUMN negotiation.faq_entries.origin_question_id IS
    'La question d''expert dont l''entrée est née, par promotion. Seul lien vers la question, dont l''auteure reste invisible.';

-- 10.6 — Le parcours « Ma première COP »
CREATE TABLE negotiation.pathway_groups (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    label         platform.i18n_text NOT NULL,
    sort_order    smallint    NOT NULL DEFAULT 0,
    is_published  boolean     NOT NULL DEFAULT false,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_pathway_groups_updated_at BEFORE UPDATE ON negotiation.pathway_groups
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_pathway_groups_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.pathway_groups
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.pathway_groups IS
    'Groupes du parcours « Ma première COP » : Avant de partir, Le premier jour, En salle, Le soir.';

CREATE TABLE negotiation.pathway_steps (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- Un groupe qui porte des étapes ne se supprime pas.
    group_id          uuid        NOT NULL REFERENCES negotiation.pathway_groups(id) ON DELETE RESTRICT,
    label             platform.i18n_text NOT NULL,
    detail            platform.i18n_text,
    origin_label      platform.i18n_text,
    link_kind         text,
    link_document_id  uuid        REFERENCES negotiation.documents(id) ON DELETE RESTRICT,
    link_page         smallint    CHECK (link_page IS NULL OR link_page > 0),
    link_section      text,
    link_faq_id       uuid        REFERENCES negotiation.faq_entries(id) ON DELETE RESTRICT,
    link_glossary_id  uuid        REFERENCES negotiation.glossary_entries(id) ON DELETE RESTRICT,
    link_label        platform.i18n_text,
    sort_order        smallint    NOT NULL DEFAULT 0,
    is_published      boolean     NOT NULL DEFAULT false,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    -- CASE et non OR : `link_kind = 'faq'` vaut NULL sans genre, et un CHECK
    -- NULL laisserait passer une cible orpheline.
    CONSTRAINT ck_pathway_steps_link CHECK (CASE link_kind
        WHEN 'document' THEN link_document_id IS NOT NULL
            AND num_nonnulls(link_faq_id, link_glossary_id) = 0
        WHEN 'faq' THEN link_faq_id IS NOT NULL
            AND num_nonnulls(link_document_id, link_page, link_section, link_glossary_id) = 0
        WHEN 'glossary' THEN link_glossary_id IS NOT NULL
            AND num_nonnulls(link_document_id, link_page, link_section, link_faq_id) = 0
        ELSE link_kind IS NULL
            AND num_nonnulls(link_document_id, link_page, link_section, link_faq_id, link_glossary_id) = 0
    END)
);

CREATE INDEX ix_pathway_steps_group ON negotiation.pathway_steps (group_id, sort_order);

CREATE TRIGGER tg_pathway_steps_updated_at BEFORE UPDATE ON negotiation.pathway_steps
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_pathway_steps_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.pathway_steps
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.pathway_steps IS
    'Étapes du parcours « Ma première COP ». Une étape déjà cochée se dépublie plutôt que se supprimer.';
COMMENT ON COLUMN negotiation.pathway_steps.origin_label IS
    'D''où vient l''étape, sans lien : « Réunions de la Francophonie ».';
COMMENT ON COLUMN negotiation.pathway_steps.link_kind IS
    'document, faq ou glossary ; NULL sans lien. S''accorde avec la seule cible posée (ck_pathway_steps_link).';
COMMENT ON COLUMN negotiation.pathway_steps.link_page IS
    'Page du document ouverte par le lien : index de page du lecteur, de 1 au nombre de pages.';

CREATE TABLE negotiation.pathway_checks (
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_pathway_checks_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    step_id     uuid        NOT NULL REFERENCES negotiation.pathway_steps(id) ON DELETE CASCADE,
    checked_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, step_id)
);

CREATE INDEX ix_pathway_checks_step ON negotiation.pathway_checks (step_id);

COMMENT ON TABLE negotiation.pathway_checks IS
    'Étapes cochées par un compte : une ligne par geste (R7). La coche d''une étape non publiée n''est pas comptée à l''affichage.';

-- 10.7 — Favoris et termes proposés
CREATE TABLE negotiation.glossary_favorites (
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_glossary_favorites_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    entry_id    uuid        NOT NULL REFERENCES negotiation.glossary_entries(id) ON DELETE CASCADE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, entry_id)
);

CREATE INDEX ix_glossary_favorites_entry ON negotiation.glossary_favorites (entry_id);

COMMENT ON TABLE negotiation.glossary_favorites IS
    'Termes favoris d''un compte : une ligne par geste (R7).';

CREATE TABLE negotiation.glossary_proposals (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    term                text        NOT NULL,
    term_norm           text        GENERATED ALWAYS AS (platform.normalize_label(term)) STORED,
    status              negotiation.proposal_status NOT NULL DEFAULT 'pending',
    -- L'entrée née de la proposition acceptée, en brouillon.
    glossary_entry_id   uuid        REFERENCES negotiation.glossary_entries(id) ON DELETE SET NULL,
    rejection_reason    text,
    handled_by          uuid        CONSTRAINT xmod_fk_glossary_proposals_handler
                                    REFERENCES identity.people(id) ON DELETE RESTRICT,
    handled_at          timestamptz,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_glossary_proposals_term CHECK (term_norm IS NOT NULL AND char_length(term) <= 200),
    CONSTRAINT ck_glossary_proposals_decision CHECK (
        (status = 'pending') = (handled_at IS NULL) AND
        (status = 'pending') = (handled_by IS NULL) AND
        (status <> 'rejected' OR (rejection_reason IS NOT NULL AND btrim(rejection_reason) <> '')))
);

-- Une proposition du même terme normalisé s'ajoute à celle qui attend.
CREATE UNIQUE INDEX ux_glossary_proposals_pending ON negotiation.glossary_proposals (term_norm)
    WHERE status = 'pending';
CREATE INDEX ix_glossary_proposals_queue ON negotiation.glossary_proposals (created_at) WHERE status = 'pending';

CREATE TRIGGER tg_glossary_proposals_updated_at BEFORE UPDATE ON negotiation.glossary_proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
CREATE TRIGGER tg_glossary_proposals_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.glossary_proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.glossary_proposals IS
    'Termes proposés au lexique, regroupés par forme normalisée tant qu''ils attendent. Les auteurs vivent dans glossary_proposal_authors et ne sont jamais rendus (R9).';

CREATE TABLE negotiation.glossary_proposal_authors (
    proposal_id  uuid        NOT NULL REFERENCES negotiation.glossary_proposals(id) ON DELETE CASCADE,
    person_id    uuid        NOT NULL CONSTRAINT xmod_fk_glossary_proposal_authors_person
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    client_ref   uuid        NOT NULL,
    context      text,
    created_at   timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (proposal_id, person_id),
    CONSTRAINT ux_glossary_proposal_authors_client_ref UNIQUE (person_id, client_ref),
    CONSTRAINT ck_glossary_proposal_authors_context CHECK (context IS NULL OR char_length(context) <= 600)
);

CREATE INDEX ix_glossary_proposal_authors_person ON negotiation.glossary_proposal_authors (person_id, created_at DESC);

COMMENT ON TABLE negotiation.glossary_proposal_authors IS
    'Qui a proposé un terme, et dans quel contexte. Chacun reçoit un courriel à la publication de l''entrée qui en naît.';

-- 10.8 — Résolution d'un texte et empreinte du paquet
CREATE OR REPLACE FUNCTION negotiation.glossary_resolve(p_text text)
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    WITH q AS (
        SELECT platform.normalize_label(p_text) AS n, platform.slugify(p_text) AS s
    )
    SELECT e.id
      FROM negotiation.glossary_entries e, q
     WHERE e.status <> 'draft'
       AND q.n IS NOT NULL
       AND (e.term_norm = q.n OR e.acronym_norm = q.n OR q.n = ANY (e.variants_norm) OR e.slug = q.s)
     ORDER BY CASE
                WHEN e.term_norm = q.n          THEN 1
                WHEN e.acronym_norm = q.n       THEN 2
                WHEN q.n = ANY (e.variants_norm) THEN 3
                ELSE 4
              END,
              e.id
     LIMIT 1;
$$;

COMMENT ON FUNCTION negotiation.glossary_resolve(text) IS
    'Entrée du lexique publiée ou « À revoir » que désigne un texte, sur les formes normalisées : le terme, puis le sigle, une variante, le slug. Jamais d''approché : NULL plutôt qu''une mauvaise entrée (R6).';

-- L'ETag du paquet, calculé sans sérialiser le corpus. Le jour y entre pour
-- « les plus lues », qui bougent sans qu'aucune entrée change.
CREATE OR REPLACE FUNCTION negotiation.knowledge_fingerprint()
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT md5(concat_ws('|',
        (SELECT count(*) || ':' || coalesce(extract(epoch FROM max(updated_at))::text, '')
           FROM negotiation.faq_entries),
        (SELECT count(*) || ':' || coalesce(extract(epoch FROM max(updated_at))::text, '')
           FROM negotiation.glossary_entries),
        (SELECT count(*) || ':' || coalesce(extract(epoch FROM max(updated_at))::text, '')
           FROM negotiation.knowledge_sources),
        (SELECT count(*) FROM negotiation.faq_related),
        (SELECT count(*) FROM negotiation.glossary_related),
        (SELECT count(*) || ':' || coalesce(extract(epoch FROM max(updated_at))::text, '')
           FROM negotiation.pathway_groups),
        (SELECT count(*) || ':' || coalesce(extract(epoch FROM max(updated_at))::text, '')
           FROM negotiation.pathway_steps),
        (SELECT count(*) || ':' || coalesce(extract(epoch FROM max(updated_at))::text, '')
           FROM reference.taxonomy_terms
          WHERE taxonomy_code IN ('faq_section', 'glossary_family')),
        to_char(now() AT TIME ZONE 'Europe/Paris', 'YYYY-MM-DD')
    ));
$$;

COMMENT ON FUNCTION negotiation.knowledge_fingerprint() IS
    'Empreinte du paquet du savoir : nombre de lignes et dernier updated_at de chaque table servie, et le jour à Paris. Sert d''ETag à GET /negotiation/knowledge.';
