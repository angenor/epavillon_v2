-- =============================================================================
-- ePavillon v2 — 125_training.sql
-- Module Formations : formations en ligne / présentiel / hybride, chapitres,
-- ressources, quiz de fin de chapitre, évaluation finale, progression des
-- apprenants et attestations vérifiables.
--
-- Dépend de : 000, 010, 020, 030, 040, 050, 060, 080
-- (060 pour event.events, 080 pour live.meetings et live.streams)
--
-- CITATION DU CADRAGE, mot pour mot :
--   « Concernant les formations, en général on le fait en ligne, en présentiel
--     ou en hybride. Lorsque la formation est en ligne ou hybride via zoom, on
--     met les enregistrements dans un chapitre ainsi que les fichiers de
--     présentation, exercices, annexe etc. Chaque chapitre peut se terminer par
--     un quiz QCM/QCD ou pas. Une formation peut se terminer par une évaluation
--     QCM/QCD ou pas. Le module formation est important, il doit être construit
--     au MVP. »
--
-- -----------------------------------------------------------------------------
-- DÉCISION STRUCTURANTE N°1 — POURQUOI CE MODULE NE RÉUTILISE PAS tool.surveys
--
-- Le module `tool` porte déjà `tool.surveys` avec un mode `quiz` : questions,
-- points, bonne réponse, seuil de réussite. La tentation de tout y ramener est
-- réelle, et ce serait une erreur d'architecture.
--
-- `tool` est construit pour être EXTRAIT en service autonome — c'est écrit en
-- toutes lettres dans son en-tête : zéro clé étrangère sortante vers les modules
-- métier, rattachement par triplet libre (context_schema, context_table,
-- context_id) sans contrainte, libellé dénormalisé pour rester lisible une fois
-- débranché. Ce contrat n'est tenable que parce qu'un sondage est jetable : s'il
-- part sur un autre domaine, la plateforme continue de fonctionner sans lui.
--
-- Un quiz de formation n'a pas ce statut. Il est INDISSOCIABLE du parcours :
--   - sa réussite achève le chapitre et fait avancer la progression ;
--   - sa note conditionne l'obtention de l'attestation, document opposable ;
--   - il est nominatif par nature — on certifie une personne, pas un anonyme.
-- Faire dépendre les formations de `tool`, c'est accepter que le jour où
-- l'outil sondage déménage, la progression des apprenants et la délivrance des
-- attestations traversent le réseau, avec une intégrité référentielle rompue :
-- plus de FK entre une réponse et l'inscription qu'elle note.
--
-- Les deux sous-systèmes se ressemblent en surface mais ne servent pas le même
-- contrat :
--       tool.surveys           |  training.quizzes
--       anonyme et détachable  |  nominatif, noté, couplé au parcours
--       aucune FK métier       |  FK dures vers enrollments et chapters
--       résultat = statistique |  résultat = note, attestation, preuve
-- Une ressemblance de forme ne justifie pas un couplage. C'est le prix — assumé
-- et faible — d'une duplication de STRUCTURE pour éviter une dépendance de
-- CYCLE DE VIE.
--
-- -----------------------------------------------------------------------------
-- CE QUE LA V1 FAISAIT ET QUI EST CORRIGÉ ICI
--
-- D1. TROIS SYSTÈMES DE QUESTIONNAIRES CONCURRENTS
--     La v1 posait `live_quizzes` + `quiz_questions` + `quiz_responses` +
--     `quiz_results`, PUIS `evaluations` + `evaluation_questions` +
--     `evaluation_answers` + `evaluation_results`, PUIS `polls` : trois modèles,
--     trois tables de réponses, trois tables de résultats, trois écrans
--     d'administration et trois jeux de bugs pour une seule et même chose —
--     poser des questions, collecter, corriger, compter.
--     En v2, UN SEUL modèle question/réponse dans `training`, utilisé aussi bien
--     par le quiz de fin de chapitre que par l'évaluation finale. La différence
--     entre les deux est le RATTACHEMENT (chapitre ou formation), pas la
--     structure : `training.quizzes` porte un XOR strict entre `chapter_id` et
--     `training_id`. Ajouter demain un quiz de positionnement en entrée de
--     formation ne demandera ni table ni migration.
--
-- D2. DEUX NIVEAUX DE GRANULARITÉ, DEUX TABLES DE PROGRESSION
--     `training_chapters` + `lesson_contents` d'un côté,
--     `participant_chapter_progress` + `participant_lesson_progress` de l'autre.
--     Un chapitre contenant des leçons contenant des fichiers, pour un besoin
--     qui tient en une phrase : « on met les enregistrements dans un chapitre
--     ainsi que les fichiers de présentation, exercices, annexe ».
--     En v2 : chapitre -> ressources, point. Et UNE SEULE table de progression,
--     par (inscription, chapitre). La granularité ressource n'est pas perdue
--     pour autant — voir training.chapter_progress.viewed_resource_ids et sa
--     justification : elle vit dans la ligne du chapitre, pas dans une seconde
--     table dont chaque clic aurait été une écriture.
--
-- D3. AUCUN LIEN ENTRE UNE FORMATION EN LIGNE ET SA SÉANCE ZOOM
--     La v1 ignorait que les formations en ligne se tiennent sur Zoom : ni
--     réunion, ni enregistrement, seulement un `youtube_url` en texte libre au
--     niveau de la leçon. En v2, un chapitre porte la référence de sa séance
--     (`live.meetings`) et de son enregistrement (`live.streams` pour un replay
--     diffusé, `media.assets` pour un fichier archivé) — les deux chemins
--     existent parce que les deux pratiques existent.
--
-- D4. `objectives TEXT[] NOT NULL`
--     Un tableau de texte brut : non traduit (donc invisible pour la moitié
--     anglophone du public), non ordonnable autrement que par l'ordre physique
--     du tableau, et impossible à corriger sans réécrire l'ensemble. Remplacé
--     par `platform.i18n_text` : l'ordre est porté par le texte rédigé, la
--     traduction est native, et aucune table supplémentaire n'est créée pour une
--     donnée que personne ne manipule objectif par objectif (YAGNI).
--
-- D5. DES CERTIFICATS SANS RÈGLE D'OBTENTION
--     `training_participants.certificate_url TEXT` : une URL, posée à la main,
--     sans condition, sans numéro, sans vérifiabilité, sans révocation
--     possible. En v2, les conditions sont explicites et portées par la
--     formation (`min_completion_percent`, `min_final_score_percent`),
--     l'éligibilité est une fonction (`training.is_eligible_for_certificate`),
--     l'émission la contrôle (`training.issue_certificate`), et l'attestation
--     porte un numéro unique et un code de vérification publiable.
--
-- -----------------------------------------------------------------------------
-- NOTE DE VOCABULAIRE — « QCM / QCD »
-- Le cadrage parle de « quiz QCM/QCD ». « QCM » est sans ambiguïté (question à
-- choix multiples). « QCD » n'est pas un sigle standard ; il est traité ici comme
-- l'opposition classique question à CHOIX UNIQUE (`single_choice`) / question à
-- CHOIX MULTIPLES (`multiple_choice`). LE LIBELLÉ EXACT RESTE À CONFIRMER AUPRÈS
-- DE L'IFDD : si « QCD » désigne autre chose (question à choix dichotomique,
-- c'est-à-dire vrai/faux — déjà couvert par `true_false`), aucune migration ne
-- sera nécessaire, seul le libellé d'interface changera.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 0. Schéma et enregistrement du module
--
-- Le schéma n'était pas déclaré dans 000_bootstrap.sql : il est créé ici, de
-- façon idempotente, avec la même sémantique que les autres (un schéma = un
-- module = une frontière de service potentielle).
-- -----------------------------------------------------------------------------
CREATE SCHEMA IF NOT EXISTS training;

COMMENT ON SCHEMA training IS
    'Module Formations : catalogue, chapitres et ressources, quiz et évaluation finale, progression et attestations.';

INSERT INTO platform.modules (code, schema_name, display_name, depends_on) VALUES
    ('training', 'training', '{"fr":"Formations","en":"Training"}', '{identity,org,media,live,event}')
ON CONFLICT (code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 1. Types du module
--
-- Rappel de la règle : les ENUM sont réservés aux MACHINES À ÉTATS et aux
-- vocabulaires fermés dont dépend du code. Tout ce qui est ouvert — thématiques,
-- catégories, niveaux — passe par reference.taxonomy_terms + entity_terms.
-- La `training_category` de la v1 ('climate', 'desertification', 'biodiversity',
-- 'other') était précisément un vocabulaire ouvert enfermé dans un ENUM : elle
-- devient un rattachement à la taxonomie `activity_theme` (ou `negotiation_track`
-- pour la convention de référence), sans DDL pour en ajouter une nouvelle.
-- -----------------------------------------------------------------------------

-- Modalité de déroulement. Type propre au module et NON `event.participation_mode` :
-- reprendre le type d'un autre schéma métier créerait une dépendance de plus à
-- couper le jour d'une extraction, pour économiser trois lignes de DDL.
CREATE TYPE training.delivery_mode AS ENUM ('online', 'in_person', 'hybrid');

CREATE TYPE training.training_status AS ENUM (
    'draft',      -- en cours de conception, invisible
    'published',  -- annoncée au catalogue, inscriptions pas encore ouvertes
    'open',       -- inscriptions ouvertes
    'running',    -- en cours de déroulement
    'completed',  -- terminée, contenus consultables
    'archived'    -- retirée du catalogue, conservée pour l'historique
);

CREATE TYPE training.trainer_role AS ENUM (
    'lead',        -- formateur principal (un seul par formation)
    'co_trainer',  -- co-formateur
    'assistant'    -- assistant pédagogique, appui technique
);

CREATE TYPE training.enrollment_status AS ENUM (
    'pending',     -- demande enregistrée, en attente de validation
    'confirmed',   -- inscription acquise
    'waitlisted',  -- liste d'attente (jauge atteinte)
    'cancelled',   -- annulée par l'apprenant ou l'IFDD
    'completed',   -- parcours achevé et conditions remplies
    'failed'       -- parcours achevé sans atteindre le seuil requis
);

CREATE TYPE training.progress_state AS ENUM ('not_started', 'in_progress', 'completed');

CREATE TYPE training.resource_kind AS ENUM (
    'recording',  -- enregistrement de la séance (Zoom, replay)
    'slides',     -- support de présentation
    'exercise',   -- exercice, travail dirigé
    'annex',      -- annexe, document complémentaire
    'reading',    -- lecture obligatoire ou conseillée
    'link',       -- ressource externe
    'video'       -- vidéo pédagogique autre qu'un enregistrement de séance
);

-- Voir la NOTE DE VOCABULAIRE en en-tête pour « QCM / QCD ».
CREATE TYPE training.question_type AS ENUM (
    'single_choice',    -- une seule bonne réponse à cocher
    'multiple_choice',  -- plusieurs bonnes réponses à cocher
    'true_false',       -- vrai / faux (cas particulier du choix unique à 2 options)
    'open_text'         -- réponse rédigée : correction manuelle
);

CREATE TYPE training.attempt_status AS ENUM (
    'in_progress',  -- tentative ouverte, réponses modifiables
    'submitted',    -- remise, en attente de correction (questions ouvertes)
    'graded',       -- corrigée et notée
    'abandoned'     -- non remise dans le temps imparti
);

-- Quand l'apprenant voit-il le corrigé ? Afficher la correction entre deux
-- tentatives revient à donner les réponses ; ne jamais l'afficher supprime la
-- valeur pédagogique du quiz. Le choix est donc une décision de l'auteur.
CREATE TYPE training.correction_visibility AS ENUM (
    'never',
    'after_attempt',        -- après chaque tentative
    'after_final_attempt',  -- une fois les tentatives épuisées ou le quiz réussi
    'after_training_end'    -- à la clôture de la formation
);

-- -----------------------------------------------------------------------------
-- 2. Fonctions utilitaires du module
-- -----------------------------------------------------------------------------

-- Validation des langues d'une formation. Un tableau ne peut pas porter de clé
-- étrangère ; on valide donc par fonction, comme le fait déjà
-- `platform.timezone_name` avec `platform.is_valid_timezone()` (000_bootstrap).
CREATE OR REPLACE FUNCTION training.are_known_locales(p_codes text[])
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT p_codes IS NULL
        OR (cardinality(p_codes) > 0
            AND NOT EXISTS (
                SELECT 1 FROM unnest(p_codes) AS c(code)
                WHERE NOT EXISTS (SELECT 1 FROM reference.locales l WHERE l.code = c.code)
            ));
$$;

COMMENT ON FUNCTION training.are_known_locales(text[]) IS
    'Vérifie que chaque code de langue existe dans reference.locales. Supplée l''absence de FK sur un tableau.';

-- Comparaison ensembliste d'identifiants : cocher « A puis B » ou « B puis A »
-- est la même réponse. Trier avant de comparer évite d'écrire la correction
-- comme une double inclusion illisible.
CREATE OR REPLACE FUNCTION training.sorted_uuids(p_ids uuid[])
RETURNS uuid[]
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT COALESCE(ARRAY(SELECT DISTINCT u FROM unnest(COALESCE(p_ids, '{}'::uuid[])) AS t(u) ORDER BY u), '{}'::uuid[]);
$$;

-- -----------------------------------------------------------------------------
-- 3. Formations
-- -----------------------------------------------------------------------------
CREATE TABLE training.trainings (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    slug                platform.slug NOT NULL,   -- segment d'URL stable : /formations/{slug}
    title               platform.i18n_text NOT NULL,
    summary             platform.i18n_text,       -- chapeau du catalogue et meta description
    description         platform.i18n_text NOT NULL,

    -- D4 — remplace `objectives TEXT[] NOT NULL` : traduit, ordonné par la
    -- rédaction, modifiable sans réécrire un tableau entier.
    objectives          platform.i18n_text,
    methodology         platform.i18n_text,
    target_audience     platform.i18n_text,

    format              training.delivery_mode NOT NULL,
    status              training.training_status NOT NULL DEFAULT 'draft',

    -- Organisation organisatrice. NULL = formation portée directement par l'IFDD
    -- sans co-organisateur déclaré.
    organizer_organization_id uuid  CONSTRAINT xmod_fk_trainings_organizer
                                    REFERENCES org.organizations(id) ON DELETE SET NULL,
    -- Formateur principal : projection maintenue par trigger depuis
    -- training.trainers (rôle 'lead'). Dénormalisé pour que la page catalogue
    -- affiche un nom sans jointure supplémentaire.
    lead_trainer_id     uuid        CONSTRAINT xmod_fk_trainings_lead_trainer
                                    REFERENCES identity.people(id) ON DELETE SET NULL,

    -- Rattachement facultatif à une édition d'événement : une formation peut
    -- être un atelier tenu pendant une COP comme un cycle totalement autonome.
    event_id            uuid        CONSTRAINT xmod_fk_trainings_event
                                    REFERENCES event.events(id) ON DELETE SET NULL,

    -- Calendrier. Nullable : une formation en autoformation permanente n'a pas
    -- de date de début. Le fuseau sert à convertir les horaires des séances.
    timezone            platform.timezone_name NOT NULL DEFAULT 'UTC',
    starts_on           date,
    ends_on             date,
    enrollment_opens_at  timestamptz,
    enrollment_closes_at timestamptz,

    capacity            integer     CHECK (capacity IS NULL OR capacity > 0),
    waitlist_enabled    boolean     NOT NULL DEFAULT false,

    -- Tarif estimé (v1 : `estimated_price DECIMAL(10,2)` sans devise — un montant
    -- sans unité n'est pas un prix).
    estimated_price     numeric(10,2) CHECK (estimated_price IS NULL OR estimated_price >= 0),
    currency_code       text        NOT NULL DEFAULT 'EUR' CHECK (currency_code ~ '^[A-Z]{3}$'),

    -- Langue(s) de la formation. `default_locale` porte la langue principale (FK
    -- réelle), `locales` l'ensemble des langues proposées (validé par fonction).
    default_locale      text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    locales             text[]      NOT NULL DEFAULT '{fr}',

    cover_asset_id      uuid        CONSTRAINT xmod_fk_trainings_cover
                                    REFERENCES media.assets(id) ON DELETE SET NULL,

    -- D5 — conditions d'obtention de l'attestation, explicites et opposables.
    certificate_enabled     boolean      NOT NULL DEFAULT true,
    min_completion_percent  numeric(5,2) NOT NULL DEFAULT 100
                                         CHECK (min_completion_percent BETWEEN 0 AND 100),
    -- NULL = aucune exigence de note propre à la formation ; le seuil de
    -- l'évaluation finale (quizzes.passing_score_percent) fait alors foi.
    min_final_score_percent numeric(5,2) CHECK (min_final_score_percent IS NULL
                                                OR min_final_score_percent BETWEEN 0 AND 100),
    certificate_template    text,        -- gabarit PDF utilisé par le worker d'émission

    search_vector       tsvector    GENERATED ALWAYS AS (
        to_tsvector('french',
            coalesce(title ->> 'fr', '') || ' ' || coalesce(summary ->> 'fr', '') || ' ' ||
            coalesce(target_audience ->> 'fr', ''))
        || to_tsvector('english',
            coalesce(title ->> 'en', '') || ' ' || coalesce(summary ->> 'en', ''))
    ) STORED,

    published_at        timestamptz,

    -- Suppression douce : une formation citée dans une attestation délivrée ne
    -- doit jamais disparaître de la base.
    deleted_at          timestamptz,
    deleted_by          uuid        CONSTRAINT xmod_fk_trainings_deleter
                                    REFERENCES identity.people(id) ON DELETE SET NULL,

    created_by          uuid        CONSTRAINT xmod_fk_trainings_creator
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_trainings_period CHECK (ends_on IS NULL OR starts_on IS NULL OR ends_on >= starts_on),
    CONSTRAINT ck_trainings_enrollment_window
        CHECK (enrollment_closes_at IS NULL OR enrollment_opens_at IS NULL
               OR enrollment_closes_at > enrollment_opens_at),
    CONSTRAINT ck_trainings_locales CHECK (training.are_known_locales(locales)),
    CONSTRAINT ck_trainings_published_shape
        CHECK (status IN ('draft', 'archived') OR published_at IS NOT NULL),
    CONSTRAINT ck_trainings_delete_shape
        CHECK (deleted_by IS NULL OR deleted_at IS NOT NULL)
);

-- Une suppression libère l'URL : l'unicité ne porte que sur les formations vivantes.
CREATE UNIQUE INDEX ux_trainings_slug ON training.trainings (slug) WHERE deleted_at IS NULL;

CREATE INDEX ix_trainings_catalog ON training.trainings (status, starts_on DESC)
    WHERE deleted_at IS NULL AND published_at IS NOT NULL;
CREATE INDEX ix_trainings_organizer ON training.trainings (organizer_organization_id, starts_on DESC)
    WHERE organizer_organization_id IS NOT NULL;
CREATE INDEX ix_trainings_event    ON training.trainings (event_id) WHERE event_id IS NOT NULL;
CREATE INDEX ix_trainings_search   ON training.trainings USING gin (search_vector);

CREATE TRIGGER tg_trainings_updated_at
    BEFORE UPDATE ON training.trainings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_trainings_audit
    AFTER INSERT OR UPDATE OR DELETE ON training.trainings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE training.trainings IS
    'Formation en ligne, en présentiel ou hybride. Remplace public.trainings (v1) : i18n, statut explicite, règles de certification.';
COMMENT ON COLUMN training.trainings.objectives IS
    'Objectifs pédagogiques rédigés et traduits. Remplace le TEXT[] non traduit de la v1 (D4).';
COMMENT ON COLUMN training.trainings.locales IS
    'Langues proposées. Validé par training.are_known_locales() : un tableau ne peut pas porter de clé étrangère.';
COMMENT ON COLUMN training.trainings.min_completion_percent IS
    'Part du parcours à achever pour prétendre à l''attestation. Condition d''obtention explicite (D5).';
COMMENT ON COLUMN training.trainings.lead_trainer_id IS
    'Projection du formateur de rôle « lead » (training.trainers), maintenue par trigger. Ne jamais l''écrire directement.';

-- Thématiques : AUCUNE table de liaison dédiée, aucun tableau d'ENUM.
--   INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, role)
--   SELECT 'training', 'trainings', :training_id, t.id, 'primary'
--   FROM reference.taxonomy_terms t
--   WHERE t.taxonomy_code = 'activity_theme' AND t.code = 'adaptation';
--   -- lecture : SELECT reference.terms_of('training', 'trainings', :id, 'activity_theme');

-- Génération du slug quand l'application ne le fournit pas. Le référencement de
-- la page publique repose dessus : il ne peut pas être laissé à l'appelant.
CREATE OR REPLACE FUNCTION training.tg_trainings_ensure_slug()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_base   text;
    v_slug   text;
    v_suffix integer := 1;
BEGIN
    IF NEW.slug IS NOT NULL THEN
        RETURN NEW;
    END IF;

    v_base := left(coalesce(platform.slugify(NEW.title ->> 'fr'), 'formation'), 150);
    IF length(v_base) < 2 THEN
        v_base := 'formation-' || v_base;
    END IF;
    v_slug := v_base;

    WHILE EXISTS (
        SELECT 1 FROM training.trainings t
        WHERE t.slug = v_slug AND t.deleted_at IS NULL AND t.id IS DISTINCT FROM NEW.id
    ) LOOP
        v_suffix := v_suffix + 1;
        v_slug   := v_base || '-' || v_suffix;
    END LOOP;

    NEW.slug := v_slug;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_trainings_ensure_slug
    BEFORE INSERT OR UPDATE OF title, slug ON training.trainings
    FOR EACH ROW EXECUTE FUNCTION training.tg_trainings_ensure_slug();

-- -----------------------------------------------------------------------------
-- 4. Intervenants
--
-- La v1 ne connaissait aucun formateur : `created_by` faisait office d'auteur et
-- de formateur, ce qui rendait impossible d'afficher une équipe pédagogique ou
-- de retrouver les formations d'un intervenant.
-- -----------------------------------------------------------------------------
CREATE TABLE training.trainers (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    training_id           uuid        NOT NULL REFERENCES training.trainings(id) ON DELETE CASCADE,
    person_id             uuid        NOT NULL CONSTRAINT xmod_fk_trainers_person
                                      REFERENCES identity.people(id) ON DELETE RESTRICT,
    role                  training.trainer_role NOT NULL DEFAULT 'co_trainer',
    -- Instantanés : la fonction et l'organisation affichées sur l'attestation
    -- sont celles du jour de la formation, pas celles d'aujourd'hui.
    job_title_snapshot    text,
    organization_snapshot text,
    bio                   platform.i18n_text,
    sort_order            smallint    NOT NULL DEFAULT 0,
    created_at            timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_trainers UNIQUE (training_id, person_id, role)
);

-- Un seul formateur principal par formation : c'est lui qui signe l'attestation.
CREATE UNIQUE INDEX ux_trainers_lead ON training.trainers (training_id) WHERE role = 'lead';

CREATE INDEX ix_trainers_training ON training.trainers (training_id, sort_order);
CREATE INDEX ix_trainers_person   ON training.trainers (person_id);

COMMENT ON TABLE training.trainers IS
    'Équipe pédagogique d''une formation. Le rôle « lead » est unique et alimente trainings.lead_trainer_id.';

-- Synchronisation de la projection `trainings.lead_trainer_id`. Sans elle, deux
-- sources de vérité finiraient par diverger — c'est la faute classique de la
-- dénormalisation laissée à la charge de l'applicatif.
CREATE OR REPLACE FUNCTION training.tg_sync_lead_trainer()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_training uuid := COALESCE(NEW.training_id, OLD.training_id);
BEGIN
    UPDATE training.trainings t
    SET lead_trainer_id = (
            SELECT tr.person_id FROM training.trainers tr
            WHERE tr.training_id = v_training AND tr.role = 'lead'
            LIMIT 1
        )
    WHERE t.id = v_training;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_trainers_sync_lead
    AFTER INSERT OR DELETE OR UPDATE OF person_id, role ON training.trainers
    FOR EACH ROW EXECUTE FUNCTION training.tg_sync_lead_trainer();

-- -----------------------------------------------------------------------------
-- 5. Chapitres
--
-- D2 — un seul niveau : la formation contient des chapitres, le chapitre contient
-- des ressources. La table `lesson_contents` de la v1 disparaît.
-- D3 — le chapitre porte sa séance visio et son enregistrement.
-- -----------------------------------------------------------------------------
CREATE TABLE training.chapters (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    training_id         uuid        NOT NULL REFERENCES training.trainings(id) ON DELETE CASCADE,

    position            smallint    NOT NULL CHECK (position > 0),
    title               platform.i18n_text NOT NULL,
    description         platform.i18n_text,
    estimated_minutes   smallint    CHECK (estimated_minutes IS NULL OR estimated_minutes > 0),

    -- D3 — séance de visioconférence associée (Zoom, Teams...). Le module `live`
    -- détient les identifiants du fournisseur et les secrets ; on n'en garde ici
    -- que la référence.
    meeting_id          uuid        CONSTRAINT xmod_fk_chapters_meeting
                                    REFERENCES live.meetings(id) ON DELETE SET NULL,
    -- Enregistrement de la séance, par l'un ou l'autre des deux chemins réels :
    --   - `recording_stream_id` : replay diffusé (YouTube et consorts) ;
    --   - `recording_asset_id`  : fichier archivé dans la médiathèque.
    -- Les deux peuvent coexister (un replay public + une copie de conservation).
    recording_stream_id uuid        CONSTRAINT xmod_fk_chapters_recording_stream
                                    REFERENCES live.streams(id) ON DELETE SET NULL,
    recording_asset_id  uuid        CONSTRAINT xmod_fk_chapters_recording_asset
                                    REFERENCES media.assets(id) ON DELETE SET NULL,

    -- Verrou temporel : un chapitre reste fermé tant que sa séance n'a pas eu
    -- lieu. NULL = jamais ouvert (chapitre en préparation).
    published_at        timestamptz,

    -- Prérequis facultatif : le chapitre précédent doit être achevé. Modélisé
    -- comme une référence explicite et non comme « position - 1 », car un
    -- parcours peut comporter des chapitres optionnels que l'on saute.
    prerequisite_chapter_id uuid    REFERENCES training.chapters(id) ON DELETE SET NULL,

    -- Un chapitre facultatif (bonus, approfondissement) ne pèse pas dans le
    -- calcul de la progression.
    is_mandatory        boolean     NOT NULL DEFAULT true,

    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    -- Différée : réordonner un sommaire est un seul UPDATE, pas une danse de
    -- positions temporaires.
    CONSTRAINT ux_chapters_position UNIQUE (training_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT ck_chapters_prerequisite_not_self CHECK (prerequisite_chapter_id IS DISTINCT FROM id)
);

CREATE INDEX ix_chapters_training ON training.chapters (training_id, position);
CREATE INDEX ix_chapters_meeting  ON training.chapters (meeting_id) WHERE meeting_id IS NOT NULL;

CREATE TRIGGER tg_chapters_updated_at
    BEFORE UPDATE ON training.chapters
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE training.chapters IS
    'Chapitre d''une formation : une séance, ses ressources et, éventuellement, son quiz de fin de chapitre.';
COMMENT ON COLUMN training.chapters.published_at IS
    'Date d''ouverture aux inscrits. Un chapitre reste verrouillé tant qu''elle n''est pas atteinte (séance non encore tenue).';
COMMENT ON COLUMN training.chapters.prerequisite_chapter_id IS
    'Chapitre à achever avant celui-ci. Référence explicite : un parcours peut comporter des chapitres optionnels.';

-- Un prérequis appartient forcément à la même formation : sans ce contrôle, une
-- manipulation en back-office rendrait un chapitre définitivement inaccessible.
CREATE OR REPLACE FUNCTION training.tg_check_chapter_prerequisite()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_training uuid;
BEGIN
    IF NEW.prerequisite_chapter_id IS NULL THEN
        RETURN NEW;
    END IF;

    SELECT training_id INTO v_training FROM training.chapters WHERE id = NEW.prerequisite_chapter_id;

    IF v_training IS DISTINCT FROM NEW.training_id THEN
        RAISE EXCEPTION 'Prérequis invalide : le chapitre % appartient à une autre formation.',
            NEW.prerequisite_chapter_id
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_chapters_check_prerequisite
    BEFORE INSERT OR UPDATE OF prerequisite_chapter_id, training_id ON training.chapters
    FOR EACH ROW EXECUTE FUNCTION training.tg_check_chapter_prerequisite();

-- -----------------------------------------------------------------------------
-- 6. Ressources d'un chapitre
--
-- « on met les enregistrements dans un chapitre ainsi que les fichiers de
--   présentation, exercices, annexe etc. »
--
-- Une ressource est SOIT un objet de la médiathèque, SOIT un lien externe —
-- jamais les deux, jamais aucun (XOR strict). La v1 posait `file_url` ET
-- `youtube_url` côte à côte, tous deux nullables : on ne pouvait pas savoir, en
-- lisant la ligne, ce qu'il fallait afficher, et rien n'empêchait une ressource
-- vide.
-- -----------------------------------------------------------------------------
CREATE TABLE training.chapter_resources (
    id               uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    chapter_id       uuid        NOT NULL REFERENCES training.chapters(id) ON DELETE CASCADE,

    kind             training.resource_kind NOT NULL,
    title            platform.i18n_text NOT NULL,
    description      platform.i18n_text,

    -- ON DELETE RESTRICT assumé : tant qu'une ressource pédagogique pointe vers
    -- un objet, celui-ci ne peut pas être supprimé physiquement. La médiathèque
    -- pratique la suppression douce puis la purge (media.assets.purge_after) ;
    -- ce verrou empêche la purge d'emporter un support de cours.
    asset_id         uuid        CONSTRAINT xmod_fk_chapter_resources_asset
                                 REFERENCES media.assets(id) ON DELETE RESTRICT,
    external_url     platform.url,

    position         smallint    NOT NULL CHECK (position > 0),
    -- Ne s'applique qu'aux ressources hébergées : un lien externe n'est pas
    -- « téléchargeable » au sens de la plateforme.
    is_downloadable  boolean     NOT NULL DEFAULT true,
    duration_seconds integer     CHECK (duration_seconds IS NULL OR duration_seconds > 0),

    created_at       timestamptz NOT NULL DEFAULT now(),
    updated_at       timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_chapter_resources_position UNIQUE (chapter_id, position) DEFERRABLE INITIALLY DEFERRED,
    CONSTRAINT ck_chapter_resources_source CHECK (num_nonnulls(asset_id, external_url) = 1)
);

CREATE INDEX ix_chapter_resources_chapter ON training.chapter_resources (chapter_id, position);
CREATE INDEX ix_chapter_resources_asset   ON training.chapter_resources (asset_id) WHERE asset_id IS NOT NULL;

CREATE TRIGGER tg_chapter_resources_updated_at
    BEFORE UPDATE ON training.chapter_resources
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE training.chapter_resources IS
    'Ressource d''un chapitre : enregistrement, support, exercice, annexe, lecture ou lien. Remplace lesson_contents (v1).';
COMMENT ON CONSTRAINT ck_chapter_resources_source ON training.chapter_resources IS
    'XOR strict : une ressource est un objet de la médiathèque OU un lien externe. Jamais les deux, jamais aucun.';

-- -----------------------------------------------------------------------------
-- 7. Inscriptions
-- -----------------------------------------------------------------------------
CREATE TABLE training.enrollments (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    training_id         uuid        NOT NULL REFERENCES training.trainings(id) ON DELETE CASCADE,
    -- Une seule colonne quel que soit le profil : la personne existe toujours,
    -- avec ou sans compte (même principe que programme.registrations).
    person_id           uuid        NOT NULL CONSTRAINT xmod_fk_enrollments_person
                                    REFERENCES identity.people(id) ON DELETE CASCADE,
    organization_id     uuid        CONSTRAINT xmod_fk_enrollments_organization
                                    REFERENCES org.organizations(id) ON DELETE SET NULL,

    status              training.enrollment_status NOT NULL DEFAULT 'pending',
    enrolled_at         timestamptz NOT NULL DEFAULT now(),
    confirmed_at        timestamptz,
    waitlist_position   integer,

    locale              text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),

    -- Projections recalculées par training.compute_progress(). Dénormalisées
    -- pour que la liste des inscrits d'une formation — l'écran le plus consulté
    -- du back-office — ne recompte pas les chapitres à chaque affichage.
    progress_percent    numeric(5,2) NOT NULL DEFAULT 0 CHECK (progress_percent BETWEEN 0 AND 100),
    final_score_percent numeric(5,2) CHECK (final_score_percent IS NULL
                                            OR final_score_percent BETWEEN 0 AND 100),
    completed_at        timestamptz,

    source              text        NOT NULL DEFAULT 'web'
                                    CHECK (source IN ('web', 'import', 'admin', 'api', 'partner')),
    cancelled_at        timestamptz,
    cancelled_reason    text,
    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_enrollments_waitlist
        CHECK ((status = 'waitlisted') = (waitlist_position IS NOT NULL)),
    CONSTRAINT ck_enrollments_cancel_shape
        CHECK ((status = 'cancelled') = (cancelled_at IS NOT NULL))
);

-- Anti-doublon : une personne ne s'inscrit qu'une fois à une formation ; une
-- annulation libère la place et autorise une réinscription (même règle que
-- programme.registrations).
CREATE UNIQUE INDEX ux_enrollments_person_training
    ON training.enrollments (training_id, person_id)
    WHERE status <> 'cancelled';

CREATE INDEX ix_enrollments_training ON training.enrollments (training_id, status);
CREATE INDEX ix_enrollments_person   ON training.enrollments (person_id, enrolled_at DESC);
CREATE INDEX ix_enrollments_organization ON training.enrollments (organization_id)
    WHERE organization_id IS NOT NULL;
CREATE INDEX ix_enrollments_completed ON training.enrollments (training_id, completed_at DESC)
    WHERE completed_at IS NOT NULL;

CREATE TRIGGER tg_enrollments_updated_at
    BEFORE UPDATE ON training.enrollments
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_enrollments_audit
    AFTER INSERT OR UPDATE OR DELETE ON training.enrollments
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE training.enrollments IS
    'Inscription d''une personne à une formation. Remplace training_participants (v1) et porte la progression dénormalisée.';
COMMENT ON COLUMN training.enrollments.progress_percent IS
    'Projection recalculée par training.compute_progress(). Ne jamais l''écrire à la main : elle serait aussitôt écrasée.';

-- Contrôle d'ouverture des inscriptions et gestion de la jauge. Appliqué en base :
-- aucun chemin d'écriture — formulaire public, import, console — ne peut créer
-- une inscription hors fenêtre ni dépasser silencieusement la capacité.
CREATE OR REPLACE FUNCTION training.tg_validate_enrollment()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_training training.trainings%ROWTYPE;
    v_taken    integer;
BEGIN
    SELECT * INTO v_training FROM training.trainings WHERE id = NEW.training_id;

    IF v_training.deleted_at IS NOT NULL OR v_training.status = 'archived' THEN
        RAISE EXCEPTION 'Formation archivée ou supprimée : inscription impossible.'
            USING ERRCODE = 'restrict_violation';
    END IF;

    IF TG_OP = 'INSERT' AND v_training.enrollment_closes_at IS NOT NULL
       AND now() > v_training.enrollment_closes_at THEN
        RAISE EXCEPTION 'Les inscriptions à cette formation sont closes depuis le %.',
            v_training.enrollment_closes_at USING ERRCODE = 'restrict_violation';
    END IF;

    IF TG_OP = 'INSERT' AND v_training.capacity IS NOT NULL
       AND NEW.status IN ('pending', 'confirmed') THEN
        SELECT count(*) INTO v_taken
        FROM training.enrollments e
        WHERE e.training_id = NEW.training_id
          AND e.status IN ('pending', 'confirmed', 'completed', 'failed');

        IF v_taken >= v_training.capacity THEN
            IF v_training.waitlist_enabled THEN
                NEW.status := 'waitlisted';
                SELECT COALESCE(max(waitlist_position), 0) + 1 INTO NEW.waitlist_position
                FROM training.enrollments WHERE training_id = NEW.training_id;
            ELSE
                RAISE EXCEPTION 'Capacité atteinte (% places).', v_training.capacity
                    USING ERRCODE = 'restrict_violation';
            END IF;
        END IF;
    END IF;

    IF NEW.status = 'confirmed' AND NEW.confirmed_at IS NULL THEN
        NEW.confirmed_at := now();
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_enrollments_validate
    BEFORE INSERT OR UPDATE OF status ON training.enrollments
    FOR EACH ROW EXECUTE FUNCTION training.tg_validate_enrollment();

-- Événement de domaine : déclenche la confirmation par courriel, l'inscription
-- aux séances visio et les rappels, sans que ce module connaisse `engagement`
-- ni `live`.
CREATE OR REPLACE FUNCTION training.tg_enrollments_emit_events()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM platform.emit_event(
        'training', 'enrollment', NEW.id,
        CASE WHEN TG_OP = 'INSERT' THEN 'training.enrollment.created'
             ELSE 'training.enrollment.' || NEW.status::text END,
        jsonb_build_object(
            'training_id', NEW.training_id,
            'person_id',   NEW.person_id,
            'status',      NEW.status,
            'locale',      NEW.locale,
            'progress',    NEW.progress_percent
        )
    );
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_enrollments_events
    AFTER INSERT OR UPDATE OF status ON training.enrollments
    FOR EACH ROW EXECUTE FUNCTION training.tg_enrollments_emit_events();

-- -----------------------------------------------------------------------------
-- 8. Progression par chapitre
--
-- D2 — UNE SEULE table de progression, par (inscription, chapitre).
--
-- POURQUOI PAS DE TABLE (participant, ressource)
-- La v1 en avait une (`participant_lesson_progress`) et en tirait : un compteur
-- de vues, une date de dernière consultation et des minutes visionnées — par
-- fichier. Cette donnée n'était affichée nulle part et n'a jamais servi à
-- décider quoi que ce soit ; elle imposait en revanche une écriture à chaque
-- ouverture de PDF. Le seul besoin réel de l'interface est de cocher les
-- ressources déjà ouvertes dans le sommaire du chapitre : c'est exactement ce
-- que porte `viewed_resource_ids`, dans la ligne du chapitre, sans jointure,
-- sans seconde table et sans multiplier les écritures. Cette colonne n'est
-- jamais agrégée ni jointe — si elle devait l'être un jour, ce serait le signal
-- qu'une vraie table est justifiée.
-- -----------------------------------------------------------------------------
CREATE TABLE training.chapter_progress (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    enrollment_id      uuid        NOT NULL REFERENCES training.enrollments(id) ON DELETE CASCADE,
    chapter_id         uuid        NOT NULL REFERENCES training.chapters(id) ON DELETE CASCADE,

    state              training.progress_state NOT NULL DEFAULT 'not_started',
    first_viewed_at    timestamptz,
    last_viewed_at     timestamptz,
    completed_at       timestamptz,
    time_spent_seconds integer     NOT NULL DEFAULT 0 CHECK (time_spent_seconds >= 0),
    view_count         integer     NOT NULL DEFAULT 0 CHECK (view_count >= 0),

    -- Ressources déjà ouvertes : granularité conservée sans seconde table.
    viewed_resource_ids uuid[]     NOT NULL DEFAULT '{}',

    updated_at         timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_chapter_progress UNIQUE (enrollment_id, chapter_id),
    CONSTRAINT ck_chapter_progress_completion
        CHECK ((state = 'completed') = (completed_at IS NOT NULL))
);

CREATE INDEX ix_chapter_progress_enrollment ON training.chapter_progress (enrollment_id, state);
CREATE INDEX ix_chapter_progress_chapter    ON training.chapter_progress (chapter_id, state);

CREATE TRIGGER tg_chapter_progress_updated_at
    BEFORE UPDATE ON training.chapter_progress
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE training.chapter_progress IS
    'Avancement d''un inscrit sur un chapitre. Table unique : remplace participant_chapter_progress ET participant_lesson_progress (D2).';
COMMENT ON COLUMN training.chapter_progress.viewed_resource_ids IS
    'Ressources déjà ouvertes. Sert uniquement à cocher le sommaire côté client : jamais agrégée, jamais jointe.';

-- Le chapitre est-il accessible à cet inscrit ? Une seule expression de la
-- règle, utilisée par l'API comme par les tests : date d'ouverture atteinte ET
-- prérequis achevé.
CREATE OR REPLACE FUNCTION training.is_chapter_unlocked(p_enrollment_id uuid, p_chapter_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT c.published_at IS NOT NULL
       AND c.published_at <= now()
       AND (
            c.prerequisite_chapter_id IS NULL
            OR EXISTS (
                SELECT 1 FROM training.chapter_progress p
                WHERE p.enrollment_id = p_enrollment_id
                  AND p.chapter_id    = c.prerequisite_chapter_id
                  AND p.state         = 'completed'
            )
       )
    FROM training.chapters c
    WHERE c.id = p_chapter_id;
$$;

COMMENT ON FUNCTION training.is_chapter_unlocked(uuid, uuid) IS
    'Vrai si le chapitre est ouvert (date de publication atteinte) et si son prérequis éventuel est achevé.';

-- -----------------------------------------------------------------------------
-- 9. Questionnaires — quiz de chapitre ET évaluation finale
--
-- D1 — UN SEUL modèle. Le XOR ci-dessous porte toute la différence entre les
-- deux objets que la v1 dupliquait sur huit tables :
--   chapter_id renseigné  -> quiz de fin de chapitre ;
--   training_id renseigné -> évaluation finale de la formation.
-- Tout le reste — questions, options, tentatives, réponses, correction — est
-- rigoureusement commun.
-- -----------------------------------------------------------------------------
CREATE TABLE training.quizzes (
    id                    uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    chapter_id            uuid        REFERENCES training.chapters(id) ON DELETE CASCADE,
    training_id           uuid        REFERENCES training.trainings(id) ON DELETE CASCADE,
    -- Portée dérivée : évite de réécrire le CASE dans chaque requête et chaque vue.
    scope                 text        GENERATED ALWAYS AS (
                                          CASE WHEN chapter_id IS NOT NULL THEN 'chapter' ELSE 'final' END
                                      ) STORED,

    title                 platform.i18n_text NOT NULL,
    instructions          platform.i18n_text,

    passing_score_percent smallint    NOT NULL DEFAULT 60
                                      CHECK (passing_score_percent BETWEEN 0 AND 100),
    -- NULL = tentatives illimitées (usage courant d'un quiz d'entraînement de
    -- fin de chapitre) ; 1 = examen.
    max_attempts          smallint    CHECK (max_attempts IS NULL OR max_attempts > 0),
    time_limit_minutes    smallint    CHECK (time_limit_minutes IS NULL OR time_limit_minutes > 0),

    shuffle_questions     boolean     NOT NULL DEFAULT false,
    shuffle_options       boolean     NOT NULL DEFAULT false,
    correction_visibility training.correction_visibility NOT NULL DEFAULT 'after_attempt',

    -- Si vrai, le chapitre n'est achevé qu'une fois le quiz réussi. Sans effet
    -- pour une évaluation finale, qui conditionne l'attestation et non un chapitre.
    is_mandatory          boolean     NOT NULL DEFAULT false,

    published_at          timestamptz,
    created_by            uuid        CONSTRAINT xmod_fk_quizzes_creator
                                      REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at            timestamptz NOT NULL DEFAULT now(),
    updated_at            timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_quizzes_scope CHECK (num_nonnulls(chapter_id, training_id) = 1)
);

-- « Chaque chapitre peut se terminer par un quiz [...] ou pas » : zéro ou un.
CREATE UNIQUE INDEX ux_quizzes_chapter ON training.quizzes (chapter_id) WHERE chapter_id IS NOT NULL;
-- « Une formation peut se terminer par une évaluation [...] ou pas » : zéro ou une.
CREATE UNIQUE INDEX ux_quizzes_training ON training.quizzes (training_id) WHERE training_id IS NOT NULL;

CREATE TRIGGER tg_quizzes_updated_at
    BEFORE UPDATE ON training.quizzes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE training.quizzes IS
    'Questionnaire noté, rattaché SOIT à un chapitre (quiz de fin de chapitre) SOIT à une formation (évaluation finale). Modèle unique (D1).';
COMMENT ON COLUMN training.quizzes.scope IS
    'Portée dérivée du rattachement : « chapter » ou « final ». Aucune saisie, donc aucune incohérence possible.';
COMMENT ON COLUMN training.quizzes.max_attempts IS
    'Nombre de tentatives autorisées. NULL = illimité (entraînement) ; 1 = examen.';

CREATE TABLE training.quiz_questions (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    quiz_id       uuid        NOT NULL REFERENCES training.quizzes(id) ON DELETE CASCADE,
    position      smallint    NOT NULL CHECK (position > 0),
    question_type training.question_type NOT NULL,
    statement     platform.i18n_text NOT NULL,   -- énoncé
    help_text     platform.i18n_text,
    -- Affichée après coup, selon quizzes.correction_visibility. C'est elle qui
    -- fait la valeur pédagogique du quiz : sans explication, une mauvaise réponse
    -- n'apprend rien.
    explanation   platform.i18n_text,
    points        numeric(6,2) NOT NULL DEFAULT 1 CHECK (points > 0),
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_quiz_questions_position UNIQUE (quiz_id, position) DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX ix_quiz_questions_quiz ON training.quiz_questions (quiz_id, position);

CREATE TRIGGER tg_quiz_questions_updated_at
    BEFORE UPDATE ON training.quiz_questions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Options de réponse : TABLE DÉDIÉE plutôt qu'un jsonb.
-- La v1 stockait `options JSONB` et `correct_answers TEXT[]` : corriger une
-- réponse demandait de comparer des chaînes libres à un tableau, et la question
-- « quelle option a été la plus choisie ? » n'avait pas de réponse en SQL simple.
-- Avec une table, la correction automatique et les statistiques par option
-- deviennent des requêtes ordinaires, et un libellé traduit peut être corrigé
-- sans invalider les réponses déjà collectées — qui pointent vers un identifiant,
-- jamais vers un texte.
CREATE TABLE training.quiz_options (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    question_id uuid        NOT NULL REFERENCES training.quiz_questions(id) ON DELETE CASCADE,
    label       platform.i18n_text NOT NULL,
    is_correct  boolean     NOT NULL DEFAULT false,
    position    smallint    NOT NULL CHECK (position > 0),
    created_at  timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_quiz_options_position UNIQUE (question_id, position) DEFERRABLE INITIALLY DEFERRED
);

CREATE INDEX ix_quiz_options_question ON training.quiz_options (question_id, position);
CREATE INDEX ix_quiz_options_correct  ON training.quiz_options (question_id) WHERE is_correct;

COMMENT ON TABLE training.quiz_options IS
    'Option de réponse. Table dédiée (et non jsonb) : la correction automatique et les statistiques par option sont de simples requêtes.';

-- -----------------------------------------------------------------------------
-- 9 bis. Validation d'un questionnaire À LA PUBLICATION
--
-- POURQUOI PAS UNE CONTRAINTE À L'ÉCRITURE
-- Un back-office crée la question, puis ses options, requête après requête. Une
-- contrainte différée en fin de transaction rejetterait la première écriture —
-- une question seule est temporairement incomplète, c'est normal. Interdire cet
-- état intermédiaire obligerait l'interface à construire le questionnaire entier
-- avant de pouvoir enregistrer quoi que ce soit.
-- La cohérence est donc exigée au moment qui compte : la PUBLICATION. Un quiz
-- publié est forcément passable ; un brouillon peut être incomplet.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION training.validate_quiz(p_quiz_id uuid)
RETURNS void
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    v_row record;
    v_questions integer;
BEGIN
    SELECT count(*) INTO v_questions FROM training.quiz_questions WHERE quiz_id = p_quiz_id;
    IF v_questions = 0 THEN
        RAISE EXCEPTION 'Questionnaire % : aucune question, publication impossible.', p_quiz_id
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    FOR v_row IN
        SELECT q.id,
               q.position,
               q.question_type,
               count(o.id)                                   AS option_count,
               count(o.id) FILTER (WHERE o.is_correct)       AS correct_count
        FROM training.quiz_questions q
        LEFT JOIN training.quiz_options o ON o.question_id = q.id
        WHERE q.quiz_id = p_quiz_id
        GROUP BY q.id, q.position, q.question_type
    LOOP
        IF v_row.question_type = 'open_text' THEN
            IF v_row.option_count > 0 THEN
                RAISE EXCEPTION 'Question % : une question ouverte ne porte pas d''options.', v_row.position
                    USING ERRCODE = 'integrity_constraint_violation';
            END IF;
        ELSE
            IF v_row.option_count < 2 THEN
                RAISE EXCEPTION 'Question % : au moins deux options sont nécessaires (% trouvée(s)).',
                    v_row.position, v_row.option_count
                    USING ERRCODE = 'integrity_constraint_violation';
            END IF;
            IF v_row.question_type IN ('single_choice', 'true_false') AND v_row.correct_count <> 1 THEN
                RAISE EXCEPTION 'Question % : une question à choix unique doit avoir exactement une bonne réponse (% trouvée(s)).',
                    v_row.position, v_row.correct_count
                    USING ERRCODE = 'integrity_constraint_violation';
            END IF;
            IF v_row.question_type = 'multiple_choice' AND v_row.correct_count < 1 THEN
                RAISE EXCEPTION 'Question % : aucune bonne réponse déclarée.', v_row.position
                    USING ERRCODE = 'integrity_constraint_violation';
            END IF;
            IF v_row.question_type = 'true_false' AND v_row.option_count <> 2 THEN
                RAISE EXCEPTION 'Question % : une question vrai/faux porte exactement deux options.', v_row.position
                    USING ERRCODE = 'integrity_constraint_violation';
            END IF;
        END IF;
    END LOOP;
END;
$$;

COMMENT ON FUNCTION training.validate_quiz(uuid) IS
    'Contrôle de cohérence d''un questionnaire, appliqué à la publication : questions présentes, options suffisantes, bonnes réponses déclarées.';

CREATE OR REPLACE FUNCTION training.tg_quizzes_validate_publication()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM training.validate_quiz(NEW.id);
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_quizzes_validate_publication
    BEFORE UPDATE OF published_at ON training.quizzes
    FOR EACH ROW
    WHEN (NEW.published_at IS NOT NULL AND OLD.published_at IS NULL)
    EXECUTE FUNCTION training.tg_quizzes_validate_publication();

-- -----------------------------------------------------------------------------
-- 10. Tentatives
-- -----------------------------------------------------------------------------
CREATE TABLE training.quiz_attempts (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    enrollment_id      uuid        NOT NULL REFERENCES training.enrollments(id) ON DELETE CASCADE,
    quiz_id            uuid        NOT NULL REFERENCES training.quizzes(id) ON DELETE CASCADE,

    attempt_number     smallint    NOT NULL CHECK (attempt_number > 0),
    status             training.attempt_status NOT NULL DEFAULT 'in_progress',

    started_at         timestamptz NOT NULL DEFAULT now(),
    -- PIÈGE ÉVITÉ : `started_at + interval` est STABLE, donc interdit dans une
    -- colonne GENERATED ALWAYS AS ... STORED. L'échéance est posée par le trigger
    -- BEFORE INSERT ci-dessous, à partir de quizzes.time_limit_minutes.
    expires_at         timestamptz,
    submitted_at       timestamptz,
    graded_at          timestamptz,

    score              numeric(8,2) CHECK (score IS NULL OR score >= 0),
    max_score          numeric(8,2) CHECK (max_score IS NULL OR max_score >= 0),
    -- Arithmétique pure : immuable, donc légitime en colonne générée.
    score_percent      numeric(5,2) GENERATED ALWAYS AS (
                           CASE WHEN max_score IS NULL OR max_score = 0 THEN NULL
                                ELSE round(100 * score / max_score, 2) END
                       ) STORED,
    passed             boolean,
    -- Vrai tant qu'une question ouverte attend une correction humaine : la note
    -- affichée serait alors partielle et trompeuse.
    pending_manual_review boolean  NOT NULL DEFAULT false,

    time_spent_seconds integer     CHECK (time_spent_seconds IS NULL OR time_spent_seconds >= 0),
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_quiz_attempts UNIQUE (enrollment_id, quiz_id, attempt_number),
    CONSTRAINT ck_quiz_attempts_period
        CHECK (submitted_at IS NULL OR submitted_at >= started_at)
);

CREATE INDEX ix_quiz_attempts_quiz       ON training.quiz_attempts (quiz_id, status);
CREATE INDEX ix_quiz_attempts_enrollment ON training.quiz_attempts (enrollment_id, quiz_id, attempt_number DESC);
CREATE INDEX ix_quiz_attempts_review     ON training.quiz_attempts (quiz_id, submitted_at)
    WHERE pending_manual_review;

CREATE TRIGGER tg_quiz_attempts_updated_at
    BEFORE UPDATE ON training.quiz_attempts
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE training.quiz_attempts IS
    'Tentative d''un inscrit sur un questionnaire. Nominative et notée : c''est ce qui la distingue d''une soumission tool.surveys.';
COMMENT ON COLUMN training.quiz_attempts.expires_at IS
    'Échéance = started_at + time_limit_minutes, posée par trigger : timestamptz + interval est STABLE et interdit en colonne générée.';

-- Recevabilité d'une tentative : formation cohérente, quiz publié, quota de
-- tentatives respecté, numérotation automatique, échéance calculée.
CREATE OR REPLACE FUNCTION training.tg_validate_attempt()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_quiz              training.quizzes%ROWTYPE;
    v_quiz_training     uuid;
    v_enrollment_training uuid;
    v_status            training.enrollment_status;
    v_used              smallint;
BEGIN
    SELECT * INTO v_quiz FROM training.quizzes WHERE id = NEW.quiz_id;

    IF v_quiz.published_at IS NULL THEN
        RAISE EXCEPTION 'Questionnaire non publié : aucune tentative possible.'
            USING ERRCODE = 'restrict_violation';
    END IF;

    -- Le questionnaire et l'inscription doivent relever de la même formation.
    IF v_quiz.training_id IS NOT NULL THEN
        v_quiz_training := v_quiz.training_id;
    ELSE
        SELECT c.training_id INTO v_quiz_training
        FROM training.chapters c WHERE c.id = v_quiz.chapter_id;
    END IF;

    SELECT e.training_id, e.status INTO v_enrollment_training, v_status
    FROM training.enrollments e WHERE e.id = NEW.enrollment_id;

    IF v_enrollment_training IS DISTINCT FROM v_quiz_training THEN
        RAISE EXCEPTION 'Tentative refusée : l''inscription et le questionnaire relèvent de deux formations différentes.'
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    IF v_status IN ('cancelled', 'waitlisted') THEN
        RAISE EXCEPTION 'Tentative refusée : inscription %.', v_status
            USING ERRCODE = 'restrict_violation';
    END IF;

    -- Numérotation automatique : laisser l'applicatif la calculer, c'est
    -- s'exposer à deux tentatives n°1 en cas de double soumission.
    IF NEW.attempt_number IS NULL THEN
        SELECT COALESCE(max(a.attempt_number), 0) + 1 INTO NEW.attempt_number
        FROM training.quiz_attempts a
        WHERE a.enrollment_id = NEW.enrollment_id AND a.quiz_id = NEW.quiz_id;
    END IF;

    IF v_quiz.max_attempts IS NOT NULL THEN
        SELECT count(*) INTO v_used
        FROM training.quiz_attempts a
        WHERE a.enrollment_id = NEW.enrollment_id AND a.quiz_id = NEW.quiz_id;

        IF v_used >= v_quiz.max_attempts THEN
            RAISE EXCEPTION 'Nombre de tentatives épuisé (% autorisée(s)).', v_quiz.max_attempts
                USING ERRCODE = 'restrict_violation';
        END IF;
    END IF;

    IF v_quiz.time_limit_minutes IS NOT NULL AND NEW.expires_at IS NULL THEN
        NEW.expires_at := NEW.started_at + make_interval(mins => v_quiz.time_limit_minutes);
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_quiz_attempts_validate
    BEFORE INSERT ON training.quiz_attempts
    FOR EACH ROW EXECUTE FUNCTION training.tg_validate_attempt();

-- -----------------------------------------------------------------------------
-- 11. Réponses
--
-- Une seule table, quel que soit le type de question : les options cochées dans
-- `selected_option_ids`, la rédaction dans `text_answer`. La v1 stockait
-- `answer TEXT[]` dans deux tables distinctes (quiz_responses et
-- evaluation_answers) et comparait des chaînes de caractères pour corriger.
-- -----------------------------------------------------------------------------
CREATE TABLE training.quiz_answers (
    id                   uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    attempt_id           uuid        NOT NULL REFERENCES training.quiz_attempts(id) ON DELETE CASCADE,
    question_id          uuid        NOT NULL REFERENCES training.quiz_questions(id) ON DELETE CASCADE,

    -- Pas de FK possible sur un tableau : l'appartenance des options à la
    -- question est vérifiée par training.tg_validate_answer().
    selected_option_ids  uuid[]      NOT NULL DEFAULT '{}',
    text_answer          text,

    is_correct           boolean,
    points_earned        numeric(8,2) CHECK (points_earned IS NULL OR points_earned >= 0),
    graded_automatically boolean     NOT NULL DEFAULT false,
    graded_by            uuid        CONSTRAINT xmod_fk_quiz_answers_grader
                                     REFERENCES identity.people(id) ON DELETE SET NULL,
    grader_comment       text,

    answered_at          timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_quiz_answers UNIQUE (attempt_id, question_id),
    CONSTRAINT ck_quiz_answers_content
        CHECK (cardinality(selected_option_ids) > 0 OR text_answer IS NOT NULL)
);

CREATE INDEX ix_quiz_answers_question ON training.quiz_answers (question_id);
CREATE INDEX ix_quiz_answers_options  ON training.quiz_answers USING gin (selected_option_ids);
CREATE INDEX ix_quiz_answers_manual   ON training.quiz_answers (attempt_id)
    WHERE points_earned IS NULL;

CREATE TRIGGER tg_quiz_answers_updated_at
    BEFORE UPDATE ON training.quiz_answers
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE training.quiz_answers IS
    'Réponse à une question : options cochées ou texte libre. Modèle unique pour le quiz de chapitre et l''évaluation finale.';

-- Intégrité de la réponse : la question relève bien du questionnaire de la
-- tentative, et les options cochées appartiennent bien à cette question.
CREATE OR REPLACE FUNCTION training.tg_validate_answer()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_attempt_quiz  uuid;
    v_question_quiz uuid;
    v_type          training.question_type;
    v_foreign       integer;
BEGIN
    SELECT a.quiz_id INTO v_attempt_quiz FROM training.quiz_attempts a WHERE a.id = NEW.attempt_id;
    SELECT q.quiz_id, q.question_type INTO v_question_quiz, v_type
    FROM training.quiz_questions q WHERE q.id = NEW.question_id;

    IF v_attempt_quiz IS DISTINCT FROM v_question_quiz THEN
        RAISE EXCEPTION 'Réponse refusée : la question n''appartient pas au questionnaire de cette tentative.'
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    IF v_type = 'open_text' THEN
        IF cardinality(NEW.selected_option_ids) > 0 THEN
            RAISE EXCEPTION 'Réponse refusée : une question ouverte n''accepte pas d''options.'
                USING ERRCODE = 'integrity_constraint_violation';
        END IF;
        IF NEW.text_answer IS NULL THEN
            RAISE EXCEPTION 'Réponse refusée : une question ouverte attend un texte.'
                USING ERRCODE = 'not_null_violation';
        END IF;
    ELSE
        IF cardinality(NEW.selected_option_ids) = 0 THEN
            RAISE EXCEPTION 'Réponse refusée : aucune option cochée.'
                USING ERRCODE = 'not_null_violation';
        END IF;
        IF v_type IN ('single_choice', 'true_false')
           AND cardinality(training.sorted_uuids(NEW.selected_option_ids)) <> 1 THEN
            RAISE EXCEPTION 'Réponse refusée : une seule option est attendue pour une question à choix unique.'
                USING ERRCODE = 'integrity_constraint_violation';
        END IF;

        SELECT count(*) INTO v_foreign
        FROM unnest(NEW.selected_option_ids) AS s(option_id)
        WHERE NOT EXISTS (
            SELECT 1 FROM training.quiz_options o
            WHERE o.id = s.option_id AND o.question_id = NEW.question_id
        );

        IF v_foreign > 0 THEN
            RAISE EXCEPTION 'Réponse refusée : % option(s) cochée(s) n''appartiennent pas à cette question.', v_foreign
                USING ERRCODE = 'integrity_constraint_violation';
        END IF;
    END IF;

    NEW.selected_option_ids := training.sorted_uuids(NEW.selected_option_ids);
    RETURN NEW;
END;
$$;

-- Restreint aux colonnes de saisie : la correction (points_earned, is_correct)
-- écrit sur ces lignes sans repasser par la validation.
CREATE TRIGGER tg_quiz_answers_validate
    BEFORE INSERT OR UPDATE OF selected_option_ids, text_answer, question_id ON training.quiz_answers
    FOR EACH ROW EXECUTE FUNCTION training.tg_validate_answer();

-- -----------------------------------------------------------------------------
-- 12. Correction automatique et calcul de la progression
-- -----------------------------------------------------------------------------

-- Correction d'une tentative. Les types fermés sont corrigés en base : la règle
-- de notation ne doit pas dépendre du client qui appelle. Les questions ouvertes
-- restent en attente d'un correcteur humain — la tentative est alors marquée
-- `pending_manual_review` et sa réussite reste indéterminée (NULL), jamais
-- « échouée » par défaut.
CREATE OR REPLACE FUNCTION training.score_attempt(p_attempt_id uuid)
RETURNS numeric
LANGUAGE plpgsql
AS $$
DECLARE
    v_attempt   training.quiz_attempts%ROWTYPE;
    v_quiz      training.quizzes%ROWTYPE;
    v_score     numeric;
    v_max       numeric;
    v_percent   numeric;
    v_pending   boolean;
    v_passed    boolean;
BEGIN
    SELECT * INTO v_attempt FROM training.quiz_attempts WHERE id = p_attempt_id;
    IF NOT FOUND THEN
        RETURN NULL;
    END IF;

    SELECT * INTO v_quiz FROM training.quizzes WHERE id = v_attempt.quiz_id;

    -- Correction ensembliste : cocher « A puis B » ou « B puis A » est la même
    -- réponse. Tout ou rien — un barème partiel se discuterait question par
    -- question, ce que le métier n'a pas demandé (YAGNI).
    UPDATE training.quiz_answers a
    SET is_correct = CASE WHEN q.question_type = 'open_text' THEN a.is_correct
                          ELSE training.sorted_uuids(a.selected_option_ids) = c.correct_ids END,
        points_earned = CASE
                          WHEN q.question_type = 'open_text' THEN a.points_earned
                          WHEN training.sorted_uuids(a.selected_option_ids) = c.correct_ids THEN q.points
                          ELSE 0
                        END,
        graded_automatically = (q.question_type <> 'open_text')
    FROM training.quiz_questions q
    CROSS JOIN LATERAL (
        SELECT training.sorted_uuids(
                   COALESCE(array_agg(o.id) FILTER (WHERE o.is_correct), '{}'::uuid[])
               ) AS correct_ids
        FROM training.quiz_options o WHERE o.question_id = q.id
    ) AS c
    WHERE q.id = a.question_id
      AND a.attempt_id = p_attempt_id;

    SELECT COALESCE(sum(a.points_earned), 0) INTO v_score
    FROM training.quiz_answers a WHERE a.attempt_id = p_attempt_id;

    -- Barème sur la TOTALITÉ des questions du questionnaire : une question sans
    -- réponse vaut zéro, elle ne disparaît pas du dénominateur.
    SELECT COALESCE(sum(q.points), 0) INTO v_max
    FROM training.quiz_questions q WHERE q.quiz_id = v_attempt.quiz_id;

    SELECT EXISTS (
        SELECT 1
        FROM training.quiz_answers a
        JOIN training.quiz_questions q ON q.id = a.question_id
        WHERE a.attempt_id = p_attempt_id
          AND q.question_type = 'open_text'
          AND a.points_earned IS NULL
    ) INTO v_pending;

    v_percent := CASE WHEN v_max > 0 THEN round(100 * v_score / v_max, 2) END;
    v_passed  := CASE WHEN v_pending OR v_percent IS NULL THEN NULL
                      ELSE v_percent >= v_quiz.passing_score_percent END;

    UPDATE training.quiz_attempts
    SET score        = v_score,
        max_score    = v_max,
        passed       = v_passed,
        pending_manual_review = v_pending,
        graded_at    = CASE WHEN v_pending THEN NULL ELSE now() END,
        submitted_at = COALESCE(submitted_at, now()),
        status       = CASE WHEN v_pending THEN 'submitted'::training.attempt_status
                            ELSE 'graded'::training.attempt_status END
    WHERE id = p_attempt_id;

    -- Un quiz de chapitre réussi achève le chapitre : l'apprenant n'a pas à
    -- cocher lui-même une case pour que sa progression avance.
    IF COALESCE(v_passed, false) AND v_quiz.chapter_id IS NOT NULL THEN
        INSERT INTO training.chapter_progress (enrollment_id, chapter_id, state, completed_at,
                                               first_viewed_at, last_viewed_at)
        VALUES (v_attempt.enrollment_id, v_quiz.chapter_id, 'completed', now(), now(), now())
        ON CONFLICT (enrollment_id, chapter_id) DO UPDATE
        SET state          = 'completed',
            completed_at   = COALESCE(chapter_progress.completed_at, now()),
            last_viewed_at = now();
    END IF;

    PERFORM training.compute_progress(v_attempt.enrollment_id);

    PERFORM platform.emit_event(
        'training', 'quiz_attempt', p_attempt_id, 'training.quiz_attempt.graded',
        jsonb_build_object(
            'quiz_id',       v_attempt.quiz_id,
            'scope',         v_quiz.scope,
            'enrollment_id', v_attempt.enrollment_id,
            'score',         v_score,
            'max_score',     v_max,
            'percent',       v_percent,
            'passed',        v_passed,
            'pending_review', v_pending
        )
    );

    RETURN v_score;
END;
$$;

COMMENT ON FUNCTION training.score_attempt(uuid) IS
    'Corrige automatiquement les questions fermées d''une tentative, calcule la note, achève le chapitre si le quiz est réussi.';

-- Correction déclenchée par la remise. La clause WHEN interdit toute récursion :
-- la correction renseigne `score`, ce qui rend la condition fausse au second tour.
CREATE OR REPLACE FUNCTION training.tg_on_attempt_submitted()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM training.score_attempt(NEW.id);
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_quiz_attempts_submitted
    AFTER UPDATE OF status ON training.quiz_attempts
    FOR EACH ROW
    WHEN (NEW.status = 'submitted'
          AND OLD.status IS DISTINCT FROM 'submitted'
          AND NEW.score IS NULL)
    EXECUTE FUNCTION training.tg_on_attempt_submitted();

-- Progression d'une inscription : part des chapitres obligatoires achevés, plus
-- la meilleure note obtenue à l'évaluation finale. Les chapitres non encore
-- publiés comptent dans le dénominateur — un parcours à moitié ouvert n'est pas
-- un parcours à moitié fait.
CREATE OR REPLACE FUNCTION training.compute_progress(p_enrollment_id uuid)
RETURNS numeric
LANGUAGE plpgsql
AS $$
DECLARE
    v_training uuid;
    v_status   training.enrollment_status;
    v_total    integer;
    v_done     integer;
    v_percent  numeric(5,2);
    v_final    numeric(5,2);
BEGIN
    SELECT e.training_id, e.status INTO v_training, v_status
    FROM training.enrollments e WHERE e.id = p_enrollment_id;

    IF v_training IS NULL THEN
        RETURN NULL;
    END IF;

    SELECT count(*) INTO v_total
    FROM training.chapters c WHERE c.training_id = v_training AND c.is_mandatory;

    SELECT count(*) INTO v_done
    FROM training.chapter_progress p
    JOIN training.chapters c ON c.id = p.chapter_id AND c.is_mandatory
    WHERE p.enrollment_id = p_enrollment_id AND p.state = 'completed';

    v_percent := CASE WHEN v_total = 0 THEN 0 ELSE round(100.0 * v_done / v_total, 2) END;

    -- Meilleure note obtenue à l'évaluation finale, corrections humaines comprises.
    SELECT max(a.score_percent) INTO v_final
    FROM training.quiz_attempts a
    JOIN training.quizzes q ON q.id = a.quiz_id AND q.training_id = v_training
    WHERE a.enrollment_id = p_enrollment_id
      AND a.status = 'graded';

    UPDATE training.enrollments
    SET progress_percent    = v_percent,
        final_score_percent = COALESCE(v_final, final_score_percent)
    WHERE id = p_enrollment_id;

    -- Bascule automatique en « completed » dès que les conditions sont réunies :
    -- l'état de l'inscription est une conséquence des faits, pas une saisie.
    IF v_status IN ('pending', 'confirmed') AND training.is_eligible_for_certificate(p_enrollment_id) THEN
        UPDATE training.enrollments
        SET status       = 'completed',
            completed_at = COALESCE(completed_at, now())
        WHERE id = p_enrollment_id;
    END IF;

    RETURN v_percent;
END;
$$;

COMMENT ON FUNCTION training.compute_progress(uuid) IS
    'Recalcule la progression d''une inscription (chapitres obligatoires achevés) et sa note finale, puis clôt le parcours si les conditions sont réunies.';

-- Toute avancée sur un chapitre met la projection à jour : la liste des inscrits
-- ne recompte jamais les chapitres.
CREATE OR REPLACE FUNCTION training.tg_chapter_progress_recompute()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    PERFORM training.compute_progress(NEW.enrollment_id);
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_chapter_progress_recompute
    AFTER INSERT OR UPDATE OF state ON training.chapter_progress
    FOR EACH ROW EXECUTE FUNCTION training.tg_chapter_progress_recompute();

-- D5 — l'éligibilité à l'attestation est une RÈGLE, pas une appréciation :
--   1. la formation délivre des attestations ;
--   2. l'inscription n'est ni annulée ni en liste d'attente ;
--   3. la part de parcours achevée atteint le seuil de la formation ;
--   4. s'il existe une évaluation finale, elle a été réussie, au seuil le plus
--      exigeant entre celui de la formation et celui de l'évaluation.
CREATE OR REPLACE FUNCTION training.is_eligible_for_certificate(p_enrollment_id uuid)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT t.certificate_enabled
       AND e.status NOT IN ('cancelled', 'waitlisted')
       AND e.progress_percent >= t.min_completion_percent
       AND (
            fq.id IS NULL
            OR EXISTS (
                SELECT 1
                FROM training.quiz_attempts a
                WHERE a.enrollment_id = e.id
                  AND a.quiz_id       = fq.id
                  AND a.status        = 'graded'
                  AND COALESCE(a.passed, false)
                  AND a.score_percent >= GREATEST(
                          COALESCE(t.min_final_score_percent, 0),
                          fq.passing_score_percent
                      )
            )
       )
    FROM training.enrollments e
    JOIN training.trainings t ON t.id = e.training_id
    LEFT JOIN training.quizzes fq ON fq.training_id = t.id
    WHERE e.id = p_enrollment_id;
$$;

COMMENT ON FUNCTION training.is_eligible_for_certificate(uuid) IS
    'Conditions d''obtention de l''attestation : taux de complétion minimal et, s''il existe une évaluation finale, note minimale (D5).';

-- -----------------------------------------------------------------------------
-- 13. Attestations
--
-- D5 — la v1 posait une URL et rien d'autre. Ici : un numéro unique, un code de
-- vérification publiable (page /attestations/verifier), la trace des conditions
-- au moment de l'émission — une règle peut changer, l'attestation délivrée doit
-- rester justifiable — et une révocation possible en cas de fraude.
-- -----------------------------------------------------------------------------
CREATE SEQUENCE training.certificate_number_seq;

CREATE OR REPLACE FUNCTION training.next_certificate_number()
RETURNS text
LANGUAGE sql
VOLATILE
AS $$
    SELECT format('IFDD-FORM-%s-%s',
                  to_char(now(), 'YYYY'),
                  lpad(nextval('training.certificate_number_seq')::text, 6, '0'));
$$;

CREATE TABLE training.certificates (
    id                  uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    enrollment_id       uuid        NOT NULL REFERENCES training.enrollments(id) ON DELETE CASCADE,

    -- Numéro lisible et opposable, imprimé sur le document.
    certificate_number  text        NOT NULL DEFAULT training.next_certificate_number(),
    -- Jeton court saisi sur la page publique de vérification. Distinct du
    -- numéro : il n'est ni séquentiel ni devinable, on ne peut donc pas
    -- énumérer les attestations délivrées.
    verification_code   text        NOT NULL DEFAULT upper(encode(gen_random_bytes(6), 'hex')),

    issued_at           timestamptz NOT NULL DEFAULT now(),
    -- Instantané des conditions au moment de l'émission.
    completion_percent  numeric(5,2) NOT NULL CHECK (completion_percent BETWEEN 0 AND 100),
    final_score_percent numeric(5,2) CHECK (final_score_percent IS NULL
                                            OR final_score_percent BETWEEN 0 AND 100),

    asset_id            uuid        CONSTRAINT xmod_fk_certificates_asset
                                    REFERENCES media.assets(id) ON DELETE SET NULL,
    locale              text        NOT NULL DEFAULT 'fr' REFERENCES reference.locales(code),
    issued_by           uuid        CONSTRAINT xmod_fk_certificates_issuer
                                    REFERENCES identity.people(id) ON DELETE SET NULL,

    revoked_at          timestamptz,
    revoked_by          uuid        CONSTRAINT xmod_fk_certificates_revoker
                                    REFERENCES identity.people(id) ON DELETE SET NULL,
    revocation_reason   text,

    created_at          timestamptz NOT NULL DEFAULT now(),
    updated_at          timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_certificates_number UNIQUE (certificate_number),
    CONSTRAINT ux_certificates_verification UNIQUE (verification_code),
    CONSTRAINT ck_certificates_revocation
        CHECK (revoked_at IS NULL OR revocation_reason IS NOT NULL)
);

-- Une seule attestation valide par inscription. Une attestation révoquée reste
-- en base — c'est la preuve qu'elle a existé — et n'empêche pas une réémission.
CREATE UNIQUE INDEX ux_certificates_active
    ON training.certificates (enrollment_id)
    WHERE revoked_at IS NULL;

CREATE INDEX ix_certificates_issued ON training.certificates (issued_at DESC);

CREATE TRIGGER tg_certificates_updated_at
    BEFORE UPDATE ON training.certificates
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_certificates_audit
    AFTER INSERT OR UPDATE OR DELETE ON training.certificates
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE training.certificates IS
    'Attestation de formation : numéro unique, code de vérification publiable, conditions figées à l''émission, révocation possible (D5).';
COMMENT ON COLUMN training.certificates.verification_code IS
    'Jeton non séquentiel saisi sur la page publique de vérification : le numéro seul permettrait d''énumérer les attestations.';
COMMENT ON COLUMN training.certificates.completion_percent IS
    'Taux de complétion au moment de l''émission. Figé : une évolution des règles ne doit pas invalider un document déjà délivré.';

-- Émission contrôlée : l'éligibilité est vérifiée en base, pas dans le client
-- qui appelle. C'est ce qui distingue une attestation d'une simple URL collée.
CREATE OR REPLACE FUNCTION training.issue_certificate(
    p_enrollment_id uuid,
    p_asset_id      uuid DEFAULT NULL
)
RETURNS uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_enrollment training.enrollments%ROWTYPE;
    v_existing   uuid;
    v_id         uuid;
BEGIN
    SELECT * INTO v_enrollment FROM training.enrollments WHERE id = p_enrollment_id;
    IF NOT FOUND THEN
        RAISE EXCEPTION 'Inscription % introuvable.', p_enrollment_id USING ERRCODE = 'no_data_found';
    END IF;

    SELECT id INTO v_existing FROM training.certificates
    WHERE enrollment_id = p_enrollment_id AND revoked_at IS NULL;
    IF v_existing IS NOT NULL THEN
        RETURN v_existing;   -- idempotent : un double clic ne délivre pas deux attestations
    END IF;

    IF NOT training.is_eligible_for_certificate(p_enrollment_id) THEN
        RAISE EXCEPTION 'Attestation refusée : les conditions d''obtention ne sont pas remplies (progression % %%, note %).',
            v_enrollment.progress_percent, COALESCE(v_enrollment.final_score_percent::text, 'néant')
            USING ERRCODE = 'restrict_violation';
    END IF;

    INSERT INTO training.certificates (
        enrollment_id, completion_percent, final_score_percent, asset_id, locale, issued_by
    )
    VALUES (
        p_enrollment_id, v_enrollment.progress_percent, v_enrollment.final_score_percent,
        p_asset_id, v_enrollment.locale, platform.current_actor_id()
    )
    RETURNING id INTO v_id;

    PERFORM platform.emit_event(
        'training', 'certificate', v_id, 'training.certificate.issued',
        jsonb_build_object(
            'enrollment_id', p_enrollment_id,
            'training_id',   v_enrollment.training_id,
            'person_id',     v_enrollment.person_id,
            'locale',        v_enrollment.locale
        )
    );

    RETURN v_id;
END;
$$;

COMMENT ON FUNCTION training.issue_certificate(uuid, uuid) IS
    'Délivre l''attestation d''une inscription après contrôle des conditions. Idempotente : un second appel renvoie l''attestation existante.';

-- Vérification publique d'une attestation : le seul point d'entrée exposé au
-- public, volontairement minimal (aucune donnée personnelle superflue).
CREATE OR REPLACE FUNCTION training.verify_certificate(p_code text)
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    SELECT jsonb_build_object(
        'valid',        c.revoked_at IS NULL,
        'number',       c.certificate_number,
        'issued_at',    c.issued_at,
        'revoked_at',   c.revoked_at,
        'training',     t.title,
        'recipient',    p.display_name,
        'completion',   c.completion_percent,
        'final_score',  c.final_score_percent
    )
    FROM training.certificates c
    JOIN training.enrollments e ON e.id = c.enrollment_id
    JOIN training.trainings  t ON t.id = e.training_id
    JOIN identity.people     p ON p.id = e.person_id
    WHERE c.verification_code = upper(btrim(p_code));
$$;

-- -----------------------------------------------------------------------------
-- 14. Vues de lecture
-- -----------------------------------------------------------------------------

-- Catalogue public : tout ce qu'affiche la page /formations, en une requête.
CREATE OR REPLACE VIEW training.v_catalog AS
SELECT
    t.id,
    t.slug,
    t.title,
    t.summary,
    t.format,
    t.status,
    t.starts_on,
    t.ends_on,
    t.timezone,
    t.default_locale,
    t.locales,
    t.estimated_price,
    t.currency_code,
    t.capacity,
    t.cover_asset_id,
    t.event_id,
    t.organizer_organization_id,
    o.legal_name                AS organizer_name,
    o.acronym                   AS organizer_acronym,
    t.lead_trainer_id,
    p.display_name              AS lead_trainer_name,
    (SELECT count(*) FROM training.chapters c WHERE c.training_id = t.id) AS chapter_count,
    (SELECT COALESCE(sum(c.estimated_minutes), 0)
       FROM training.chapters c WHERE c.training_id = t.id)               AS estimated_minutes,
    (SELECT count(*) FROM training.enrollments e
      WHERE e.training_id = t.id AND e.status IN ('pending', 'confirmed', 'completed'))
                                                                          AS enrolled_count,
    EXISTS (SELECT 1 FROM training.quizzes q WHERE q.training_id = t.id)  AS has_final_evaluation,
    t.certificate_enabled,
    reference.terms_of('training', 'trainings', t.id, 'activity_theme')   AS theme_codes,
    -- Places restantes : NULL si la formation n'a pas de jauge.
    CASE WHEN t.capacity IS NULL THEN NULL
         ELSE greatest(t.capacity - (SELECT count(*) FROM training.enrollments e
                                      WHERE e.training_id = t.id
                                        AND e.status IN ('pending', 'confirmed', 'completed')), 0)
    END AS seats_left,
    -- État temporel dérivé, calculé une fois en base plutôt que dans chaque
    -- composant du frontend.
    CASE
        WHEN t.status = 'archived'                       THEN 'archived'
        WHEN t.ends_on   IS NOT NULL AND t.ends_on   < current_date THEN 'past'
        WHEN t.starts_on IS NOT NULL AND t.starts_on > current_date THEN 'upcoming'
        ELSE 'ongoing'
    END AS temporal_state,
    (t.enrollment_opens_at  IS NULL OR t.enrollment_opens_at  <= now())
     AND (t.enrollment_closes_at IS NULL OR t.enrollment_closes_at > now())
     AND t.status IN ('published', 'open', 'running') AS enrollment_open
FROM training.trainings t
LEFT JOIN org.organizations o ON o.id = t.organizer_organization_id
LEFT JOIN identity.people   p ON p.id = t.lead_trainer_id
WHERE t.deleted_at IS NULL
  AND t.published_at IS NOT NULL
  AND t.status <> 'draft';

COMMENT ON VIEW training.v_catalog IS
    'Catalogue public des formations, prêt à l''affichage : organisateur, formateur, volume, places restantes, état temporel.';

-- Suivi des apprenants : l'écran de pilotage du formateur et le tableau de bord
-- « mes formations » de l'apprenant partagent la même vue.
CREATE OR REPLACE VIEW training.v_learner_progress AS
SELECT
    e.id                        AS enrollment_id,
    e.training_id,
    t.title                     AS training_title,
    t.slug                      AS training_slug,
    e.person_id,
    p.display_name              AS learner_name,
    p.primary_email             AS learner_email,
    e.organization_id,
    e.status,
    e.enrolled_at,
    e.progress_percent,
    e.final_score_percent,
    e.completed_at,
    stats.chapters_total,
    stats.chapters_completed,
    stats.time_spent_seconds,
    stats.last_activity_at,
    (SELECT count(*) FROM training.quiz_attempts a
      WHERE a.enrollment_id = e.id)                    AS attempt_count,
    (SELECT count(*) FROM training.quiz_attempts a
      WHERE a.enrollment_id = e.id AND a.pending_manual_review) AS attempts_pending_review,
    training.is_eligible_for_certificate(e.id)         AS certificate_eligible,
    c.certificate_number,
    c.issued_at                 AS certificate_issued_at
FROM training.enrollments e
JOIN training.trainings t ON t.id = e.training_id
JOIN identity.people    p ON p.id = e.person_id
LEFT JOIN training.certificates c ON c.enrollment_id = e.id AND c.revoked_at IS NULL
CROSS JOIN LATERAL (
    SELECT
        (SELECT count(*) FROM training.chapters ch
          WHERE ch.training_id = e.training_id AND ch.is_mandatory)        AS chapters_total,
        COALESCE(count(*) FILTER (WHERE cp.state = 'completed'), 0)        AS chapters_completed,
        COALESCE(sum(cp.time_spent_seconds), 0)                           AS time_spent_seconds,
        max(cp.last_viewed_at)                                            AS last_activity_at
    FROM training.chapter_progress cp
    WHERE cp.enrollment_id = e.id
) AS stats;

COMMENT ON VIEW training.v_learner_progress IS
    'Avancement d''un inscrit : chapitres achevés, temps passé, tentatives, éligibilité à l''attestation et attestation délivrée.';

-- -----------------------------------------------------------------------------
-- 15. Déclarations transverses
-- -----------------------------------------------------------------------------

-- Références vers les organisations : permet à org.merge_organizations() de
-- réaffecter automatiquement ce module lors d'une fusion de fiches (comme le
-- font 060, 070 et 075).
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy) VALUES
    ('training', 'trainings',   'organizer_organization_id', 'reassign'),
    ('training', 'enrollments', 'organization_id',           'reassign')
ON CONFLICT DO NOTHING;

-- Rattachements média autorisés : la bannière du catalogue et les documents
-- généraux de la formation (programme, convention). Les ressources de chapitre,
-- elles, portent leur objet en clé étrangère directe et ne passent pas par
-- media.attachments.
INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size) VALUES
    ('training', 'trainings', 'cover',    '{"fr":"Image de couverture","en":"Cover image"}',  false, '{image/*}',                            10485760),
    ('training', 'trainings', 'document', '{"fr":"Document de la formation","en":"Training document"}', true,  '{application/pdf,application/vnd.*}', 26214400)
ON CONFLICT DO NOTHING;
