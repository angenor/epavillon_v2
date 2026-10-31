-- Atelier de l'image de couverture : couleur libre (31/10). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql (programme.proposal_cover_designs).
--
-- La couleur accepte aussi `#rrggbb` en minuscules. Les logos IFDD/OIF et
-- celui de l'organisation deviennent deux clés de `options` : aucune reprise,
-- l'API lit l'ancienne `show_logos` pour les deux tant qu'elles manquent.
BEGIN;

ALTER TABLE programme.proposal_cover_designs DROP CONSTRAINT IF EXISTS ck_proposal_cover_designs_accent;
ALTER TABLE programme.proposal_cover_designs ADD CONSTRAINT ck_proposal_cover_designs_accent
    CHECK (accent ~ '^([a-z][a-z0-9-]{1,40}|#[0-9a-f]{6})$');

COMMIT;
