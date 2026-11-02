-- Copie visible des courriels automatiques à l'équipe (02/11/2026), à la
-- demande du commanditaire : la boîte d'expédition ne se lit pas encore.
-- Réglage lu par GardedMailer à chaque envoi ; sans effet avant la mise en
-- ligne du code qui le lit. Le vider ('[]') coupe la copie sans redéploiement.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
INSERT INTO platform.settings (key, value, description) VALUES
    ('engagement.mail.automatic_cc', '["angenor99@gmail.com", "issa.bado@francophonie.org"]',
     'Adresses en copie visible de chaque courriel automatique, sauf ceux qui portent un lien personnel ou d''inscription (création de compte, mot de passe, invitation, confirmations d''inscription) ; un rappel n''est copié qu''une fois par activité et par délai. Lu par GardedMailer.')
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, description = EXCLUDED.description;
COMMIT;
