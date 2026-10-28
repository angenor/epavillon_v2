-- Galerie de photos d'une activité (reprise v1, D7, 28/10). Rejouable.
-- Modèle : docs/database/050_media.sql (media.attachments.caption_override,
-- media.attached_image(), media.attached_images(), media.attachable_roles).
--
-- Une activité (programme.sessions) porte une galerie : rôle `gallery`,
-- plusieurs photos, ordre réglable, texte alternatif obligatoire, légende sur
-- le rattachement. La page publique d'une activité lit les photos prêtes et
-- publiques par media.attached_images().
-- Déployer l'API avec : elle sert la galerie dans le détail public d'une
-- séance, et sa garde réserve ce rôle à l'équipe qui planifie l'édition.
-- Ensuite seulement, le lot 96 (ops/reprise-v1/lots/96-galeries.sh).
BEGIN;

ALTER TABLE media.attachments ADD COLUMN IF NOT EXISTS caption_override platform.i18n_text;

COMMENT ON COLUMN media.attachments.caption_override IS
    'Légende propre à cet usage ; prime sur media.assets.caption. Une galerie y écrit la sienne.';

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
               'caption',  COALESCE(t.caption_override, a.caption),
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

CREATE OR REPLACE FUNCTION media.attached_images(
    p_owner_schema text,
    p_owner_table  text,
    p_owner_id     uuid,
    p_role         media.attachment_role DEFAULT 'gallery'
)
RETURNS jsonb
LANGUAGE sql
STABLE
AS $$
    SELECT COALESCE(jsonb_agg(jsonb_build_object(
               'asset_id', a.id,
               'url',      media.object_url(a.bucket, a.object_key),
               'width',    a.width,
               'height',   a.height,
               'alt_text', COALESCE(t.alt_text_override, a.alt_text),
               'caption',  COALESCE(t.caption_override, a.caption),
               'credit',   a.credit,
               'sources',  media.asset_sources(a.id))
             ORDER BY t.sort_order, t.created_at), '[]'::jsonb)
    FROM media.attachments t
    JOIN media.assets a ON a.id = t.asset_id
    WHERE t.owner_schema = p_owner_schema
      AND t.owner_table  = p_owner_table
      AND t.owner_id     = p_owner_id
      AND t.role         = p_role
      AND a.deleted_at IS NULL
      AND a.status = 'ready'
      AND a.visibility = 'public';
$$;

COMMENT ON FUNCTION media.attached_images(text, text, uuid, media.attachment_role) IS
    'Images publiques et servables d''un rôle multiple (galerie), dans l''ordre réglé. Même forme d''élément que media.attached_image(). Tableau vide sans image.';

INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size, expected_aspect_ratio)
VALUES
    ('programme', 'sessions', 'gallery', '{"fr":"Galerie de l''activité","en":"Activity gallery"}',
     true, '{image/*}', 15728640, NULL)
ON CONFLICT DO NOTHING;

COMMIT;
