-- Lot 93 (phase 2) : photos des intervenants sans adresse repris au lot 92.
--
-- Sans personne, pas de photo de profil : la photo se rattache à la ligne
-- programme.proposal_speakers (rôle `avatar`, migration
-- 2026-10-27-photo-intervenant-sans-adresse), et le détail public d'une séance
-- l'y lit. Trois fichiers de la v1 (bucket `epavillonp`, dossier `intervenants/`),
-- écartés au lot 60 avec leur intervenant.
--
-- `lot93_photo`, écrit par 93-photos-intervenants.sh : chemin v1, taille, type,
-- sha256. Identifiants d'objets fixés ici, les mêmes sur le clone qui dépose les
-- fichiers dans Garage et en production. Sur un clone local, `-v depot=1` écrit
-- `legacy.fichiers_a_deposer` pour 60-fichiers.sh.
-- Propriétaire de l'objet : la personne qui a déposé le dossier et son
-- organisation, comme les couvertures d'activité (lot 60).
-- Rien ne part : `media.asset.uploaded` marqué publié, comme au lot 60.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-93', true) \g /dev/null

CREATE TEMP TABLE l93 ON COMMIT DROP AS
SELECT v.*, p.taille, p.mime, p.sha256,
       legacy.map('public.activity_speakers', v.intervenant_v1::text) AS ligne
FROM (VALUES
    ('6d7b03f8-c6cf-409f-8a75-46f35cfde35a'::uuid, 'bf2d3496-0b3f-4123-b6cb-6a8ee36029cb',
     '01a17f3b-770c-7702-8c12-4c7c29df9642'::uuid,
     'epavillonp/intervenants/6d7b03f8-c6cf-409f-8a75-46f35cfde35a_hq_1759256116608_img_0951.jpg'),
    ('27446304-c6ba-4f6c-9719-3c9f2fd01cb2'::uuid, 'd786f4db-8fce-4eb2-9220-e6898a574513',
     '01a17f3b-770d-7718-a0df-8db8a657bfc8'::uuid,
     'epavillonp/intervenants/27446304-c6ba-4f6c-9719-3c9f2fd01cb2_hq_1761828564132_photo_js_20240913_002_.jpg'),
    ('5bbd54f3-22ba-47e9-9533-38adfb052f82'::uuid, '3e7876a0-f4c1-43d2-b69a-38c6f9842a2b',
     '01a17f3b-770e-7402-a3a5-65ed7c5eb1f3'::uuid,
     'epavillonp/intervenants/5bbd54f3-22ba-47e9-9533-38adfb052f82_hq_1791878963490_48083.jpg')
) AS v(intervenant_v1, objet_v1, asset_id, fichier)
LEFT JOIN lot93_photo p USING (fichier);

DO $$
DECLARE
    v_seau text := (SELECT value #>> '{}' FROM platform.settings WHERE key = 'media.default_bucket');
BEGIN
    PERFORM set_config('lot93.seau', v_seau, true);
    IF EXISTS (SELECT 1 FROM l93 WHERE sha256 IS NULL OR mime NOT LIKE 'image/%') THEN
        RAISE EXCEPTION 'lot 93 : photo absente du flux ou qui n''est pas une image';
    END IF;
    IF EXISTS (SELECT 1 FROM l93 x LEFT JOIN programme.proposal_speakers ps ON ps.id = x.ligne
               WHERE ps.id IS NULL OR ps.person_id IS NOT NULL) THEN
        RAISE EXCEPTION 'lot 93 : intervenant sans ligne sans personne (lot 92 joué ?)';
    END IF;
    IF EXISTS (SELECT 1 FROM l93 x JOIN media.attachments a
               ON a.owner_schema = 'programme' AND a.owner_table = 'proposal_speakers'
              AND a.owner_id = x.ligne AND a.role = 'avatar')
       OR EXISTS (SELECT 1 FROM l93 x JOIN media.assets a ON a.id = x.asset_id)
       OR EXISTS (SELECT 1 FROM l93 WHERE media.find_by_checksum(sha256, v_seau) IS NOT NULL) THEN
        RAISE EXCEPTION 'lot 93 : photo déjà posée, le lot ne se joue qu''une fois';
    END IF;
    IF EXISTS (SELECT 1 FROM l93 x JOIN legacy.id_map m
               ON m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1
               WHERE m.disposition <> 'skipped' OR m.note NOT LIKE '%D16%')
       OR (SELECT count(*) FROM l93 x JOIN legacy.id_map m
           ON m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1) <> 3 THEN
        RAISE EXCEPTION 'lot 93 : objet v1 non consigné comme écarté pour D16';
    END IF;
END $$;

INSERT INTO media.assets (id, bucket, object_key, checksum_sha256, mime_type, byte_size,
                          original_filename, owner_person_id, owner_organization_id,
                          visibility, status, alt_text)
SELECT x.asset_id, current_setting('lot93.seau'),
       '2026/10/' || replace(x.asset_id::text, '-', '') || '/' || regexp_replace(x.fichier, '^.*/', ''),
       x.sha256, x.mime, x.taille, regexp_replace(x.fichier, '^.*/', ''),
       pr.submitted_by, pr.organization_id, 'public', 'uploaded',
       jsonb_build_object('fr', 'Photo de ' || ps.first_name || ' ' || ps.last_name)
FROM l93 x
JOIN programme.proposal_speakers ps ON ps.id = x.ligne
JOIN programme.proposals pr ON pr.id = ps.proposal_id;

INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role, created_by)
SELECT 'programme', 'proposal_speakers', ligne, asset_id, 'avatar', platform.current_actor_id()
FROM l93;

UPDATE legacy.id_map m
SET disposition = 'migrated', target_schema = 'media', target_table = 'assets', target_id = x.asset_id,
    note = 'photo d''un intervenant sans adresse, rattachée à sa ligne du dossier (D16)'
FROM l93 x
WHERE m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1;

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
FROM l93 x JOIN media.assets a ON a.id = x.asset_id;
\endif

-- Outbox : `media.asset.uploaded` marqué publié (le lot ne notifie rien), comme au
-- lot 60 ; le traitement des images reste en file pour le worker (déclinaisons).
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-93'
               AND event_type <> 'media.asset.uploaded') THEN
        RAISE EXCEPTION 'lot 93 : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-93');
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now() AND task <> 'media.process_asset') THEN
        RAISE EXCEPTION 'lot 93 : travail inattendu créé par la transaction';
    END IF;
    RAISE NOTICE 'lot 93 : % photos posées sur leur ligne d''intervenant, % traitements d''image en file',
        (SELECT count(*) FROM media.attachments a JOIN l93 x ON a.asset_id = x.asset_id),
        (SELECT count(*) FROM platform.jobs WHERE created_at = now() AND task = 'media.process_asset');
END $$;

UPDATE platform.outbox_events SET published_at = now()
WHERE correlation_id = 'reprise-v1-93' AND published_at IS NULL;

COMMIT;
