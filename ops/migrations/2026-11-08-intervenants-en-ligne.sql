-- Intervenants en ligne (08/11/2026), demande du commanditaire pendant la
-- CdP31 : chaque activité a un lien d'inscription d'intervenant, l'intervenant
-- reçoit son lien Zoom personnel (qui ouvre l'application), la régie inscrit
-- et renvoie. Modèle : 075 § 2 bis, 080 (places des intervenants), 110
-- (copie des courriels).
--   1. programme.sessions.speaker_link_token : un jeton par activité ;
--   2. programme.session_speakers : source, is_listed, country_id ;
--   3. live.meeting_participants : link_requested_at, speaker_session_id ;
--   4. réglages : copie (Angenor, Issa) et envois d'office ÉTEINTS.
-- Rejouable. À jouer AVANT l'API, qui lit ces colonnes.
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;
SELECT set_config('app.actor_id', '01a0627c-f3be-7185-89c8-9a4015ad158e', true),
       set_config('app.request_id', 'script:2026-11-08-intervenants-en-ligne', true);

ALTER TABLE programme.sessions
    ADD COLUMN IF NOT EXISTS speaker_link_token text NOT NULL DEFAULT encode(gen_random_bytes(24), 'hex');
CREATE UNIQUE INDEX IF NOT EXISTS ux_sessions_speaker_link_token ON programme.sessions (speaker_link_token);
COMMENT ON COLUMN programme.sessions.speaker_link_token IS
    'Jeton du lien d''inscription des intervenants en ligne (/intervenant?jeton=). En clair : il se renvoie à l''identique au porteur. Sans effet après la fin de l''activité.';

ALTER TABLE programme.session_speakers
    ADD COLUMN IF NOT EXISTS source text NOT NULL DEFAULT 'dossier'
        CONSTRAINT ck_session_speakers_source CHECK (source IN ('dossier', 'regie', 'lien')),
    ADD COLUMN IF NOT EXISTS is_listed boolean NOT NULL DEFAULT true,
    ADD COLUMN IF NOT EXISTS country_id uuid REFERENCES reference.countries(id) ON DELETE SET NULL;
COMMENT ON COLUMN programme.session_speakers.source IS
    'dossier : recopié du dossier et tenu sur lui ; regie : inscrit par l''équipe pendant l''événement ; lien : inscrit par le lien d''intervenant de l''activité.';
COMMENT ON COLUMN programme.session_speakers.is_listed IS
    'Figure sur la page publique de l''activité. Faux pour un inscrit par le lien, jusqu''au clic « Afficher sur la page ».';
COMMENT ON COLUMN programme.session_speakers.country_id IS
    'Pays déclaré à l''inscription d''un intervenant en ligne.';

ALTER TABLE live.meeting_participants
    ADD COLUMN IF NOT EXISTS link_requested_at timestamptz,
    ADD COLUMN IF NOT EXISTS speaker_session_id uuid CONSTRAINT xmod_fk_meeting_participants_speaker_session
        REFERENCES programme.sessions(id) ON DELETE SET NULL;
COMMENT ON COLUMN live.meeting_participants.link_requested_at IS
    'Dernière demande d''envoi du lien Zoom personnel à un intervenant (veille de l''activité, inscription, renvoi par la régie). Nulle : aucun courriel de lien dû.';
COMMENT ON COLUMN live.meeting_participants.speaker_session_id IS
    'Activité où la personne intervient, celle que nomme le courriel de son lien Zoom.';

INSERT INTO platform.settings (key, value, description) VALUES
    ('engagement.speaker_links.cc', '["angenor99@gmail.com", "issa.bado@francophonie.org"]',
     'Adresses en copie de chaque courriel d''intervenant en ligne : lien Zoom personnel, lien d''inscription envoyé au porteur.'),
    ('live.speaker_links.auto', 'false',
     'Envoie d''office le lien d''inscription des intervenants au porteur 2 jours avant chaque activité diffusée, et son lien Zoom à chaque intervenant la veille. Les gestes de la régie partent dans tous les cas.')
ON CONFLICT (key) DO NOTHING;

DO $$
DECLARE v_n int;
BEGIN
    SELECT count(*) INTO v_n FROM programme.sessions;
    RAISE NOTICE 'Activités pourvues d''un lien d''inscription d''intervenant : %', v_n;
END;
$$;

COMMIT;
