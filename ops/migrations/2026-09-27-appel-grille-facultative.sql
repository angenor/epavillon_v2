-- Appel à propositions : formats retirés (le mode de l'édition fait foi), revues visées
-- facultatives, grille de critères désactivable. Modèle : docs/database/060_events.sql.
BEGIN;
ALTER TABLE event.calls_for_proposals DROP COLUMN allowed_formats;
ALTER TABLE event.calls_for_proposals ALTER COLUMN required_reviews DROP NOT NULL;
ALTER TABLE event.calls_for_proposals ALTER COLUMN required_reviews DROP DEFAULT;
ALTER TABLE event.calls_for_proposals ADD COLUMN uses_scoring_grid boolean NOT NULL DEFAULT true;
COMMENT ON COLUMN event.calls_for_proposals.uses_scoring_grid IS
    'Grille de critères pondérés en usage. Faux : ni le site public ni l''évaluation ne la montrent, l''évaluation se fait par la note sur 20.';
COMMENT ON COLUMN event.calls_for_proposals.required_reviews IS
    'Nombre de revues visé par dossier : objectif d''avancement, jamais un préalable à la décision. Nul = aucun objectif.';
CREATE OR REPLACE FUNCTION event.activity_formats(p_mode event.participation_mode)
RETURNS event.participation_mode[]
LANGUAGE sql
IMMUTABLE
AS $$
    SELECT CASE p_mode
        WHEN 'hybrid' THEN ARRAY['in_person', 'online']::event.participation_mode[]
        ELSE ARRAY[p_mode]
    END;
$$;
COMMIT;
