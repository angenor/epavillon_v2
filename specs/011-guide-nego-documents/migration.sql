-- =============================================================================
-- ePavillon v2 — migration de l'étape 1 de Guide Négo : les documents
--
-- Du schéma en service après 0c vers celui de docs/database/ après l'étape 1.
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
--   1. media : le bucket privé et le contrat de lecture (phase 2)
--   2. media : les colonnes qui désignent un objet sans le rattacher
--   3. reference : deux types de document, et « Guide »
--   4. negotiation.documents : brouillon sans source, COP, date, éditeur,
--      dépublication, remplacement sans boucle
--   5. negotiation : l'extraction, les pages, les notes de correction
--   6. negotiation : le rôle expert et ses deux permissions
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/. Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. media — les objets privés quittent le bucket ouvert au web (R4)
-- -----------------------------------------------------------------------------

INSERT INTO platform.settings (key, value, description) VALUES
    ('media.private_bucket', '"epavillon-prive"',
     'Bucket des objets de visibilité private. Fermé au web : seule l''API les lit, après avoir vérifié l''accès.')
ON CONFLICT (key) DO NOTHING;

CREATE OR REPLACE FUNCTION media.object_location(p_asset_id uuid)
RETURNS TABLE (
    bucket     text,
    object_key text,
    status     media.asset_status,
    byte_size  bigint,
    mime_type  text
)
LANGUAGE sql
STABLE
AS $$
    SELECT a.bucket, a.object_key, a.status, a.byte_size, a.mime_type
      FROM media.assets a
     WHERE a.id = p_asset_id
       AND a.deleted_at IS NULL;
$$;

COMMENT ON FUNCTION media.object_location(uuid) IS
    'Bucket, clé, état, poids et type d''un objet non supprimé. Contrat de lecture des autres modules : ils ne lisent jamais media.assets.';
-- -----------------------------------------------------------------------------
-- 2. media — un objet désigné par une colonne n'est pas orphelin
-- -----------------------------------------------------------------------------

-- Les colonnes qui désignent un objet SANS passer par `media.attachments` : le
-- PDF d'un document de négociation est la donnée du document, pas une
-- illustration rattachée. Chaque module y déclare les siennes, comme il déclare
-- ses références d'organisation à `org.organization_references`. Un objet ainsi
-- désigné n'est jamais orphelin.
CREATE TABLE IF NOT EXISTS media.asset_references (
    ref_schema  text NOT NULL,
    ref_table   text NOT NULL,
    ref_column  text NOT NULL,
    PRIMARY KEY (ref_schema, ref_table, ref_column)
);

COMMENT ON TABLE media.asset_references IS
    'Colonnes des autres modules qui désignent un objet directement. media.find_orphan_assets() les consulte : un objet désigné n''est pas orphelin.';

CREATE OR REPLACE FUNCTION media.is_asset_referenced(p_asset_id uuid)
RETURNS boolean
LANGUAGE plpgsql
STABLE
AS $$
DECLARE
    r record;
    v_designe boolean;
BEGIN
    FOR r IN SELECT * FROM media.asset_references LOOP
        EXECUTE format('SELECT EXISTS (SELECT 1 FROM %I.%I WHERE %I = $1)',
                       r.ref_schema, r.ref_table, r.ref_column)
            INTO v_designe USING p_asset_id;
        IF v_designe THEN
            RETURN true;
        END IF;
    END LOOP;
    RETURN false;
END;
$$;

COMMENT ON FUNCTION media.is_asset_referenced(uuid) IS
    'Vrai si une colonne déclarée dans media.asset_references désigne l''objet.';

CREATE OR REPLACE FUNCTION media.find_orphan_assets(p_min_age_days integer DEFAULT 30)
RETURNS TABLE (
    asset_id        uuid,
    bucket          text,
    object_key      text,
    byte_size       bigint,
    rendition_bytes bigint,
    owner_organization_id uuid,
    created_at      timestamptz,
    age_days        integer
)
LANGUAGE sql
STABLE
AS $$
    SELECT a.id, a.bucket, a.object_key, a.byte_size,
           COALESCE((SELECT sum(r.byte_size) FROM media.renditions r
                     WHERE r.asset_id = a.id AND r.status = 'ready'), 0)::bigint,
           a.owner_organization_id,
           a.created_at,
           extract(day FROM now() - a.created_at)::integer
    FROM media.assets a
    WHERE a.status = 'ready'
      AND a.deleted_at IS NULL
      AND a.created_at < now() - make_interval(days => p_min_age_days)
      AND NOT EXISTS (SELECT 1 FROM media.attachments t WHERE t.asset_id = a.id)
      AND NOT media.is_asset_referenced(a.id)
    ORDER BY a.byte_size DESC;
$$;

COMMENT ON FUNCTION media.find_orphan_assets(integer) IS
    'Objets prêts, ni rattachés ni désignés (media.asset_references) depuis N jours : candidats à la purge. Réponse à la contrainte d''espace disque du VPS.';

-- -----------------------------------------------------------------------------
-- 3. reference — « Résumé », « Bulletin », et « Guide »
-- -----------------------------------------------------------------------------

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, sort_order) VALUES
    ('document_type', 'summary',             '{"fr":"Résumé","en":"Summary"}', 15),
    ('document_type', 'bulletin',            '{"fr":"Bulletin","en":"Bulletin"}', 25)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- Seulement si le libellé est encore celui du semis : un libellé retouché au
-- back-office ne s'écrase pas.
UPDATE reference.taxonomy_terms
   SET label = '{"fr":"Guide","en":"Guide"}'
 WHERE taxonomy_code = 'document_type' AND code = 'negotiation_guide'
   AND label = '{"fr":"Guide de négociation","en":"Negotiation guide"}'::jsonb;

-- -----------------------------------------------------------------------------
-- 4. negotiation.documents
-- -----------------------------------------------------------------------------

-- L'éditeur vaut pour toute source. Le vecteur de recherche suit le renommage.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM information_schema.columns
                WHERE table_schema = 'negotiation' AND table_name = 'documents'
                  AND column_name = 'external_publisher') THEN
        ALTER TABLE negotiation.documents RENAME COLUMN external_publisher TO publisher;
    END IF;
END;
$$;

ALTER TABLE negotiation.documents ALTER COLUMN is_rag_eligible SET DEFAULT false;

ALTER TABLE negotiation.documents ADD COLUMN IF NOT EXISTS event_id uuid;
ALTER TABLE negotiation.documents ADD COLUMN IF NOT EXISTS issued_on date;
ALTER TABLE negotiation.documents ADD COLUMN IF NOT EXISTS unpublished_at timestamptz;

DO $$
BEGIN
    ALTER TABLE negotiation.documents ADD CONSTRAINT xmod_fk_documents_event
        FOREIGN KEY (event_id) REFERENCES event.events(id) ON DELETE RESTRICT;
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

ALTER TABLE negotiation.documents DROP CONSTRAINT IF EXISTS ck_documents_source_xor;

DO $$
BEGIN
    ALTER TABLE negotiation.documents ADD CONSTRAINT ck_documents_source_at_most_one
        CHECK (num_nonnulls(asset_id, external_url) <= 1);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

DO $$
BEGIN
    ALTER TABLE negotiation.documents ADD CONSTRAINT ck_documents_published_has_source
        CHECK (published_at IS NULL OR num_nonnulls(asset_id, external_url) = 1);
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

CREATE INDEX IF NOT EXISTS ix_documents_event     ON negotiation.documents (event_id) WHERE event_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS ux_documents_supersedes ON negotiation.documents (supersedes_id)
    WHERE supersedes_id IS NOT NULL;

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

DROP TRIGGER IF EXISTS tg_documents_no_supersede_cycle ON negotiation.documents;
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

-- -----------------------------------------------------------------------------
-- 5. negotiation — la forme lisible et les notes de correction
-- -----------------------------------------------------------------------------

DO $$
BEGIN
    CREATE TYPE negotiation.rendition_status AS ENUM ('pending', 'extracting', 'ready', 'failed');
EXCEPTION
    WHEN duplicate_object THEN NULL;
END;
$$;

COMMENT ON TYPE negotiation.rendition_status IS
    'pending → extracting → ready | failed. Relancer l''extraction ramène à pending.';

CREATE TABLE IF NOT EXISTS negotiation.document_renditions (
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
    -- Le choix de l'administratrice : lire en pages d'origine. Il se change sans
    -- republier le fichier.
    serve_as_is     boolean     NOT NULL DEFAULT false,
    -- Poids de la copie que garde le téléphone : le texte, et les images des
    -- seules pages à tableau ou figure (ADR-021). C'est la taille annoncée.
    reading_bytes   bigint      CHECK (reading_bytes IS NULL OR reading_bytes >= 0),
    -- Outil et version, pour savoir quoi réextraire le jour où ils changent.
    extractor       text,
    failure_reason  text,
    attempts        integer     NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    extracted_at    timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_document_renditions_ready  CHECK (status <> 'ready' OR page_count > 0),
    CONSTRAINT ck_document_renditions_failed CHECK (status <> 'failed' OR failure_reason IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS ix_document_renditions_asset ON negotiation.document_renditions (asset_id);

DROP TRIGGER IF EXISTS tg_document_renditions_updated_at ON negotiation.document_renditions;
CREATE TRIGGER tg_document_renditions_updated_at BEFORE UPDATE ON negotiation.document_renditions
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE negotiation.document_renditions IS
    'L''extraction d''un document fichier : son état, son verdict, son sommaire, et le choix « ouvrir tel quel ». Une ligne par document.';

CREATE TABLE IF NOT EXISTS negotiation.document_pages (
    document_id       uuid     NOT NULL REFERENCES negotiation.documents(id) ON DELETE CASCADE,
    -- De 1 au nombre de pages : clé de la progression et des notes.
    page_index        integer  NOT NULL CHECK (page_index > 0),
    -- L'étiquette imprimée, « 59 ».
    label             text     NOT NULL,
    -- Les blocs de la page, en grammaire close ; vide en mode « tel quel ».
    blocks            jsonb    NOT NULL DEFAULT '[]',
    -- La concaténation du texte des blocs : rien ne se cherche qui ne s'affiche pas.
    plain_text        text     NOT NULL DEFAULT '',
    search_vector     tsvector GENERATED ALWAYS AS (to_tsvector('french', plain_text)) STORED,
    -- L'image de la page, dans le bucket privé.
    image_key         text,
    image_bytes       integer  CHECK (image_bytes IS NULL OR image_bytes > 0),
    -- Un tableau ou une figure : l'image de la page part avec la copie gardée.
    has_origin_block  boolean  NOT NULL DEFAULT false,
    PRIMARY KEY (document_id, page_index)
);

CREATE INDEX IF NOT EXISTS ix_document_pages_search ON negotiation.document_pages USING gin (search_vector);

COMMENT ON TABLE negotiation.document_pages IS
    'La forme lisible d''un document, page par page. Une nouvelle extraction remplace toutes les lignes du document, dans une transaction.';

-- La note d'un expert sur un passage dépassé. Elle se pose PAR-DESSUS le texte,
-- sans jamais le modifier, et ne se supprime pas : un retrait se date.
CREATE TABLE IF NOT EXISTS negotiation.correction_notes (
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

CREATE INDEX IF NOT EXISTS ix_correction_notes_live ON negotiation.correction_notes (document_id, page_index)
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

DROP TRIGGER IF EXISTS tg_correction_notes_page_exists ON negotiation.correction_notes;
CREATE TRIGGER tg_correction_notes_page_exists
    BEFORE INSERT OR UPDATE OF document_id, page_index ON negotiation.correction_notes
    FOR EACH ROW EXECUTE FUNCTION negotiation.tg_correction_notes_page_exists();

DROP TRIGGER IF EXISTS tg_correction_notes_audit ON negotiation.correction_notes;
CREATE TRIGGER tg_correction_notes_audit AFTER INSERT OR UPDATE OR DELETE ON negotiation.correction_notes
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE negotiation.correction_notes IS
    'Notes de correction d''un expert, posées sur une page sans modifier le texte. Jamais supprimées : un retrait se date (withdrawn_at, withdrawn_by).';
COMMENT ON COLUMN negotiation.correction_notes.passage IS
    'L''extrait cité du passage visé, retrouvé dans le texte à l''affichage ; NULL, ou introuvable : la note se pose en tête de page.';

INSERT INTO media.asset_references (ref_schema, ref_table, ref_column) VALUES
    ('negotiation', 'documents',           'asset_id'),
    ('negotiation', 'documents',           'cover_asset_id'),
    ('negotiation', 'document_renditions', 'asset_id')
ON CONFLICT DO NOTHING;

-- -----------------------------------------------------------------------------
-- 6. negotiation — l'expert
-- -----------------------------------------------------------------------------

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('negotiation.correction.post', '{"fr":"Poser une note de correction","en":"Post a correction note"}',            'negotiation'),
    ('negotiation.correction.withdraw','{"fr":"Retirer une note de correction","en":"Withdraw a correction note"}',   'negotiation')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.roles (code, label, description, allowed_scopes, is_system) VALUES
    ('expert',
     '{"fr":"Expert","en":"Expert"}',
     '{"fr":"Corrige le fond des contenus de Guide Négo : notes de correction sur les documents","en":"Corrects the substance of Guide Négo content: correction notes on documents"}',
     '{global}', true)
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('expert',     'negotiation.correction.post'),
    ('expert',     'negotiation.correction.withdraw')
ON CONFLICT DO NOTHING;

COMMIT;
