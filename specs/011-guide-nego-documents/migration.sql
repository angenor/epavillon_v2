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
--   rien, et ne fait échouer aucune étape déjà passée.
--
-- ORDRE
--   1. media : le bucket privé et le contrat de lecture (phase 2)
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

COMMIT;
