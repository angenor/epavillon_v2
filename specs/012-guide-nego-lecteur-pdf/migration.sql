-- =============================================================================
-- ePavillon v2 — migration de l'étape 1b de Guide Négo : le lecteur montre le PDF
--
-- Du schéma en service après l'étape 1 (specs/011-guide-nego-documents/
-- migration.sql) vers celui de docs/database/ après l'étape 1b.
--
-- POURQUOI CE FICHIER EXISTE
--   `docs/database/` n'est chargé qu'à la création du volume : une base qui
--   porte des comptes se migre, selon le § 13 de docs/DEPLOIEMENT.md. Il vaut
--   AUSSI en local. **`down -v` n'est jamais la procédure.**
--
-- REJOUABLE SANS DÉGÂT
--   Le renommage ne se fait qu'une fois, et c'est lui seul qui efface les choix
--   « tel quel » : rejouer le script ne touche pas un choix « Texte agrandi »
--   posé depuis. Les commentaires sont repris MOT POUR MOT du modèle.
--
-- APRÈS
--   `reading_bytes` se recalcule par une relance d'extraction des documents
--   publiés (POST …/extraction), pas ici : le poids du JSON est celui que
--   l'API sert (DEPLOIEMENT.md § 15).
--
-- CONTRÔLE APRÈS COUP (§ 13, étape 6)
--   pg_dump --schema-only de cette base, comparé à celui d'une base chargée
--   depuis docs/database/. Comparer après tri des lignes.
-- =============================================================================

BEGIN;

-- -----------------------------------------------------------------------------
-- 1. « Ouvrir tel quel » devient « Texte agrandi » (R11)
-- -----------------------------------------------------------------------------

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
         WHERE table_schema = 'negotiation' AND table_name = 'document_renditions'
           AND column_name = 'serve_as_is'
    ) THEN
        ALTER TABLE negotiation.document_renditions RENAME COLUMN serve_as_is TO large_text_choice;
        ALTER TABLE negotiation.document_renditions ALTER COLUMN large_text_choice DROP NOT NULL;
        ALTER TABLE negotiation.document_renditions ALTER COLUMN large_text_choice DROP DEFAULT;
        -- Les choix « tel quel » des postes d'essai n'ont pas de sens ici :
        -- tout document repart du verdict de l'extraction.
        UPDATE negotiation.document_renditions SET large_text_choice = NULL;
    END IF;
END;
$$;

COMMENT ON TABLE negotiation.document_renditions IS
    'L''extraction d''un document fichier : son état, son verdict, son sommaire, et le choix « Texte agrandi ». Une ligne par document.';
COMMENT ON COLUMN negotiation.document_renditions.large_text_choice IS
    'Proposer « Texte agrandi » au téléphone. NULL = suit le verdict de l''extraction (is_reflowable) ; vrai ou faux = choix de l''administratrice, qui se change sans republier (arbitré le 24/09). Une relance d''extraction le garde. Lu par negotiation.document_reading_modes().';
COMMENT ON COLUMN negotiation.document_renditions.reading_bytes IS
    'Octets du PDF + octets du JSON de lecture : la copie gardée depuis l''étape 1b (ADR-022). Calculé à la fin de l''extraction ; c''est la taille annoncée sur la fiche.';

-- -----------------------------------------------------------------------------
-- 2. Les images de pages : l'aperçu du back-office seul
-- -----------------------------------------------------------------------------

COMMENT ON COLUMN negotiation.document_pages.image_key IS
    'Image de la page dans le bucket privé : aperçu du back-office — jamais servi au téléphone depuis l''étape 1b (ADR-022).';
COMMENT ON COLUMN negotiation.document_pages.image_bytes IS
    'Poids de l''image de la page : aperçu du back-office — jamais servi au téléphone depuis l''étape 1b.';
COMMENT ON COLUMN negotiation.document_pages.has_origin_block IS
    'La page porte un tableau ou une figure : « Texte agrandi » y renvoie à la page d''origine, et l''aperçu du back-office la montre. Aperçu du back-office — jamais d''image servie au téléphone depuis l''étape 1b.';

-- -----------------------------------------------------------------------------
-- 3. La règle « Texte agrandi », écrite une fois
-- -----------------------------------------------------------------------------

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

COMMIT;
