-- Envoi de courriels en fenêtre unique : gabarit, valeurs de secours des
-- variables, bouton Répondre, bouton d'action (28/10, après usage en
-- production). Rejouable.
-- Modèle : docs/database/110_engagement.sql § 12, engagement.mailings.
--
-- Additive : cinq colonnes et leurs contraintes. Déployer avec l'API et le
-- worker, qui rendent les gabarits.
BEGIN;

ALTER TABLE engagement.mailings
    ADD COLUMN IF NOT EXISTS template text NOT NULL DEFAULT 'sobre',
    ADD COLUMN IF NOT EXISTS variable_fallbacks jsonb NOT NULL DEFAULT '{"prenom": "Madame, Monsieur"}'::jsonb,
    ADD COLUMN IF NOT EXISTS reply_button boolean NOT NULL DEFAULT false,
    ADD COLUMN IF NOT EXISTS cta_label text,
    ADD COLUMN IF NOT EXISTS cta_url platform.url;

ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS ck_mailings_template;
ALTER TABLE engagement.mailings ADD CONSTRAINT ck_mailings_template
    CHECK (template IN ('sobre', 'institutionnel', 'annonce'));
ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS ck_mailings_fallbacks;
ALTER TABLE engagement.mailings ADD CONSTRAINT ck_mailings_fallbacks
    CHECK (jsonb_typeof(variable_fallbacks) = 'object');
ALTER TABLE engagement.mailings DROP CONSTRAINT IF EXISTS ck_mailings_cta;
ALTER TABLE engagement.mailings ADD CONSTRAINT ck_mailings_cta
    CHECK ((cta_label IS NULL) = (cta_url IS NULL));

COMMIT;
