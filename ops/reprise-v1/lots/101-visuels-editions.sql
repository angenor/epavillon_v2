-- Lot 101 (phase 2) : bandeau et logo de la CdP30, logo de la CdP31 (29/10).
--
-- Trois fichiers, trois objets, trois rattachements à `event.events` :
--   CdP30, rôle `banner` : la photo de Belém (marché Ver-o-Peso, 1920 x 1080),
--          relevée sur l'ancien serveur le 28/10 (`images_uploades`), absente de
--          Supabase ;
--   CdP30, rôle `logo`   : le logo officiel de la COP30, même provenance ;
--   CdP31, rôle `logo`   : `v1.events.logo_url`, un objet Supabase consigné
--          « skipped » au lot 60 faute de rôle (migration
--          2026-10-29-logo-edition-et-gabarits, qui pose le rôle `logo`).
-- Le bandeau de la CdP31 est déjà posé (lot 60). Le bandeau CdP30 est un 16:9,
-- comme celui de la CdP31 : le rôle annonce un 32:9, mais les dimensions ne
-- sont connues qu'après le traitement du worker, et la forme ne se contrôle
-- qu'à la pose (tg_validate_attachment).
--
-- `lot101_fichier`, écrit par 101-visuels-editions.sh : clé, chemin sous
-- FICHIERS_V1, taille, type, sha256, et pour la CdP31 l'objet v1. Identifiants
-- d'objets et de rattachements fixés ici, les mêmes sur le clone qui dépose
-- les fichiers dans Garage et en production. Sur un clone local, `-v depot=1`
-- écrit `legacy.fichiers_a_deposer` pour 60-fichiers.sh.
-- Rien ne part : `media.asset.uploaded` marqué publié, comme aux lots 60 et 96.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-101', true) \g /dev/null

CREATE TEMP TABLE l101 ON COMMIT DROP AS
SELECT f.*, v.edition_v1, v.role, v.asset_id, v.rattachement_id, v.alt_text,
       legacy.map('public.events', v.edition_v1::text) AS event_id
FROM lot101_fichier f
FULL JOIN (VALUES
    ('cdp30_banniere', '8303bbfa-fb3a-4da2-9e16-19d1d0db66f8'::uuid, 'banner'::media.attachment_role,
     '01a18709-cc36-71b3-82ab-081e573f260e'::uuid, '01a18709-cc37-75ac-b8c7-bb0018515430'::uuid,
     'Le marché Ver-o-Peso de Belém au coucher du soleil, et le logo de la COP30 Amazônia'),
    ('cdp30_logo', '8303bbfa-fb3a-4da2-9e16-19d1d0db66f8'::uuid, 'logo'::media.attachment_role,
     '01a18709-cc37-7811-ab4a-b04a8b519fce'::uuid, '01a18709-cc37-737e-9030-c3314cdce02d'::uuid,
     'Logo officiel de la COP30, Belém, Brésil'),
    ('cdp31_logo', 'd03adb7b-b51c-4e78-8fcd-7f956272c30d'::uuid, 'logo'::media.attachment_role,
     '01a18709-cc37-7248-86f0-3839f7d4e93d'::uuid, '01a18709-cc37-744a-8595-04f51cbef8a7'::uuid,
     'Logo de la COP31, Türkiye, Antalya')
) AS v(cle, edition_v1, role, asset_id, rattachement_id, alt_text) USING (cle);

DO $$
DECLARE
    v_seau text := (SELECT value #>> '{}' FROM platform.settings WHERE key = 'media.default_bucket');
BEGIN
    PERFORM set_config('lot101.seau', v_seau, true);
    IF EXISTS (SELECT 1 FROM l101 WHERE fichier IS NULL OR asset_id IS NULL) THEN
        RAISE EXCEPTION 'lot 101 : les fichiers du flux ne sont pas ceux du lot';
    END IF;
    IF EXISTS (SELECT 1 FROM l101 WHERE sha256 IS NULL OR mime NOT IN ('image/jpeg', 'image/png')) THEN
        RAISE EXCEPTION 'lot 101 : fichier absent du flux ou qui n''est ni un JPEG ni un PNG';
    END IF;
    IF EXISTS (SELECT 1 FROM l101 WHERE event_id IS NULL) THEN
        RAISE EXCEPTION 'lot 101 : édition v1 sans édition en v2 (lot 40 joué ?)';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM media.attachable_roles
                   WHERE owner_schema = 'event' AND owner_table = 'events' AND role = 'logo') THEN
        RAISE EXCEPTION 'lot 101 : rôle « logo » absent (migration 2026-10-29-logo-edition-et-gabarits jouée ?)';
    END IF;
    IF EXISTS (SELECT 1 FROM l101 x JOIN media.assets a ON a.id = x.asset_id)
       OR EXISTS (SELECT 1 FROM l101 x JOIN media.attachments a ON a.id = x.rattachement_id)
       OR EXISTS (SELECT 1 FROM legacy.id_map WHERE legacy_table = 'ancien_serveur.images_uploades') THEN
        RAISE EXCEPTION 'lot 101 : visuels déjà posés, le lot ne se joue qu''une fois';
    END IF;
    IF EXISTS (SELECT 1 FROM l101 x JOIN media.attachments t
               ON t.owner_schema = 'event' AND t.owner_table = 'events'
              AND t.owner_id = x.event_id AND t.role = x.role) THEN
        RAISE EXCEPTION 'lot 101 : une édition porte déjà ce visuel, à trancher';
    END IF;
    IF EXISTS (SELECT 1 FROM l101 WHERE media.find_by_checksum(sha256, v_seau) IS NOT NULL)
       OR EXISTS (SELECT 1 FROM l101 GROUP BY sha256 HAVING count(*) > 1) THEN
        RAISE EXCEPTION 'lot 101 : contenu déjà présent en v2 ou en double, à trancher';
    END IF;
    IF (SELECT count(*) FROM l101 x JOIN legacy.id_map m
        ON m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1::text
        AND m.disposition = 'skipped' AND m.note LIKE '%logo d''événement%') <> 1 THEN
        RAISE EXCEPTION 'lot 101 : logo v1 de la CdP31 non consigné comme écarté au lot 60';
    END IF;
END $$;

INSERT INTO media.assets (id, bucket, object_key, checksum_sha256, mime_type, byte_size,
                          original_filename, owner_person_id, visibility, status, alt_text,
                          created_at, updated_at)
SELECT x.asset_id, current_setting('lot101.seau'),
       to_char(now() AT TIME ZONE 'UTC', 'YYYY/MM') || '/' || replace(x.asset_id::text, '-', '')
           || '/' || replace(lower(regexp_replace(x.fichier, '^.*/', '')), '_', '-'),
       x.sha256, x.mime, x.taille, regexp_replace(x.fichier, '^.*/', ''),
       '01a00000-0000-7000-8000-00000000f1a1', 'public', 'uploaded',
       jsonb_build_object('fr', x.alt_text), now(), now()
FROM l101 x;

INSERT INTO media.attachments (id, owner_schema, owner_table, owner_id, asset_id, role, sort_order, created_by)
SELECT rattachement_id, 'event', 'events', event_id, asset_id, role, 0,
       '01a00000-0000-7000-8000-00000000f1a1'
FROM l101;

-- Correspondance ------------------------------------------------------------------

UPDATE legacy.id_map m
SET disposition = 'migrated', target_id = x.asset_id,
    note = 'logo d''événement, rattaché à son édition (lot 101)'
FROM l101 x
WHERE m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1::text;

UPDATE legacy.id_map
SET note = replace(note, 'logo d''événement : phase 2, rôle de média absent', 'logo d''événement : lot 101')
WHERE legacy_table = 'public.events'
  AND legacy_id IN (SELECT edition_v1::text FROM l101);

SELECT legacy.remember('ancien_serveur.images_uploades', regexp_replace(fichier, '^.*/', ''),
                       'media', 'assets', asset_id, 'migrated',
                       'visuel de la CdP30 relevé sur l''ancien serveur (28/10), rôle ' || role)
FROM l101 WHERE objet_v1 IS NULL \g /dev/null

\if :{?depot}
CREATE TABLE IF NOT EXISTS legacy.fichiers_a_deposer (
    bucket      text   NOT NULL,
    object_key  text   NOT NULL,
    source      text   NOT NULL,
    sha256      text   NOT NULL,
    byte_size   bigint NOT NULL,
    mime_type   text   NOT NULL,
    PRIMARY KEY (bucket, object_key)
);
INSERT INTO legacy.fichiers_a_deposer
SELECT a.bucket, a.object_key, x.fichier, a.checksum_sha256, a.byte_size, a.mime_type
FROM l101 x JOIN media.assets a ON a.id = x.asset_id;
\endif

-- Outbox et compte rendu ------------------------------------------------------------
DO $$
DECLARE r record;
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-101'
               AND event_type <> 'media.asset.uploaded') THEN
        RAISE EXCEPTION 'lot 101 : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-101');
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now() AND task <> 'media.process_asset') THEN
        RAISE EXCEPTION 'lot 101 : travail inattendu créé par la transaction';
    END IF;
    FOR r IN SELECT e.acronym, x.role, x.mime, x.taille
             FROM l101 x JOIN event.events e ON e.id = x.event_id ORDER BY 1, 2 LOOP
        RAISE NOTICE 'lot 101 : % % (%, % octets)', r.acronym, r.role, r.mime, r.taille;
    END LOOP;
    RAISE NOTICE 'lot 101 : % objets, % rattachements, % traitements d''image en file, % événements d''outbox marqués publiés',
        (SELECT count(*) FROM media.assets a JOIN l101 x ON x.asset_id = a.id),
        (SELECT count(*) FROM media.attachments a JOIN l101 x ON x.rattachement_id = a.id),
        (SELECT count(*) FROM platform.jobs WHERE created_at = now() AND task = 'media.process_asset'),
        (SELECT count(*) FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-101');
END $$;

UPDATE platform.outbox_events SET published_at = now()
WHERE correlation_id = 'reprise-v1-101' AND published_at IS NULL;

COMMIT;
