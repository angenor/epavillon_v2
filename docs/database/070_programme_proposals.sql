-- =============================================================================
-- ePavillon v2 — 070_programme_proposals.sql
-- Module Programmation, partie 1 : propositions d'activité, cycle de vie,
-- évaluation par le comité, échanges avec le soumissionnaire.
--
-- Dépend de : 000, 010, 020, 030, 040, 050, 060
--
-- DÉCISION STRUCTURANTE — séparer la PROPOSITION de la SESSION PROGRAMMÉE
-- La v1 n'avait qu'une table `activities` portant à la fois le dossier soumis
-- par l'organisation et l'activité diffusée au public. Trois conséquences :
--   - la colonne `validation_status` mélangeait des états de dossier (draft,
--     submitted, under_review, approved, rejected) et des états de diffusion
--     (live, completed), rendant impossible toute machine à états cohérente ;
--   - une proposition ne pouvait donner qu'UNE activité ; le cycle PACO, dont
--     un même webinaire compte plusieurs éditions, a dû être rattrapé par une
--     colonne `session_edition INTEGER` ajoutée dans `activity_registrations` ;
--   - les dates proposées et les dates finales cohabitaient dans la même ligne
--     (`proposed_start_date` / `final_start_date`), sans historique des
--     arbitrages de planification.
--
-- En v2 : la proposition est un DOSSIER (ce fichier), la session est une
-- OCCURRENCE PROGRAMMÉE (fichier 075). Une proposition acceptée engendre une ou
-- plusieurs sessions ; une session peut exister sans proposition lorsque
-- l'IFDD programme directement une activité.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Cycle de vie
-- -----------------------------------------------------------------------------
CREATE TYPE programme.proposal_status AS ENUM (
    'draft',             -- brouillon de l'organisation, modifiable librement
    'submitted',         -- déposé, en attente d'affectation au comité
    'under_review',      -- en cours d'évaluation
    'changes_requested', -- renvoyé au soumissionnaire pour complément
    'accepted',          -- retenu : donnera lieu à une ou plusieurs sessions
    'rejected',          -- non retenu
    'withdrawn',         -- retiré par l'organisation
    'cancelled'          -- annulé par l'IFDD après acceptation
);

-- Transitions autorisées, en données plutôt qu'en code : ajouter un état ou
-- ouvrir un chemin devient une ligne, pas une relecture du service Rust. La
-- permission requise est portée ici, ce qui garantit qu'aucun chemin d'écriture
-- ne peut court-circuiter le contrôle d'accès.
CREATE TABLE programme.proposal_transitions_allowed (
    from_status         programme.proposal_status NOT NULL,
    to_status           programme.proposal_status NOT NULL,
    required_permission text,
    -- Le soumissionnaire lui-même peut-il déclencher cette transition ?
    allowed_for_owner   boolean NOT NULL DEFAULT false,
    requires_reason     boolean NOT NULL DEFAULT false,
    PRIMARY KEY (from_status, to_status)
);

INSERT INTO programme.proposal_transitions_allowed
    (from_status, to_status, required_permission, allowed_for_owner, requires_reason) VALUES
    ('draft',             'submitted',         'programme.proposal.submit', true,  false),
    ('draft',             'withdrawn',         NULL,                        true,  false),
    ('submitted',         'under_review',      'programme.proposal.decide', false, false),
    ('submitted',         'changes_requested', 'programme.review.write',    false, true),
    ('submitted',         'withdrawn',         NULL,                        true,  true),
    ('under_review',      'changes_requested', 'programme.review.write',    false, true),
    ('under_review',      'accepted',          'programme.proposal.decide', false, false),
    ('under_review',      'rejected',          'programme.proposal.decide', false, true),
    ('under_review',      'withdrawn',         NULL,                        true,  true),
    ('changes_requested', 'submitted',         'programme.proposal.submit', true,  false),
    ('changes_requested', 'withdrawn',         NULL,                        true,  false),
    ('changes_requested', 'rejected',          'programme.proposal.decide', false, true),
    ('accepted',          'cancelled',         'programme.proposal.decide', false, true),
    ('rejected',          'under_review',      'programme.proposal.decide', false, true)
ON CONFLICT DO NOTHING;

COMMENT ON TABLE programme.proposal_transitions_allowed IS
    'Machine à états des propositions, exprimée en données. Vérifiée par trigger : aucun chemin d''écriture ne peut la contourner.';

-- -----------------------------------------------------------------------------
-- 2. Propositions
-- -----------------------------------------------------------------------------
CREATE TABLE programme.proposals (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- Numéro lisible communiqué à l'organisation (« COP30-0147 ») : indispensable
    -- pour tout échange par courriel ou téléphone.
    reference_code    text        NOT NULL UNIQUE,

    call_id           uuid        CONSTRAINT xmod_fk_proposals_call
                                  REFERENCES event.calls_for_proposals(id) ON DELETE RESTRICT,
    -- Dénormalisation assumée : l'événement reste connu même si l'appel est
    -- supprimé, et la quasi-totalité des requêtes filtre par événement.
    event_id          uuid        NOT NULL CONSTRAINT xmod_fk_proposals_event
                                  REFERENCES event.events(id) ON DELETE CASCADE,

    organization_id   uuid        NOT NULL CONSTRAINT xmod_fk_proposals_organization
                                  REFERENCES org.organizations(id) ON DELETE RESTRICT,
    submitted_by      uuid        NOT NULL CONSTRAINT xmod_fk_proposals_submitter
                                  REFERENCES identity.people(id) ON DELETE RESTRICT,
    contact_person_id uuid        CONSTRAINT xmod_fk_proposals_contact
                                  REFERENCES identity.people(id) ON DELETE SET NULL,

    title             platform.i18n_text NOT NULL,
    slug              platform.slug NOT NULL,
    summary           platform.i18n_text,
    objectives        platform.i18n_text NOT NULL,
    -- TEXTE RICHE : fragment HTML restreint (gras, italique, listes, sous-titres,
    -- citations, liens), et non du texte brut. Le jeu de balises autorisé est
    -- celui de l'éditeur du frontend ; l'API l'assainit à l'écriture — une
    -- proposition est rédigée par un tiers, son contenu n'est jamais de
    -- confiance. Ni police ni couleur ne sont saisissables : la mise en forme
    -- appartient à la charte, pas au déposant.
    detailed_presentation platform.i18n_text NOT NULL,
    expected_outcomes platform.i18n_text,
    -- PUBLICS VISÉS, un par entrée. Une seule chaîne « Ministères, ONG,
    -- journalistes » ne se réaffiche pas : elle s'imprime telle quelle, ne se
    -- compte pas, ne se filtre pas, et se découpe à la virgule par quiconque
    -- essaie — ce que la v1 faisait dans ses gabarits. Le tableau de
    -- `platform.i18n_text` conserve l'exigence du français SUR CHAQUE ENTRÉE,
    -- la contrainte du domaine s'appliquant élément par élément.
    target_audiences  platform.i18n_text[] NOT NULL DEFAULT '{}',

    format            event.participation_mode NOT NULL,
    -- Type d'activité (side event, journée pays, autre) : code de la taxonomie
    -- `activity_category`. Les thématiques passent par reference.entity_terms.
    activity_type_code text,
    language_codes    text[]      NOT NULL DEFAULT '{fr}',
    country_id        uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,

    -- Créneau souhaité par l'organisation. Le créneau retenu vit sur la session.
    preferred_start_at timestamptz,
    preferred_end_at   timestamptz,
    duration_minutes   smallint   CHECK (duration_minutes IS NULL OR duration_minutes BETWEEN 15 AND 600),
    -- Nombre d'occurrences demandées : un cycle de webinaires en annonce
    -- plusieurs dès la soumission (cas PACO).
    requested_sessions smallint   NOT NULL DEFAULT 1 CHECK (requested_sessions BETWEEN 1 AND 50),
    scheduling_constraints text,

    status            programme.proposal_status NOT NULL DEFAULT 'draft',
    submitted_at      timestamptz,
    decided_at        timestamptz,
    decision_reason   text,
    decided_by        uuid        CONSTRAINT xmod_fk_proposals_decider
                                  REFERENCES identity.people(id) ON DELETE SET NULL,

    -- Score consolidé, recalculé à chaque revue (voir programme.refresh_proposal_score).
    -- Dénormalisé pour permettre le tri et la pagination du back-office sans
    -- recalcul à chaque page.
    average_score     numeric(6,2),
    weighted_score    numeric(6,2),
    review_count      smallint    NOT NULL DEFAULT 0,
    is_knocked_out    boolean     NOT NULL DEFAULT false,

    view_count        integer     NOT NULL DEFAULT 0,
    search_vector     tsvector    GENERATED ALWAYS AS (
        to_tsvector('french',
            coalesce(title ->> 'fr', '') || ' ' ||
            coalesce(summary ->> 'fr', '') || ' ' ||
            coalesce(objectives ->> 'fr', ''))
    ) STORED,

    deleted_at        timestamptz,
    deleted_by        uuid        CONSTRAINT xmod_fk_proposals_deleter
                                  REFERENCES identity.people(id) ON DELETE SET NULL,
    deleted_reason    text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_proposals_preferred_period
        CHECK (preferred_end_at IS NULL OR preferred_start_at IS NULL OR preferred_end_at > preferred_start_at),
    CONSTRAINT ck_proposals_submitted_at
        CHECK (status = 'draft' OR submitted_at IS NOT NULL),
    CONSTRAINT ux_proposals_slug UNIQUE (event_id, slug)
);

CREATE INDEX ix_proposals_event_status ON programme.proposals (event_id, status) WHERE deleted_at IS NULL;
CREATE INDEX ix_proposals_call         ON programme.proposals (call_id, status) WHERE deleted_at IS NULL;
CREATE INDEX ix_proposals_organization ON programme.proposals (organization_id, created_at DESC);
CREATE INDEX ix_proposals_submitter    ON programme.proposals (submitted_by, created_at DESC);
CREATE INDEX ix_proposals_search       ON programme.proposals USING gin (search_vector);
CREATE INDEX ix_proposals_ranking      ON programme.proposals (event_id, weighted_score DESC NULLS LAST)
    WHERE status IN ('under_review', 'accepted') AND deleted_at IS NULL;

CREATE TRIGGER tg_proposals_updated_at
    BEFORE UPDATE ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_proposals_audit
    AFTER INSERT OR UPDATE OR DELETE ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN programme.proposals.reference_code IS
    'Numéro de dossier communiqué à l''organisation. Attribué à la CRÉATION de la ligne par tg_proposals_reference_code (BEFORE INSERT), donc dès le brouillon, et jamais réutilisé : le dossier porte le même numéro avant et après son dépôt.';
COMMENT ON COLUMN programme.proposals.target_audiences IS
    'Publics visés, une entrée par public. Tableau de textes multilingues : le français reste exigé sur chacun.';
COMMENT ON COLUMN programme.proposals.detailed_presentation IS
    'Présentation détaillée en HTML restreint (mise en forme structurelle seulement). Assainie par l''API à l''écriture ; la police et les couleurs viennent de la charte, jamais du contenu.';
COMMENT ON COLUMN programme.proposals.requested_sessions IS
    'Nombre d''occurrences demandées. Un cycle de webinaires en déclare plusieurs dès le dépôt.';

-- Attribution du numéro de dossier à la soumission.
CREATE SEQUENCE IF NOT EXISTS programme.proposal_reference_seq;

CREATE OR REPLACE FUNCTION programme.tg_assign_reference_code()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_prefix text;
BEGIN
    IF NEW.reference_code IS NOT NULL AND NEW.reference_code <> '' THEN
        RETURN NEW;
    END IF;

    SELECT COALESCE(upper(e.acronym), upper(left(e.slug, 8)))
      INTO v_prefix
    FROM event.events e WHERE e.id = NEW.event_id;

    NEW.reference_code := format('%s-%s',
        COALESCE(v_prefix, 'EPAV'),
        lpad(nextval('programme.proposal_reference_seq')::text, 5, '0'));
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_proposals_reference_code
    BEFORE INSERT ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_assign_reference_code();

-- -----------------------------------------------------------------------------
-- 3. Journal des transitions et garde-fou d'état
-- -----------------------------------------------------------------------------
CREATE TABLE programme.proposal_transitions (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id  uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    from_status  programme.proposal_status,
    to_status    programme.proposal_status NOT NULL,
    actor_id     uuid        CONSTRAINT xmod_fk_proposal_transitions_actor
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    reason       text,
    occurred_at  timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_proposal_transitions ON programme.proposal_transitions (proposal_id, occurred_at DESC);

-- Refuse tout changement d'état non déclaré, journalise ceux qui sont valides et
-- publie l'événement de domaine correspondant. C'est ce trigger qui rend
-- l'enchaînement du workflow fiable indépendamment de l'interface utilisée.
CREATE OR REPLACE FUNCTION programme.tg_guard_proposal_status()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_rule programme.proposal_transitions_allowed%ROWTYPE;
BEGIN
    -- État initial : journalisé depuis un déclencheur AFTER INSERT, la ligne
    -- devant exister pour satisfaire la clé étrangère du journal.
    IF TG_OP = 'INSERT' THEN
        INSERT INTO programme.proposal_transitions (proposal_id, from_status, to_status, actor_id)
        VALUES (NEW.id, NULL, NEW.status, platform.current_actor_id());
        RETURN NULL;
    END IF;

    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;

    SELECT * INTO v_rule
    FROM programme.proposal_transitions_allowed
    WHERE from_status = OLD.status AND to_status = NEW.status;

    IF NOT FOUND THEN
        RAISE EXCEPTION 'Transition interdite : % -> % (proposition %)',
            OLD.status, NEW.status, OLD.reference_code
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF v_rule.requires_reason AND COALESCE(btrim(NEW.decision_reason), '') = '' THEN
        RAISE EXCEPTION 'La transition % -> % exige un motif (decision_reason).',
            OLD.status, NEW.status
            USING ERRCODE = 'not_null_violation';
    END IF;

    IF NEW.status = 'submitted' AND NEW.submitted_at IS NULL THEN
        NEW.submitted_at := now();
    END IF;

    IF NEW.status IN ('accepted', 'rejected') THEN
        NEW.decided_at := now();
        NEW.decided_by := COALESCE(NEW.decided_by, platform.current_actor_id());
    END IF;

    INSERT INTO programme.proposal_transitions (proposal_id, from_status, to_status, actor_id, reason)
    VALUES (NEW.id, OLD.status, NEW.status, platform.current_actor_id(), NEW.decision_reason);

    PERFORM platform.emit_event(
        'programme', 'proposal', NEW.id,
        'programme.proposal.' || NEW.status::text,
        jsonb_build_object(
            'reference_code', NEW.reference_code,
            'event_id', NEW.event_id,
            'organization_id', NEW.organization_id,
            'from_status', OLD.status,
            'to_status', NEW.status,
            'reason', NEW.decision_reason
        )
    );

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_proposals_guard_status
    BEFORE UPDATE OF status ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_guard_proposal_status();

-- Même fonction, mais APRÈS l'insertion : elle n'y journalise que l'état de
-- départ, ce qui exige que la proposition soit déjà écrite.
CREATE TRIGGER tg_proposals_log_initial_status
    AFTER INSERT ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_guard_proposal_status();

-- Recevabilité : contrôle la fenêtre de l'appel et le plafond par organisation
-- au moment du dépôt. En base, donc valable pour l'API comme pour un import.
--
-- LA FENÊTRE NE VAUT QUE POUR UN PREMIER DÉPÔT, et cette distinction n'est pas
-- un confort : le comité demande ses corrections APRÈS la clôture — c'est même
-- le cas normal, l'évaluation commençant quand l'appel se ferme. Un contrôle
-- indifférencié refusait donc le renvoi d'un dossier que le comité venait
-- lui-même de réclamer (`changes_requested -> submitted`), et l'organisation se
-- retrouvait avec un dossier définitivement bloqué, un écran lui affichant
-- « 1 point à corriger » et aucune issue. Le plafond, lui, reste vérifié dans
-- les deux cas : il compte des dossiers, pas des envois — et il s'exclut
-- lui-même par `p.id <> NEW.id`.
CREATE OR REPLACE FUNCTION programme.tg_check_submission_eligibility()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_call  event.calls_for_proposals%ROWTYPE;
    v_count integer;
    v_is_first_submission boolean;
BEGIN
    IF NEW.status <> 'submitted' OR (TG_OP = 'UPDATE' AND OLD.status = 'submitted') THEN
        RETURN NEW;
    END IF;

    IF NEW.call_id IS NULL THEN
        RETURN NEW;  -- proposition hors appel, créée par l'IFDD
    END IF;

    -- Premier dépôt : insertion, ou passage depuis le brouillon. Tout autre
    -- chemin vers `submitted` est un RENVOI — aujourd'hui depuis
    -- `changes_requested`, seul état que la machine autorise à y revenir.
    v_is_first_submission := (TG_OP = 'INSERT') OR (OLD.status = 'draft');

    SELECT * INTO v_call FROM event.calls_for_proposals WHERE id = NEW.call_id;

    IF v_is_first_submission
       AND (v_call.status <> 'open'
            OR now() < v_call.opens_at
            OR now() > COALESCE(v_call.extended_until, v_call.closes_at)) THEN
        RAISE EXCEPTION 'L''appel « % » n''accepte plus de soumission (échéance : %).',
            platform.t(v_call.title), COALESCE(v_call.extended_until, v_call.closes_at)
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF v_call.max_proposals_per_organization IS NOT NULL THEN
        SELECT count(*) INTO v_count
        FROM programme.proposals p
        WHERE p.call_id = NEW.call_id
          AND p.organization_id = NEW.organization_id
          AND p.id <> NEW.id
          AND p.status NOT IN ('draft', 'withdrawn', 'rejected')
          AND p.deleted_at IS NULL;

        IF v_count >= v_call.max_proposals_per_organization THEN
            RAISE EXCEPTION 'Plafond atteint : % proposition(s) au maximum par organisation sur cet appel.',
                v_call.max_proposals_per_organization
                USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;

    IF v_call.requires_verified_organization THEN
        PERFORM 1 FROM org.organizations o
        WHERE o.id = NEW.organization_id AND o.verified_at IS NOT NULL;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'Cet appel est réservé aux organisations vérifiées par l''IFDD.'
                USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_proposals_check_eligibility
    BEFORE INSERT OR UPDATE OF status ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_submission_eligibility();

-- -----------------------------------------------------------------------------
-- 3 bis. CO-ORGANISATION
--
-- « Deux ou plusieurs organisations peuvent co-organiser une activité. »
-- La v1 ne connaissait qu'une colonne `activities.organization_id` : les
-- co-organisateurs, très fréquents dans le monde francophone où une ONG monte
-- une activité avec un ministère et une agence régionale, n'existaient nulle
-- part. Ils finissaient mentionnés dans le texte de présentation, donc
-- invisibles des statistiques, des filtres et du décompte d'activités par
-- organisation.
--
-- La colonne `proposals.organization_id` est conservée : elle désigne le
-- PORTEUR PRINCIPAL, celui qui soumet, répond aux demandes de correction et est
-- notifié de la décision. Elle est maintenue en cohérence avec la ligne de rôle
-- `lead` par trigger : une seule vérité, deux points d'accès.
-- -----------------------------------------------------------------------------
CREATE TYPE programme.organization_role AS ENUM (
    'lead',          -- porteur principal : soumet et répond de la proposition
    'co_organizer',  -- co-organisateur, sur un pied d'égalité éditoriale
    'partner',       -- partenaire associé
    'sponsor'        -- soutien financier ou institutionnel
);

CREATE TABLE programme.proposal_organizations (
    proposal_id     uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    organization_id uuid        NOT NULL CONSTRAINT xmod_fk_proposal_organizations_org
                                REFERENCES org.organizations(id) ON DELETE RESTRICT,
    role            programme.organization_role NOT NULL DEFAULT 'co_organizer',
    -- Une co-organisation annoncée engage un tiers : tant qu'elle n'est pas
    -- confirmée, elle est affichée comme « en attente » côté back-office.
    confirmed_at    timestamptz,
    sort_order      smallint    NOT NULL DEFAULT 0,
    added_by        uuid        CONSTRAINT xmod_fk_proposal_organizations_actor
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    added_at        timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (proposal_id, organization_id)
);

-- Un seul porteur principal par proposition.
CREATE UNIQUE INDEX ux_proposal_organizations_lead
    ON programme.proposal_organizations (proposal_id)
    WHERE role = 'lead';

CREATE INDEX ix_proposal_organizations_org
    ON programme.proposal_organizations (organization_id, role);

COMMENT ON TABLE programme.proposal_organizations IS
    'Organisations associées à une proposition, porteur principal compris. Rend les co-organisations comptables et filtrables.';

-- Le porteur principal déclaré sur la proposition alimente automatiquement la
-- ligne de rôle `lead` : aucune saisie en double, aucune divergence possible.
CREATE OR REPLACE FUNCTION programme.tg_sync_proposal_lead_organization()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND NEW.organization_id IS NOT DISTINCT FROM OLD.organization_id THEN
        RETURN NULL;
    END IF;

    DELETE FROM programme.proposal_organizations
    WHERE proposal_id = NEW.id AND role = 'lead' AND organization_id <> NEW.organization_id;

    INSERT INTO programme.proposal_organizations (proposal_id, organization_id, role, confirmed_at, sort_order)
    VALUES (NEW.id, NEW.organization_id, 'lead', now(), 0)
    ON CONFLICT (proposal_id, organization_id)
        DO UPDATE SET role = 'lead', confirmed_at = COALESCE(programme.proposal_organizations.confirmed_at, now());

    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_proposals_sync_lead_organization
    AFTER INSERT OR UPDATE OF organization_id ON programme.proposals
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sync_proposal_lead_organization();

-- -----------------------------------------------------------------------------
-- 4. Intervenants et pièces jointes
--
-- En v1, `activity_speakers` redupliquait nom, prénom, email et photo à chaque
-- activité : le même expert existait en autant d'exemplaires que de
-- participations, sans moyen de consolider son historique. Ici, l'intervenant
-- EST une personne (identity.people), créée à la volée si elle est inconnue.
-- -----------------------------------------------------------------------------
CREATE TYPE programme.speaker_role AS ENUM ('speaker', 'moderator', 'panelist', 'keynote', 'facilitator', 'interpreter');

CREATE TABLE programme.proposal_speakers (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id       uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    person_id         uuid        NOT NULL CONSTRAINT xmod_fk_proposal_speakers_person
                                  REFERENCES identity.people(id) ON DELETE RESTRICT,
    role              programme.speaker_role NOT NULL DEFAULT 'speaker',
    -- Fonction et organisation AU MOMENT de cette activité : une personne change
    -- d'employeur, l'archive de la COP28 ne doit pas être réécrite pour autant.
    job_title_snapshot text,
    organization_snapshot text,
    organization_id   uuid        CONSTRAINT xmod_fk_proposal_speakers_organization
                                  REFERENCES org.organizations(id) ON DELETE SET NULL,
    bio               platform.i18n_text,
    -- Confirmation par l'intervenant lui-même, via jeton envoyé par courriel
    -- (identity.one_time_tokens, usage `speaker_confirmation`).
    confirmed_at      timestamptz,
    confirmation_sent_at timestamptz,
    is_available_for_questions boolean NOT NULL DEFAULT true,
    sort_order        smallint    NOT NULL DEFAULT 0,
    created_at        timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_proposal_speakers UNIQUE (proposal_id, person_id, role)
);

CREATE INDEX ix_proposal_speakers_proposal ON programme.proposal_speakers (proposal_id, sort_order);
CREATE INDEX ix_proposal_speakers_person   ON programme.proposal_speakers (person_id);

COMMENT ON COLUMN programme.proposal_speakers.organization_snapshot IS
    'Organisation déclarée au moment de l''activité. Conservée telle quelle : l''archive ne doit pas bouger.';

CREATE TABLE programme.proposal_documents (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id  uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    asset_id     uuid        NOT NULL CONSTRAINT xmod_fk_proposal_documents_asset
                             REFERENCES media.assets(id) ON DELETE RESTRICT,
    title        platform.i18n_text NOT NULL,
    document_type_code text,   -- taxonomie `document_type`
    -- Document visible du public une fois l'activité publiée, ou pièce interne
    -- au dossier d'évaluation.
    is_public    boolean     NOT NULL DEFAULT false,
    uploaded_by  uuid        CONSTRAINT xmod_fk_proposal_documents_uploader
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    uploaded_at  timestamptz NOT NULL DEFAULT now(),
    sort_order   smallint    NOT NULL DEFAULT 0
);

CREATE INDEX ix_proposal_documents_proposal ON programme.proposal_documents (proposal_id, sort_order);

-- -----------------------------------------------------------------------------
-- 5. Évaluation
-- -----------------------------------------------------------------------------

-- Répartition de la charge : qui doit évaluer quoi, avec quelle échéance.
CREATE TABLE programme.review_assignments (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id  uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    reviewer_id  uuid        NOT NULL CONSTRAINT xmod_fk_review_assignments_reviewer
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    assigned_by  uuid        CONSTRAINT xmod_fk_review_assignments_assigner
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    assigned_at  timestamptz NOT NULL DEFAULT now(),
    due_at       timestamptz,
    -- Déport volontaire : le révisionniste déclare un lien avec l'organisation
    -- porteuse et se retire. Traçabilité de l'impartialité du comité.
    recused_at   timestamptz,
    recusal_reason text,
    CONSTRAINT ux_review_assignments UNIQUE (proposal_id, reviewer_id)
);

CREATE INDEX ix_review_assignments_reviewer
    ON programme.review_assignments (reviewer_id, due_at)
    WHERE recused_at IS NULL;

-- Une revue = l'avis complet d'un membre du comité sur une proposition.
CREATE TABLE programme.reviews (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id    uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    reviewer_id    uuid        NOT NULL CONSTRAINT xmod_fk_reviews_reviewer
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    recommendation text        NOT NULL DEFAULT 'neutral'
                               CHECK (recommendation IN ('accept', 'accept_with_changes', 'neutral', 'reject')),
    -- Note pondérée calculée à partir des critères (voir trigger ci-dessous).
    weighted_score numeric(6,2),
    -- Note ramenée sur 20, pour rester lisible par les équipes habituées à la v1.
    score_out_of_20 numeric(4,2),
    strengths      text,
    weaknesses     text,
    private_note   text,       -- visible du seul comité, jamais du soumissionnaire
    submitted_at   timestamptz,
    created_at     timestamptz NOT NULL DEFAULT now(),
    updated_at     timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_reviews UNIQUE (proposal_id, reviewer_id)
);

CREATE INDEX ix_reviews_proposal ON programme.reviews (proposal_id);
CREATE INDEX ix_reviews_pending  ON programme.reviews (reviewer_id) WHERE submitted_at IS NULL;

CREATE TRIGGER tg_reviews_updated_at
    BEFORE UPDATE ON programme.reviews
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Note par critère : c'est ici que se justifie une décision contestée.
CREATE TABLE programme.review_scores (
    review_id    uuid        NOT NULL REFERENCES programme.reviews(id) ON DELETE CASCADE,
    criterion_id uuid        NOT NULL CONSTRAINT xmod_fk_review_scores_criterion
                             REFERENCES event.review_criteria(id) ON DELETE CASCADE,
    score        numeric(5,2) NOT NULL CHECK (score >= 0),
    comment      text,
    PRIMARY KEY (review_id, criterion_id)
);

-- Contrôle du plafond propre à chaque critère : une note ne peut dépasser
-- `max_score`, valeur qui varie d'un critère et d'un appel à l'autre.
CREATE OR REPLACE FUNCTION programme.tg_check_score_bounds()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_max numeric;
BEGIN
    SELECT max_score INTO v_max FROM event.review_criteria WHERE id = NEW.criterion_id;
    IF NEW.score > v_max THEN
        RAISE EXCEPTION 'Note % supérieure au maximum autorisé (%) pour ce critère.', NEW.score, v_max
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_review_scores_bounds
    BEFORE INSERT OR UPDATE ON programme.review_scores
    FOR EACH ROW EXECUTE FUNCTION programme.tg_check_score_bounds();

-- Consolidation : note pondérée de la revue, puis moyenne de la proposition et
-- détection des critères éliminatoires. Appelée après chaque saisie de note.
CREATE OR REPLACE FUNCTION programme.refresh_proposal_score(p_proposal_id uuid)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_call_id uuid;
    v_max     numeric;
BEGIN
    SELECT call_id INTO v_call_id FROM programme.proposals WHERE id = p_proposal_id;
    v_max := COALESCE(NULLIF(event.max_weighted_score(v_call_id), 0), 0);

    -- Note pondérée de chaque revue.
    UPDATE programme.reviews r
    SET weighted_score = agg.total,
        score_out_of_20 = CASE WHEN v_max > 0 THEN round(agg.total / v_max * 20, 2) END
    FROM (
        SELECT rs.review_id, sum(rs.score * c.weight) AS total
        FROM programme.review_scores rs
        JOIN event.review_criteria c ON c.id = rs.criterion_id
        GROUP BY rs.review_id
    ) agg
    WHERE r.id = agg.review_id AND r.proposal_id = p_proposal_id;

    -- Agrégats de la proposition : seules les revues soumises comptent.
    UPDATE programme.proposals p
    SET review_count   = COALESCE(agg.n, 0),
        weighted_score = agg.avg_weighted,
        average_score  = agg.avg_20,
        is_knocked_out = COALESCE(ko.knocked_out, false)
    FROM (
        SELECT count(*) AS n,
               round(avg(weighted_score), 2) AS avg_weighted,
               round(avg(score_out_of_20), 2) AS avg_20
        FROM programme.reviews
        WHERE proposal_id = p_proposal_id AND submitted_at IS NOT NULL
    ) agg
    LEFT JOIN LATERAL (
        SELECT bool_or(rs.score = 0) AS knocked_out
        FROM programme.reviews r
        JOIN programme.review_scores rs ON rs.review_id = r.id
        JOIN event.review_criteria c ON c.id = rs.criterion_id AND c.is_knockout
        WHERE r.proposal_id = p_proposal_id AND r.submitted_at IS NOT NULL
    ) ko ON true
    WHERE p.id = p_proposal_id;
END;
$$;

COMMENT ON FUNCTION programme.refresh_proposal_score IS
    'Recalcule les notes pondérées et les agrégats d''une proposition. À appeler après toute saisie de note.';

-- -----------------------------------------------------------------------------
-- 6. Échanges autour d'une proposition
--
-- La v1 stockait les destinataires d'un commentaire dans
-- `revision_comments.shared_with_revisionists UUID[]` : un tableau d'identifiants
-- sans contrainte référentielle, impossible à indexer utilement et silencieux
-- quand un compte disparaissait. Ici, la visibilité est un état explicite.
-- -----------------------------------------------------------------------------
CREATE TYPE programme.comment_visibility AS ENUM (
    'committee',   -- comité de sélection uniquement
    'submitter',   -- partagé avec l'organisation soumissionnaire
    'private'      -- note personnelle de son auteur
);

CREATE TABLE programme.proposal_comments (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    proposal_id  uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    parent_id    uuid        REFERENCES programme.proposal_comments(id) ON DELETE CASCADE,
    author_id    uuid        NOT NULL CONSTRAINT xmod_fk_proposal_comments_author
                             REFERENCES identity.people(id) ON DELETE CASCADE,
    visibility   programme.comment_visibility NOT NULL DEFAULT 'committee',
    body         text        NOT NULL CHECK (length(btrim(body)) > 0),
    -- Demande de correction adressée au soumissionnaire, avec suivi de
    -- résolution : c'est le fil qui pilote l'état `changes_requested`.
    is_change_request boolean NOT NULL DEFAULT false,
    resolved_at  timestamptz,
    resolved_by  uuid        CONSTRAINT xmod_fk_proposal_comments_resolver
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    edited_at    timestamptz,
    deleted_at   timestamptz,
    created_at   timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_proposal_comments ON programme.proposal_comments (proposal_id, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX ix_proposal_comments_open_requests
    ON programme.proposal_comments (proposal_id)
    WHERE is_change_request AND resolved_at IS NULL AND deleted_at IS NULL;

-- Accusés de lecture : reprend `revisionniste_activity_views` en le généralisant
-- (le back-office affiche « lu par 3 membres du comité sur 5 »).
CREATE TABLE programme.proposal_reads (
    proposal_id    uuid        NOT NULL REFERENCES programme.proposals(id) ON DELETE CASCADE,
    person_id      uuid        NOT NULL CONSTRAINT xmod_fk_proposal_reads_person
                               REFERENCES identity.people(id) ON DELETE CASCADE,
    first_read_at  timestamptz NOT NULL DEFAULT now(),
    last_read_at   timestamptz NOT NULL DEFAULT now(),
    read_count     integer     NOT NULL DEFAULT 1,
    PRIMARY KEY (proposal_id, person_id)
);

CREATE OR REPLACE FUNCTION programme.record_proposal_read(p_proposal_id uuid, p_person_id uuid)
RETURNS void
LANGUAGE sql
AS $$
    INSERT INTO programme.proposal_reads (proposal_id, person_id)
    VALUES (p_proposal_id, p_person_id)
    ON CONFLICT (proposal_id, person_id) DO UPDATE
    SET last_read_at = now(), read_count = programme.proposal_reads.read_count + 1;
$$;

-- -----------------------------------------------------------------------------
-- 7. Vue de pilotage du comité
--
-- Alimente l'écran « liste des activités » du back-office : avancement des
-- revues, classement, alertes. Une seule requête, pas de N+1.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW programme.v_proposal_dashboard AS
SELECT
    p.id,
    p.reference_code,
    p.event_id,
    p.call_id,
    p.organization_id,
    o.legal_name              AS organization_name,
    -- Le titre est exposé DEUX FOIS, sous deux noms distincts :
    --   `title`      — le document multilingue brut, du même type que
    --                  programme.proposals.title et programme.v_public_schedule.title,
    --                  résolu à l'affichage par l'utilitaire du frontend ;
    --   `title_text` — la résolution française de platform.t(), pour ce que le
    --                  JSON ne sait pas faire : trier, filtrer et exporter en SQL.
    -- Une version antérieure nommait `title` la valeur déjà résolue : le même nom
    -- de champ portait alors un `text` ici et un `i18n_text` sur la table, et
    -- resolveI18nText() appliqué dessus rendait une chaîne vide sans erreur.
    p.title,
    platform.t(p.title)       AS title_text,
    p.status,
    p.submitted_at,
    p.weighted_score,
    p.average_score,
    p.is_knocked_out,
    p.review_count,
    c.required_reviews,
    GREATEST(c.required_reviews - p.review_count, 0) AS reviews_missing,
    (SELECT count(*) FROM programme.review_assignments ra
      WHERE ra.proposal_id = p.id AND ra.recused_at IS NULL)          AS assigned_reviewers,
    (SELECT count(*) FROM programme.proposal_comments pc
      WHERE pc.proposal_id = p.id AND pc.is_change_request
        AND pc.resolved_at IS NULL AND pc.deleted_at IS NULL)         AS open_change_requests,
    (SELECT count(*) FROM programme.proposal_speakers ps
      WHERE ps.proposal_id = p.id)                                    AS speaker_count,
    rank() OVER (PARTITION BY p.event_id ORDER BY p.weighted_score DESC NULLS LAST) AS event_rank,

    -- -------------------------------------------------------------------------
    -- CE QUE LA LISTE DU BACK-OFFICE MONTRE ET FILTRE (écran A7).
    --
    -- Ces colonnes ont été ajoutées le 18/08 : la vue portait l'avancement des
    -- revues et le classement, mais rien de ce qui IDENTIFIE un dossier dans un
    -- tableau de quarante lignes — son format, le pays de son porteur, ses
    -- thématiques, ses co-organisateurs, qui l'évalue. L'écran devait donc
    -- charger quatre tables de plus et refaire les correspondances lui-même,
    -- c'est-à-dire perdre la raison d'être de cette vue : répondre à un écran en
    -- une requête. Même correction que celle déjà faite sur v_public_schedule.
    -- -------------------------------------------------------------------------
    p.format,
    p.activity_type_code,
    o.acronym                 AS organization_acronym,
    -- PAYS DE L'ORGANISATION PORTEUSE, et non `proposals.country_id` : la
    -- colonne du dossier désigne le pays CONCERNÉ par l'activité, souvent nul et
    -- parfois différent. Ce que la liste range, c'est d'où vient le déposant —
    -- même choix qu'à la répartition géographique du tableau de bord.
    -- Deux colonnes, deux usages : le code ISO est stable et sert au filtre, le
    -- nom est multilingue et se résout à l'affichage.
    cn.iso2                   AS organization_country_code,
    cn.name                   AS organization_country,
    -- Co-organisateurs, PORTEUR EXCLU : c'est le « +2 » de la colonne
    -- organisation. Compté ici parce qu'une liste dense ne peut afficher trois
    -- noms par ligne, et qu'un dossier co-organisé ne se lit pas comme un
    -- dossier porté seul.
    (SELECT count(*) FROM programme.proposal_organizations po
      WHERE po.proposal_id = p.id AND po.role <> 'lead')              AS co_organizer_count,
    -- Thématiques, DEUX FOIS et pour deux usages distincts — `theme_codes` pour
    -- FILTRER (opérateurs de tableau, indexables), `themes` pour AFFICHER
    -- (libellé traduit et couleur venus de reference.taxonomy_terms, où un
    -- administrateur les modifie). N'exposer que les codes forcerait l'écran à
    -- recharger la taxonomie : c'est ainsi que les libellés se sont retrouvés
    -- figés dans le frontend de la v1.
    reference.terms_of('programme', 'proposals', p.id, 'activity_theme')    AS theme_codes,
    reference.term_badges('programme', 'proposals', p.id, 'activity_theme') AS themes,
    -- QUI ÉVALUE CE DOSSIER, déports exclus comme `assigned_reviewers`.
    -- `reviewer_ids` filtre (« les dossiers confiés à X »), `reviewers` affiche
    -- l'avancement nominatif : un « 2/3 » ne dit pas de qui on attend la revue.
    COALESCE((
        SELECT array_agg(ra.reviewer_id ORDER BY ra.assigned_at)
        FROM programme.review_assignments ra
        WHERE ra.proposal_id = p.id AND ra.recused_at IS NULL
    ), '{}'::uuid[]) AS reviewer_ids,
    COALESCE((
        SELECT jsonb_agg(jsonb_build_object(
                   'person_id',    ra.reviewer_id,
                   'name',         pe.display_name,
                   'due_at',       ra.due_at,
                   'submitted_at', rv.submitted_at)
               ORDER BY pe.display_name)
        FROM programme.review_assignments ra
        JOIN identity.people pe ON pe.id = ra.reviewer_id
        LEFT JOIN programme.reviews rv
               ON rv.proposal_id = ra.proposal_id AND rv.reviewer_id = ra.reviewer_id
        WHERE ra.proposal_id = p.id AND ra.recused_at IS NULL
    ), '[]'::jsonb) AS reviewers,
    -- EN RETARD : une revue attendue dont l'échéance est passée. C'est un état
    -- du dossier, calculé ici une fois pour toutes — l'écran de liste, le
    -- tableau de bord et la file du comité doivent en donner le même compte.
    (SELECT count(*)
       FROM programme.review_assignments ra
       LEFT JOIN programme.reviews rv
              ON rv.proposal_id = ra.proposal_id AND rv.reviewer_id = ra.reviewer_id
      WHERE ra.proposal_id = p.id
        AND ra.recused_at IS NULL
        AND ra.due_at IS NOT NULL
        AND ra.due_at < now()
        AND rv.submitted_at IS NULL)                                  AS overdue_reviews,
    -- Prochaine échéance encore due, toutes affectations confondues : ce que la
    -- liste trie quand l'équipe cherche « ce qui tombe cette semaine ».
    (SELECT min(ra.due_at)
       FROM programme.review_assignments ra
       LEFT JOIN programme.reviews rv
              ON rv.proposal_id = ra.proposal_id AND rv.reviewer_id = ra.reviewer_id
      WHERE ra.proposal_id = p.id
        AND ra.recused_at IS NULL
        AND rv.submitted_at IS NULL)                                  AS next_review_due_at,
    -- « Lu par 3 membres du comité » — programme.proposal_reads. Ce compteur est
    -- COLLECTIF ; savoir si la personne CONNECTÉE l'a ouvert dépend du lecteur et
    -- ne peut donc pas être une colonne : voir programme.unread_proposals_for().
    (SELECT count(*) FROM programme.proposal_reads pr
      WHERE pr.proposal_id = p.id)                                    AS read_count
FROM programme.proposals p
JOIN org.organizations o ON o.id = p.organization_id
LEFT JOIN event.calls_for_proposals c ON c.id = p.call_id
LEFT JOIN reference.countries cn ON cn.id = o.country_id
WHERE p.deleted_at IS NULL;

COMMENT ON VIEW programme.v_proposal_dashboard IS
    'Vue de pilotage du comité de sélection : avancement des revues, classement par événement, alertes.';
COMMENT ON COLUMN programme.v_proposal_dashboard.title IS
    'Titre multilingue brut, identique à programme.proposals.title. Un même nom de champ ne désigne jamais deux types.';
COMMENT ON COLUMN programme.v_proposal_dashboard.title_text IS
    'Titre résolu par platform.t() (repli français). Réservé au tri, au filtrage et à l''export SQL ; ne pas l''afficher à la place de title.';
COMMENT ON COLUMN programme.v_proposal_dashboard.organization_country IS
    'Pays de l''organisation PORTEUSE, multilingue. Distinct de proposals.country_id, qui désigne le pays concerné par l''activité.';
COMMENT ON COLUMN programme.v_proposal_dashboard.co_organizer_count IS
    'Organisations associées hors porteur principal : la pastille « +2 » de la liste du back-office.';
COMMENT ON COLUMN programme.v_proposal_dashboard.theme_codes IS
    'Codes des thématiques, pour FILTRER. L''affichage passe par `themes`, qui porte libellé et couleur.';
COMMENT ON COLUMN programme.v_proposal_dashboard.reviewers IS
    'Révisionnistes affectés (déports exclus) avec leur échéance et la date de remise de leur revue. Un « 2/3 » ne dit pas de qui on attend la troisième.';
COMMENT ON COLUMN programme.v_proposal_dashboard.overdue_reviews IS
    'Revues attendues dont l''échéance est dépassée. Alimente le filtre « en retard » : un seul calcul pour la liste, le tableau de bord et la file du comité.';
COMMENT ON COLUMN programme.v_proposal_dashboard.read_count IS
    'Nombre de membres du comité ayant ouvert le dossier. Collectif : pour « non consulté PAR MOI », voir programme.unread_proposals_for().';

-- Dossiers qu'une personne donnée n'a JAMAIS ouverts, sur une édition.
--
-- POURQUOI UNE FONCTION ET NON UNE COLONNE DE LA VUE. « Non consulté » n'est pas
-- une propriété du dossier mais de la relation entre un dossier et un lecteur :
-- la même ligne est lue par l'un et pas par l'autre. Une vue sans paramètre ne
-- peut pas le dire, et la faire dépendre de current_setting('app.actor_id')
-- rendrait son résultat invisible à la relecture — deux requêtes identiques, deux
-- réponses. La liste du back-office croise donc cette réponse avec la vue.
CREATE OR REPLACE FUNCTION programme.unread_proposals_for(p_person_id uuid, p_event_id uuid)
RETURNS uuid[]
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(array_agg(p.id ORDER BY p.reference_code), '{}'::uuid[])
    FROM programme.proposals p
    WHERE p.event_id = p_event_id
      AND p.deleted_at IS NULL
      AND NOT EXISTS (
          SELECT 1 FROM programme.proposal_reads pr
          WHERE pr.proposal_id = p.id AND pr.person_id = p_person_id
      );
$$;

COMMENT ON FUNCTION programme.unread_proposals_for IS
    'Dossiers d''une édition que cette personne n''a jamais ouverts. Alimente l''indicateur discret « non consulté » de la liste du back-office.';

-- -----------------------------------------------------------------------------
-- 7 bis. Historique des modifications d'une proposition
--
-- Une organisation corrige son dossier, l'IFDD ajuste un intitulé, une date
-- proposée change : tout doit rester retraçable. Le trigger d'audit alimente
-- platform.audit_log, cette fonction le restitue champ par champ.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION programme.proposal_history(p_proposal_id uuid)
RETURNS TABLE (
    occurred_at timestamptz, actor_id uuid, actor_label text,
    action text, field text, old_value jsonb, new_value jsonb
)
LANGUAGE sql
STABLE
AS $$
    SELECT * FROM platform.entity_history(
        'programme', 'proposals', p_proposal_id,
        ARRAY['updated_at', 'search_vector', 'view_count',
              'average_score', 'weighted_score', 'review_count', 'is_knocked_out']
    );
$$;

COMMENT ON FUNCTION programme.proposal_history IS
    'Historique complet des modifications d''un dossier, hors champs recalculés. Alimente l''onglet « Historique » du back-office.';

-- -----------------------------------------------------------------------------
-- 8. Déclaration des références vers les organisations (pour la fusion)
-- -----------------------------------------------------------------------------
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy, dedupe_on) VALUES
    ('programme', 'proposals',              'organization_id', 'reassign', '{}'),
    ('programme', 'proposal_speakers',      'organization_id', 'reassign', '{}'),
    ('programme', 'proposal_organizations', 'organization_id', 'reassign', '{proposal_id}')
ON CONFLICT DO NOTHING;
