-- =============================================================================
-- ePavillon v2 — 120_tools.sql
-- Module Outils : sondages / évaluations, et assistants IA (RAG).
--
-- Dépend de : 000, 010, 020, 030
--
-- CE MODULE EST LE CANDIDAT N°1 À L'EXTRACTION EN SERVICE AUTONOME
-- Citation du cadrage : « Outils — ils doivent être développés de façon
-- modulaire, il peut nous arriver dans les phases à venir de les déployer sous
-- un autre lien à part. Outil de sondage (création de sondage indépendant ou
-- après une session Zoom ou physique). Agent IA pour créer des réunions et
-- plein d'autres choses. Plusieurs autres outils seront intégrés au fur et à
-- mesure. »
--
-- Trois décisions structurelles en découlent. Ce sont elles qui rendent le
-- `UPDATE platform.modules SET deployment = 'external', base_url = ...`
-- réalisable en une soirée plutôt qu'en un trimestre.
--
--   D1. ZÉRO CLÉ ÉTRANGÈRE SORTANTE VERS LES SCHÉMAS MÉTIER
--       Aucun REFERENCES vers `event`, `programme`, `negotiation`, `org` ni
--       `media`. Le rattachement — un sondage à une session Zoom, une source
--       documentaire à un document de négociation — passe par le triplet libre
--       (`context_schema`, `context_table`, `context_id`) SANS contrainte,
--       doublé d'un libellé dénormalisé `context_label`. Ce libellé n'est pas
--       une redondance paresseuse : il permet à l'outil débranché d'afficher
--       encore « Atelier Genre et climat, 12 novembre » au lieu d'un UUID
--       orphelin. Prix payé : un libellé qui peut vieillir.
--
--   D2. UNE SEULE FK À COUPER : identity.people
--       Auteur et répondant identifié pointent vers `identity.people(id)` en
--       `xmod_fk_*` ON DELETE SET NULL — seule dépendance dure du module, et en
--       SET NULL pour que l'effacement d'un compte vide le lien sans détruire la
--       réponse. L'extraction se résume alors à exécuter
--       `platform.generate_module_decoupling_script('tool')`. En regard,
--       `respondent_email` / `respondent_label` portent l'identité déclarée des
--       répondants externes — cas majoritaire d'un sondage ouvert par lien public.
--
--   D3. ANONYMAT RÉEL, PAS DÉCLARATIF
--       La v1 posait `is_anonymous` sur `polls` tout en écrivant `user_id` dans
--       `poll_responses` : l'anonymat n'était qu'une consigne d'affichage, sa
--       levée tenait en une requête. Ici un sondage anonyme ne PEUT PAS stocker
--       le lien vers la personne (drapeau recopié + CHECK) ; le dédoublonnage se
--       fait sur un jeton haché, jamais sur une identité.
--
-- FUSION DES TROIS SYSTÈMES CONCURRENTS DE LA V1
-- `polls`, `evaluations`/`evaluation_questions`/`_answers` et `live_quizzes`/
-- `quiz_questions`/`quiz_responses`/`quiz_results` faisaient trois fois la même
-- chose — poser des questions, collecter, compter — avec trois modèles, trois
-- écrans d'administration et trois jeux de bugs. Tout est ramené à `surveys` +
-- `questions` + `submissions` + `answers`, différenciés par `kind` : un quiz
-- n'est qu'un sondage qui porte des points et une bonne réponse.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Contexte libre et validateurs partagés
--
-- Le triplet de contexte remplace ce qui serait normalement une clé étrangère.
-- On ne peut pas garantir l'intégrité référentielle — c'est le prix assumé de
-- l'autonomie — mais on garantit au moins la COHÉRENCE DE FORME : pas d'`id`
-- flottant sans schéma ni table pour l'interpréter.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION tool.is_context_consistent(p_schema text, p_table text, p_id uuid)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT (p_schema IS NULL AND p_table IS NULL AND p_id IS NULL)
        OR (p_schema IS NOT NULL AND p_table IS NOT NULL);
$$;

COMMENT ON FUNCTION tool.is_context_consistent(text, text, uuid) IS
    'Valide la forme du rattachement libre (context_schema, context_table, context_id) : tout nul, ou schéma et table renseignés.';

-- Liste d'options d'une question : tableau d'objets {value, label i18n}.
-- Valider la forme en base évite qu'un formulaire mal codé n'écrive des options
-- inexploitables que l'on ne découvrira qu'au dépouillement, sondage clos.
CREATE OR REPLACE FUNCTION tool.is_option_list(p_value jsonb)
RETURNS boolean
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT jsonb_typeof(p_value) = 'array'
       AND NOT EXISTS (
           SELECT 1 FROM jsonb_array_elements(p_value) AS e(item)
           WHERE jsonb_typeof(e.item) <> 'object'
              OR jsonb_typeof(e.item -> 'value') IS DISTINCT FROM 'string'
              OR NOT COALESCE(platform.is_i18n_text(e.item -> 'label'), false)
       );
$$;

-- Aplatissement d'une valeur de réponse en texte : alimente la colonne générée
-- de recherche plein texte, quel que soit le type de la question.
CREATE OR REPLACE FUNCTION tool.jsonb_to_text(p_value jsonb)
RETURNS text
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT CASE jsonb_typeof(p_value)
        WHEN 'string' THEN p_value #>> '{}'
        WHEN 'number' THEN p_value #>> '{}'
        WHEN 'array'  THEN (SELECT string_agg(e.item #>> '{}', ' ') FROM jsonb_array_elements(p_value) AS e(item))
        WHEN 'object' THEN (SELECT string_agg(e.value #>> '{}', ' ') FROM jsonb_each(p_value) AS e(key, value))
        ELSE NULL
    END;
$$;

-- Normalisation d'un tableau JSON pour comparaison ensembliste : cocher
-- « A puis B » ou « B puis A » est la même réponse à un choix multiple.
CREATE OR REPLACE FUNCTION tool.jsonb_sorted(p_value jsonb)
RETURNS jsonb
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT CASE WHEN jsonb_typeof(p_value) = 'array'
        THEN COALESCE((SELECT jsonb_agg(e.item ORDER BY e.item::text) FROM jsonb_array_elements(p_value) AS e(item)), '[]'::jsonb)
        ELSE p_value
    END;
$$;

-- =============================================================================
-- PARTIE A — SONDAGES, QUIZ ET ÉVALUATIONS
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 2. Sondages
-- -----------------------------------------------------------------------------
CREATE TYPE tool.survey_kind AS ENUM (
    'poll',        -- sondage d'opinion, souvent une seule question, résultats immédiats
    'quiz',        -- questions notées, bonne réponse connue, score et seuil de réussite
    'evaluation',  -- évaluation de fin de formation ou de session, notée ou non
    'feedback'     -- satisfaction à chaud après une session Zoom ou physique
);

CREATE TYPE tool.survey_status AS ENUM ('draft', 'open', 'closed', 'archived');

-- Quand les répondants voient-ils les résultats agrégés ? Une décision de
-- méthode : afficher les résultats en cours de collecte biaise les réponses
-- suivantes (effet de conformité).
CREATE TYPE tool.result_visibility AS ENUM ('never', 'after_answer', 'after_close');

CREATE TABLE tool.surveys (
    id                   uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    kind                 tool.survey_kind NOT NULL DEFAULT 'poll',

    -- URL autonome : `/s/{slug}`. Indispensable pour un déploiement séparé — le
    -- lien distribué aux participants ne doit dépendre d'aucun identifiant de la
    -- plateforme hôte, ni casser le jour où l'outil change de domaine.
    slug                 platform.slug NOT NULL,
    title                platform.i18n_text NOT NULL,
    description          platform.i18n_text,
    closing_message      platform.i18n_text,

    -- D1 — rattachement libre, sans clé étrangère. Exemples :
    --   ('programme', 'sessions', <uuid>, 'Atelier Genre et climat — 12 nov.')
    --   ('event', 'events', <uuid>, 'COP30 — Belém')
    --   (NULL, NULL, NULL, NULL) pour un sondage indépendant.
    context_schema       text,
    context_table        text,
    context_id           uuid,
    context_label        text,

    opens_at             timestamptz,
    closes_at            timestamptz,

    is_anonymous         boolean     NOT NULL DEFAULT false,
    allow_multiple_submissions boolean NOT NULL DEFAULT false,
    requires_invitation  boolean     NOT NULL DEFAULT false,
    result_visibility    tool.result_visibility NOT NULL DEFAULT 'after_close',

    -- Seuil de réussite en pourcentage (quiz et évaluations seulement).
    passing_score        smallint    CHECK (passing_score BETWEEN 0 AND 100),

    default_locale       text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    status               tool.survey_status NOT NULL DEFAULT 'draft',

    -- D2 — la seule FK sortante du module, avec celle des soumissions.
    created_by           uuid        CONSTRAINT xmod_fk_surveys_author
                                     REFERENCES identity.people(id) ON DELETE SET NULL,
    created_by_label     text,       -- dénormalisé : l'outil détaché reste lisible
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_surveys_window
        CHECK (opens_at IS NULL OR closes_at IS NULL OR closes_at > opens_at),
    CONSTRAINT ck_surveys_context
        CHECK (tool.is_context_consistent(context_schema, context_table, context_id)),
    CONSTRAINT ck_surveys_passing_score
        CHECK (passing_score IS NULL OR kind IN ('quiz', 'evaluation'))
);

CREATE UNIQUE INDEX ux_surveys_slug ON tool.surveys (slug);
-- Retrouver tous les sondages rattachés à une session : la requête que le
-- module hôte posera en boucle depuis la page d'une activité.
CREATE INDEX ix_surveys_context ON tool.surveys (context_schema, context_table, context_id)
    WHERE context_id IS NOT NULL;
CREATE INDEX ix_surveys_open    ON tool.surveys (status, closes_at) WHERE status = 'open';
CREATE INDEX ix_surveys_author  ON tool.surveys (created_by) WHERE created_by IS NOT NULL;

CREATE TRIGGER tg_surveys_updated_at
    BEFORE UPDATE ON tool.surveys
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_surveys_audit
    AFTER INSERT OR UPDATE OR DELETE ON tool.surveys
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE tool.surveys IS
    'Sondage, quiz ou évaluation. Fusionne polls, evaluations et live_quizzes de la v1 : un seul modèle, différencié par kind.';
COMMENT ON COLUMN tool.surveys.slug IS
    'Segment d''URL autonome (/s/{slug}). Reste valide si l''outil est déployé sur un domaine distinct.';
COMMENT ON COLUMN tool.surveys.context_label IS
    'Libellé du rattachement, dénormalisé volontairement : l''outil détaché de la plateforme reste compréhensible.';
COMMENT ON COLUMN tool.surveys.is_anonymous IS
    'Anonymat structurel : recopié sur chaque soumission, où une contrainte CHECK interdit alors tout lien vers une personne.';

-- -----------------------------------------------------------------------------
-- 3. Questions
-- -----------------------------------------------------------------------------
CREATE TYPE tool.question_type AS ENUM (
    'single_choice', 'multiple_choice', 'text', 'long_text',
    'rating', 'scale', 'yes_no', 'ranking', 'date'
);

CREATE TABLE tool.survey_questions (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    survey_id     uuid        NOT NULL REFERENCES tool.surveys(id) ON DELETE CASCADE,
    position      smallint    NOT NULL CHECK (position >= 0),
    question_type tool.question_type NOT NULL,
    label         platform.i18n_text NOT NULL,
    help_text     platform.i18n_text,
    is_required   boolean     NOT NULL DEFAULT false,

    -- [{"value": "tres_satisfait", "label": {"fr": "Très satisfait", "en": "Very satisfied"}}]
    options       jsonb       NOT NULL DEFAULT '[]'::jsonb,
    scale_min     smallint,
    scale_max     smallint,

    -- Mode quiz : barème et corrigé. Nuls pour un sondage d'opinion.
    points        smallint    NOT NULL DEFAULT 0 CHECK (points >= 0),
    correct_answer jsonb,

    -- Logique conditionnelle : {"question_id": "<uuid>", "operator": "equals",
    -- "value": "oui"}. Évaluée côté client ET revalidée à la soumission.
    show_if       jsonb,

    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),

    -- Différée : réordonner un questionnaire est un seul UPDATE, pas une danse
    -- de positions temporaires négatives.
    CONSTRAINT ux_survey_questions_position UNIQUE (survey_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT ck_survey_questions_options
        CHECK (tool.is_option_list(options)
               AND (question_type NOT IN ('single_choice', 'multiple_choice', 'ranking')
                    OR jsonb_array_length(options) >= 2)),
    CONSTRAINT ck_survey_questions_scale
        CHECK ((scale_min IS NULL AND scale_max IS NULL)
               OR (question_type IN ('rating', 'scale') AND scale_max > scale_min))
);

CREATE INDEX ix_survey_questions_survey ON tool.survey_questions (survey_id, position);

CREATE TRIGGER tg_survey_questions_updated_at
    BEFORE UPDATE ON tool.survey_questions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON COLUMN tool.survey_questions.options IS
    'Options typées [{value, label i18n}] : le libellé est traduisible sans casser les réponses déjà collectées, qui stockent la valeur.';
COMMENT ON COLUMN tool.survey_questions.show_if IS
    'Condition d''affichage référençant une autre question du même sondage. Permet les questionnaires à branches sans table dédiée.';

-- -----------------------------------------------------------------------------
-- 4. Invitations nominatives
--
-- Un jeton par destinataire, stocké HACHÉ. Deux bénéfices : mesurer un taux de
-- réponse réel (envoyés / ouverts / répondus) sans imposer d'authentification,
-- et permettre une relance ciblée. Le jeton en clair ne vit que dans l'email.
-- -----------------------------------------------------------------------------
CREATE TABLE tool.survey_invitations (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    survey_id       uuid        NOT NULL REFERENCES tool.surveys(id) ON DELETE CASCADE,
    person_id       uuid        CONSTRAINT xmod_fk_survey_invitations_person
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    recipient_email platform.email,
    recipient_label text,
    token_hash      bytea       NOT NULL,
    locale          text        REFERENCES reference.locales(code),
    sent_at         timestamptz,
    opened_at       timestamptz,
    responded_at    timestamptz,
    expires_at      timestamptz,
    reminder_count  smallint    NOT NULL DEFAULT 0,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_survey_invitations_recipient
        CHECK (person_id IS NOT NULL OR recipient_email IS NOT NULL)
);

CREATE UNIQUE INDEX ux_survey_invitations_token ON tool.survey_invitations (token_hash);
CREATE UNIQUE INDEX ux_survey_invitations_email
    ON tool.survey_invitations (survey_id, recipient_email) WHERE recipient_email IS NOT NULL;
CREATE INDEX ix_survey_invitations_pending
    ON tool.survey_invitations (survey_id) WHERE responded_at IS NULL;

COMMENT ON COLUMN tool.survey_invitations.token_hash IS
    'Empreinte SHA-256 du jeton (digest(token, ''sha256'')). Le jeton en clair n''est jamais stocké : une fuite de base ne donne pas accès aux sondages.';

-- -----------------------------------------------------------------------------
-- 5. Soumissions
-- -----------------------------------------------------------------------------
CREATE TYPE tool.submission_status AS ENUM ('in_progress', 'completed', 'abandoned');

CREATE TABLE tool.survey_submissions (
    id               uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    survey_id        uuid        NOT NULL REFERENCES tool.surveys(id) ON DELETE CASCADE,
    invitation_id    uuid        REFERENCES tool.survey_invitations(id) ON DELETE SET NULL,

    -- D2 — seconde et dernière FK sortante du module.
    respondent_id    uuid        CONSTRAINT xmod_fk_survey_submissions_respondent
                                 REFERENCES identity.people(id) ON DELETE SET NULL,
    respondent_email platform.email,
    respondent_label text,

    -- D3 — dédoublonnage sans identité : empreinte d'un cookie, d'un jeton
    -- d'invitation ou d'un couple (IP, agent). Permet d'empêcher le double vote
    -- sans jamais savoir qui a voté.
    dedup_hash       bytea,

    -- Copies dénormalisées des drapeaux du sondage. Un CHECK et un index partiel
    -- ne peuvent pas lire une autre table : sans ces copies, l'anonymat et
    -- l'unicité resteraient des règles applicatives, donc contournables. Les deux
    -- colonnes sont renseignées par trigger et rendues immuables.
    is_anonymous     boolean     NOT NULL,
    allows_multiple  boolean     NOT NULL,

    locale           text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    status           tool.submission_status NOT NULL DEFAULT 'in_progress',
    started_at       timestamptz NOT NULL DEFAULT now(),
    submitted_at     timestamptz,
    duration_seconds integer     GENERATED ALWAYS AS (
                                     CASE WHEN submitted_at IS NULL THEN NULL
                                          ELSE GREATEST(0, EXTRACT(EPOCH FROM (submitted_at - started_at))::integer)
                                     END) STORED,

    -- Mode quiz : renseignés par tool.score_submission().
    score            numeric(8,2),
    max_score        numeric(8,2),
    passed           boolean,

    CONSTRAINT ck_survey_submissions_completed
        CHECK (status <> 'completed' OR submitted_at IS NOT NULL),
    -- D3 — LE VERROU D'ANONYMAT. Aucune colonne identifiante n'est tolérée sur
    -- une soumission anonyme, invitation comprise (elle mènerait à la personne).
    CONSTRAINT ck_survey_submissions_anonymity
        CHECK (NOT is_anonymous
               OR (respondent_id IS NULL AND respondent_email IS NULL
                   AND respondent_label IS NULL AND invitation_id IS NULL)),
    CONSTRAINT ck_survey_submissions_dedup
        CHECK (NOT is_anonymous OR allows_multiple OR dedup_hash IS NOT NULL)
);

-- Une seule soumission par répondant quand le sondage l'exige — sur les trois
-- identités possibles : compte, email déclaré, jeton anonyme.
CREATE UNIQUE INDEX ux_survey_submissions_one_per_person
    ON tool.survey_submissions (survey_id, respondent_id)
    WHERE respondent_id IS NOT NULL AND NOT allows_multiple;
CREATE UNIQUE INDEX ux_survey_submissions_one_per_email
    ON tool.survey_submissions (survey_id, respondent_email)
    WHERE respondent_email IS NOT NULL AND NOT allows_multiple;
CREATE UNIQUE INDEX ux_survey_submissions_one_per_token
    ON tool.survey_submissions (survey_id, dedup_hash)
    WHERE dedup_hash IS NOT NULL AND NOT allows_multiple;

CREATE INDEX ix_survey_submissions_survey ON tool.survey_submissions (survey_id, status, submitted_at DESC);
CREATE INDEX ix_survey_submissions_person ON tool.survey_submissions (respondent_id) WHERE respondent_id IS NOT NULL;

-- Recopie des drapeaux et NETTOYAGE ACTIF des colonnes identifiantes. On efface
-- plutôt que l'on rejette : l'anonymat ne doit pas dépendre de la correction du
-- client qui écrit. Le CHECK reste en second rideau pour les écritures directes.
CREATE OR REPLACE FUNCTION tool.tg_apply_survey_privacy()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_survey tool.surveys%ROWTYPE;
BEGIN
    SELECT * INTO v_survey FROM tool.surveys WHERE id = NEW.survey_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Sondage % introuvable', NEW.survey_id USING ERRCODE = 'foreign_key_violation';
    END IF;

    NEW.is_anonymous    := v_survey.is_anonymous;
    NEW.allows_multiple := v_survey.allow_multiple_submissions;
    NEW.locale          := COALESCE(NEW.locale, v_survey.default_locale);

    IF NEW.is_anonymous THEN
        -- L'invitation est marquée comme honorée AVANT d'être détachée : le taux
        -- de réponse reste mesurable, le lien vers la réponse disparaît.
        IF NEW.invitation_id IS NOT NULL THEN
            UPDATE tool.survey_invitations
            SET responded_at = COALESCE(responded_at, now())
            WHERE id = NEW.invitation_id;
        END IF;
        NEW.respondent_id    := NULL;
        NEW.respondent_email := NULL;
        NEW.respondent_label := NULL;
        NEW.invitation_id    := NULL;
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_survey_submissions_privacy
    BEFORE INSERT ON tool.survey_submissions
    FOR EACH ROW EXECUTE FUNCTION tool.tg_apply_survey_privacy();

CREATE TRIGGER tg_survey_submissions_freeze_anonymity
    BEFORE UPDATE ON tool.survey_submissions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_forbid_column_update('is_anonymous');

CREATE TRIGGER tg_survey_submissions_freeze_multiple
    BEFORE UPDATE ON tool.survey_submissions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_forbid_column_update('allows_multiple');

COMMENT ON COLUMN tool.survey_submissions.dedup_hash IS
    'Empreinte de dédoublonnage (cookie, jeton, empreinte de session). Seule protection anti-double-vote d''un sondage anonyme.';
COMMENT ON COLUMN tool.survey_submissions.allows_multiple IS
    'Copie de tool.surveys.allow_multiple_submissions : un index partiel unique ne peut pas interroger une autre table.';

-- -----------------------------------------------------------------------------
-- 6. Réponses
--
-- Une seule colonne `value jsonb` pour tous les types de questions : "Paris"
-- pour un texte, ["a","c"] pour un choix multiple, 4 pour une note, "2026-03-01"
-- pour une date. La v1 multipliait `selected_options text[]`, `text_response`,
-- `answer text[]` selon la table ; chaque nouveau type de question demandait une
-- migration. Ici, le type vit dans la question, pas dans le stockage.
-- -----------------------------------------------------------------------------
CREATE TABLE tool.survey_answers (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    submission_id uuid        NOT NULL REFERENCES tool.survey_submissions(id) ON DELETE CASCADE,
    question_id   uuid        NOT NULL REFERENCES tool.survey_questions(id) ON DELETE CASCADE,
    value         jsonb       NOT NULL,
    -- Projection texte pour la recherche plein texte et l'export tableur.
    answer_text   text        GENERATED ALWAYS AS (tool.jsonb_to_text(value)) STORED,
    is_correct    boolean,
    points_earned numeric(8,2),
    answered_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_survey_answers UNIQUE (submission_id, question_id)
);

CREATE INDEX ix_survey_answers_question ON tool.survey_answers (question_id);
-- jsonb_path_ops : index deux fois plus compact que le GIN par défaut, suffisant
-- pour les opérateurs de confinement (« qui a coché cette option ? »).
CREATE INDEX ix_survey_answers_value ON tool.survey_answers USING gin (value jsonb_path_ops);
CREATE INDEX ix_survey_answers_fulltext
    ON tool.survey_answers USING gin (to_tsvector('french'::regconfig, answer_text))
    WHERE answer_text IS NOT NULL;

COMMENT ON COLUMN tool.survey_answers.value IS
    'Réponse au format uniforme : "texte" | ["a","b"] | 4 | "2026-03-01". Le type est porté par la question, pas par la colonne.';

-- -----------------------------------------------------------------------------
-- 7. Notation des quiz
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION tool.score_submission(p_submission_id uuid)
RETURNS numeric
LANGUAGE plpgsql
AS $$
DECLARE
    v_survey_id uuid;
    v_score     numeric;
    v_max       numeric;
    v_passing   smallint;
BEGIN
    SELECT s.survey_id INTO v_survey_id FROM tool.survey_submissions s WHERE s.id = p_submission_id;
    IF v_survey_id IS NULL THEN
        RETURN NULL;
    END IF;

    -- Comparaison ensembliste — cocher « A puis B » ou « B puis A » est la même
    -- réponse — sauf pour un classement, où l'ordre EST la réponse.
    UPDATE tool.survey_answers a
    SET is_correct = CASE
                       WHEN q.correct_answer IS NULL THEN NULL
                       WHEN q.question_type = 'ranking' THEN a.value = q.correct_answer
                       ELSE tool.jsonb_sorted(a.value) = tool.jsonb_sorted(q.correct_answer)
                     END,
        points_earned = CASE
                       WHEN q.correct_answer IS NULL THEN NULL
                       WHEN q.question_type = 'ranking' AND a.value = q.correct_answer THEN q.points
                       WHEN q.question_type <> 'ranking'
                            AND tool.jsonb_sorted(a.value) = tool.jsonb_sorted(q.correct_answer) THEN q.points
                       ELSE 0
                     END
    FROM tool.survey_questions q
    WHERE q.id = a.question_id AND a.submission_id = p_submission_id;

    SELECT COALESCE(sum(a.points_earned), 0) INTO v_score
    FROM tool.survey_answers a WHERE a.submission_id = p_submission_id;

    SELECT COALESCE(sum(q.points), 0) INTO v_max
    FROM tool.survey_questions q WHERE q.survey_id = v_survey_id;

    SELECT passing_score INTO v_passing FROM tool.surveys WHERE id = v_survey_id;

    UPDATE tool.survey_submissions
    SET score = v_score,
        max_score = v_max,
        passed = CASE WHEN v_passing IS NULL OR v_max = 0 THEN NULL
                      ELSE (100.0 * v_score / v_max) >= v_passing END
    WHERE id = p_submission_id;

    RETURN v_score;
END;
$$;

COMMENT ON FUNCTION tool.score_submission(uuid) IS
    'Corrige et note une soumission de quiz ou d''évaluation, puis évalue le seuil de réussite.';

-- À la clôture d'une soumission : notation automatique et publication d'un
-- événement d'outbox. C'est ce qui permet au module hôte (relance, attestation,
-- statistique) de réagir sans que ce module ne connaisse son existence.
CREATE OR REPLACE FUNCTION tool.tg_on_submission_completed()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_kind tool.survey_kind;
BEGIN
    SELECT kind INTO v_kind FROM tool.surveys WHERE id = NEW.survey_id;

    IF v_kind IN ('quiz', 'evaluation') THEN
        PERFORM tool.score_submission(NEW.id);
    END IF;

    PERFORM platform.emit_event(
        'tool', 'survey_submission', NEW.id, 'tool.survey.submitted',
        jsonb_build_object('survey_id', NEW.survey_id, 'kind', v_kind, 'anonymous', NEW.is_anonymous)
    );
    RETURN NULL;
END;
$$;

-- La notation ne touche pas `status` : pas de récursion possible via la clause WHEN.
CREATE TRIGGER tg_survey_submissions_completed
    AFTER UPDATE OF status ON tool.survey_submissions
    FOR EACH ROW
    WHEN (NEW.status = 'completed' AND OLD.status IS DISTINCT FROM 'completed')
    EXECUTE FUNCTION tool.tg_on_submission_completed();

-- -----------------------------------------------------------------------------
-- 8. Dépouillement
--
-- Agrégation par option, directement consommable par le frontend. Les réponses
-- à choix multiple sont éclatées : une réponse cochant trois options compte pour
-- une voix sur chacune. Seules les soumissions complètes sont dépouillées.
-- Les questions libres se lisent via survey_answers.answer_text.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW tool.survey_question_results AS
WITH exploded AS (
    SELECT q.survey_id,
           q.id            AS question_id,
           q.position,
           q.label,
           q.question_type,
           CASE WHEN jsonb_typeof(a.value) = 'array' THEN e.item ELSE a.value END AS choice
    FROM tool.survey_answers a
    JOIN tool.survey_questions q   ON q.id = a.question_id
    JOIN tool.survey_submissions s ON s.id = a.submission_id
    LEFT JOIN LATERAL jsonb_array_elements(
        CASE WHEN jsonb_typeof(a.value) = 'array' THEN a.value ELSE '[]'::jsonb END
    ) AS e(item) ON true
    WHERE s.status = 'completed'
      AND q.question_type IN ('single_choice', 'multiple_choice', 'yes_no', 'rating', 'scale', 'ranking')
)
SELECT survey_id,
       question_id,
       position,
       label,
       question_type,
       choice #>> '{}' AS option_value,
       count(*)        AS response_count,
       round((100.0 * count(*) / NULLIF(sum(count(*)) OVER (PARTITION BY question_id), 0))::numeric, 2) AS percentage
FROM exploded
WHERE choice IS NOT NULL
GROUP BY survey_id, question_id, position, label, question_type, choice #>> '{}';

COMMENT ON VIEW tool.survey_question_results IS
    'Dépouillement par question et par option (comptage et pourcentage) sur les soumissions complètes.';

-- Résultat complet d'un sondage en un seul appel et un seul objet JSON, libellés
-- déjà traduits : le frontend affiche, il ne recompose pas.
CREATE OR REPLACE FUNCTION tool.survey_results(p_survey_id uuid, p_locale text DEFAULT 'fr')
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    SELECT jsonb_build_object(
        'survey_id',   p_survey_id,
        'completed',   (SELECT count(*) FROM tool.survey_submissions s
                        WHERE s.survey_id = p_survey_id AND s.status = 'completed'),
        'in_progress', (SELECT count(*) FROM tool.survey_submissions s
                        WHERE s.survey_id = p_survey_id AND s.status = 'in_progress'),
        'questions',   COALESCE((
            SELECT jsonb_agg(
                jsonb_build_object(
                    'question_id', q.id,
                    'position',    q.position,
                    'type',        q.question_type,
                    'label',       platform.t(q.label, p_locale),
                    'answers',     (SELECT count(*) FROM tool.survey_answers a WHERE a.question_id = q.id),
                    'options',     COALESCE((
                        SELECT jsonb_agg(jsonb_build_object(
                                   'value',   r.option_value,
                                   'count',   r.response_count,
                                   'percent', r.percentage)
                               ORDER BY r.response_count DESC)
                        FROM tool.survey_question_results r WHERE r.question_id = q.id
                    ), '[]'::jsonb)
                ) ORDER BY q.position)
            FROM tool.survey_questions q WHERE q.survey_id = p_survey_id
        ), '[]'::jsonb)
    );
$$;

COMMENT ON FUNCTION tool.survey_results(uuid, text) IS
    'Résultats agrégés d''un sondage en un objet JSON localisé, directement exploitable par le frontend.';

-- =============================================================================
-- PARTIE B — ASSISTANTS IA ET RAG
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 9. Assistants
--
-- Un assistant est une CONFIGURATION, pas du code. Le cadrage en cite deux :
-- l'assistant de négociation (RAG sur les documents d'aide, réponses sourcées)
-- et l'agent d'automatisation interne (« créer des réunions et plein d'autres
-- choses »). Ils ne diffèrent que par leur prompt, leur portée documentaire et
-- les outils qu'on leur autorise. En ajouter un troisième est un INSERT.
-- -----------------------------------------------------------------------------
CREATE TABLE tool.assistants (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    code              text        NOT NULL UNIQUE CHECK (code ~ '^[a-z][a-z0-9_]{2,48}$'),
    name              platform.i18n_text NOT NULL,
    description       platform.i18n_text,

    provider          text        NOT NULL DEFAULT 'anthropic',
    model             text        NOT NULL,
    temperature       numeric(3,2) NOT NULL DEFAULT 0.20 CHECK (temperature BETWEEN 0 AND 2),
    max_output_tokens integer     NOT NULL DEFAULT 1024 CHECK (max_output_tokens > 0),
    system_prompt     text        NOT NULL,

    -- Portée documentaire : codes de périmètre autorisés, confrontés à
    -- knowledge_sources.scope_code. Un tableau vide signifie « tout le corpus ».
    scope_codes       text[]      NOT NULL DEFAULT '{}',
    -- Outils exposés à l'agent (ex. 'create_meeting', 'list_sessions'). Une
    -- liste blanche : ce qui n'est pas listé n'est pas appelable.
    allowed_tools     text[]      NOT NULL DEFAULT '{}',

    is_active         boolean     NOT NULL DEFAULT true,
    requires_authentication boolean NOT NULL DEFAULT true,
    created_by        uuid        CONSTRAINT xmod_fk_assistants_author
                                  REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now()
);

CREATE INDEX ix_assistants_scope ON tool.assistants USING gin (scope_codes);

CREATE TRIGGER tg_assistants_updated_at
    BEFORE UPDATE ON tool.assistants
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_assistants_audit
    AFTER INSERT OR UPDATE OR DELETE ON tool.assistants
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON COLUMN tool.assistants.system_prompt IS
    'Consigne système versionnée en base : modifier le comportement d''un assistant ne demande aucun redéploiement.';
COMMENT ON COLUMN tool.assistants.allowed_tools IS
    'Liste blanche des actions appelables par l''agent (création de réunion, etc.). Tout ce qui n''y figure pas est refusé.';

-- -----------------------------------------------------------------------------
-- 10. Corpus indexé
-- -----------------------------------------------------------------------------
CREATE TYPE tool.indexing_status AS ENUM ('pending', 'indexing', 'indexed', 'stale', 'failed');

CREATE TABLE tool.knowledge_sources (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    scope_code      text        NOT NULL DEFAULT 'general',
    title           text        NOT NULL,
    -- D1 — rattachement libre : ('negotiation','documents',<uuid>,'Projet de
    -- décision CMA.7'). Pas de FK : le corpus survit à l'extraction du module,
    -- et une source peut aussi être une page web sans équivalent en base.
    context_schema  text,
    context_table   text,
    context_id      uuid,
    context_label   text,
    source_url      platform.url,
    asset_id        uuid,       -- media.assets.id, volontairement sans clé étrangère
    locale          text        REFERENCES reference.locales(code),
    -- Empreinte du contenu : ré-indexer coûte des appels d'API facturés. Si
    -- l'empreinte n'a pas bougé, on ne recalcule rien.
    content_hash    bytea,
    chunk_count     integer     NOT NULL DEFAULT 0 CHECK (chunk_count >= 0),
    status          tool.indexing_status NOT NULL DEFAULT 'pending',
    last_indexed_at timestamptz,
    last_error      text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_knowledge_sources_context
        CHECK (tool.is_context_consistent(context_schema, context_table, context_id)),
    CONSTRAINT ck_knowledge_sources_origin
        CHECK (source_url IS NOT NULL OR asset_id IS NOT NULL OR context_id IS NOT NULL)
);

CREATE UNIQUE INDEX ux_knowledge_sources_context
    ON tool.knowledge_sources (context_schema, context_table, context_id)
    WHERE context_id IS NOT NULL;
CREATE INDEX ix_knowledge_sources_scope   ON tool.knowledge_sources (scope_code, status);
CREATE INDEX ix_knowledge_sources_pending ON tool.knowledge_sources (updated_at)
    WHERE status IN ('pending', 'stale', 'failed');

CREATE TRIGGER tg_knowledge_sources_updated_at
    BEFORE UPDATE ON tool.knowledge_sources
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON COLUMN tool.knowledge_sources.scope_code IS
    'Périmètre documentaire (ex. negotiation_climate). Confronté à tool.assistants.scope_codes : un assistant ne cite que son corpus.';

CREATE TABLE tool.knowledge_chunks (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    source_id   uuid        NOT NULL REFERENCES tool.knowledge_sources(id) ON DELETE CASCADE,
    position    integer     NOT NULL CHECK (position >= 0),
    content     text        NOT NULL,
    -- {"page": 5, "section": "Article 6.4", "start_char": 0, "end_char": 1200}
    metadata    jsonb       NOT NULL DEFAULT '{}'::jsonb,
    embedding   vector(1536),
    token_count integer     CHECK (token_count IS NULL OR token_count > 0),
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_knowledge_chunks_position UNIQUE (source_id, position)
);

-- CHOIX D'INDEX VECTORIEL : HNSW PLUTÔT QU'IVFFLAT
-- La v1 utilisait `ivfflat (lists = 100)`. IVFFlat partitionne l'espace par
-- k-means : il doit être CONSTRUIT SUR UN CORPUS DÉJÀ REPRÉSENTATIF, se dégrade
-- silencieusement à mesure que les documents s'ajoutent — le régime exact d'un
-- corpus qui grossit à chaque COP — et créé sur une table quasi vide, comme à
-- l'installation, il est inutilisable. HNSW est un graphe navigable construit
-- incrémentalement : aucune phase d'entraînement, aucun paramètre à réviser
-- après chargement, et un rappel supérieur à volumétrie modeste et croissante.
-- Coût : plus de mémoire et une construction plus lente — sans objet ici.
-- Réglage de lecture : `SET hnsw.ef_search = 100` pour privilégier le rappel.
CREATE INDEX ix_knowledge_chunks_embedding
    ON tool.knowledge_chunks USING hnsw (embedding vector_cosine_ops)
    WITH (m = 16, ef_construction = 64);

CREATE INDEX ix_knowledge_chunks_source   ON tool.knowledge_chunks (source_id, position);
CREATE INDEX ix_knowledge_chunks_metadata ON tool.knowledge_chunks USING gin (metadata jsonb_path_ops);
-- Recherche lexicale, complément indispensable du vectoriel : un embedding
-- retrouve mal un sigle rare ou une cote de document (« FCCC/CP/2025/L.4 »).
CREATE INDEX ix_knowledge_chunks_fulltext
    ON tool.knowledge_chunks USING gin (to_tsvector('french'::regconfig, content));

COMMENT ON TABLE tool.knowledge_chunks IS
    'Fragments indexés du corpus, avec leur embedding (1536 dimensions). Index HNSW pour la recherche par similarité cosinus.';

-- -----------------------------------------------------------------------------
-- 11. Conversations
-- -----------------------------------------------------------------------------
CREATE TYPE tool.message_role AS ENUM ('user', 'assistant', 'system', 'tool');

CREATE TABLE tool.conversations (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    assistant_id        uuid        NOT NULL REFERENCES tool.assistants(id) ON DELETE CASCADE,
    person_id           uuid        CONSTRAINT xmod_fk_conversations_person
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    -- Visiteur non authentifié : empreinte de session, jamais d'identité.
    visitor_hash        bytea,
    title               text,
    locale              text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    context_schema      text,
    context_table       text,
    context_id          uuid,
    context_label       text,
    -- Compteurs entretenus par trigger : le tableau de bord des coûts ne doit
    -- pas rebalayer la table des messages à chaque affichage.
    message_count       integer     NOT NULL DEFAULT 0,
    total_input_tokens  bigint      NOT NULL DEFAULT 0,
    total_output_tokens bigint      NOT NULL DEFAULT 0,
    total_cost          numeric(12,6) NOT NULL DEFAULT 0,
    is_archived         boolean     NOT NULL DEFAULT false,
    last_message_at     timestamptz,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_conversations_owner CHECK (person_id IS NOT NULL OR visitor_hash IS NOT NULL),
    CONSTRAINT ck_conversations_context
        CHECK (tool.is_context_consistent(context_schema, context_table, context_id))
);

CREATE INDEX ix_conversations_person    ON tool.conversations (person_id, last_message_at DESC) WHERE person_id IS NOT NULL;
CREATE INDEX ix_conversations_assistant ON tool.conversations (assistant_id, created_at DESC);

CREATE TRIGGER tg_conversations_updated_at
    BEFORE UPDATE ON tool.conversations
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TABLE tool.conversation_messages (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    conversation_id uuid        NOT NULL REFERENCES tool.conversations(id) ON DELETE CASCADE,
    position        integer     NOT NULL,
    role            tool.message_role NOT NULL,
    content         text        NOT NULL,
    -- Fragments cités : [{"chunk_id": "...", "source_id": "...", "title": "...",
    -- "similarity": 0.87}]. Copie figée, et non jointure : une réponse doit
    -- rester justifiable même si la source est ré-indexée ou retirée du corpus.
    citations       jsonb       NOT NULL DEFAULT '[]'::jsonb,
    tool_name       text,
    tool_payload    jsonb,
    model           text,
    input_tokens    integer,
    output_tokens   integer,
    cost            numeric(12,6),
    latency_ms      integer,
    finish_reason   text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_conversation_messages_position UNIQUE (conversation_id, position),
    CONSTRAINT ck_conversation_messages_tool CHECK (role <> 'tool' OR tool_name IS NOT NULL)
);

CREATE INDEX ix_conversation_messages_conversation ON tool.conversation_messages (conversation_id, position);
CREATE INDEX ix_conversation_messages_citations    ON tool.conversation_messages USING gin (citations jsonb_path_ops);
CREATE INDEX ix_conversation_messages_cost         ON tool.conversation_messages (created_at DESC) WHERE cost IS NOT NULL;

-- Numérotation automatique et mise à jour des compteurs de la conversation.
CREATE OR REPLACE FUNCTION tool.tg_number_message()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.position IS NULL THEN
        SELECT COALESCE(max(position), -1) + 1 INTO NEW.position
        FROM tool.conversation_messages WHERE conversation_id = NEW.conversation_id;
    END IF;
    RETURN NEW;
END;
$$;

CREATE OR REPLACE FUNCTION tool.tg_accumulate_usage()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE tool.conversations
    SET message_count       = message_count + 1,
        total_input_tokens  = total_input_tokens + COALESCE(NEW.input_tokens, 0),
        total_output_tokens = total_output_tokens + COALESCE(NEW.output_tokens, 0),
        total_cost          = total_cost + COALESCE(NEW.cost, 0),
        last_message_at     = NEW.created_at
    WHERE id = NEW.conversation_id;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_conversation_messages_number
    BEFORE INSERT ON tool.conversation_messages
    FOR EACH ROW EXECUTE FUNCTION tool.tg_number_message();

CREATE TRIGGER tg_conversation_messages_usage
    AFTER INSERT ON tool.conversation_messages
    FOR EACH ROW EXECUTE FUNCTION tool.tg_accumulate_usage();

-- -----------------------------------------------------------------------------
-- 12. Retours utilisateurs
--
-- La boucle d'amélioration. Sans elle, la qualité d'un assistant RAG relève de
-- l'impression : on ignore quelles questions produisent des réponses fausses,
-- donc quels documents manquent au corpus ou quel prompt est à revoir. Le motif
-- codifié est ce qui rend les retours dénombrables — un champ libre seul ne se
-- dépouille pas.
-- -----------------------------------------------------------------------------
CREATE TYPE tool.feedback_rating AS ENUM ('up', 'down');

CREATE TABLE tool.message_feedback (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    message_id  uuid        NOT NULL REFERENCES tool.conversation_messages(id) ON DELETE CASCADE,
    person_id   uuid        CONSTRAINT xmod_fk_message_feedback_person
                            REFERENCES identity.people(id) ON DELETE SET NULL,
    visitor_hash bytea,
    rating      tool.feedback_rating NOT NULL,
    reason_code text        CHECK (reason_code IN ('inaccurate', 'incomplete', 'off_topic',
                                                   'outdated', 'unclear', 'harmful', 'other')),
    comment     text,
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX ux_message_feedback_person
    ON tool.message_feedback (message_id, person_id) WHERE person_id IS NOT NULL;
CREATE INDEX ix_message_feedback_negative
    ON tool.message_feedback (created_at DESC) WHERE rating = 'down';

COMMENT ON TABLE tool.message_feedback IS
    'Retours pouce haut / pouce bas sur les réponses. Source de la revue qualité : les motifs négatifs désignent les lacunes du corpus.';

-- -----------------------------------------------------------------------------
-- 13. Quotas d'usage
--
-- GARDE-FOU DE COÛT, PAS CONFORT D'ADMINISTRATION. Un assistant IA exposé
-- publiquement sans plafond est une facture non bornée : chaque message part
-- chez un fournisseur qui facture au jeton, et rien n'empêche une boucle mal
-- écrite — ou un robot — d'en émettre des milliers en une nuit. Le plafond
-- s'applique par personne et par jour, avec une ligne par défaut (assistant NUL,
-- personne NULLE) qui couvre tous les cas non prévus. La règle la plus
-- spécifique l'emporte.
-- -----------------------------------------------------------------------------
CREATE TABLE tool.usage_quotas (
    id                   uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    assistant_id         uuid        REFERENCES tool.assistants(id) ON DELETE CASCADE,
    person_id            uuid        CONSTRAINT xmod_fk_usage_quotas_person
                                     REFERENCES identity.people(id) ON DELETE CASCADE,
    max_messages_per_day integer     NOT NULL DEFAULT 50  CHECK (max_messages_per_day >= 0),
    max_tokens_per_day   integer     NOT NULL DEFAULT 200000 CHECK (max_tokens_per_day >= 0),
    max_cost_per_day     numeric(8,4),
    note                 text,
    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now()
);

-- Une seule règle par couple (assistant, personne), la ligne par défaut incluse.
CREATE UNIQUE INDEX ux_usage_quotas_target ON tool.usage_quotas (
    COALESCE(assistant_id, '00000000-0000-0000-0000-000000000000'::uuid),
    COALESCE(person_id,    '00000000-0000-0000-0000-000000000000'::uuid)
);

CREATE TRIGGER tg_usage_quotas_updated_at
    BEFORE UPDATE ON tool.usage_quotas
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Consommation du jour confrontée au plafond applicable. À appeler AVANT tout
-- appel au fournisseur, pas après.
CREATE OR REPLACE FUNCTION tool.check_usage_quota(p_assistant_id uuid, p_person_id uuid)
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    WITH quota AS (
        SELECT max_messages_per_day, max_tokens_per_day, max_cost_per_day
        FROM tool.usage_quotas
        WHERE (assistant_id = p_assistant_id OR assistant_id IS NULL)
          AND (person_id = p_person_id OR person_id IS NULL)
        ORDER BY (assistant_id IS NOT NULL)::int + (person_id IS NOT NULL)::int DESC
        LIMIT 1
    ),
    used AS (
        SELECT count(*) FILTER (WHERE m.role = 'user')                    AS messages,
               COALESCE(sum(COALESCE(m.input_tokens, 0) + COALESCE(m.output_tokens, 0)), 0) AS tokens,
               COALESCE(sum(m.cost), 0)                                   AS cost
        FROM tool.conversation_messages m
        JOIN tool.conversations c ON c.id = m.conversation_id
        WHERE c.assistant_id = p_assistant_id
          AND c.person_id IS NOT DISTINCT FROM p_person_id
          AND m.created_at >= date_trunc('day', now())
    )
    SELECT jsonb_build_object(
        'allowed', q.max_messages_per_day IS NULL
                   OR (u.messages < q.max_messages_per_day
                       AND u.tokens < q.max_tokens_per_day
                       AND (q.max_cost_per_day IS NULL OR u.cost < q.max_cost_per_day)),
        'messages_used',      u.messages,
        'messages_limit',     q.max_messages_per_day,
        'tokens_used',        u.tokens,
        'tokens_limit',       q.max_tokens_per_day,
        'cost_used',          u.cost,
        'cost_limit',         q.max_cost_per_day,
        'resets_at',          date_trunc('day', now()) + interval '1 day'
    )
    FROM used u LEFT JOIN quota q ON true;
$$;

COMMENT ON FUNCTION tool.check_usage_quota(uuid, uuid) IS
    'État du quota journalier d''une personne pour un assistant. À évaluer avant l''appel au fournisseur de modèle.';

-- -----------------------------------------------------------------------------
-- 14. Recherche sémantique
--
-- Cœur du RAG : les fragments les plus proches de la question, restreints au
-- corpus autorisé de l'assistant. L'ORDER BY porte directement sur l'opérateur
-- `<=>`, seule forme que l'index HNSW sait servir.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION tool.search_chunks(
    p_assistant_code text,
    p_embedding      vector(1536),
    p_limit          int DEFAULT 8
)
RETURNS TABLE (
    chunk_id      uuid,
    source_id     uuid,
    source_title  text,
    content       text,
    metadata      jsonb,
    context_label text,
    similarity    real
)
LANGUAGE sql
STABLE
AS $$
    SELECT c.id,
           s.id,
           s.title,
           c.content,
           c.metadata,
           s.context_label,
           (1 - (c.embedding <=> p_embedding))::real
    FROM tool.knowledge_chunks c
    JOIN tool.knowledge_sources s ON s.id = c.source_id
    JOIN tool.assistants a        ON a.code = p_assistant_code
    WHERE a.is_active
      AND s.status = 'indexed'
      AND c.embedding IS NOT NULL
      AND (a.scope_codes = '{}'::text[] OR s.scope_code = ANY (a.scope_codes))
    ORDER BY c.embedding <=> p_embedding
    LIMIT GREATEST(COALESCE(p_limit, 8), 1);
$$;

COMMENT ON FUNCTION tool.search_chunks(text, vector, int) IS
    'Fragments les plus proches d''un embedding, filtrés sur la portée documentaire de l''assistant (similarité cosinus, index HNSW).';

-- -----------------------------------------------------------------------------
-- 15. Amorçage
--
-- Les deux assistants cités au cadrage, et le quota par défaut. Le passage en
-- service autonome se résume ensuite à :
--   UPDATE platform.modules
--   SET deployment = 'external', base_url = 'https://outils.ifdd.francophonie.org'
--   WHERE code = 'tool';
-- puis à exécuter platform.generate_module_decoupling_script('tool').
-- -----------------------------------------------------------------------------
INSERT INTO tool.assistants (code, name, description, model, temperature, system_prompt, scope_codes, allowed_tools) VALUES
    ('negotiation_helper',
     '{"fr":"Assistant de négociation","en":"Negotiation assistant"}',
     '{"fr":"Répond aux questions des négociateurs francophones en citant les documents d''aide.","en":"Answers francophone negotiators using the support documents."}',
     'claude-sonnet-4-5', 0.10,
     'Tu assistes des négociateurs francophones. Réponds uniquement à partir des extraits fournis, cite systématiquement tes sources, et indique explicitement lorsque l''information ne figure pas dans le corpus.',
     '{negotiation_climate,negotiation_biodiversity,negotiation_desertification}', '{}'),
    ('operations_agent',
     '{"fr":"Agent d''automatisation","en":"Automation agent"}',
     '{"fr":"Agent interne : création de réunions et tâches d''administration courante.","en":"Internal agent: meeting creation and routine administration."}',
     'claude-sonnet-4-5', 0.00,
     'Tu es un agent d''automatisation interne de l''IFDD. Tu n''exécutes que les actions explicitement autorisées et tu confirmes chaque action avant de l''exécuter.',
     '{internal_operations}', '{create_meeting,update_meeting,list_sessions}')
ON CONFLICT (code) DO NOTHING;

INSERT INTO tool.usage_quotas (assistant_id, person_id, max_messages_per_day, max_tokens_per_day, max_cost_per_day, note)
VALUES (NULL, NULL, 50, 200000, 5.0000, 'Plafond par défaut appliqué à toute personne et tout assistant sans règle plus spécifique.')
ON CONFLICT DO NOTHING;
