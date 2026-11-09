-- Une personne inscrite sans compte à une activité seule n'avait aucun moyen
-- d'entrer : la salle exige une connexion. Comme pour les journées, le courriel
-- porte désormais un lien personnel, dont la base ne garde que l'empreinte.
-- À jouer AVANT la mise en ligne : l'API lit cette colonne.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

ALTER TABLE programme.registrations
    ADD COLUMN IF NOT EXISTS manage_token_hash bytea;

ALTER TABLE programme.registrations
    DROP CONSTRAINT IF EXISTS ck_registrations_token,
    ADD CONSTRAINT ck_registrations_token
        CHECK (manage_token_hash IS NULL OR octet_length(manage_token_hash) = 32);

CREATE UNIQUE INDEX IF NOT EXISTS ux_registrations_manage_token
    ON programme.registrations (manage_token_hash)
    WHERE manage_token_hash IS NOT NULL;

COMMENT ON COLUMN programme.registrations.manage_token_hash IS
    'Empreinte SHA-256 du jeton du lien d''entrée dans le direct d''une inscription sans compte (09/11). Le jeton en clair ne vit que dans le courriel ; un nouvel envoi remplace l''empreinte, et l''ancien lien cesse de valoir. Jamais rendue par l''API.';

COMMIT;
