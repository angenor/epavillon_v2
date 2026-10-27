-- Cale de répétition, hors lot : retouche D1 du modèle (plan § 5), en attendant
-- qu'elle arrive par docs/database/030_identity.sql et sa migration. À jouer sur
-- un clone du banc seulement, avant le lot 20. À retirer dès que la migration
-- est au banc.

\set ON_ERROR_STOP on
BEGIN;

ALTER TABLE identity.accounts
    ADD COLUMN password_reset_required_at timestamptz;

ALTER TABLE identity.accounts DROP CONSTRAINT ck_accounts_password_shape;
ALTER TABLE identity.accounts ADD CONSTRAINT ck_accounts_password_shape
    CHECK ((provider = 'password' AND provider_subject IS NULL
            AND (password_hash IS NOT NULL OR password_reset_required_at IS NOT NULL))
        OR (provider <> 'password' AND provider_subject IS NOT NULL));

COMMIT;
