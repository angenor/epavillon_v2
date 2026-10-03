-- La séance recopie la présentation détaillée de son dossier ; les séances déjà
-- nées la reçoivent. Modèle : docs/database/075_programme_sessions.sql.
BEGIN;
COMMENT ON COLUMN programme.sessions.title IS
    'Titre public. Recopié du dossier à l''acceptation, puis à chaque correction du dossier par l''équipe — jamais par une correction de l''organisation (FR-091).';
COMMENT ON COLUMN programme.sessions.description IS
    'Présentation publique, en HTML restreint comme programme.proposals.detailed_presentation, dont elle est recopiée à l''acceptation puis à chaque correction du dossier par l''équipe.';

UPDATE programme.sessions s
   SET description = p.detailed_presentation
  FROM programme.proposals p
 WHERE s.proposal_id = p.id
   AND s.description IS NULL;
COMMIT;
