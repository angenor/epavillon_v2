-- Durée minimale d'une activité ramenée à 10 minutes (06/11/2026), à la demande
-- du commanditaire : « COP Bike Ride » ne se corrigeait plus au back-office
-- (« La durée doit être comprise entre 45 et 150 minutes »). L'appel garde ses
-- bornes (le formulaire propose toujours 45 à 150) ; l'API accepte dès 10
-- minutes, et le garde-fou de la colonne passe de 15 à 10.
-- Rejouable. À jouer AVANT l'API, sans quoi une durée de 10 à 14 minutes
-- acceptée par l'API serait refusée par la base.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

ALTER TABLE programme.proposals DROP CONSTRAINT IF EXISTS proposals_duration_minutes_check;
ALTER TABLE programme.proposals ADD CONSTRAINT proposals_duration_minutes_check
    CHECK (duration_minutes IS NULL OR duration_minutes BETWEEN 10 AND 600);

COMMIT;
