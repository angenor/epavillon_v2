-- Les textes et les intervenants d'un dossier suivent sur l'activité (31/10). Rejouable.
-- Modèle : docs/database/075_programme_sessions.sql (commentaires seulement ; la
-- recopie est faite par l'API à chaque enregistrement du dossier).
COMMENT ON COLUMN programme.sessions.title IS
    'Titre public. Recopié du dossier à l''acceptation, puis à chaque enregistrement du dossier, par l''équipe comme par l''organisation (arbitrage du 31/10).';
COMMENT ON COLUMN programme.sessions.description IS
    'Présentation publique, en HTML restreint comme programme.proposals.detailed_presentation, dont elle est recopiée à l''acceptation puis à chaque enregistrement du dossier (31/10).';
