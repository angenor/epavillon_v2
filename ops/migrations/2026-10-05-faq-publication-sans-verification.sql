-- Une réponse de la FAQ de Guide Négo se publie sans vérification datée : l'administration
-- publie sur instruction de sa hiérarchie. Rejouable. Modèle : docs/database/100_negotiations.sql.
BEGIN;
ALTER TABLE negotiation.faq_entries DROP CONSTRAINT IF EXISTS ck_faq_entries_verified;
COMMENT ON TYPE negotiation.knowledge_status IS
    'draft → published (la FAQ se publie avec ou sans vérification datée, tranché le 05/10) ; published ⇄ to_review ; published | to_review → draft (dépublier). to_review reste servi au téléphone, avec sa mention (tranché le 25/09).';
COMMENT ON TABLE negotiation.faq_entries IS
    'Entrées de la FAQ de Guide Négo. La vérification datée et signée par un expert est facultative : l''administration peut publier sur instruction de sa hiérarchie, sans elle (tranché le 05/10). Publiée une fois, une entrée ne se supprime plus.';
COMMENT ON COLUMN negotiation.faq_entries.verified_on IS
    '« Vérifié le » : posé avec verified_by par un expert. NULL : jamais vérifiée, le téléphone ne dit alors pas « Vérifié le ».';
COMMIT;
