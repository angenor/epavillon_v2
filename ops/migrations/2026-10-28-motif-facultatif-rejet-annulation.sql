-- Rejeter ou annuler un dossier sans motif (28/10). Rejouable.
-- Modèle : docs/database/070_programme_proposals.sql § 1 (transitions autorisées),
-- docs/database/075_programme_sessions.sql (programme.sessions).
--
-- Demande du commanditaire : le motif d'un rejet ou d'une annulation n'est plus
-- obligatoire, l'écran rappelle seulement qu'il manque. Une demande de
-- corrections en exige toujours un. Annuler un dossier retenu annule ses
-- séances publiées avec son motif : une séance annulée peut donc n'en avoir
-- aucun.
--
-- Déployer avec l'API : sans elle, l'annulation d'un dossier sans motif
-- laisserait ses séances publiées à l'affiche.
BEGIN;

UPDATE programme.proposal_transitions_allowed
   SET requires_reason = false
 WHERE (from_status, to_status) IN (('under_review', 'rejected'),
                                    ('changes_requested', 'rejected'),
                                    ('accepted', 'cancelled'));

ALTER TABLE programme.sessions DROP CONSTRAINT IF EXISTS ck_sessions_cancelled_reason;

COMMIT;
