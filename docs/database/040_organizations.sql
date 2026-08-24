-- =============================================================================
-- ePavillon v2 — 040_organizations.sql
-- Module Organisations : référentiel unique, dénominations, domaines,
-- adhésions, détection et fusion des doublons.
--
-- Dépend de : 000, 010, 020, 030
--
-- PROBLÈME N°1 HÉRITÉ DE LA V1 — les doublons d'organisations
-- Citation du cadrage : « 2 personnes ont créé deux fois la même organisation
-- et on voit les 2 sur la plateforme sans possibilité de fusionner. Nous avons
-- mis un mécanisme de recherche avant création, mais certains cherchaient par
-- nom complet tandis que d'autres par sigle. »
--
-- La v1 disposait pourtant de `organization_aliases` et de `is_duplicate` :
-- ce qui manquait n'était pas la table, c'était la CHAÎNE COMPLÈTE. Quatre
-- verrous sont posés ici, du plus préventif au plus curatif :
--
--   V1. UNE ORGANISATION, PLUSIEURS DÉNOMINATIONS (org.organization_names)
--       Le sigle n'est plus un champ à côté du nom : c'est une dénomination de
--       plein droit, indexée comme le nom légal. Chercher « IFDD » ou
--       « Institut de la Francophonie pour le Développement Durable » interroge
--       le même index et retourne la même organisation.
--
--   V2. RECHERCHE MULTI-SIGNAUX AVANT CRÉATION (org.find_similar_organizations)
--       Trigramme, similarité de mot et préfixe sur toutes les dénominations
--       normalisées, plus égalité de domaine web/email, plus proximité
--       géographique. Le formulaire n'affiche plus « aucun résultat » parce que
--       l'utilisateur a tapé le sigle — ni parce qu'il a tapé les huit premières
--       lettres du nom complet, ce que le trigramme seul ne savait pas voir
--       (voir le commentaire détaillé du § 5).
--
--   V3. IMPOSSIBILITÉ STRUCTURELLE DU DOUBLON EXACT (index uniques partiels)
--       Deux organisations actives ne peuvent pas partager le même nom
--       normalisé dans le même pays, ni le même domaine vérifié. La base refuse
--       l'écriture, quelle que soit l'interface qui la tente.
--
--   V4. FUSION RÉVERSIBLE ET TRAÇABLE (org.merge_organizations)
--       Ce qui manquait totalement. Une fusion réattribue les rattachements,
--       conserve la fiche absorbée en état `merged` avec un pointeur, et laisse
--       une trace exploitable. Les anciennes URL continuent de résoudre.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Organisations
-- -----------------------------------------------------------------------------
CREATE TYPE org.organization_status AS ENUM (
    'candidate',  -- créée par un utilisateur, en attente de rapprochement/validation
    'active',     -- fiche de référence
    'merged',     -- absorbée par une autre fiche (voir merged_into_id)
    'archived',   -- sans activité, conservée pour l'historique
    'rejected'    -- création refusée (spam, doublon évident, entité inexistante)
);

CREATE TABLE org.organizations (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    -- Dénomination de référence. Les variantes vivent dans organization_names.
    legal_name         text        NOT NULL CHECK (length(btrim(legal_name)) >= 2),
    -- Forme canonique : socle des index d'unicité et de similarité.
    legal_name_normalized text     GENERATED ALWAYS AS (platform.normalize_label(legal_name)) STORED,
    acronym            text        CHECK (acronym IS NULL OR length(btrim(acronym)) BETWEEN 2 AND 32),
    acronym_normalized text        GENERATED ALWAYS AS (platform.normalize_label(acronym)) STORED,
    slug               platform.slug NOT NULL,

    organization_type_code text    NOT NULL,   -- reference.taxonomy_terms.code (taxonomie organization_type)
    country_id         uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,
    city               text,
    description        platform.i18n_text,
    website            platform.url,
    contact_email      platform.email,
    contact_phone      text,

    status             org.organization_status NOT NULL DEFAULT 'candidate',
    -- Fiche absorbante. Renseignée si et seulement si status = 'merged'.
    merged_into_id     uuid        REFERENCES org.organizations(id) ON DELETE RESTRICT,
    merged_at          timestamptz,

    -- Vérification par l'IFDD : distincte du statut. Une organisation peut être
    -- active sans être vérifiée (elle soumet, mais son sceau n'est pas affiché).
    verified_at        timestamptz,
    verified_by        uuid        CONSTRAINT xmod_fk_organizations_verifier
                                   REFERENCES identity.people(id) ON DELETE SET NULL,

    -- Score de confiance calculé (0-100) : dénominations renseignées, domaine
    -- vérifié, membres actifs, activités validées. Sert au tri du back-office
    -- pour traiter en priorité les fiches douteuses.
    trust_score        smallint    NOT NULL DEFAULT 0 CHECK (trust_score BETWEEN 0 AND 100),

    created_by         uuid        CONSTRAINT xmod_fk_organizations_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_organizations_merge_shape
        CHECK ((status = 'merged' AND merged_into_id IS NOT NULL AND merged_at IS NOT NULL)
            OR (status <> 'merged' AND merged_into_id IS NULL)),
    CONSTRAINT ck_organizations_no_self_merge
        CHECK (merged_into_id IS DISTINCT FROM id)
);

-- V3 — VERROU D'UNICITÉ. Deux fiches actives ne peuvent porter le même nom
-- normalisé dans le même pays. Les fiches fusionnées ou rejetées en sont
-- exclues : l'historique reste consultable.
CREATE UNIQUE INDEX ux_organizations_name_country
    ON org.organizations (legal_name_normalized, COALESCE(country_id, '00000000-0000-0000-0000-000000000000'::uuid))
    WHERE status IN ('candidate', 'active');

CREATE UNIQUE INDEX ux_organizations_slug ON org.organizations (slug);

CREATE INDEX ix_organizations_name_trgm    ON org.organizations USING gin (legal_name_normalized gin_trgm_ops);
CREATE INDEX ix_organizations_acronym_trgm ON org.organizations USING gin (acronym_normalized gin_trgm_ops)
    WHERE acronym_normalized IS NOT NULL;
CREATE INDEX ix_organizations_status       ON org.organizations (status, country_id);
CREATE INDEX ix_organizations_merged_into  ON org.organizations (merged_into_id) WHERE merged_into_id IS NOT NULL;

CREATE TRIGGER tg_organizations_updated_at
    BEFORE UPDATE ON org.organizations
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_organizations_audit
    AFTER INSERT OR UPDATE OR DELETE ON org.organizations
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN org.organizations.merged_into_id IS
    'Fiche absorbante. Utiliser org.resolve_organization() pour obtenir la fiche courante en suivant la chaîne.';

-- Interdiction des chaînes de fusion (A -> B -> C) : la cible d'une fusion doit
-- toujours être une fiche vivante. Sans cette règle, résoudre une organisation
-- deviendrait une récursion non bornée.
CREATE OR REPLACE FUNCTION org.tg_forbid_merge_chains()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_target_status org.organization_status;
BEGIN
    IF NEW.merged_into_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT status INTO v_target_status FROM org.organizations WHERE id = NEW.merged_into_id;

    IF v_target_status = 'merged' THEN
        RAISE EXCEPTION 'Fusion impossible : la fiche cible % est elle-même fusionnée. Cibler la fiche finale.',
            NEW.merged_into_id USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_organizations_no_merge_chain
    BEFORE INSERT OR UPDATE OF merged_into_id ON org.organizations
    FOR EACH ROW EXECUTE FUNCTION org.tg_forbid_merge_chains();

-- Résolution : renvoie toujours la fiche vivante. Les anciennes URL et les
-- références historiques restent valides après une fusion.
CREATE OR REPLACE FUNCTION org.resolve_organization(p_id uuid)
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(o.merged_into_id, o.id) FROM org.organizations o WHERE o.id = p_id;
$$;

-- -----------------------------------------------------------------------------
-- 2. V1 — Dénominations
--
-- Toutes les manières de désigner l'organisation : nom légal, sigle, nom court,
-- traduction, ancien nom, faute de frappe fréquente rencontrée à l'import.
-- Chacune est normalisée et indexée : le canal de recherche est le même quel
-- que soit ce que l'utilisateur tape.
-- -----------------------------------------------------------------------------
CREATE TYPE org.name_kind AS ENUM ('legal', 'acronym', 'short', 'translation', 'former', 'misspelling');

CREATE TABLE org.organization_names (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    organization_id uuid        NOT NULL REFERENCES org.organizations(id) ON DELETE CASCADE,
    name            text        NOT NULL CHECK (length(btrim(name)) >= 2),
    name_normalized text        GENERATED ALWAYS AS (platform.normalize_label(name)) STORED,
    kind            org.name_kind NOT NULL DEFAULT 'short',
    locale          text        REFERENCES reference.locales(code),
    -- Une dénomination issue d'un import ou d'une saisie utilisateur est
    -- exploitée pour la recherche mais n'est pas affichée tant qu'un
    -- administrateur ne l'a pas confirmée.
    is_confirmed    boolean     NOT NULL DEFAULT false,
    created_by      uuid        CONSTRAINT xmod_fk_organization_names_creator
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_organization_names UNIQUE (organization_id, name_normalized, kind)
);

CREATE INDEX ix_organization_names_trgm ON org.organization_names USING gin (name_normalized gin_trgm_ops);
CREATE INDEX ix_organization_names_org  ON org.organization_names (organization_id);

COMMENT ON TABLE org.organization_names IS
    'Toutes les dénominations d''une organisation (nom, sigle, traduction, ancien nom). Base de la recherche unifiée.';

-- Le nom légal et le sigle de la fiche alimentent automatiquement la table des
-- dénominations : aucune synchronisation manuelle à maintenir.
CREATE OR REPLACE FUNCTION org.tg_sync_organization_names()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed)
    VALUES (NEW.id, NEW.legal_name, 'legal', true)
    ON CONFLICT (organization_id, name_normalized, kind) DO NOTHING;

    IF NEW.acronym IS NOT NULL THEN
        INSERT INTO org.organization_names (organization_id, name, kind, is_confirmed)
        VALUES (NEW.id, NEW.acronym, 'acronym', true)
        ON CONFLICT (organization_id, name_normalized, kind) DO NOTHING;
    END IF;

    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_organizations_sync_names
    AFTER INSERT OR UPDATE OF legal_name, acronym ON org.organizations
    FOR EACH ROW EXECUTE FUNCTION org.tg_sync_organization_names();

-- -----------------------------------------------------------------------------
-- 3. Domaines : le signal de dédoublonnage le plus fiable
--
-- Deux fiches partageant un domaine vérifié (ifdd.francophonie.org) sont la
-- même entité, quels que soient les libellés saisis. C'est aussi le mécanisme
-- de rattachement automatique : un agent qui s'inscrit avec une adresse
-- @ifdd.francophonie.org rejoint l'organisation sans intervention humaine.
-- -----------------------------------------------------------------------------
CREATE TABLE org.organization_domains (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    organization_id uuid        NOT NULL REFERENCES org.organizations(id) ON DELETE CASCADE,
    domain          text        NOT NULL CHECK (domain ~ '^[a-z0-9]([a-z0-9-]*[a-z0-9])?(\.[a-z0-9]([a-z0-9-]*[a-z0-9])?)+$'),
    -- Vérification par jeton DNS TXT ou email envoyé à postmaster@.
    verified_at     timestamptz,
    verification_method text    CHECK (verification_method IN ('dns_txt', 'email_challenge', 'manual')),
    -- Rattachement automatique des nouveaux inscrits portant ce domaine.
    auto_join       boolean     NOT NULL DEFAULT false,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_domain_autojoin_requires_verification
        CHECK (NOT auto_join OR verified_at IS NOT NULL)
);

-- V3 (suite) — un domaine vérifié appartient à une seule organisation.
CREATE UNIQUE INDEX ux_organization_domains_verified
    ON org.organization_domains (domain)
    WHERE verified_at IS NOT NULL;

CREATE INDEX ix_organization_domains_org ON org.organization_domains (organization_id);
CREATE INDEX ix_organization_domains_lookup ON org.organization_domains (domain);

-- Les domaines génériques ne prouvent aucune appartenance : les exclure de tout
-- rapprochement automatique évite de fusionner deux ONG parce que leurs
-- référents utilisent Gmail.
CREATE TABLE org.public_email_domains (
    domain text PRIMARY KEY
);

INSERT INTO org.public_email_domains (domain) VALUES
    ('gmail.com'), ('yahoo.com'), ('yahoo.fr'), ('hotmail.com'), ('hotmail.fr'),
    ('outlook.com'), ('outlook.fr'), ('live.com'), ('live.fr'), ('icloud.com'),
    ('protonmail.com'), ('proton.me'), ('orange.fr'), ('wanadoo.fr'), ('free.fr'),
    ('aol.com'), ('gmx.com'), ('mail.com'), ('yandex.com'), ('zoho.com')
ON CONFLICT DO NOTHING;

-- -----------------------------------------------------------------------------
-- 4. Adhésions
--
-- La v1 posait une simple colonne `users.organization_id` : pas d'historique,
-- pas de référent identifié, pas de possibilité d'appartenir à deux structures.
--
-- UNE ADHÉSION « EN ATTENTE » A DEUX ORIGINES OPPOSÉES, et les confondre coûte
-- cher. Une personne qui DEMANDE à rejoindre attend qu'un référent l'accepte ;
-- une personne qu'un référent INVITE attend, elle, d'accepter elle-même. Le
-- statut `pending` ne dit pas laquelle des deux : d'où `invited_by` et
-- `invited_at`, qui portent la DIRECTION de la demande. Sans eux, la file de
-- modération d'un référent mêle ses propres invitations aux demandes qu'il a
-- reçues, et il approuve une adhésion que l'intéressé n'a jamais acceptée.
-- L'invitation elle-même voyage par `identity.one_time_tokens`, finalité
-- `invitation` : la ligne d'adhésion dit qu'elle a été émise, le jeton dit
-- comment elle s'honore.
-- -----------------------------------------------------------------------------
CREATE TYPE org.membership_role AS ENUM ('manager', 'member', 'contributor');
CREATE TYPE org.membership_status AS ENUM ('pending', 'active', 'revoked');

CREATE TABLE org.memberships (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    organization_id uuid        NOT NULL REFERENCES org.organizations(id) ON DELETE CASCADE,
    person_id       uuid        NOT NULL CONSTRAINT xmod_fk_memberships_person
                                REFERENCES identity.people(id) ON DELETE CASCADE,
    role            org.membership_role NOT NULL DEFAULT 'member',
    status          org.membership_status NOT NULL DEFAULT 'pending',
    -- Défaut à `false` : une personne peut appartenir à plusieurs structures, et
    -- une seconde adhésion ne doit pas être refusée par l'index d'unicité parce
    -- que l'appelant a omis le champ. La primauté est attribuée automatiquement
    -- à la première adhésion active (voir tg_memberships_default_primary).
    is_primary      boolean     NOT NULL DEFAULT false,
    -- Fonction occupée dans l'organisation, telle que la personne la DÉCLARE
    -- ELLE-MÊME. Exigée dès que l'adhésion est active (ck_memberships_job_title
    -- plus bas) : appartenir à une organisation sans dire à quel titre laisse
    -- l'équipe deviner qui répond de quoi, et c'est ce qui rendait l'annuaire de
    -- la v1 inexploitable.
    job_title       text,
    -- Adhésion ÉMISE PAR L'ORGANISATION, et non demandée par la personne : elle
    -- attend alors que l'invité l'accepte, pas qu'un référent l'approuve.
    invited_by      uuid        CONSTRAINT xmod_fk_memberships_inviter
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    invited_at      timestamptz,
    approved_by     uuid        CONSTRAINT xmod_fk_memberships_approver
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    approved_at     timestamptz,
    revoked_at      timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_memberships UNIQUE (organization_id, person_id),
    -- Les deux colonnes de l'invitation vont ensemble : une date sans auteur ne
    -- permet ni de relancer, ni de dire qui répond de l'invitation.
    CONSTRAINT ck_memberships_invitation
        CHECK ((invited_at IS NULL) = (invited_by IS NULL)),
    -- UNE ADHÉSION ACTIVE PORTE TOUJOURS SA FONCTION.
    --
    -- Elle n'est PAS exigée en attente, et c'est le cœur de la règle : un
    -- référent qui invite quelqu'un ne connaît pas toujours son intitulé exact,
    -- et le lui faire inventer produirait une donnée fausse que personne ne
    -- corrigerait ensuite. C'est la personne invitée qui la déclare EN
    -- ACCEPTANT — au moment où elle parle d'elle-même, et où l'adhésion devient
    -- active. Une demande de rattachement, elle, la porte dès le départ.
    --
    -- Une adhésion révoquée garde la sienne : elle dit à quel titre la personne
    -- a siégé, et l'effacer réécrirait l'histoire.
    CONSTRAINT ck_memberships_job_title
        CHECK (status <> 'active' OR job_title IS NOT NULL)
);

-- Une seule organisation principale par personne.
CREATE UNIQUE INDEX ux_memberships_primary
    ON org.memberships (person_id)
    WHERE is_primary AND status = 'active';

CREATE INDEX ix_memberships_org    ON org.memberships (organization_id, status);
CREATE INDEX ix_memberships_person ON org.memberships (person_id, status);

-- Les deux files d'attente, distinguées par la DIRECTION de la demande :
-- ce qu'un référent doit trancher, et ce qu'une personne doit accepter.
CREATE INDEX ix_memberships_requests
    ON org.memberships (organization_id, created_at DESC)
    WHERE status = 'pending' AND invited_at IS NULL;
CREATE INDEX ix_memberships_invitations
    ON org.memberships (person_id, invited_at DESC)
    WHERE status = 'pending' AND invited_at IS NOT NULL;

COMMENT ON COLUMN org.memberships.invited_at IS
    'Date d''émission de l''invitation par l''organisation. Nulle pour une DEMANDE spontanée : c''est ce qui distingue les deux sortes de « pending ».';
COMMENT ON COLUMN org.memberships.invited_by IS
    'Référent à l''origine de l''invitation. L''invitation elle-même voyage par identity.one_time_tokens, finalité « invitation ».';

CREATE TRIGGER tg_memberships_updated_at
    BEFORE UPDATE ON org.memberships
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Attribution automatique de la primauté : la première adhésion active d'une
-- personne devient son rattachement principal. Les suivantes restent
-- secondaires, sauf demande explicite. L'appelant n'a donc rien à calculer, et
-- une seconde adhésion ne se heurte jamais à l'index d'unicité.
CREATE OR REPLACE FUNCTION org.tg_default_primary_membership()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = 'active' AND NOT NEW.is_primary THEN
        IF NOT EXISTS (
            SELECT 1 FROM org.memberships m
            WHERE m.person_id = NEW.person_id
              AND m.status = 'active'
              AND m.is_primary
              AND m.id IS DISTINCT FROM NEW.id
        ) THEN
            NEW.is_primary := true;
        END IF;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_memberships_default_primary
    BEFORE INSERT OR UPDATE OF status, is_primary ON org.memberships
    FOR EACH ROW EXECUTE FUNCTION org.tg_default_primary_membership();

-- Le rattachement principal de la personne suit l'adhésion principale : une
-- seule vérité, maintenue par la base.
CREATE OR REPLACE FUNCTION org.tg_sync_primary_organization()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.is_primary AND NEW.status = 'active' THEN
        UPDATE identity.people
        SET primary_organization_id = NEW.organization_id
        WHERE id = NEW.person_id
          AND primary_organization_id IS DISTINCT FROM NEW.organization_id;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_memberships_sync_primary
    AFTER INSERT OR UPDATE OF is_primary, status, organization_id ON org.memberships
    FOR EACH ROW EXECUTE FUNCTION org.tg_sync_primary_organization();

-- FK différée du module identity vers org (déclarée ici car org est créé après).
ALTER TABLE identity.people
    ADD CONSTRAINT xmod_fk_people_primary_organization
    FOREIGN KEY (primary_organization_id) REFERENCES org.organizations(id) ON DELETE SET NULL;

-- -----------------------------------------------------------------------------
-- 5. V2 — Recherche multi-signaux avant création
--
-- Appelée par le formulaire de création (frappe au clavier, debounce 300 ms) et
-- par le back-office. Le score combine :
--   - similarité maximale sur TOUTES les dénominations (0-100)
--   - bonus 40 si le domaine du site/email correspond à un domaine enregistré
--   - bonus 10 si le pays correspond
--   - bonus 25 en cas d'égalité exacte de sigle normalisé
-- Un score >= 85 déclenche un blocage doux côté interface (« cette organisation
-- existe déjà, souhaitez-vous la rejoindre ? ») plutôt qu'une simple suggestion.
--
-- TROIS FAÇONS DE RECONNAÎTRE UNE DÉNOMINATION, ET IL EN FAUT TROIS. La
-- similarité trigramme seule — `%`, seuil 0,3 — compare deux chaînes ENTIÈRES et
-- se normalise sur leur longueur : mesuré sur la base, `similarity('institut',
-- 'institut de la francophonie pour le developpement durable')` vaut 0,17, sous
-- le seuil. Autrement dit, quelqu'un qui tape le DÉBUT DU NOM COMPLET — le geste
-- le plus naturel qui soit — n'obtenait AUCUN résultat, et l'écran lui proposait
-- de créer la fiche qui existait déjà. C'est le défaut n°1 de la v1 reproduit par
-- la fonction censée le corriger. S'ajoutent donc :
--   - `word_similarity(q, nom)` (opérateur `<%`) : le terme est-il proche d'un ou
--     plusieurs MOTS du nom ? « institut » ou « francophonie » y répondent ;
--   - le PRÉFIXE littéral, qui couvre les deux ou trois premières lettres, en
--     deçà de ce que les trigrammes savent mesurer.
-- Ces deux signaux sont volontairement pondérés PLUS BAS qu'une correspondance
-- sur le nom entier : reconnaître un fragment ne vaut pas reconnaître le nom, et
-- ils ne doivent pas franchir seuls le seuil de 85 qui affirme « c'est la même ».
--
-- CE QUE LA FONCTION RENVOIE EST CE QU'IL FAUT POUR RECONNAÎTRE SA FICHE, et
-- pas seulement pour la désigner : type, ville, sceau de vérification et nombre
-- de membres déjà inscrits. Sans eux, l'écran de rattachement rechargerait une
-- table entière à chaque frappe pour afficher « 4 membres » — ou renoncerait, et
-- l'utilisateur trancherait entre deux fiches homonymes sans rien pour les
-- distinguer.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION org.find_similar_organizations(
    p_name       text,
    p_country_id uuid    DEFAULT NULL,
    p_email      text    DEFAULT NULL,
    p_website    text    DEFAULT NULL,
    p_limit      integer DEFAULT 10
)
RETURNS TABLE (
    organization_id        uuid,
    legal_name             text,
    acronym                text,
    organization_type_code text,
    country_id             uuid,
    city                   text,
    status                 org.organization_status,
    verified_at            timestamptz,
    member_count           integer,
    matched_name           text,
    score                  numeric,
    match_reasons          text[]
)
LANGUAGE sql
STABLE
AS $$
WITH input AS (
    SELECT
        platform.normalize_label(p_name) AS q,
        NULLIF(platform.extract_domain(COALESCE(p_website, p_email)), '') AS domain
),
-- Domaine générique (gmail...) : neutralisé, il ne prouve rien.
usable_domain AS (
    SELECT CASE WHEN i.domain IN (SELECT domain FROM org.public_email_domains) THEN NULL ELSE i.domain END AS domain
    FROM input i
),
name_matches AS (
    SELECT
        n.organization_id,
        n.name AS matched_name,
        max(GREATEST(
            -- Le nom entier, ou peu s'en faut.
            similarity(n.name_normalized, i.q),
            -- Un ou plusieurs mots du nom.
            (word_similarity(i.q, n.name_normalized) * 0.6)::real,
            -- Les premières lettres, en deçà de ce que mesurent les trigrammes.
            CASE WHEN n.name_normalized LIKE i.q || '%' THEN 0.55::real ELSE 0::real END
        )) AS name_score,
        -- Départage deux dénominations d'égal score : à l'écran, « trouvée sous :
        -- <faute d'orthographe connue> » décrédibilise un résultat par ailleurs
        -- juste. Sans cet ordre, le choix revient à l'ordre physique des lignes.
        min(CASE n.kind WHEN 'legal' THEN 1 WHEN 'acronym' THEN 2 WHEN 'short' THEN 3
                        WHEN 'translation' THEN 4 WHEN 'former' THEN 5 ELSE 6 END
            + CASE WHEN n.is_confirmed THEN 0 ELSE 10 END) AS name_rank
    FROM org.organization_names n
    CROSS JOIN input i
    WHERE i.q IS NOT NULL
      AND (
            n.name_normalized % i.q            -- trigramme, servi par l'index GIN
         OR i.q <% n.name_normalized           -- similarité de mot, même index
         OR n.name_normalized LIKE i.q || '%'  -- préfixe
      )
    GROUP BY n.organization_id, n.name
),
domain_matches AS (
    SELECT DISTINCT d.organization_id
    FROM org.organization_domains d
    CROSS JOIN usable_domain u
    WHERE u.domain IS NOT NULL AND d.domain = u.domain
),
-- Une organisation ne doit apparaître qu'UNE fois, avec la meilleure de ses
-- dénominations : sans ce regroupement, une fiche portant à la fois un nom
-- légal et un sigle serait proposée deux fois à l'utilisateur.
candidates AS (
    SELECT
        u.organization_id,
        (array_agg(u.matched_name ORDER BY u.name_score DESC NULLS LAST, u.name_rank))[1] AS matched_name,
        max(u.name_score) AS name_score
    FROM (
        SELECT organization_id, matched_name, name_score, name_rank FROM name_matches
        UNION ALL
        SELECT organization_id, NULL::text, 0::real, 99 FROM domain_matches
    ) u
    GROUP BY u.organization_id
)
SELECT
    o.id,
    o.legal_name,
    o.acronym,
    o.organization_type_code,
    o.country_id,
    o.city,
    o.status,
    o.verified_at,
    -- Membres DÉJÀ INSCRITS : c'est ce qui fait dire « c'est bien la mienne ».
    -- Les adhésions en attente ne comptent pas — elles ne prouvent encore rien.
    (SELECT count(*) FROM org.memberships m
      WHERE m.organization_id = o.id AND m.status = 'active')::integer AS member_count,
    c.matched_name,
    round((
        COALESCE(c.name_score, 0) * 100
        + CASE WHEN dm.organization_id IS NOT NULL THEN 40 ELSE 0 END
        + CASE WHEN p_country_id IS NOT NULL AND o.country_id = p_country_id THEN 10 ELSE 0 END
        + CASE WHEN o.acronym_normalized IS NOT NULL
                AND o.acronym_normalized = (SELECT q FROM input) THEN 25 ELSE 0 END
    )::numeric, 1) AS score,
    ARRAY_REMOVE(ARRAY[
        CASE WHEN COALESCE(c.name_score, 0) > 0.3 THEN 'name_similarity' END,
        CASE WHEN dm.organization_id IS NOT NULL THEN 'shared_domain' END,
        CASE WHEN p_country_id IS NOT NULL AND o.country_id = p_country_id THEN 'same_country' END,
        CASE WHEN o.acronym_normalized = (SELECT q FROM input) THEN 'acronym_match' END
    ], NULL) AS match_reasons
FROM candidates c
JOIN org.organizations o ON o.id = c.organization_id
LEFT JOIN domain_matches dm ON dm.organization_id = o.id
WHERE o.status IN ('candidate', 'active')
ORDER BY score DESC, o.legal_name
LIMIT p_limit;
$$;

COMMENT ON FUNCTION org.find_similar_organizations IS
    'Recherche de doublons potentiels avant création. Interroge nom, sigle, traductions et domaines en une passe.';

-- Détection systématique en arrière-plan : balaie les fiches actives et
-- consigne les paires suspectes pour revue humaine. Exécutée par le worker.
CREATE TABLE org.duplicate_candidates (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    left_id         uuid        NOT NULL REFERENCES org.organizations(id) ON DELETE CASCADE,
    right_id        uuid        NOT NULL REFERENCES org.organizations(id) ON DELETE CASCADE,
    score           numeric(5,1) NOT NULL,
    reasons         text[]      NOT NULL DEFAULT '{}',
    detected_at     timestamptz NOT NULL DEFAULT now(),
    reviewed_at     timestamptz,
    reviewed_by     uuid        CONSTRAINT xmod_fk_duplicate_candidates_reviewer
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    -- 'merged' : fusion effectuée ; 'distinct' : entités réellement différentes,
    -- ne plus proposer la paire.
    decision        text        CHECK (decision IN ('merged', 'distinct', 'deferred')),
    CONSTRAINT ck_duplicate_candidates_ordered CHECK (left_id < right_id),
    CONSTRAINT ux_duplicate_candidates UNIQUE (left_id, right_id)
);

CREATE INDEX ix_duplicate_candidates_pending
    ON org.duplicate_candidates (score DESC)
    WHERE reviewed_at IS NULL;

-- -----------------------------------------------------------------------------
-- 6. V4 — Fusion
--
-- Ce que la v1 ne savait pas faire. La fusion est une opération de premier
-- ordre : atomique, tracée, et sans perte. Les rattachements des autres modules
-- sont réaffectés par une table de registre, ce qui évite de réécrire cette
-- fonction chaque fois qu'un module ajoute une référence aux organisations.
-- -----------------------------------------------------------------------------

-- Registre des colonnes pointant vers une organisation. Chaque module y déclare
-- ses références lors de sa migration ; la fusion les parcourt dynamiquement.
CREATE TABLE org.organization_references (
    ref_schema  text NOT NULL,
    ref_table   text NOT NULL,
    ref_column  text NOT NULL,
    -- 'reassign' : basculer vers la fiche cible.
    -- 'delete'   : supprimer la ligne.
    strategy    text NOT NULL DEFAULT 'reassign' CHECK (strategy IN ('reassign', 'delete')),
    -- Colonnes formant une contrainte d'unicité AVEC la colonne d'organisation.
    -- Si les deux fiches partagent une même valeur — la même personne adhérente,
    -- la même proposition co-organisée — la bascule produirait un doublon et
    -- ferait échouer la fusion. Les lignes en conflit côté source sont donc
    -- supprimées avant la réaffectation.
    dedupe_on   text[] NOT NULL DEFAULT '{}',
    PRIMARY KEY (ref_schema, ref_table, ref_column)
);

INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy, dedupe_on) VALUES
    ('org',      'organization_names',   'organization_id',         'reassign', '{}'),
    -- `dedupe_on = {domain}` : deux fiches en doublon déclarent presque toujours
    -- LE MÊME domaine — c'est même le signal qui les a fait remonter (§ 3). Sans
    -- ce dédoublonnage, la fusion laissait la fiche absorbante avec deux lignes
    -- pour `osed-sahel.org`, l'une vérifiée et l'autre non : aucun index ne
    -- l'interdit (ux_organization_domains_verified ne porte que sur les domaines
    -- VÉRIFIÉS), et la fiche héritait donc d'un doublon interne au moment même où
    -- l'on corrigeait un doublon d'organisation.
    ('org',      'organization_domains', 'organization_id',         'reassign', '{domain}'),
    ('org',      'memberships',          'organization_id',         'reassign', '{person_id}'),
    ('identity', 'people',               'primary_organization_id', 'reassign', '{}')
ON CONFLICT DO NOTHING;

CREATE TABLE org.merge_log (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    source_id       uuid        NOT NULL,
    source_snapshot jsonb       NOT NULL,   -- fiche absorbée avant fusion : permet une reprise manuelle
    target_id       uuid        NOT NULL REFERENCES org.organizations(id) ON DELETE RESTRICT,
    performed_by    uuid        CONSTRAINT xmod_fk_merge_log_actor
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    performed_at    timestamptz NOT NULL DEFAULT now(),
    rows_reassigned jsonb       NOT NULL DEFAULT '{}'::jsonb,
    reason          text
);

CREATE INDEX ix_merge_log_target ON org.merge_log (target_id, performed_at DESC);

CREATE OR REPLACE FUNCTION org.merge_organizations(
    p_source_id uuid,
    p_target_id uuid,
    p_reason    text DEFAULT NULL
)
RETURNS uuid
LANGUAGE plpgsql
SECURITY DEFINER
SET search_path = org, identity, platform, pg_temp
AS $$
DECLARE
    v_ref      record;
    v_count    bigint;
    v_counts   jsonb := '{}'::jsonb;
    v_snapshot jsonb;
BEGIN
    IF p_source_id = p_target_id THEN
        RAISE EXCEPTION 'Fusion impossible : source et cible identiques.' USING ERRCODE = 'invalid_parameter_value';
    END IF;

    SELECT to_jsonb(o) INTO v_snapshot FROM org.organizations o WHERE o.id = p_source_id FOR UPDATE;
    IF v_snapshot IS NULL THEN
        RAISE EXCEPTION 'Organisation source % introuvable.', p_source_id USING ERRCODE = 'no_data_found';
    END IF;

    PERFORM 1 FROM org.organizations WHERE id = p_target_id AND status <> 'merged' FOR UPDATE;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Organisation cible % introuvable ou déjà fusionnée.', p_target_id USING ERRCODE = 'no_data_found';
    END IF;

    -- 1. Les dénominations de la source deviennent des variantes de la cible :
    --    une recherche sur l'ancien nom continue de trouver la bonne fiche.
    UPDATE org.organization_names
    SET organization_id = p_target_id
    WHERE organization_id = p_source_id
      AND NOT EXISTS (
          SELECT 1 FROM org.organization_names t
          WHERE t.organization_id = p_target_id
            AND t.name_normalized = org.organization_names.name_normalized
            AND t.kind = org.organization_names.kind
      );
    DELETE FROM org.organization_names WHERE organization_id = p_source_id;

    -- 2. Réaffectation générique de toutes les autres références déclarées.
    FOR v_ref IN
        SELECT * FROM org.organization_references
        WHERE NOT (ref_schema = 'org' AND ref_table IN ('organization_names'))
    LOOP
        IF v_ref.strategy = 'reassign' THEN
            -- Suppression préalable des lignes qui deviendraient des doublons :
            -- une personne adhérente des deux fiches, une proposition déjà
            -- co-organisée par la cible, etc.
            IF array_length(v_ref.dedupe_on, 1) > 0 THEN
                EXECUTE format(
                    'DELETE FROM %I.%I s WHERE s.%I = $2 AND EXISTS ('
                    || 'SELECT 1 FROM %I.%I t WHERE t.%I = $1 AND %s)',
                    v_ref.ref_schema, v_ref.ref_table, v_ref.ref_column,
                    v_ref.ref_schema, v_ref.ref_table, v_ref.ref_column,
                    (SELECT string_agg(format('t.%I IS NOT DISTINCT FROM s.%I', col, col), ' AND ')
                     FROM unnest(v_ref.dedupe_on) AS col)
                )
                USING p_target_id, p_source_id;
            END IF;

            EXECUTE format('UPDATE %I.%I SET %I = $1 WHERE %I = $2',
                           v_ref.ref_schema, v_ref.ref_table, v_ref.ref_column, v_ref.ref_column)
            USING p_target_id, p_source_id;
        ELSE
            EXECUTE format('DELETE FROM %I.%I WHERE %I = $1',
                           v_ref.ref_schema, v_ref.ref_table, v_ref.ref_column)
            USING p_source_id;
        END IF;

        GET DIAGNOSTICS v_count = ROW_COUNT;
        v_counts := v_counts || jsonb_build_object(
            format('%s.%s.%s', v_ref.ref_schema, v_ref.ref_table, v_ref.ref_column), v_count
        );
    END LOOP;

    -- 4. La fiche source survit en état `merged` : les URL et identifiants
    --    externes déjà diffusés continuent de résoudre vers la bonne page.
    UPDATE org.organizations
    SET status         = 'merged',
        merged_into_id = p_target_id,
        merged_at      = now()
    WHERE id = p_source_id;

    INSERT INTO org.merge_log (source_id, source_snapshot, target_id, performed_by, rows_reassigned, reason)
    VALUES (p_source_id, v_snapshot, p_target_id, platform.current_actor_id(), v_counts, p_reason);

    UPDATE org.duplicate_candidates
    SET decision = 'merged', reviewed_at = now(), reviewed_by = platform.current_actor_id()
    WHERE (left_id = p_source_id AND right_id = p_target_id)
       OR (left_id = p_target_id AND right_id = p_source_id);

    PERFORM platform.emit_event(
        'org', 'organization', p_target_id, 'org.organization.merged',
        jsonb_build_object('source_id', p_source_id, 'target_id', p_target_id, 'rows', v_counts)
    );

    RETURN p_target_id;
END;
$$;

COMMENT ON FUNCTION org.merge_organizations IS
    'Fusionne deux organisations de façon atomique et tracée. Les modules déclarent leurs références dans org.organization_references.';

-- -----------------------------------------------------------------------------
-- 7. Score de confiance
--
-- Recalculé par le worker. Sert au tri du back-office : traiter en priorité les
-- fiches faibles, plutôt que de parcourir une liste alphabétique de plusieurs
-- centaines d'entrées.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION org.compute_trust_score(p_organization_id uuid)
RETURNS smallint
LANGUAGE sql
STABLE
AS $$
    SELECT LEAST(100, (
          CASE WHEN o.verified_at IS NOT NULL THEN 40 ELSE 0 END
        + CASE WHEN EXISTS (SELECT 1 FROM org.organization_domains d
                            WHERE d.organization_id = o.id AND d.verified_at IS NOT NULL) THEN 25 ELSE 0 END
        + CASE WHEN o.website IS NOT NULL THEN 5 ELSE 0 END
        + CASE WHEN o.country_id IS NOT NULL THEN 5 ELSE 0 END
        + CASE WHEN o.description IS NOT NULL THEN 5 ELSE 0 END
        + LEAST(20, 5 * (SELECT count(*) FROM org.memberships m
                         WHERE m.organization_id = o.id AND m.status = 'active'))
    ))::smallint
    FROM org.organizations o
    WHERE o.id = p_organization_id;
$$;
