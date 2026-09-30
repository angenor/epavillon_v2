-- Couleurs des thématiques : la programmation publique les affichait en gris.
-- Modèle : docs/database/020_reference.sql. Ne remplit que les couleurs vides.
BEGIN;
UPDATE reference.taxonomy_terms AS t
SET color_hex = v.color_hex
FROM (VALUES
    ('mitigation', '#00A1E4'),
    ('adaptation', '#F28C28'),
    ('climate_ambition_ndc', '#5B6CD9'),
    ('loss_and_damage', '#A8456B'),
    ('water_fisheries', '#14A3A3'),
    ('renewable_energy_land', '#E3B51B'),
    ('health_solidarity', '#F2A07B'),
    ('industry_transition_and_technology', '#7A8795'),
    ('transport_urbanization', '#9C7BD6'),
    ('climate_justice_indigenous', '#A0522D'),
    ('agriculture_food', '#A3B82C'),
    ('sustainable_livestock', '#8B6B3E'),
    ('climate_finance', '#2E8B57'),
    ('gender', '#D85FA8'),
    ('transparency', '#4A90A4'),
    ('biodiversity', '#57A639'),
    ('desertification', '#C8A165')
) AS v(code, color_hex)
WHERE t.taxonomy_code = 'activity_theme'
  AND t.code = v.code
  AND t.color_hex IS NULL;
COMMIT;
