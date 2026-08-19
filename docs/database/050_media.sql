-- =============================================================================
-- ePavillon v2 — 050_media.sql
-- Module Média : objets stockés sur Garage (API compatible S3), variantes
-- d'images, rattachements polymorphes contrôlés, quotas et purge des orphelins.
--
-- Dépend de : 000, 010, 020, 030, 040
--
-- PROBLÈME HÉRITÉ DE LA V1 — l'URL en dur, dupliquée par format
-- La v1 stockait des URL absolues, une colonne par déclinaison :
--   events                : banner_high_quality_32_9_url, banner_high_quality_16_9_url,
--                           banner_high_quality_1_1_url, banner_low_quality_32_9_url,
--                           banner_low_quality_16_9_url, banner_low_quality_1_1_url  (SIX)
--   innovations_practices : cover_image_hd_16_9_url, cover_image_lg_16_9_url,
--                           cover_image_hd_1_1_url, cover_image_ld_1_1_url            (QUATRE)
--   activities            : cover_image_high_url, cover_image_low_url, banner_url     (TROIS)
--   users                 : profile_photo_url, profile_photo_thumbnail_url            (DEUX)
--
-- Conséquences constatées :
--   - ajouter un format (AVIF, ratio 4:5 pour les réseaux) = une migration ;
--   - changer d'hébergement = réécrire toutes les URL de toutes les tables ;
--   - aucune déduplication : le même logo téléversé vingt fois occupe vingt fois
--     l'espace disque ;
--   - impossible de savoir quel fichier est encore utilisé, donc impossible de
--     purger : le stockage ne fait que croître ;
--   - aucun quota par organisation, aucun texte alternatif, aucune licence.
--
-- DÉCISION V2 — la base stocke une RÉFÉRENCE LOGIQUE, jamais une URL
--   1. Un fichier = une ligne `media.assets` décrite par (bucket, object_key).
--      L'URL publique est COMPOSÉE À LA LECTURE par l'API à partir du point
--      d'accès courant (platform.settings « media.public_base_url »). Migrer de
--      Garage auto-hébergé vers un fournisseur cloud en phase 2 = changer un
--      réglage, pas réécrire la base.
--   2. Les formats deviennent des LIGNES (`media.renditions`), plus des
--      colonnes. Ajouter l'AVIF ou un ratio 4:5 est une insertion, pas un DDL.
--   3. Le lien entre un objet et son porteur passe par `media.attachments`,
--      validé contre une table blanche `media.attachable_roles` : on ne peut ni
--      inventer un rôle, ni poser deux logos, ni téléverser un PDF là où une
--      image est attendue.
--   4. L'espace disque est une contrainte réelle du VPS : déduplication par
--      empreinte SHA-256, quota par organisation, et détection des orphelins.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Types du module
-- -----------------------------------------------------------------------------
CREATE TYPE media.asset_visibility AS ENUM (
    'public',         -- servi sans authentification (bannières, logos, galeries)
    'authenticated',  -- réservé aux comptes connectés
    'private'         -- URL signée à durée limitée (documents de négociation)
);

CREATE TYPE media.asset_status AS ENUM (
    'uploaded',     -- objet déposé sur le stockage, métadonnées enregistrées
    'scanning',     -- analyse antivirus en cours
    'processing',   -- génération des variantes en cours
    'ready',        -- exploitable : seul état servi au public
    'quarantined',  -- analyse positive ou contenu refusé : jamais servi
    'failed'        -- traitement en échec définitif
);

CREATE TYPE media.scan_verdict AS ENUM ('pending', 'clean', 'infected', 'unsupported', 'error');

CREATE TYPE media.rendition_format AS ENUM ('webp', 'avif', 'jpeg', 'png', 'mp4', 'pdf');

CREATE TYPE media.rendition_status AS ENUM ('pending', 'generating', 'ready', 'failed');

-- Rôle d'un rattachement. Fermé, et c'est le point : la table blanche
-- `attachable_roles` déclare quelles combinaisons (entité, rôle) existent, et
-- un rôle qui n'est pas ici ne peut pas être déclaré par erreur.
--
-- `video` A ÉTÉ AJOUTÉ pour le module `content` (115_content.sql) : l'énumération
-- ne portait que des rôles d'image et de document, alors que la plateforme
-- stocke déjà des enregistrements de séance et fait défiler des fonds vidéo sur
-- sa page d'accueil. Le détourner sur `attachment` — le fourre-tout — aurait
-- rendu illisible le seul endroit qui dit à quoi sert un fichier.
CREATE TYPE media.attachment_role AS ENUM (
    'cover', 'banner', 'logo', 'gallery', 'document', 'avatar', 'video', 'attachment'
);

-- -----------------------------------------------------------------------------
-- 2. Objets stockés
--
-- `bucket` + `object_key` remplacent l'URL. Aucune colonne de ce schéma ne
-- contient de protocole ni de nom d'hôte : c'est ce qui rend le changement de
-- fournisseur de stockage indolore.
-- -----------------------------------------------------------------------------
CREATE TABLE media.assets (
    id                   uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    bucket               text        NOT NULL DEFAULT 'epavillon'
                                     CHECK (bucket ~ '^[a-z0-9][a-z0-9.-]{1,62}$'),
    -- Chemin de l'objet dans le bucket, sans barre oblique initiale ni espace.
    -- Convention : <année>/<mois>/<uuid>/<nom-normalisé>.<ext>
    object_key           text        NOT NULL
                                     CHECK (length(object_key) BETWEEN 1 AND 1024
                                            AND object_key !~ '^/' AND object_key !~ '\s'),

    -- Empreinte du contenu : socle de la déduplication et du contrôle d'intégrité.
    checksum_sha256      text        NOT NULL CHECK (checksum_sha256 ~ '^[0-9a-f]{64}$'),

    mime_type            text        NOT NULL CHECK (mime_type ~ '^[a-z]+/[a-z0-9.+-]+$'),
    byte_size            bigint      NOT NULL CHECK (byte_size > 0),
    original_filename    text,

    -- Dimensions : NULL pour un document. `duration_seconds` pour l'audio/vidéo.
    width                integer     CHECK (width IS NULL OR width > 0),
    height               integer     CHECK (height IS NULL OR height > 0),
    duration_seconds     numeric(10,3) CHECK (duration_seconds IS NULL OR duration_seconds > 0),

    owner_person_id      uuid        CONSTRAINT xmod_fk_assets_owner_person
                                     REFERENCES identity.people(id) ON DELETE SET NULL,
    owner_organization_id uuid       CONSTRAINT xmod_fk_assets_owner_organization
                                     REFERENCES org.organizations(id) ON DELETE SET NULL,

    visibility           media.asset_visibility NOT NULL DEFAULT 'public',
    status               media.asset_status     NOT NULL DEFAULT 'uploaded',

    -- Analyse antivirus : un site institutionnel accepte des téléversements
    -- publics, il doit pouvoir prouver qu'ils ont été inspectés.
    scan_verdict         media.scan_verdict NOT NULL DEFAULT 'pending',
    scan_engine          text,
    scanned_at           timestamptz,
    scan_details         jsonb,

    -- Accessibilité : le texte alternatif est une obligation d'accessibilité
    -- (RGAA/WCAG) pour une plateforme de la Francophonie, pas une option.
    alt_text             platform.i18n_text,
    caption              platform.i18n_text,
    credit               text,                    -- auteur, photographe, source
    license_code         text,                    -- reference.taxonomy_terms (taxonomie media_license)

    -- Suppression logique puis physique : `purge_after` donne au worker la date
    -- à partir de laquelle l'objet peut réellement quitter le stockage.
    deleted_at           timestamptz,
    deleted_by           uuid        CONSTRAINT xmod_fk_assets_deleted_by
                                     REFERENCES identity.people(id) ON DELETE SET NULL,
    purge_after          timestamptz,
    purged_at            timestamptz,

    created_at           timestamptz NOT NULL DEFAULT now(),
    updated_at           timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_assets_owner_present
        CHECK (owner_person_id IS NOT NULL OR owner_organization_id IS NOT NULL),
    -- Une image publiée sans texte alternatif est un défaut d'accessibilité :
    -- la base refuse de la faire passer à l'état servi.
    CONSTRAINT ck_assets_alt_text_required
        CHECK (status <> 'ready' OR mime_type NOT LIKE 'image/%' OR alt_text IS NOT NULL),
    CONSTRAINT ck_assets_scan_before_ready
        CHECK (status <> 'ready' OR scan_verdict IN ('clean', 'unsupported')),
    -- Une date de purge n'a de sens qu'après une suppression logique.
    CONSTRAINT ck_assets_delete_shape
        CHECK (purge_after IS NULL OR deleted_at IS NOT NULL)
);

-- Un objet occupe une place et une seule dans le bucket.
CREATE UNIQUE INDEX ux_assets_object ON media.assets (bucket, object_key);

-- DÉDUPLICATION. Le même contenu ne peut exister qu'une fois par bucket : avant
-- tout téléversement, l'API calcule l'empreinte et interroge cet index (cf.
-- media.find_by_checksum). S'il existe déjà, elle crée un simple rattachement
-- supplémentaire vers l'asset existant au lieu d'écrire un second objet. Le
-- texte alternatif propre à un usage donné se surcharge sur le rattachement.
CREATE UNIQUE INDEX ux_assets_checksum
    ON media.assets (bucket, checksum_sha256)
    WHERE deleted_at IS NULL;

CREATE INDEX ix_assets_owner_organization ON media.assets (owner_organization_id, created_at DESC)
    WHERE owner_organization_id IS NOT NULL;
CREATE INDEX ix_assets_owner_person ON media.assets (owner_person_id, created_at DESC)
    WHERE owner_person_id IS NOT NULL;
CREATE INDEX ix_assets_pipeline ON media.assets (status, created_at)
    WHERE status IN ('uploaded', 'scanning', 'processing');

-- Index partiel de purge : la file « à supprimer physiquement » reste minuscule
-- quel que soit le volume historisé.
CREATE INDEX ix_assets_purgeable
    ON media.assets (purge_after)
    WHERE deleted_at IS NOT NULL AND purged_at IS NULL;

CREATE TRIGGER tg_assets_updated_at
    BEFORE UPDATE ON media.assets
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Audité : les métadonnées de droits (crédit, licence) et les changements de
-- visibilité doivent être justifiables a posteriori.
CREATE TRIGGER tg_assets_audit
    AFTER INSERT OR UPDATE OR DELETE ON media.assets
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE media.assets IS
    'Objet stocké sur Garage/S3, décrit par (bucket, object_key). Aucune URL absolue en base : elle est composée à la lecture.';
COMMENT ON COLUMN media.assets.object_key IS
    'Chemin de l''objet dans le bucket. Ne contient ni schéma ni nom d''hôte : changer de fournisseur ne touche pas les données.';
COMMENT ON COLUMN media.assets.checksum_sha256 IS
    'Empreinte du contenu. Support de la déduplication (ux_assets_checksum) et du contrôle d''intégrité après migration.';
COMMENT ON COLUMN media.assets.alt_text IS
    'Texte alternatif multilingue. Obligatoire pour toute image servie (contrainte ck_assets_alt_text_required).';
COMMENT ON COLUMN media.assets.purge_after IS
    'Date à partir de laquelle le worker peut supprimer physiquement l''objet. Fenêtre de rétention avant perte définitive.';

-- Recherche de doublon avant téléversement : appelée par l'API dès que
-- l'empreinte du fichier est connue côté client ou après réception du flux.
CREATE OR REPLACE FUNCTION media.find_by_checksum(p_checksum text, p_bucket text DEFAULT 'epavillon')
RETURNS uuid
LANGUAGE sql
STABLE
AS $$
    SELECT a.id FROM media.assets a
    WHERE a.bucket = p_bucket AND a.checksum_sha256 = p_checksum AND a.deleted_at IS NULL;
$$;

COMMENT ON FUNCTION media.find_by_checksum(text, text) IS
    'Retourne l''asset existant pour une empreinte donnée. Évite d''écrire deux fois le même contenu sur le disque.';

-- Composition de l'URL publique à la lecture. Le point d'accès vit dans les
-- réglages : bascule Garage -> fournisseur cloud sans migration de données.
CREATE OR REPLACE FUNCTION media.object_url(p_bucket text, p_object_key text)
RETURNS text
LANGUAGE sql
STABLE
AS $$
    SELECT rtrim(COALESCE(
               (SELECT s.value #>> '{}' FROM platform.settings s WHERE s.key = 'media.public_base_url'),
               'https://media.epavillonclimatique.francophonie.org'
           ), '/') || '/' || p_bucket || '/' || p_object_key;
$$;

COMMENT ON FUNCTION media.object_url(text, text) IS
    'Compose l''URL publique depuis le point d''accès courant (platform.settings « media.public_base_url »).';

-- -----------------------------------------------------------------------------
-- 3. Variantes
--
-- Ce qui remplace les six colonnes de bannière : une ligne par déclinaison.
-- La génération n'est pas synchrone — elle est confiée à `platform.jobs`
-- (tâche « media.generate_renditions »), déclenchée par le trigger d'insertion
-- ci-dessous. L'API sert la variante disponible la plus proche du besoin et se
-- rabat sur l'original tant que le traitement n'a pas abouti.
-- -----------------------------------------------------------------------------
CREATE TABLE media.renditions (
    id            uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    asset_id      uuid        NOT NULL REFERENCES media.assets(id) ON DELETE CASCADE,

    -- Code de variante libre mais normalisé : 'original', 'hd_16_9', 'card_1_1',
    -- 'thumb', 'avif_lg'... Ajouter un format = insérer, jamais migrer.
    variant_code  text        NOT NULL CHECK (variant_code ~ '^[a-z0-9]+(_[a-z0-9]+)*$'),
    format        media.rendition_format NOT NULL,

    width         integer     CHECK (width IS NULL OR width > 0),
    height        integer     CHECK (height IS NULL OR height > 0),
    object_key    text        NOT NULL CHECK (length(object_key) BETWEEN 1 AND 1024
                                              AND object_key !~ '^/' AND object_key !~ '\s'),
    byte_size     bigint      CHECK (byte_size IS NULL OR byte_size > 0),

    status        media.rendition_status NOT NULL DEFAULT 'pending',
    generated_at  timestamptz,
    last_error    text,
    created_at    timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_renditions UNIQUE (asset_id, variant_code, format),
    CONSTRAINT ck_renditions_ready_shape
        CHECK (status <> 'ready' OR (byte_size IS NOT NULL AND generated_at IS NOT NULL))
);

-- La clé d'objet d'une variante est préfixée par l'identifiant de l'asset :
-- elle est donc unique tous buckets confondus.
CREATE UNIQUE INDEX ux_renditions_object_key ON media.renditions (object_key);
CREATE INDEX ix_renditions_asset  ON media.renditions (asset_id, variant_code);
CREATE INDEX ix_renditions_queue  ON media.renditions (created_at) WHERE status IN ('pending', 'generating');

COMMENT ON TABLE media.renditions IS
    'Déclinaisons d''un objet (ratio, résolution, format). Remplace les colonnes banner_*_url de la v1. Générées par platform.jobs.';
COMMENT ON COLUMN media.renditions.variant_code IS
    'Code de la déclinaison. La liste vit dans la configuration du worker, pas dans le schéma : ajouter l''AVIF ne migre rien.';

-- Mise en file de la génération dès que l'objet est déposé. La clé
-- d'idempotence empêche le double traitement en cas de rejeu de l'événement.
CREATE OR REPLACE FUNCTION media.tg_enqueue_processing()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO platform.jobs (queue, task, payload, idempotency_key, priority)
    VALUES (
        'media',
        'media.process_asset',
        jsonb_build_object('asset_id', NEW.id, 'bucket', NEW.bucket,
                           'object_key', NEW.object_key, 'mime_type', NEW.mime_type),
        NEW.id::text,
        50
    )
    ON CONFLICT DO NOTHING;

    PERFORM platform.emit_event('media', 'asset', NEW.id, 'media.asset.uploaded',
        jsonb_build_object('mime_type', NEW.mime_type, 'byte_size', NEW.byte_size));
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_assets_enqueue_processing
    AFTER INSERT ON media.assets
    FOR EACH ROW EXECUTE FUNCTION media.tg_enqueue_processing();

-- Jeu de variantes prêt à servir, sous forme exploitable par un <picture> Nuxt.
-- Un seul appel remplace la lecture des six colonnes de la v1.
CREATE OR REPLACE FUNCTION media.asset_sources(p_asset_id uuid)
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(jsonb_object_agg(
               r.variant_code || '_' || r.format,
               jsonb_build_object('url', media.object_url(a.bucket, r.object_key),
                                  'width', r.width, 'height', r.height, 'bytes', r.byte_size)
           ), '{}'::jsonb)
    FROM media.assets a
    JOIN media.renditions r ON r.asset_id = a.id AND r.status = 'ready'
    WHERE a.id = p_asset_id AND a.deleted_at IS NULL AND a.status = 'ready';
$$;

-- -----------------------------------------------------------------------------
-- 4. Rattachements polymorphes CONTRÔLÉS
--
-- Le polymorphisme sans garde-fou (owner_schema/owner_table/owner_id libres)
-- reproduirait le désordre de la v1 sous une autre forme. La table blanche
-- `media.attachable_roles` déclare ce qui est permis ; le trigger de validation
-- refuse tout le reste. On ne peut donc ni se tromper de rôle, ni poser deux
-- logos, ni glisser un PDF dans une bannière.
-- -----------------------------------------------------------------------------
CREATE TABLE media.attachable_roles (
    owner_schema          text NOT NULL,
    owner_table           text NOT NULL,
    role                  media.attachment_role NOT NULL,
    label                 platform.i18n_text NOT NULL,
    -- false : un seul objet pour ce rôle (logo, avatar, bannière).
    is_multiple           boolean NOT NULL DEFAULT false,
    -- Préfixes MIME acceptés, motif '*' autorisé. Tableau vide = tout accepté.
    allowed_mime_prefixes text[]  NOT NULL DEFAULT '{}',
    max_byte_size         bigint  CHECK (max_byte_size IS NULL OR max_byte_size > 0),
    is_active             boolean NOT NULL DEFAULT true,
    PRIMARY KEY (owner_schema, owner_table, role)
);

COMMENT ON TABLE media.attachable_roles IS
    'Table blanche des rattachements autorisés (entité porteuse x rôle). Toute combinaison non déclarée est rejetée par trigger.';

CREATE TABLE media.attachments (
    id             uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    owner_schema   text        NOT NULL,
    owner_table    text        NOT NULL,
    owner_id       uuid        NOT NULL,
    asset_id       uuid        NOT NULL REFERENCES media.assets(id) ON DELETE CASCADE,
    role           media.attachment_role NOT NULL,
    sort_order     smallint    NOT NULL DEFAULT 0,

    -- Un asset dédupliqué est partagé entre plusieurs usages : le texte
    -- alternatif pertinent peut différer d'un contexte à l'autre.
    alt_text_override platform.i18n_text,

    -- Renseigné par le trigger depuis la règle : permet un index unique partiel,
    -- seule protection efficace contre deux insertions concurrentes.
    is_exclusive   boolean     NOT NULL DEFAULT false,

    created_by     uuid        CONSTRAINT xmod_fk_attachments_creator
                               REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at     timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ux_attachments UNIQUE (owner_schema, owner_table, owner_id, role, asset_id)
);

-- Un seul objet par rôle exclusif, y compris sous concurrence.
CREATE UNIQUE INDEX ux_attachments_exclusive_role
    ON media.attachments (owner_schema, owner_table, owner_id, role)
    WHERE is_exclusive;

-- Lecture principale : « tous les médias de cette entité », servie par l'API à
-- chaque affichage de fiche.
CREATE INDEX ix_attachments_owner ON media.attachments (owner_schema, owner_table, owner_id, role, sort_order);
-- Lecture inverse : « cet objet est-il encore utilisé ? », socle de la purge.
CREATE INDEX ix_attachments_asset ON media.attachments (asset_id);

COMMENT ON TABLE media.attachments IS
    'Rattachement d''un objet à une entité porteuse pour un rôle donné. Remplace les colonnes d''URL disséminées dans la v1.';

CREATE OR REPLACE FUNCTION media.tg_validate_attachment()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_rule  media.attachable_roles%ROWTYPE;
    v_asset media.assets%ROWTYPE;
BEGIN
    SELECT * INTO v_rule FROM media.attachable_roles
    WHERE owner_schema = NEW.owner_schema AND owner_table = NEW.owner_table AND role = NEW.role;

    IF NOT FOUND OR NOT v_rule.is_active THEN
        RAISE EXCEPTION 'Rattachement refusé : le rôle « % » n''est pas déclaré pour %.%.',
            NEW.role, NEW.owner_schema, NEW.owner_table
            USING ERRCODE = 'integrity_constraint_violation',
                  HINT = 'Déclarer la combinaison dans media.attachable_roles avant de l''utiliser.';
    END IF;

    SELECT * INTO v_asset FROM media.assets WHERE id = NEW.asset_id;
    IF v_asset.deleted_at IS NOT NULL THEN
        RAISE EXCEPTION 'Rattachement refusé : l''objet % est supprimé.', NEW.asset_id
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    IF v_asset.status = 'quarantined' THEN
        RAISE EXCEPTION 'Rattachement refusé : l''objet % est en quarantaine.', NEW.asset_id
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    -- Contrôle du type MIME attendu pour ce rôle.
    IF cardinality(v_rule.allowed_mime_prefixes) > 0
       AND NOT EXISTS (
           SELECT 1 FROM unnest(v_rule.allowed_mime_prefixes) AS p
           WHERE v_asset.mime_type LIKE replace(p, '*', '%')
       ) THEN
        RAISE EXCEPTION 'Rattachement refusé : type % non accepté pour le rôle « % » (attendu : %).',
            v_asset.mime_type, NEW.role, array_to_string(v_rule.allowed_mime_prefixes, ', ')
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    IF v_rule.max_byte_size IS NOT NULL AND v_asset.byte_size > v_rule.max_byte_size THEN
        RAISE EXCEPTION 'Rattachement refusé : % octets dépassent la limite de % pour le rôle « % ».',
            v_asset.byte_size, v_rule.max_byte_size, NEW.role
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    NEW.is_exclusive := NOT v_rule.is_multiple;

    -- Rôle unique déjà pourvu : message explicite plutôt qu'une violation d'index.
    IF NEW.is_exclusive AND EXISTS (
        SELECT 1 FROM media.attachments a
        WHERE a.owner_schema = NEW.owner_schema AND a.owner_table = NEW.owner_table
          AND a.owner_id = NEW.owner_id AND a.role = NEW.role
          AND a.id IS DISTINCT FROM NEW.id
    ) THEN
        RAISE EXCEPTION 'Rattachement refusé : le rôle « % » n''accepte qu''un seul objet pour %.% (%).',
            NEW.role, NEW.owner_schema, NEW.owner_table, NEW.owner_id
            USING ERRCODE = 'unique_violation',
                  HINT = 'Remplacer le rattachement existant plutôt qu''en ajouter un second.';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_attachments_validate
    BEFORE INSERT OR UPDATE ON media.attachments
    FOR EACH ROW EXECUTE FUNCTION media.tg_validate_attachment();


-- Image rattachée à une entité pour un rôle donné, prête pour l'affichage.
--
-- POURQUOI UNE FONCTION PLUTÔT QU'UNE JOINTURE PAR VUE. Le rattachement est
-- polymorphe : sans elle, chaque vue qui veut montrer une vignette réécrit la
-- même jointure latérale, la même composition d'URL et le même ordre de repli du
-- texte alternatif. Trois écritures, trois occasions de diverger — et la
-- première divergence est silencieuse, puisqu'une image manquante ressemble à
-- une entité sans image.
--
-- CE QU'ELLE REND, et pourquoi chaque champ y est :
--   url      l'ORIGINAL, toujours. La génération des variantes est asynchrone
--            (platform.jobs) : entre le téléversement et le passage du worker,
--            `sources` est vide alors que l'image est parfaitement valide. Sans
--            ce champ, la carte resterait vide pendant ce délai.
--   sources  les variantes prêtes, pour un <picture>. Objet vide tant que le
--            worker n'a rien produit.
--   alt_text la surcharge du rattachement d'abord, le texte de l'objet ensuite.
--            La déduplication fait qu'un même fichier sert plusieurs fiches : le
--            texte pertinent n'y est pas le même.
--
-- Ne rend que ce qui est réellement servable : objet `ready` et non supprimé.
-- Un objet en quarantaine ou en cours de traitement est traité comme absent.
-- Renvoie NULL si l'entité n'a pas d'image, et NULL si p_owner_id est nul — ce
-- qui permet d'enchaîner les replis par COALESCE sans garde préalable.
CREATE OR REPLACE FUNCTION media.attached_image(
    p_owner_schema text,
    p_owner_table  text,
    p_owner_id     uuid,
    p_role         media.attachment_role DEFAULT 'cover'
)
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    SELECT jsonb_build_object(
               'asset_id', a.id,
               'url',      media.object_url(a.bucket, a.object_key),
               'width',    a.width,
               'height',   a.height,
               'alt_text', COALESCE(t.alt_text_override, a.alt_text),
               'caption',  a.caption,
               'credit',   a.credit,
               'sources',  media.asset_sources(a.id))
    FROM media.attachments t
    JOIN media.assets a ON a.id = t.asset_id
    WHERE t.owner_schema = p_owner_schema
      AND t.owner_table  = p_owner_table
      AND t.owner_id     = p_owner_id
      AND t.role         = p_role
      AND a.deleted_at IS NULL
      AND a.status = 'ready'
    ORDER BY t.sort_order, t.created_at
    LIMIT 1;
$$;

COMMENT ON FUNCTION media.attached_image(text, text, uuid, media.attachment_role) IS
    'Image rattachée à une entité pour un rôle, prête à l''affichage : URL de l''original, variantes disponibles, texte alternatif résolu. NULL si aucune image servable.';

-- -----------------------------------------------------------------------------
-- 5. Quotas de stockage
--
-- Le VPS de la phase 1 a un disque fini : le quota n'est pas une commodité,
-- c'est ce qui empêche une organisation de saturer la plateforme entière.
--
-- CHOIX : compteur incrémental maintenu par trigger, plutôt que vue
-- matérialisée. Le quota doit être opposable AU MOMENT du téléversement ; une
-- vue matérialisée rafraîchie périodiquement autoriserait un dépassement entre
-- deux rafraîchissements. La valeur exacte reste recalculable à tout moment par
-- media.organization_storage_usage() et media.reconcile_storage_quotas().
-- -----------------------------------------------------------------------------
CREATE TABLE media.storage_quotas (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- NULL = ligne par défaut, applicable à toute organisation sans règle propre.
    -- NULLS NOT DISTINCT garantit qu'il n'existe qu'une seule ligne par défaut.
    organization_id uuid        UNIQUE NULLS NOT DISTINCT
                                CONSTRAINT xmod_fk_storage_quotas_organization
                                REFERENCES org.organizations(id) ON DELETE CASCADE,
    max_bytes       bigint      NOT NULL CHECK (max_bytes > 0),
    max_files       integer     NOT NULL CHECK (max_files > 0),
    used_bytes      bigint      NOT NULL DEFAULT 0 CHECK (used_bytes >= 0),
    used_files      integer     NOT NULL DEFAULT 0 CHECK (used_files >= 0),
    note            text,
    updated_at      timestamptz NOT NULL DEFAULT now()
);

CREATE TRIGGER tg_storage_quotas_updated_at
    BEFORE UPDATE ON media.storage_quotas
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE media.storage_quotas IS
    'Quota de stockage par organisation. La ligne à organization_id NULL est le quota par défaut.';
COMMENT ON COLUMN media.storage_quotas.used_bytes IS
    'Consommation courante (objets + variantes), maintenue par trigger. Recalculable par media.reconcile_storage_quotas().';

-- Quota par défaut : 5 Gio et 5 000 fichiers par organisation.
INSERT INTO media.storage_quotas (organization_id, max_bytes, max_files, note)
VALUES (NULL, 5368709120, 5000, 'Quota par défaut appliqué à toute organisation sans règle propre.')
ON CONFLICT DO NOTHING;

-- Consommation réelle, calculée à la demande (audit, réconciliation, tableau de bord).
CREATE OR REPLACE FUNCTION media.organization_storage_usage(p_organization_id uuid)
RETURNS TABLE (asset_bytes bigint, rendition_bytes bigint, total_bytes bigint, file_count bigint)
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(sum(a.byte_size), 0)::bigint,
           COALESCE(sum(r.bytes), 0)::bigint,
           (COALESCE(sum(a.byte_size), 0) + COALESCE(sum(r.bytes), 0))::bigint,
           count(*)::bigint
    FROM media.assets a
    LEFT JOIN LATERAL (
        SELECT COALESCE(sum(x.byte_size), 0) AS bytes
        FROM media.renditions x WHERE x.asset_id = a.id AND x.status = 'ready'
    ) r ON true
    WHERE a.owner_organization_id = p_organization_id AND a.deleted_at IS NULL;
$$;

COMMENT ON FUNCTION media.organization_storage_usage(uuid) IS
    'Consommation réelle d''une organisation (objets + variantes prêtes). Source de vérité pour la réconciliation.';

-- Application d'un delta au compteur. Crée la ligne de quota à la volée en
-- héritant des plafonds par défaut.
CREATE OR REPLACE FUNCTION media.apply_quota_delta(
    p_organization_id uuid,
    p_bytes           bigint,
    p_files           integer
)
RETURNS void
LANGUAGE plpgsql
AS $$
DECLARE
    v_max_bytes bigint;
    v_max_files integer;
BEGIN
    IF p_organization_id IS NULL OR (p_bytes = 0 AND p_files = 0) THEN
        RETURN;
    END IF;

    SELECT d.max_bytes, d.max_files INTO v_max_bytes, v_max_files
    FROM media.storage_quotas d WHERE d.organization_id IS NULL;

    INSERT INTO media.storage_quotas AS q (organization_id, max_bytes, max_files, used_bytes, used_files)
    VALUES (p_organization_id,
            COALESCE(v_max_bytes, 5368709120), COALESCE(v_max_files, 5000),
            GREATEST(p_bytes, 0), GREATEST(p_files, 0))
    ON CONFLICT (organization_id) DO UPDATE
       SET used_bytes = GREATEST(0, q.used_bytes + p_bytes),
           used_files = GREATEST(0, q.used_files + p_files),
           updated_at = now();
END;
$$;

COMMENT ON FUNCTION media.apply_quota_delta(uuid, bigint, integer) IS
    'Applique un delta au compteur de consommation. Crée la ligne de quota à la volée en héritant des plafonds par défaut.';

-- Espace occupé par les variantes réellement écrites d'un objet.
CREATE OR REPLACE FUNCTION media.rendition_bytes(p_asset_id uuid)
RETURNS bigint
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(sum(byte_size), 0)::bigint
    FROM media.renditions WHERE asset_id = p_asset_id AND status = 'ready';
$$;

-- Trigger unique pour les deux tables consommatrices d'espace.
--
-- Un objet et ses variantes forment un BLOC INDISSOCIABLE : libérer l'objet doit
-- libérer les variantes, sans quoi le compteur dérive à chaque suppression. Le
-- volet DELETE des objets s'exécute donc en BEFORE, seul moment où les variantes
-- sont encore comptables (la cascade les efface après). Symétriquement, le volet
-- variantes ne s'applique que tant que l'objet porteur est vivant : si celui-ci
-- vient d'être supprimé ou masqué, son propre trigger a déjà rendu le bloc entier.
CREATE OR REPLACE FUNCTION media.tg_track_storage_usage()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_old_org uuid;   v_new_org uuid;
    v_old_bytes bigint := 0; v_new_bytes bigint := 0;
    v_old_files integer := 0; v_new_files integer := 0;
    v_asset_id uuid;
    v_asset    media.assets%ROWTYPE;
BEGIN
    IF TG_TABLE_NAME = 'assets' THEN
        IF TG_OP <> 'INSERT' AND OLD.deleted_at IS NULL THEN
            v_old_org   := OLD.owner_organization_id;
            v_old_bytes := OLD.byte_size + media.rendition_bytes(OLD.id);
            v_old_files := 1;
        END IF;
        IF TG_OP <> 'DELETE' AND NEW.deleted_at IS NULL THEN
            v_new_org   := NEW.owner_organization_id;
            v_new_bytes := NEW.byte_size + media.rendition_bytes(NEW.id);
            v_new_files := 1;
        END IF;
    ELSE
        v_asset_id := CASE WHEN TG_OP = 'DELETE' THEN OLD.asset_id ELSE NEW.asset_id END;
        SELECT * INTO v_asset FROM media.assets a WHERE a.id = v_asset_id;
        IF NOT FOUND OR v_asset.deleted_at IS NOT NULL THEN
            RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
        END IF;

        IF TG_OP <> 'INSERT' AND OLD.status = 'ready' THEN
            v_old_org   := v_asset.owner_organization_id;
            v_old_bytes := COALESCE(OLD.byte_size, 0);
        END IF;
        IF TG_OP <> 'DELETE' AND NEW.status = 'ready' THEN
            v_new_org   := v_asset.owner_organization_id;
            v_new_bytes := COALESCE(NEW.byte_size, 0);
        END IF;
    END IF;

    IF v_old_org IS NOT DISTINCT FROM v_new_org THEN
        PERFORM media.apply_quota_delta(v_new_org, v_new_bytes - v_old_bytes, v_new_files - v_old_files);
    ELSE
        PERFORM media.apply_quota_delta(v_old_org, -v_old_bytes, -v_old_files);
        PERFORM media.apply_quota_delta(v_new_org,  v_new_bytes,  v_new_files);
    END IF;

    -- BEFORE DELETE sur les objets : renvoyer OLD, sous peine d'annuler la suppression.
    RETURN CASE WHEN TG_OP = 'DELETE' THEN OLD ELSE NEW END;
END;
$$;

CREATE TRIGGER tg_assets_storage_usage
    AFTER INSERT OR UPDATE OF byte_size, deleted_at, owner_organization_id ON media.assets
    FOR EACH ROW EXECUTE FUNCTION media.tg_track_storage_usage();

CREATE TRIGGER tg_assets_storage_release
    BEFORE DELETE ON media.assets
    FOR EACH ROW EXECUTE FUNCTION media.tg_track_storage_usage();

CREATE TRIGGER tg_renditions_storage_usage
    AFTER INSERT OR UPDATE OF byte_size, status OR DELETE ON media.renditions
    FOR EACH ROW EXECUTE FUNCTION media.tg_track_storage_usage();

-- Contrôle opposable au téléversement.
CREATE OR REPLACE FUNCTION media.has_storage_capacity(p_organization_id uuid, p_additional_bytes bigint)
RETURNS boolean
LANGUAGE sql
STABLE
AS $$
    SELECT CASE
        WHEN p_organization_id IS NULL THEN true
        ELSE COALESCE(
            (SELECT q.used_bytes + p_additional_bytes <= q.max_bytes AND q.used_files < q.max_files
             FROM media.storage_quotas q WHERE q.organization_id = p_organization_id),
            (SELECT p_additional_bytes <= d.max_bytes
             FROM media.storage_quotas d WHERE d.organization_id IS NULL),
            true)
    END;
$$;

CREATE OR REPLACE FUNCTION media.tg_enforce_quota()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NOT media.has_storage_capacity(NEW.owner_organization_id, NEW.byte_size) THEN
        RAISE EXCEPTION 'Quota de stockage atteint pour l''organisation %.', NEW.owner_organization_id
            USING ERRCODE = 'disk_full',
                  HINT = 'Supprimer des médias inutilisés ou demander un relèvement du quota.';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_assets_enforce_quota
    BEFORE INSERT ON media.assets
    FOR EACH ROW EXECUTE FUNCTION media.tg_enforce_quota();

-- Réconciliation périodique : réaligne les compteurs sur la réalité calculée.
CREATE OR REPLACE FUNCTION media.reconcile_storage_quotas()
RETURNS integer
LANGUAGE sql
AS $$
    WITH real_usage AS (
        SELECT q.id,
               COALESCE(sum(a.byte_size + COALESCE(r.bytes, 0)), 0)::bigint AS total_bytes,
               count(a.id)::integer AS file_count
        FROM media.storage_quotas q
        LEFT JOIN media.assets a
               ON a.owner_organization_id = q.organization_id AND a.deleted_at IS NULL
        LEFT JOIN LATERAL (
            SELECT sum(x.byte_size) AS bytes FROM media.renditions x
            WHERE x.asset_id = a.id AND x.status = 'ready'
        ) r ON true
        WHERE q.organization_id IS NOT NULL
        GROUP BY q.id
    ),
    updated AS (
        UPDATE media.storage_quotas q
        SET used_bytes = u.total_bytes,
            used_files = u.file_count,
            updated_at = now()
        FROM real_usage u
        WHERE q.id = u.id
          AND (q.used_bytes, q.used_files) IS DISTINCT FROM (u.total_bytes, u.file_count)
        RETURNING 1
    )
    SELECT count(*)::integer FROM updated;
$$;

COMMENT ON FUNCTION media.reconcile_storage_quotas() IS
    'Réaligne les compteurs incrémentaux sur la consommation réelle. Exécuté périodiquement par le worker.';

-- -----------------------------------------------------------------------------
-- 6. Orphelins et purge
--
-- Le mécanisme qui manquait totalement à la v1 : sans registre des usages, un
-- fichier retiré d'une page restait sur le disque pour toujours. Ici, l'absence
-- de rattachement est une information de première classe.
-- -----------------------------------------------------------------------------
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
    ORDER BY a.byte_size DESC;
$$;

COMMENT ON FUNCTION media.find_orphan_assets(integer) IS
    'Objets prêts, non rattachés depuis N jours : candidats à la purge. Réponse à la contrainte d''espace disque du VPS.';

-- Suppression logique avec fenêtre de rétention : l'objet reste récupérable
-- pendant la période indiquée avant que le worker ne l'efface du stockage.
CREATE OR REPLACE FUNCTION media.schedule_asset_purge(p_asset_id uuid, p_retention_days integer DEFAULT 30)
RETURNS void
LANGUAGE plpgsql
AS $$
BEGIN
    UPDATE media.assets
    SET deleted_at  = COALESCE(deleted_at, now()),
        deleted_by  = COALESCE(deleted_by, platform.current_actor_id()),
        purge_after = now() + make_interval(days => p_retention_days)
    WHERE id = p_asset_id AND purged_at IS NULL;

    PERFORM platform.emit_event('media', 'asset', p_asset_id, 'media.asset.purge_scheduled',
        jsonb_build_object('retention_days', p_retention_days));
END;
$$;

-- -----------------------------------------------------------------------------
-- 7. Déclaration des références vers les organisations
--
-- Le module s'inscrit au registre de fusion (cf. org.merge_organizations) :
-- absorber une organisation réaffecte ses médias sans toucher à cette fonction.
-- Le quota de la fiche absorbée est supprimé puis recalculé par réconciliation :
-- le réaffecter violerait l'unicité de organization_id.
-- -----------------------------------------------------------------------------
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy) VALUES
    ('media', 'assets',         'owner_organization_id', 'reassign'),
    ('media', 'storage_quotas', 'organization_id',       'delete')
ON CONFLICT DO NOTHING;

-- -----------------------------------------------------------------------------
-- 8. Amorçage : réglages et table blanche des rattachements
--
-- Ces lignes remplacent, à elles seules, les quinze colonnes d'URL de la v1.
-- -----------------------------------------------------------------------------
INSERT INTO platform.settings (key, value, description) VALUES
    ('media.public_base_url', '"https://media.epavillonclimatique.francophonie.org"',
     'Point d''accès public du stockage objet. Seule valeur à changer lors d''une migration Garage -> cloud.'),
    ('media.default_bucket', '"epavillon"', 'Bucket par défaut des nouveaux téléversements.'),
    ('media.orphan_retention_days', '30', 'Ancienneté minimale avant qu''un objet non rattaché soit proposé à la purge.')
ON CONFLICT (key) DO NOTHING;

INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size) VALUES
    ('org',         'organizations', 'logo',     '{"fr":"Logo","en":"Logo"}',                       false, '{image/*}',                       5242880),
    ('event',       'events',        'banner',   '{"fr":"Bannière","en":"Banner"}',                 false, '{image/*}',                      15728640),
    ('event',       'events',        'gallery',  '{"fr":"Galerie","en":"Gallery"}',                 true,  '{image/*}',                      15728640),
    ('programme',   'proposals',     'cover',    '{"fr":"Image de couverture","en":"Cover image"}', false, '{image/*}',                      10485760),
    ('programme',   'proposals',     'document', '{"fr":"Document joint","en":"Attached document"}',true,  '{application/pdf,application/vnd.*}', 26214400),
    ('identity',    'people',        'avatar',   '{"fr":"Photo de profil","en":"Profile picture"}', false, '{image/*}',                       5242880),
    ('publication', 'articles',      'cover',    '{"fr":"Image de couverture","en":"Cover image"}', false, '{image/*}',                      10485760)
ON CONFLICT DO NOTHING;
