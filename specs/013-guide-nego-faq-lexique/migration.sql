-- =============================================================================
-- ePavillon v2 — migration de l'étape 2 de Guide Négo : FAQ, parcours et lexique
--
-- Du schéma en service après l'étape 1b vers celui de docs/database/ après
-- l'étape 2.
--
-- POURQUOI CE FICHIER EXISTE
--   `docs/database/` n'est chargé qu'à la création du volume. On ne recharge
--   pas une base qui porte des comptes : on la migre, selon le § 13 de
--   docs/DEPLOIEMENT.md. Il vaut AUSSI en local — la base de développement ne
--   se détruit pas, on joue ce script, et SQLx compile contre elle.
--   **`down -v` n'est jamais la procédure.**
--
-- REJOUABLE SANS DÉGÂT
--   Chaque objet est créé sous condition. Le rejouer ne duplique rien, ne perd
--   rien, et ne fait échouer aucune étape déjà passée. Les blocs repris du
--   modèle le sont MOT POUR MOT : un commentaire qui diffère ressortirait comme
--   un écart à la comparaison des schémas.
--
-- ORDRE
--   1. reference : les vocabulaires `faq_section` et `glossary_family`
--   2. negotiation : les deux permissions du savoir (§ 9)
--   3. negotiation : le savoir (§ 10)
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/. Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. reference — rubriques de la FAQ, familles du lexique
-- -----------------------------------------------------------------------------

INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    -- Rubriques de la FAQ et familles du lexique de Guide Négo (étape 2).
    -- is_system : l'application les désigne par leur code, et la rubrique porte
    -- le nom de son pictogramme dans `icon`.
    ('faq_section',        '{"fr":"Rubriques de la FAQ","en":"FAQ sections"}', '{"fr":"Rubriques de la FAQ de Guide Négo","en":"Guide Négo FAQ sections"}', false, false, true),
    ('glossary_family',    '{"fr":"Familles du lexique","en":"Glossary families"}', '{"fr":"Filtres du lexique de Guide Négo : réunions, textes, thématiques","en":"Guide Négo glossary filters: meetings, texts, themes"}', false, false, true)
ON CONFLICT (code) DO NOTHING;

-- Rubriques de la FAQ de Guide Négo, dans l'ordre de la maquette. `icon` nomme un
-- pictogramme de l'application (frontend/app/utils/guide-nego/pictogrammes.ts).
INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, icon, sort_order) VALUES
    ('faq_section',     'first_cop',          '{"fr":"Ma première COP","en":"My first COP"}',                  'star', 10),
    ('faq_section',     'process',            '{"fr":"Le processus","en":"The process"}',                      'toc',  20),
    ('faq_section',     'negotiating_groups', '{"fr":"Les groupes de négociation","en":"Negotiating groups"}', 'user', 30),
    ('faq_section',     'on_site',            '{"fr":"Sur place","en":"On site"}',                             'pin',  40),
    ('glossary_family', 'meetings',           '{"fr":"Réunions","en":"Meetings"}',                             NULL,   10),
    ('glossary_family', 'texts',              '{"fr":"Textes","en":"Texts"}',                                  NULL,   20),
    ('glossary_family', 'themes',             '{"fr":"Thématiques","en":"Themes"}',                            NULL,   30)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 2. negotiation — les permissions du savoir
-- -----------------------------------------------------------------------------

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('negotiation.knowledge.publish','{"fr":"Publier la FAQ, le parcours et le lexique","en":"Publish the FAQ, pathway and glossary"}', 'negotiation'),
    ('negotiation.knowledge.review', '{"fr":"Vérifier le savoir et traiter la file des experts","en":"Verify knowledge and handle the expert queue"}', 'negotiation')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin',      'negotiation.knowledge.publish'),
    ('expert',     'negotiation.knowledge.publish'),
    ('expert',     'negotiation.knowledge.review')
ON CONFLICT DO NOTHING;

UPDATE identity.roles
   SET description = '{"fr":"Corrige le fond des contenus de Guide Négo : notes de correction sur les documents, vérification de la FAQ, questions et termes proposés","en":"Corrects the substance of Guide Négo content: correction notes on documents, FAQ verification, questions and proposed terms"}'
 WHERE code = 'expert';

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

DO $$
BEGIN
    CREATE TYPE negotiation.knowledge_status AS ENUM ('draft', 'published', 'to_review');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;
DO $$
BEGIN
    CREATE TYPE negotiation.question_status AS ENUM ('pending', 'answered', 'added_to_faq');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;
DO $$
BEGIN
    CREATE TYPE negotiation.proposal_status AS ENUM ('pending', 'accepted', 'rejected');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;
-- Nommé report_status jusqu'au 26/09 : 3b prend ce nom pour les signalements de
-- sessions. On ne renomme que le nôtre, reconnu à sa valeur 'open'.
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_type t
        JOIN pg_namespace n ON n.oid = t.typnamespace
        JOIN pg_enum e ON e.enumtypid = t.oid
        WHERE n.nspname = 'negotiation' AND t.typname = 'report_status' AND e.enumlabel = 'open'
    ) AND NOT EXISTS (
        SELECT 1 FROM pg_type t
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE n.nspname = 'negotiation' AND t.typname = 'faq_report_status'
    ) THEN
        ALTER TYPE negotiation.report_status RENAME TO faq_report_status;
    END IF;
END;
$$;
DO $$
BEGIN
    CREATE TYPE negotiation.faq_report_status AS ENUM ('open', 'closed');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

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
CREATE TABLE IF NOT EXISTS negotiation.faq_entries (
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

CREATE INDEX IF NOT EXISTS ix_faq_entries_section    ON negotiation.faq_entries (section_term_id, status);
CREATE INDEX IF NOT EXISTS ix_faq_entries_updated    ON negotiation.faq_entries (updated_at);
CREATE INDEX IF NOT EXISTS ix_faq_entries_question_trgm ON negotiation.faq_entries USING gin (question_norm gin_trgm_ops);

DROP TRIGGER IF EXISTS tg_faq_entries_updated_at ON negotiation.faq_entries;
CREATE TRIGGER tg_faq_entries_updated_at BEFORE UPDATE ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_faq_entries_check_section ON negotiation.faq_entries;
CREATE TRIGGER tg_faq_entries_check_section BEFORE INSERT OR UPDATE OF section_term_id ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('section_term_id', 'faq_section');
DROP TRIGGER IF EXISTS tg_faq_entries_first_published ON negotiation.faq_entries;
CREATE TRIGGER tg_faq_entries_first_published BEFORE INSERT OR UPDATE ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_first_published();
DROP TRIGGER IF EXISTS tg_faq_entries_undeletable ON negotiation.faq_entries;
CREATE TRIGGER tg_faq_entries_undeletable BEFORE DELETE ON negotiation.faq_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_undeletable();
DROP TRIGGER IF EXISTS tg_faq_entries_audit ON negotiation.faq_entries;
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

CREATE TABLE IF NOT EXISTS negotiation.faq_related (
    entry_id    uuid     NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    related_id  uuid     NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    sort_order  smallint NOT NULL DEFAULT 0,
    PRIMARY KEY (entry_id, related_id),
    CONSTRAINT ck_faq_related_not_self CHECK (entry_id <> related_id)
);

CREATE INDEX IF NOT EXISTS ix_faq_related_related ON negotiation.faq_related (related_id);

DROP TRIGGER IF EXISTS tg_faq_related_touch_parent ON negotiation.faq_related;
CREATE TRIGGER tg_faq_related_touch_parent AFTER INSERT OR UPDATE OR DELETE ON negotiation.faq_related
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_touch_knowledge_parent('faq_entries', 'entry_id');
DROP TRIGGER IF EXISTS tg_faq_related_audit ON negotiation.faq_related;
CREATE TRIGGER tg_faq_related_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.faq_related
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('entry_id');

COMMENT ON TABLE negotiation.faq_related IS
    'Questions liées d''une entrée de FAQ, dans l''ordre choisi. Tous les liens sont servis, publiés ou non : le téléphone filtre sur ce qu''il a.';

-- 10.2 — Le lexique
CREATE TABLE IF NOT EXISTS negotiation.glossary_entries (
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
CREATE UNIQUE INDEX IF NOT EXISTS ux_glossary_entries_term_norm ON negotiation.glossary_entries (term_norm);
CREATE INDEX IF NOT EXISTS ix_glossary_entries_term_trgm ON negotiation.glossary_entries USING gin (term_norm gin_trgm_ops);
CREATE INDEX IF NOT EXISTS ix_glossary_entries_acronym   ON negotiation.glossary_entries (acronym_norm) WHERE acronym_norm IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_glossary_entries_variants  ON negotiation.glossary_entries USING gin (variants_norm);
CREATE INDEX IF NOT EXISTS ix_glossary_entries_family    ON negotiation.glossary_entries (family_term_id, status);
CREATE INDEX IF NOT EXISTS ix_glossary_entries_updated   ON negotiation.glossary_entries (updated_at);

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

DROP TRIGGER IF EXISTS tg_glossary_entries_slug ON negotiation.glossary_entries;
CREATE TRIGGER tg_glossary_entries_slug BEFORE INSERT OR UPDATE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_glossary_slug();
DROP TRIGGER IF EXISTS tg_glossary_entries_updated_at ON negotiation.glossary_entries;
CREATE TRIGGER tg_glossary_entries_updated_at BEFORE UPDATE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_glossary_entries_check_family ON negotiation.glossary_entries;
CREATE TRIGGER tg_glossary_entries_check_family BEFORE INSERT OR UPDATE OF family_term_id ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('family_term_id', 'glossary_family');
DROP TRIGGER IF EXISTS tg_glossary_entries_first_published ON negotiation.glossary_entries;
CREATE TRIGGER tg_glossary_entries_first_published BEFORE INSERT OR UPDATE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_first_published();
DROP TRIGGER IF EXISTS tg_glossary_entries_undeletable ON negotiation.glossary_entries;
CREATE TRIGGER tg_glossary_entries_undeletable BEFORE DELETE ON negotiation.glossary_entries
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_knowledge_undeletable();
DROP TRIGGER IF EXISTS tg_glossary_entries_audit ON negotiation.glossary_entries;
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

CREATE TABLE IF NOT EXISTS negotiation.glossary_related (
    entry_id    uuid     NOT NULL REFERENCES negotiation.glossary_entries(id) ON DELETE CASCADE,
    related_id  uuid     NOT NULL REFERENCES negotiation.glossary_entries(id) ON DELETE CASCADE,
    sort_order  smallint NOT NULL DEFAULT 0,
    PRIMARY KEY (entry_id, related_id),
    CONSTRAINT ck_glossary_related_not_self CHECK (entry_id <> related_id)
);

CREATE INDEX IF NOT EXISTS ix_glossary_related_related ON negotiation.glossary_related (related_id);

DROP TRIGGER IF EXISTS tg_glossary_related_touch_parent ON negotiation.glossary_related;
CREATE TRIGGER tg_glossary_related_touch_parent AFTER INSERT OR UPDATE OR DELETE ON negotiation.glossary_related
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_touch_knowledge_parent('glossary_entries', 'entry_id');
DROP TRIGGER IF EXISTS tg_glossary_related_audit ON negotiation.glossary_related;
CREATE TRIGGER tg_glossary_related_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.glossary_related
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit('entry_id');

COMMENT ON TABLE negotiation.glossary_related IS
    'Termes liés d''une entrée du lexique, dans l''ordre choisi.';

-- 10.3 — Les sources d'une entrée
CREATE TABLE IF NOT EXISTS negotiation.knowledge_sources (
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

CREATE INDEX IF NOT EXISTS ix_knowledge_sources_faq      ON negotiation.knowledge_sources (faq_entry_id, sort_order)
    WHERE faq_entry_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_knowledge_sources_glossary ON negotiation.knowledge_sources (glossary_entry_id, sort_order)
    WHERE glossary_entry_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_knowledge_sources_document ON negotiation.knowledge_sources (document_id)
    WHERE document_id IS NOT NULL;

DROP TRIGGER IF EXISTS tg_knowledge_sources_updated_at ON negotiation.knowledge_sources;
CREATE TRIGGER tg_knowledge_sources_updated_at BEFORE UPDATE ON negotiation.knowledge_sources
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_knowledge_sources_touch_parent ON negotiation.knowledge_sources;
CREATE TRIGGER tg_knowledge_sources_touch_parent AFTER INSERT OR UPDATE OR DELETE ON negotiation.knowledge_sources
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_touch_knowledge_parent(
        'faq_entries', 'faq_entry_id', 'glossary_entries', 'glossary_entry_id');
DROP TRIGGER IF EXISTS tg_knowledge_sources_audit ON negotiation.knowledge_sources;
CREATE TRIGGER tg_knowledge_sources_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.knowledge_sources
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.knowledge_sources IS
    'Sources d''une entrée de FAQ ou du lexique, exactement une des deux : un document de la bibliothèque, ou une référence extérieure (titre, lien facultatif).';
COMMENT ON COLUMN negotiation.knowledge_sources.section_label IS
    'La partie citée : « annexe A.3 », « Le contexte de négociation ».';
COMMENT ON COLUMN negotiation.knowledge_sources.page_from IS
    'Première page citée : pour un document de la bibliothèque, l''index de page du lecteur (de 1 au nombre de pages) ; pour une référence extérieure, la page imprimée.';

-- 10.4 — Lectures, retours et signalements de la FAQ
CREATE TABLE IF NOT EXISTS negotiation.faq_reads (
    entry_id  uuid    NOT NULL REFERENCES negotiation.faq_entries(id) ON DELETE CASCADE,
    day       date    NOT NULL,
    count     integer NOT NULL DEFAULT 1 CHECK (count > 0),
    PRIMARY KEY (entry_id, day)
);

CREATE INDEX IF NOT EXISTS ix_faq_reads_day ON negotiation.faq_reads (day);

COMMENT ON TABLE negotiation.faq_reads IS
    'Lectures d''une entrée de FAQ, comptées par jour, sans auteur ni appareil : la matière des « plus lues » sur trente jours. Purgées au-delà de 90 jours.';

CREATE TABLE IF NOT EXISTS negotiation.faq_feedback (
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

CREATE INDEX IF NOT EXISTS ix_faq_feedback_person ON negotiation.faq_feedback (person_id);

DROP TRIGGER IF EXISTS tg_faq_feedback_updated_at ON negotiation.faq_feedback;
CREATE TRIGGER tg_faq_feedback_updated_at BEFORE UPDATE ON negotiation.faq_feedback
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE negotiation.faq_feedback IS
    '« Cette réponse vous a-t-elle aidée ? » : une voix par personne et par entrée, la dernière gardée. La personne n''est jamais rendue par une route.';
COMMENT ON COLUMN negotiation.faq_feedback.missing_reason IS
    'Après « Non » : too_vague, off_topic ou outdated — outdated ouvre aussi un signalement (from_feedback).';

CREATE TABLE IF NOT EXISTS negotiation.faq_reports (
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
CREATE UNIQUE INDEX IF NOT EXISTS ux_faq_reports_from_feedback ON negotiation.faq_reports (entry_id, reporter_id)
    WHERE from_feedback;
CREATE INDEX IF NOT EXISTS ix_faq_reports_queue    ON negotiation.faq_reports (created_at) WHERE status = 'open';
CREATE INDEX IF NOT EXISTS ix_faq_reports_entry    ON negotiation.faq_reports (entry_id) WHERE status = 'open';
CREATE INDEX IF NOT EXISTS ix_faq_reports_reporter ON negotiation.faq_reports (reporter_id, created_at DESC);

DROP TRIGGER IF EXISTS tg_faq_reports_updated_at ON negotiation.faq_reports;
CREATE TRIGGER tg_faq_reports_updated_at BEFORE UPDATE ON negotiation.faq_reports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_faq_reports_audit ON negotiation.faq_reports;
CREATE TRIGGER tg_faq_reports_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.faq_reports
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.faq_reports IS
    '« Dépassé ou faux » sur une entrée de FAQ, traité dans la file des experts. L''auteur sert le plafond et l''unicité ; il n''est jamais rendu (R9).';
COMMENT ON COLUMN negotiation.faq_reports.client_ref IS
    'Référence posée par le téléphone : un envoi rejoué par la file hors connexion ne crée pas de seconde ligne.';
COMMENT ON COLUMN negotiation.faq_reports.reasons IS
    'Parmi rule_changed, wrong, source_mismatch ; au moins un, sauf un signalement venu du retour « Dépassée ou fausse » (from_feedback).';

-- 10.5 — Les questions aux experts
CREATE TABLE IF NOT EXISTS negotiation.expert_questions (
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

CREATE INDEX IF NOT EXISTS ix_expert_questions_queue ON negotiation.expert_questions (created_at) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS ix_expert_questions_asker ON negotiation.expert_questions (asker_id, created_at DESC);

DO $$
BEGIN
    ALTER TABLE negotiation.faq_entries ADD CONSTRAINT faq_entries_origin_question_id_fkey
        FOREIGN KEY (origin_question_id) REFERENCES negotiation.expert_questions(id) ON DELETE SET NULL;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

DROP TRIGGER IF EXISTS tg_expert_questions_updated_at ON negotiation.expert_questions;
CREATE TRIGGER tg_expert_questions_updated_at BEFORE UPDATE ON negotiation.expert_questions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_expert_questions_check_theme ON negotiation.expert_questions;
CREATE TRIGGER tg_expert_questions_check_theme BEFORE INSERT OR UPDATE OF theme_term_id ON negotiation.expert_questions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_check_term_taxonomy('theme_term_id', 'negotiation_theme');
DROP TRIGGER IF EXISTS tg_expert_questions_audit ON negotiation.expert_questions;
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

DROP TRIGGER IF EXISTS tg_expert_question_event ON negotiation.expert_questions;
CREATE TRIGGER tg_expert_question_event AFTER UPDATE OF status ON negotiation.expert_questions
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_expert_question_event();

COMMENT ON TABLE negotiation.expert_questions IS
    'Questions posées aux experts depuis Guide Négo. L''auteure est gardée pour le courriel de la réponse et « Mes questions » ; aucune route du back-office ne la rend (R9).';
COMMENT ON COLUMN negotiation.expert_questions.consent_to_faq IS
    '« Ma question, anonymisée, pourra rejoindre la FAQ » — cochée par défaut. Sans elle, pas de promotion (ck_expert_questions_promotion).';
COMMENT ON COLUMN negotiation.faq_entries.origin_question_id IS
    'La question d''expert dont l''entrée est née, par promotion. Seul lien vers la question, dont l''auteure reste invisible.';

-- 10.6 — Le parcours « Ma première COP »
CREATE TABLE IF NOT EXISTS negotiation.pathway_groups (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    label         platform.i18n_text NOT NULL,
    sort_order    smallint    NOT NULL DEFAULT 0,
    is_published  boolean     NOT NULL DEFAULT false,
    created_at    timestamptz NOT NULL DEFAULT now(),
    updated_at    timestamptz NOT NULL DEFAULT now()
);

DROP TRIGGER IF EXISTS tg_pathway_groups_updated_at ON negotiation.pathway_groups;
CREATE TRIGGER tg_pathway_groups_updated_at BEFORE UPDATE ON negotiation.pathway_groups
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_pathway_groups_audit ON negotiation.pathway_groups;
CREATE TRIGGER tg_pathway_groups_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.pathway_groups
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.pathway_groups IS
    'Groupes du parcours « Ma première COP » : Avant de partir, Le premier jour, En salle, Le soir.';

CREATE TABLE IF NOT EXISTS negotiation.pathway_steps (
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

CREATE INDEX IF NOT EXISTS ix_pathway_steps_group ON negotiation.pathway_steps (group_id, sort_order);

DROP TRIGGER IF EXISTS tg_pathway_steps_updated_at ON negotiation.pathway_steps;
CREATE TRIGGER tg_pathway_steps_updated_at BEFORE UPDATE ON negotiation.pathway_steps
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_pathway_steps_audit ON negotiation.pathway_steps;
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

CREATE TABLE IF NOT EXISTS negotiation.pathway_checks (
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_pathway_checks_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    step_id     uuid        NOT NULL REFERENCES negotiation.pathway_steps(id) ON DELETE CASCADE,
    checked_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, step_id)
);

CREATE INDEX IF NOT EXISTS ix_pathway_checks_step ON negotiation.pathway_checks (step_id);

COMMENT ON TABLE negotiation.pathway_checks IS
    'Étapes cochées par un compte : une ligne par geste (R7). La coche d''une étape non publiée n''est pas comptée à l''affichage.';

-- 10.7 — Favoris et termes proposés
CREATE TABLE IF NOT EXISTS negotiation.glossary_favorites (
    person_id   uuid        NOT NULL CONSTRAINT xmod_fk_glossary_favorites_person
                            REFERENCES identity.people(id) ON DELETE CASCADE,
    entry_id    uuid        NOT NULL REFERENCES negotiation.glossary_entries(id) ON DELETE CASCADE,
    created_at  timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, entry_id)
);

CREATE INDEX IF NOT EXISTS ix_glossary_favorites_entry ON negotiation.glossary_favorites (entry_id);

COMMENT ON TABLE negotiation.glossary_favorites IS
    'Termes favoris d''un compte : une ligne par geste (R7).';

CREATE TABLE IF NOT EXISTS negotiation.glossary_proposals (
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
CREATE UNIQUE INDEX IF NOT EXISTS ux_glossary_proposals_pending ON negotiation.glossary_proposals (term_norm)
    WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS ix_glossary_proposals_queue ON negotiation.glossary_proposals (created_at) WHERE status = 'pending';

DROP TRIGGER IF EXISTS tg_glossary_proposals_updated_at ON negotiation.glossary_proposals;
CREATE TRIGGER tg_glossary_proposals_updated_at BEFORE UPDATE ON negotiation.glossary_proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();
DROP TRIGGER IF EXISTS tg_glossary_proposals_audit ON negotiation.glossary_proposals;
CREATE TRIGGER tg_glossary_proposals_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.glossary_proposals
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.glossary_proposals IS
    'Termes proposés au lexique, regroupés par forme normalisée tant qu''ils attendent. Les auteurs vivent dans glossary_proposal_authors et ne sont jamais rendus (R9).';

CREATE TABLE IF NOT EXISTS negotiation.glossary_proposal_authors (
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

CREATE INDEX IF NOT EXISTS ix_glossary_proposal_authors_person ON negotiation.glossary_proposal_authors (person_id, created_at DESC);

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

COMMIT;
