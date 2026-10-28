-- Lot 96 (phase 2) : galeries d'activité de la v1 (D7).
--
-- `v1.media_gallery` : 11 photos sur 4 activités, toutes terminées dans la v1,
-- donc une séance en v2 (lot 50). Chaque photo devient un objet (bucket v1
-- `activity-media`, écarté au lot 60 pour D7) rattaché à la séance en rôle
-- `gallery` (migration 2026-10-28-galerie-activite), dans l'ordre v1 (date de
-- création puis identifiant). La page publique d'une activité les lit par
-- media.attached_images().
--
-- `lot96_photo`, écrit par 96-galeries.sh : ligne v1, chemin, taille, type,
-- sha256. Identifiants d'objets et de rattachements fixés ici, les mêmes sur le
-- clone qui dépose les fichiers dans Garage et en production. Sur un clone
-- local, `-v depot=1` écrit `legacy.fichiers_a_deposer` pour 60-fichiers.sh.
--
-- Légende v1 (`title`) : sur le rattachement (`caption_override`), jamais sur
-- l'objet, pour que la vider au back-office l'efface vraiment. Texte
-- alternatif, absent de la v1 et obligatoire : sur l'objet, la légende, à
-- défaut « Photo de l'activité « <titre de la séance> » ». Propriétaire :
-- l'auteur v1 et l'organisation du dossier, comme les couvertures (lot 60).
-- Une activité sans séance n'est pas une erreur : ses photos sont écartées et
-- le compte rendu le dit.
-- Rien ne part : `media.asset.uploaded` marqué publié, comme aux lots 60 et 93.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-96', true) \g /dev/null

CREATE TEMP TABLE l96 ON COMMIT DROP AS
SELECT p.*, v.photo_v1, v.asset_id, v.rattachement_id,
       pr.id AS proposal_id, pr.organization_id, s.id AS session_id, s.title AS titre_seance,
       COALESCE(legacy.map('auth.users', p.auteur_v1::text),
                '01a00000-0000-7000-8000-00000000f1a1') AS auteur,
       NULL::smallint AS rang
FROM lot96_photo p
FULL JOIN (VALUES
    ('0052ab10-5176-4ae5-acdb-aa89a90bac04'::uuid, '01a182e7-d25b-71ba-b0ae-0cb2c5558ad2'::uuid, '01a182e7-d25b-79bd-a0f9-204d20a7dcef'::uuid),
    ('1d60e3e5-1eda-441e-89c3-0cdd55f8b6a9'::uuid, '01a182e7-d25b-75d3-b6dc-91d646238892'::uuid, '01a182e7-d25b-77c9-bb3d-135d3102c399'::uuid),
    ('26d4c10d-6437-40e5-babd-abaf9b1334cb'::uuid, '01a182e7-d25b-7bb1-be63-e366355f0645'::uuid, '01a182e7-d25b-7a84-a7b2-931bda27dcd6'::uuid),
    ('41a2cc05-b1ad-4120-94f5-8ee5028db3b8'::uuid, '01a182e7-d25b-7745-9a1a-a9ad7236af22'::uuid, '01a182e7-d25b-73f0-8912-3e2a07eeec94'::uuid),
    ('630013ad-531b-49c9-905f-808ddf72a592'::uuid, '01a182e7-d25b-74af-a67f-1159f055e9a1'::uuid, '01a182e7-d25b-7ad9-a8ed-c8fbe0847611'::uuid),
    ('75aa893b-b13e-453d-8f2c-8d646f147509'::uuid, '01a182e7-d25b-7a05-a3ad-322b353a827a'::uuid, '01a182e7-d25b-7e28-831a-f042310e29ea'::uuid),
    ('7868a745-0fd3-45df-9b47-ff66dfc01031'::uuid, '01a182e7-d25b-7e80-8fb7-42deaddc6e46'::uuid, '01a182e7-d25b-71d5-80b3-d15b53ee3260'::uuid),
    ('b82c5714-be77-4143-94e9-faa0cdffe10c'::uuid, '01a182e7-d25a-76d5-a6c9-502666002ea1'::uuid, '01a182e7-d25b-781d-aca7-9a70b2c2e653'::uuid),
    ('f374e3e0-f1ab-4658-b17b-b8113b31c499'::uuid, '01a182e7-d25b-77d1-a7fc-0d140e25ad32'::uuid, '01a182e7-d25b-73db-ab14-a2597d7e0a86'::uuid),
    ('f82cdc16-de1e-4abc-bb4f-b3df1a4303e4'::uuid, '01a182e7-d25b-7d1e-99c3-8c68c7654e0d'::uuid, '01a182e7-d25b-7676-8cf6-bf5f325a0d21'::uuid),
    ('fae0104f-9960-4893-8127-130e8d15b1cc'::uuid, '01a182e7-d25b-7614-925b-484efa50a464'::uuid, '01a182e7-d25b-7385-85dd-034300e7a8e3'::uuid)
) AS v(photo_v1, asset_id, rattachement_id) ON v.photo_v1 = p.id
LEFT JOIN programme.proposals pr ON pr.id = legacy.map('public.activities', p.activite_v1::text)
LEFT JOIN programme.sessions s ON s.proposal_id = pr.id;

DO $$
DECLARE
    v_seau text := (SELECT value #>> '{}' FROM platform.settings WHERE key = 'media.default_bucket');
BEGIN
    PERFORM set_config('lot96.seau', v_seau, true);
    IF EXISTS (SELECT 1 FROM l96 WHERE id IS NULL OR photo_v1 IS NULL) THEN
        RAISE EXCEPTION 'lot 96 : les photos du flux ne sont pas celles du lot (v1 modifiée ?)';
    END IF;
    IF EXISTS (SELECT 1 FROM l96 GROUP BY id HAVING count(*) > 1) THEN
        RAISE EXCEPTION 'lot 96 : une proposition porte plusieurs séances';
    END IF;
    IF EXISTS (SELECT 1 FROM l96 WHERE contexte <> 'activity' OR genre <> 'photo' OR NOT vignette_identique) THEN
        RAISE EXCEPTION 'lot 96 : photo hors activité, vidéo ou vignette distincte : non prévu';
    END IF;
    IF EXISTS (SELECT 1 FROM l96 WHERE objet_v1 IS NULL OR sha256 IS NULL OR mime NOT LIKE 'image/%') THEN
        RAISE EXCEPTION 'lot 96 : photo sans objet v1, absente du flux ou qui n''est pas une image';
    END IF;
    IF EXISTS (SELECT 1 FROM l96 WHERE proposal_id IS NULL) THEN
        RAISE EXCEPTION 'lot 96 : activité v1 sans proposition en v2 (lot 50 joué ?)';
    END IF;
    IF EXISTS (SELECT 1 FROM l96 x JOIN media.assets a ON a.id = x.asset_id)
       OR EXISTS (SELECT 1 FROM l96 x JOIN media.attachments a ON a.id = x.rattachement_id)
       OR EXISTS (SELECT 1 FROM legacy.id_map WHERE legacy_table = 'public.media_gallery') THEN
        RAISE EXCEPTION 'lot 96 : galerie déjà posée, le lot ne se joue qu''une fois';
    END IF;
    IF EXISTS (SELECT 1 FROM l96 WHERE media.find_by_checksum(sha256, v_seau) IS NOT NULL)
       OR EXISTS (SELECT 1 FROM l96 GROUP BY sha256 HAVING count(*) > 1) THEN
        RAISE EXCEPTION 'lot 96 : contenu déjà présent en v2 ou en double dans la galerie, à trancher';
    END IF;
    IF EXISTS (SELECT 1 FROM l96 x JOIN legacy.id_map m
               ON m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1::text
               WHERE m.disposition <> 'skipped' OR m.note NOT LIKE '%D7%')
       OR (SELECT count(*) FROM l96 x JOIN legacy.id_map m
           ON m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1::text)
          <> (SELECT count(*) FROM l96) THEN
        RAISE EXCEPTION 'lot 96 : objet v1 non consigné comme écarté pour D7';
    END IF;
END $$;

UPDATE l96 x SET rang = r.rang
FROM (SELECT id, (row_number() OVER (PARTITION BY session_id ORDER BY cree_le NULLS LAST, id) - 1)::smallint AS rang
      FROM l96 WHERE session_id IS NOT NULL) r
WHERE r.id = x.id;

INSERT INTO media.assets (id, bucket, object_key, checksum_sha256, mime_type, byte_size,
                          original_filename, owner_person_id, owner_organization_id,
                          visibility, status, alt_text, created_at, updated_at)
SELECT x.asset_id, current_setting('lot96.seau'),
       to_char(COALESCE(x.cree_le, now()) AT TIME ZONE 'UTC', 'YYYY/MM') || '/'
           || replace(x.asset_id::text, '-', '') || '/' || regexp_replace(x.fichier, '^.*/', ''),
       x.sha256, x.mime, x.taille, regexp_replace(x.fichier, '^.*/', ''),
       x.auteur, x.organization_id, 'public', 'uploaded',
       jsonb_build_object('fr', COALESCE(NULLIF(btrim(x.titre), ''),
                                         'Photo de l''activité « ' || btrim(x.titre_seance->>'fr') || ' »')),
       COALESCE(x.cree_le, now()), COALESCE(x.cree_le, now())
FROM l96 x
WHERE x.session_id IS NOT NULL;

INSERT INTO media.attachments (id, owner_schema, owner_table, owner_id, asset_id, role, sort_order,
                               caption_override, created_by, created_at)
SELECT rattachement_id, 'programme', 'sessions', session_id, asset_id, 'gallery', rang,
       CASE WHEN NULLIF(btrim(titre), '') IS NOT NULL THEN jsonb_build_object('fr', btrim(titre)) END,
       auteur, COALESCE(cree_le, now())
FROM l96
WHERE session_id IS NOT NULL;

-- Correspondance ------------------------------------------------------------------

UPDATE legacy.id_map m
SET disposition = 'migrated', target_schema = 'media', target_table = 'assets', target_id = x.asset_id,
    note = 'photo de galerie, rattachée à la séance de son activité (D7)'
FROM l96 x
WHERE m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1::text AND x.session_id IS NOT NULL;

UPDATE legacy.id_map m
SET note = 'galerie d''activité (D7) : activité sans séance en v2'
FROM l96 x
WHERE m.legacy_table = 'storage.objects' AND m.legacy_id = x.objet_v1::text AND x.session_id IS NULL;

SELECT legacy.remember('public.media_gallery', id::text, 'media', 'attachments',
                       CASE WHEN session_id IS NOT NULL THEN rattachement_id END,
                       CASE WHEN session_id IS NOT NULL THEN 'migrated' ELSE 'skipped' END,
                       CASE WHEN session_id IS NULL THEN 'activité sans séance en v2 (proposition '
                                                         || (SELECT status FROM programme.proposals WHERE id = proposal_id) || ')' END)
FROM l96 \g /dev/null

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
FROM l96 x JOIN media.assets a ON a.id = x.asset_id;
\endif

-- Outbox et compte rendu ------------------------------------------------------------
-- `media.asset.uploaded` marqué publié (le lot ne notifie rien) ; le traitement
-- des images reste en file pour le worker (dimensions, déclinaisons, état `ready`).
DO $$
DECLARE r record;
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-96'
               AND event_type <> 'media.asset.uploaded') THEN
        RAISE EXCEPTION 'lot 96 : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = 'reprise-v1-96');
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now() AND task <> 'media.process_asset') THEN
        RAISE EXCEPTION 'lot 96 : travail inattendu créé par la transaction';
    END IF;

    SELECT (SELECT count(*) FROM l96) AS extraites,
           count(*) FILTER (WHERE m.disposition = 'migrated') AS migrated,
           count(*) FILTER (WHERE m.disposition = 'merged')   AS merged,
           count(*) FILTER (WHERE m.disposition = 'skipped')  AS skipped
    INTO r
    FROM legacy.id_map m WHERE m.legacy_table = 'public.media_gallery';
    RAISE NOTICE 'lot 96 : public.media_gallery extraites %, migrated %, merged %, skipped %',
        r.extraites, r.migrated, r.merged, r.skipped;
    IF r.extraites <> r.migrated + r.merged + r.skipped THEN
        RAISE EXCEPTION 'lot 96 : compte rendu incomplet pour public.media_gallery';
    END IF;

    RAISE NOTICE 'lot 96 : % photos sur % séances ; % activités sans séance (% photos écartées)',
        (SELECT count(*) FROM l96 WHERE session_id IS NOT NULL),
        (SELECT count(DISTINCT session_id) FROM l96),
        (SELECT count(DISTINCT activite_v1) FROM l96 WHERE session_id IS NULL),
        (SELECT count(*) FROM l96 WHERE session_id IS NULL);
    FOR r IN SELECT DISTINCT activite_v1 FROM l96 WHERE session_id IS NULL LOOP
        RAISE NOTICE 'lot 96 : activité v1 % sans séance, galerie écartée', r.activite_v1;
    END LOOP;
    RAISE NOTICE 'lot 96 : textes alternatifs, % tirés de la légende, % écrits d''après le titre de la séance',
        (SELECT count(*) FROM l96 WHERE session_id IS NOT NULL AND NULLIF(btrim(titre), '') IS NOT NULL),
        (SELECT count(*) FROM l96 WHERE session_id IS NOT NULL AND NULLIF(btrim(titre), '') IS NULL);
    RAISE NOTICE 'lot 96 : % traitements d''image en file, % événements d''outbox marqués publiés',
        (SELECT count(*) FROM platform.jobs WHERE created_at = now() AND task = 'media.process_asset'),
        (SELECT count(*) FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-96');
END $$;

UPDATE platform.outbox_events SET published_at = now()
WHERE correlation_id = 'reprise-v1-96' AND published_at IS NULL;

COMMIT;
