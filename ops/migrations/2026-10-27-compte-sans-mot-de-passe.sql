-- Comptes repris de la v1 sans empreinte de mot de passe (décision D1, 27/10).
-- Rejouable. Modèle : docs/database/030_identity.sql § 2.
--
-- 1. identity.accounts.password_reset_required_at : le compte doit choisir un
--    nouveau mot de passe. Colonne ajoutée en dernier, comme dans le modèle.
-- 2. ck_accounts_password_shape : un compte 'password' peut être sans empreinte
--    si, et seulement si, cette date est posée.
--
-- Aucune fonction SQL ne change : l'effacement de l'exigence et la vérification
-- de l'adresse à la réinitialisation sont dans l'API. Déployer l'API avec.
BEGIN;

ALTER TABLE identity.accounts ADD COLUMN IF NOT EXISTS password_reset_required_at timestamptz;

ALTER TABLE identity.accounts DROP CONSTRAINT IF EXISTS ck_accounts_password_shape;
ALTER TABLE identity.accounts ADD CONSTRAINT ck_accounts_password_shape
    CHECK ((provider = 'password' AND provider_subject IS NULL
            AND (password_hash IS NOT NULL OR password_reset_required_at IS NOT NULL))
        OR (provider <> 'password' AND provider_subject IS NOT NULL));

COMMENT ON COLUMN identity.accounts.password_hash IS
    'Empreinte Argon2id calculée par l''API. Aucune fonction SQL ne doit vérifier de mot de passe. NULL pour un compte ''password'' seulement si password_reset_required_at est posée.';
COMMENT ON COLUMN identity.accounts.password_reset_required_at IS
    'Depuis quand ce compte doit choisir un nouveau mot de passe avant de se connecter (comptes repris de la v1, sans empreinte). La connexion le traite comme un mot de passe faux, sans rien dire de plus. L''enregistrement d''un mot de passe (réinitialisation) efface la date.';

COMMIT;
