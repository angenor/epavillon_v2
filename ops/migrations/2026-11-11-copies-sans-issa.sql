-- Issa Bado retiré de toutes les copies automatiques (11/11/2026, demande du
-- commanditaire) : rappels et courriels automatiques, liens des intervenants en
-- ligne, copie proposée des envois. Angenor reste. Idempotent.
\set ON_ERROR_STOP on
BEGIN;

UPDATE platform.settings
   SET value = (SELECT coalesce(jsonb_agg(a), '[]'::jsonb)
                  FROM jsonb_array_elements_text(value::jsonb) a
                 WHERE lower(a) <> 'issa.bado@francophonie.org')
 WHERE key IN ('engagement.mail.automatic_cc', 'engagement.speaker_links.cc', 'engagement.mailing.team_cc')
   AND jsonb_typeof(value::jsonb) = 'array';

SELECT key, value FROM platform.settings
 WHERE key IN ('engagement.mail.automatic_cc', 'engagement.speaker_links.cc', 'engagement.mailing.team_cc')
 ORDER BY key;

COMMIT;
