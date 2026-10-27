-- Lot 00 : la personne technique « Reprise v1 », auteur (app.actor_id) de toutes
-- les écritures des lots. Sans compte : elle ne se connecte jamais.
-- Identifiant fixe, à recopier dans chaque lot :
--   01a00000-0000-7000-8000-00000000f1a1
-- Adresse en .invalid (RFC 2606) : exigée par identity.people, jamais distribuable.

\set ON_ERROR_STOP on
BEGIN;
DO $$ BEGIN PERFORM set_config('app.request_id', 'reprise-v1-00', true); END $$;

INSERT INTO identity.people (id, primary_email, first_name, last_name,
                             preferred_locale, timezone, status, is_directory_visible)
VALUES ('01a00000-0000-7000-8000-00000000f1a1', 'reprise-v1@epavillon.invalid',
        'Reprise', 'v1', 'fr', 'UTC', 'active', false);

DO $$ BEGIN PERFORM set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true); END $$;
COMMIT;
