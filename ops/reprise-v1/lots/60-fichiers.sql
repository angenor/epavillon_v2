-- Lot 60 : fichiers des buckets Supabase (storage.objects) et documents
-- d'activité (public.activity_documents, préparés par le lot 50).
--
-- Joué par 60-fichiers.sh, qui charge avant lui `lot60_manifeste` (temporaire :
-- bucket, chemin, taille, type, sha256, date) depuis le manifeste de
-- 60-telecharger.sh, et fixe `lot60.seau` (bucket Garage cible).
--
-- Ce que fait la v2 à un téléversement, refait ici :
--   - asset `uploaded`, clé `<année>/<mois>/<id sans tirets>/<nom normalisé>`
--     (domain/keys.rs), propriétaire personne = qui a déposé, organisation =
--     celle de la porteuse (service/authz.rs), visibilité `public` ;
--   - l'insertion met en file `media.process_asset` (gardé) : le worker analyse,
--     mesure, décline et passe l'objet à `ready` ;
--   - un même contenu une seule fois par bucket (empreinte) ;
--   - texte alternatif composé pour chaque image.
-- Seul l'original est repris : les vignettes v1 sont recalculées par la v2.
-- Les objets à déposer sont listés dans `legacy.fichiers_a_deposer`, lue par
-- `60-fichiers.sh deposer` (la v1 brute est retirée avant le dépôt, à la bascule).
--
-- Phase 2 (`skipped`, note « phase 2 : … ») : logo d'événement (aucun rôle),
-- galerie d'activité (D7), témoignage, document de négociation ; fichiers de
-- l'ancien serveur, hors Supabase, seulement comptés.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-60', true);

-- Outils de session ------------------------------------------------------------

CREATE FUNCTION pg_temp.url_decode(t text) RETURNS text
LANGUAGE plpgsql IMMUTABLE STRICT AS $$
DECLARE r bytea := ''; i int := 1;
BEGIN
    WHILE i <= length(t) LOOP
        IF substr(t, i, 1) = '%' AND substr(t, i + 1, 2) ~ '^[0-9A-Fa-f]{2}$' THEN
            r := r || decode(substr(t, i + 1, 2), 'hex'); i := i + 3;
        ELSE
            r := r || convert_to(substr(t, i, 1), 'UTF8'); i := i + 1;
        END IF;
    END LOOP;
    RETURN convert_from(r, 'UTF8');
END $$;

-- Chemin `<bucket>/<nom>` d'une URL publique Supabase, nul pour toute autre forme.
CREATE FUNCTION pg_temp.chemin_supabase(u text) RETURNS text
LANGUAGE sql IMMUTABLE STRICT AS $$
    SELECT pg_temp.url_decode(split_part(substring(u FROM '/storage/v1/object/public/(.+)$'), '?', 1))
$$;

-- keys.rs::translitterer
CREATE FUNCTION pg_temp.translitterer(t text) RETURNS text
LANGUAGE sql IMMUTABLE AS $$
    SELECT regexp_replace(regexp_replace(
               translate(lower(replace(replace(replace(replace(t, 'æ', 'ae'), 'Æ', 'ae'), 'œ', 'oe'), 'Œ', 'oe')),
                         'àáâäãåçèéêëìíîïñòóôöõùúûüýÿ',
                         'aaaaaaceeeeiiiinooooouuuuyy'),
               '[^a-z0-9]', '-', 'g'),
           '-{2,}', '-', 'g')
$$;

-- keys.rs::normaliser_nom
CREATE FUNCTION pg_temp.normaliser_nom(nom text) RETURNS text
LANGUAGE plpgsql IMMUTABLE AS $$
DECLARE base text := nom; ext text := ''; p int;
BEGIN
    p := length(nom) - strpos(reverse(nom), '.') + 1;
    IF strpos(nom, '.') > 0 AND p > 1 AND p < length(nom) AND length(nom) - p <= 12 THEN
        base := left(nom, p - 1); ext := substr(nom, p + 1);
    END IF;
    base := btrim(pg_temp.translitterer(base), '-');
    ext := btrim(pg_temp.translitterer(ext), '-');
    IF base = '' THEN base := 'fichier'; END IF;
    base := rtrim(left(base, greatest(160 - CASE WHEN ext = '' THEN 0 ELSE length(ext) + 1 END, 1)), '-');
    IF base = '' THEN base := 'fichier'; END IF;
    RETURN base || CASE WHEN ext = '' THEN '' ELSE '.' || ext END;
END $$;

-- Garde : manifeste complet et conforme à storage.objects ------------------------

DO $$
DECLARE v_seau text := current_setting('lot60.seau', true);
BEGIN
    IF to_regclass('pg_temp.lot60_manifeste') IS NULL THEN
        RAISE EXCEPTION 'lot 60 : lot60_manifeste absent, jouer par 60-fichiers.sh';
    END IF;
    IF v_seau IS NULL OR v_seau !~ '^[a-z0-9][a-z0-9.-]{1,62}$' THEN
        RAISE EXCEPTION 'lot 60 : bucket cible invalide (lot60.seau)';
    END IF;
    IF EXISTS (SELECT 1 FROM v1_storage.objects o
               LEFT JOIN lot60_manifeste f ON f.bucket = o.bucket_id AND f.chemin = o.name
               WHERE f.sha256 IS NULL OR f.taille <> (o.metadata->>'size')::bigint) THEN
        RAISE EXCEPTION 'lot 60 : objets absents du manifeste ou de taille différente, relancer 60-telecharger.sh';
    END IF;
END $$;

CREATE TEMP TABLE l60_objet ON COMMIT DROP AS
SELECT o.id AS objet_id, o.bucket_id || '/' || o.name AS chemin,
       regexp_replace(o.name, '^.*/', '') AS nom, f.taille, f.sha256,
       COALESCE(NULLIF(f.type, ''), o.metadata->>'mimetype') AS mime,
       COALESCE(o.created_at, f.depose_le) AS depose_le
FROM v1_storage.objects o
JOIN lot60_manifeste f ON f.bucket = o.bucket_id AND f.chemin = o.name;

-- Citations --------------------------------------------------------------------
-- usage : retenu, vignette, phase2, sans_porteur (porteur v1 écarté).
-- rang : ordre de préférence quand un rôle n'admet qu'un objet.

CREATE TEMP TABLE l60_citation (
    chemin        text NOT NULL,
    source        text NOT NULL,
    usage         text NOT NULL,
    note          text,
    owner_schema  text,
    owner_table   text,
    owner_id      uuid,
    role          text,
    doc_id        text,
    proprio_personne uuid,
    proprio_org   uuid,
    alt           text,
    rang          bigint NOT NULL DEFAULT 0
) ON COMMIT DROP;

-- Logos d'organisation. Deux fiches v1 fusionnées : la fiche retenue d'abord.
INSERT INTO l60_citation
SELECT pg_temp.chemin_supabase(o.logo_url), 'organizations.logo_url',
       CASE WHEN m.target_id IS NULL THEN 'sans_porteur' ELSE 'retenu' END,
       CASE WHEN m.target_id IS NULL THEN 'organisation non reprise' END,
       'org', 'organizations', m.target_id, 'logo', NULL,
       legacy.map('auth.users', o.created_by::text), m.target_id,
       'Logo de ' || btrim(g.legal_name),
       row_number() OVER (PARTITION BY m.target_id
                          ORDER BY (m.disposition = 'migrated') DESC, o.updated_at DESC, o.id)
FROM v1.organizations o
LEFT JOIN legacy.id_map m ON m.legacy_table = 'public.organizations' AND m.legacy_id = o.id::text
LEFT JOIN org.organizations g ON g.id = m.target_id
WHERE pg_temp.chemin_supabase(o.logo_url) IS NOT NULL;

-- Couvertures d'activité : proposition et séance ; la basse qualité est une vignette.
INSERT INTO l60_citation
SELECT pg_temp.chemin_supabase(a.cover_image_high_url), 'activities.cover_image_high_url',
       CASE WHEN p.id IS NULL THEN 'sans_porteur' ELSE 'retenu' END,
       CASE WHEN p.id IS NULL THEN 'activité non reprise' END,
       'programme', c.owner_table, c.owner_id, 'cover', NULL,
       p.submitted_by, p.organization_id,
       'Visuel de l''activité ' || btrim(p.title->>'fr'), 0
FROM v1.activities a
LEFT JOIN programme.proposals p ON p.id = legacy.map('public.activities', a.id::text)
LEFT JOIN LATERAL (
    SELECT 'proposals' AS owner_table, p.id AS owner_id
    UNION ALL
    SELECT 'sessions', s.id FROM programme.sessions s WHERE s.proposal_id = p.id
) c ON true
WHERE pg_temp.chemin_supabase(a.cover_image_high_url) IS NOT NULL;

INSERT INTO l60_citation (chemin, source, usage, note)
SELECT pg_temp.chemin_supabase(cover_image_low_url), 'activities.cover_image_low_url',
       'vignette', 'vignette v1 (basse qualité), la v2 recalcule ses déclinaisons'
FROM v1.activities WHERE pg_temp.chemin_supabase(cover_image_low_url) IS NOT NULL;

-- Photos de profil : la personne elle-même, prioritaire sur une photo d'intervenant.
INSERT INTO l60_citation
SELECT pg_temp.chemin_supabase(u.profile_photo_url), 'users.profile_photo_url',
       CASE WHEN pe.id IS NULL THEN 'sans_porteur' ELSE 'retenu' END,
       CASE WHEN pe.id IS NULL THEN 'personne non reprise' END,
       'identity', 'people', pe.id, 'avatar', NULL, pe.id, NULL,
       'Photo de ' || btrim(pe.first_name) || ' ' || btrim(pe.last_name),
       -extract(epoch FROM u.updated_at)::bigint
FROM v1.users u
LEFT JOIN identity.people pe ON pe.id = legacy.map('auth.users', u.id::text)
WHERE pg_temp.chemin_supabase(u.profile_photo_url) IS NOT NULL;

INSERT INTO l60_citation (chemin, source, usage, note)
SELECT pg_temp.chemin_supabase(profile_photo_thumbnail_url), 'users.profile_photo_thumbnail_url',
       'vignette', 'vignette v1, la v2 recalcule ses déclinaisons'
FROM v1.users WHERE pg_temp.chemin_supabase(profile_photo_thumbnail_url) IS NOT NULL;

-- Photos d'intervenants : avatar de la personne, la plus récente d'abord, après
-- sa propre photo (rang décalé). Déposée par le porteur de l'activité.
INSERT INTO l60_citation
SELECT pg_temp.chemin_supabase(s.photo_url), 'activity_speakers.photo_url',
       CASE WHEN ps.person_id IS NULL THEN 'sans_porteur' ELSE 'retenu' END,
       CASE WHEN ps.person_id IS NULL THEN 'intervenant non repris (' || COALESCE(m.note, 'lot 50') || ')' END,
       'identity', 'people', ps.person_id, 'avatar', NULL,
       COALESCE(p.submitted_by, ps.person_id), NULL,
       'Photo de ' || btrim(pe.first_name) || ' ' || btrim(pe.last_name),
       10000000000 - extract(epoch FROM s.created_at)::bigint
FROM v1.activity_speakers s
LEFT JOIN legacy.id_map m ON m.legacy_table = 'public.activity_speakers' AND m.legacy_id = s.id::text
LEFT JOIN programme.proposal_speakers ps ON ps.id = m.target_id
LEFT JOIN programme.proposals p ON p.id = ps.proposal_id
LEFT JOIN identity.people pe ON pe.id = ps.person_id
WHERE pg_temp.chemin_supabase(s.photo_url) IS NOT NULL;

INSERT INTO l60_citation (chemin, source, usage, note)
SELECT pg_temp.chemin_supabase(photo_thumbnail_url), 'activity_speakers.photo_thumbnail_url',
       'vignette', 'vignette v1, la v2 recalcule ses déclinaisons'
FROM v1.activity_speakers WHERE pg_temp.chemin_supabase(photo_thumbnail_url) IS NOT NULL;

-- Documents d'activité (lot 50) : pièce du dossier, pas un rattachement.
INSERT INTO l60_citation
SELECT pg_temp.chemin_supabase(d.file_url), 'activity_documents.file_url',
       CASE WHEN d.proposal_id IS NULL THEN 'sans_porteur' ELSE 'retenu' END,
       d.note_ecart, NULL, NULL, NULL, NULL, d.legacy_id,
       COALESCE(d.uploaded_by, p.submitted_by), p.organization_id, NULL, 0
FROM legacy.documents_activites d
LEFT JOIN programme.proposals p ON p.id = d.proposal_id
WHERE pg_temp.chemin_supabase(d.file_url) IS NOT NULL;

-- Bandeaux d'édition : la haute qualité 32:9 en `banner` ; la basse qualité 16:9
-- de Supabase, s'il y en a une, en `cover`.
INSERT INTO l60_citation
SELECT pg_temp.chemin_supabase(b.u), 'events.' || b.col,
       CASE WHEN ev.id IS NULL THEN 'sans_porteur' ELSE 'retenu' END,
       CASE WHEN ev.id IS NULL THEN 'événement non repris' END,
       'event', 'events', ev.id, b.role, NULL, NULL, NULL,
       'Bandeau de ' || btrim(ev.title->>'fr'), b.rang
FROM v1.events e
CROSS JOIN LATERAL (VALUES
    ('banner_high_quality_32_9_url', e.banner_high_quality_32_9_url, 'banner', 0),
    ('banner_low_quality_32_9_url',  e.banner_low_quality_32_9_url,  'banner', 1),
    ('banner_high_quality_16_9_url', e.banner_high_quality_16_9_url, 'cover', 0),
    ('banner_low_quality_16_9_url',  e.banner_low_quality_16_9_url,  'cover', 1),
    ('banner_high_quality_1_1_url',  e.banner_high_quality_1_1_url,  'thumbnail', 0),
    ('banner_low_quality_1_1_url',   e.banner_low_quality_1_1_url,   'thumbnail', 1)
) AS b(col, u, role, rang)
LEFT JOIN event.events ev ON ev.id = legacy.map('public.events', e.id::text)
WHERE pg_temp.chemin_supabase(b.u) IS NOT NULL;

-- Phase 2 : aucune place en v2 aujourd'hui.
INSERT INTO l60_citation (chemin, source, usage, note)
SELECT pg_temp.chemin_supabase(u), src, 'phase2', note
FROM (
    SELECT logo_url, 'events.logo_url', 'phase 2 : logo d''événement, rôle de média absent' FROM v1.events
    UNION ALL SELECT media_url, 'media_gallery.media_url', 'phase 2 : galerie d''activité (D7)' FROM v1.media_gallery
    UNION ALL SELECT thumbnail_url, 'media_gallery.thumbnail_url', 'phase 2 : galerie d''activité (D7)' FROM v1.media_gallery
    UNION ALL SELECT photo_url, 'user_testimonials.photo_url', 'phase 2 : témoignage (content.highlights)' FROM v1.user_testimonials
    UNION ALL SELECT file_url, 'negotiation_documents.file_url', 'phase 2 : document de négociation' FROM v1.negotiation_documents
    UNION ALL SELECT cover_image_url, 'negotiation_documents.cover_image_url', 'phase 2 : document de négociation' FROM v1.negotiation_documents
) AS x(u, src, note)
WHERE pg_temp.chemin_supabase(u) IS NOT NULL;

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM l60_citation c LEFT JOIN l60_objet o ON o.chemin = c.chemin WHERE o.objet_id IS NULL) THEN
        RAISE EXCEPTION 'lot 60 : % citation(s) vers un objet absent de Supabase',
            (SELECT count(*) FROM l60_citation c LEFT JOIN l60_objet o ON o.chemin = c.chemin WHERE o.objet_id IS NULL);
    END IF;
END $$;

-- Rôle à objet unique : un seul retenu par porteuse, et jamais par-dessus un
-- rattachement déjà en base.
UPDATE l60_citation c
SET usage = 'deja_pourvu',
    note = 'rôle « ' || c.role || ' » déjà pourvu pour ce porteur par un autre fichier'
FROM (
    SELECT ctid AS ligne,
           row_number() OVER (PARTITION BY owner_schema, owner_table, owner_id, role
                              ORDER BY rang, chemin) AS n
    FROM l60_citation WHERE usage = 'retenu' AND role IS NOT NULL
) r
JOIN media.attachable_roles ar ON true
WHERE c.ctid = r.ligne AND ar.owner_schema = c.owner_schema AND ar.owner_table = c.owner_table
  AND ar.role = c.role::media.attachment_role AND NOT ar.is_multiple
  AND (r.n > 1 OR EXISTS (SELECT 1 FROM media.attachments t
                          WHERE t.owner_schema = c.owner_schema AND t.owner_table = c.owner_table
                            AND t.owner_id = c.owner_id AND t.role = c.role::media.attachment_role));

-- Décision par objet -------------------------------------------------------------
-- Un contenu (empreinte) cité et retenu devient un asset ; le premier objet
-- qui le porte est `migrated`, les autres `merged`. Contenu déjà présent dans le
-- bucket cible : réutilisé, comme le ferait media.find_by_checksum à l'API.

CREATE TEMP TABLE l60_contenu ON COMMIT DROP AS
SELECT o.sha256,
       (array_agg(o.objet_id ORDER BY o.depose_le, o.objet_id))[1] AS objet_porteur,
       media.find_by_checksum(o.sha256, current_setting('lot60.seau')) AS asset_existant,
       NULL::uuid AS asset_id
FROM l60_objet o
WHERE EXISTS (SELECT 1 FROM l60_citation c WHERE c.chemin = o.chemin AND c.usage = 'retenu')
GROUP BY o.sha256;

UPDATE l60_contenu SET asset_id = COALESCE(asset_existant, platform.uuid_v7());

-- Première citation retenue du contenu : elle donne propriétaire et texte alternatif.
CREATE TEMP TABLE l60_asset ON COMMIT DROP AS
SELECT DISTINCT ON (k.sha256)
       k.asset_id, o.objet_id, o.nom, o.taille, o.mime, k.sha256, o.depose_le,
       c.proprio_personne, c.proprio_org, c.alt
FROM l60_contenu k
JOIN l60_objet o ON o.objet_id = k.objet_porteur
JOIN l60_objet o2 ON o2.sha256 = k.sha256
JOIN l60_citation c ON c.chemin = o2.chemin AND c.usage = 'retenu'
WHERE k.asset_existant IS NULL
ORDER BY k.sha256, (c.alt IS NULL), c.rang, c.source;

INSERT INTO media.assets (id, bucket, object_key, checksum_sha256, mime_type, byte_size,
                          original_filename, owner_person_id, owner_organization_id,
                          visibility, status, alt_text, created_at, updated_at)
SELECT a.asset_id, current_setting('lot60.seau'),
       to_char(a.depose_le AT TIME ZONE 'UTC', 'YYYY/MM') || '/' || replace(a.asset_id::text, '-', '')
           || '/' || pg_temp.normaliser_nom(a.nom),
       a.sha256, a.mime, a.taille, a.nom,
       COALESCE(a.proprio_personne, '01a00000-0000-7000-8000-00000000f1a1'), a.proprio_org,
       'public', 'uploaded',
       CASE WHEN a.alt IS NOT NULL THEN jsonb_build_object('fr', a.alt) END,
       a.depose_le, a.depose_le
FROM l60_asset a;

-- Rattachements ------------------------------------------------------------------

INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role,
                               alt_text_override, created_by, created_at)
SELECT c.owner_schema, c.owner_table, c.owner_id, k.asset_id, c.role::media.attachment_role,
       CASE WHEN c.alt IS DISTINCT FROM ma.alt_text->>'fr' AND c.alt IS NOT NULL
            THEN jsonb_build_object('fr', c.alt) END,
       COALESCE(c.proprio_personne, '01a00000-0000-7000-8000-00000000f1a1'), o.depose_le
FROM l60_citation c
JOIN l60_objet o ON o.chemin = c.chemin
JOIN l60_contenu k ON k.sha256 = o.sha256
JOIN media.assets ma ON ma.id = k.asset_id
WHERE c.usage = 'retenu' AND c.role IS NOT NULL;

-- Pièces du dossier (programme.proposal_documents), comme rattacher() du module.
CREATE TEMP TABLE l60_piece ON COMMIT DROP AS
SELECT d.legacy_id, platform.uuid_v7() AS piece_id, d.proposal_id, k.asset_id, d.title,
       d.document_type_code, d.is_public, c.proprio_personne AS uploaded_by,
       COALESCE(d.uploaded_at, o.depose_le) AS uploaded_at, d.sort_order
FROM legacy.documents_activites d
JOIN l60_citation c ON c.doc_id = d.legacy_id AND c.usage = 'retenu'
JOIN l60_objet o ON o.chemin = c.chemin
JOIN l60_contenu k ON k.sha256 = o.sha256;

INSERT INTO programme.proposal_documents (id, proposal_id, asset_id, title, document_type_code,
                                          is_public, uploaded_by, uploaded_at, sort_order)
SELECT piece_id, proposal_id, asset_id, title, document_type_code, is_public,
       uploaded_by, uploaded_at, sort_order
FROM l60_piece;

-- Objets à déposer dans Garage (60-fichiers.sh deposer) -----------------------------

CREATE TABLE legacy.fichiers_a_deposer (
    bucket      text   NOT NULL,
    object_key  text   NOT NULL,
    source      text   NOT NULL,   -- <bucket v1>/<chemin>, sous sauvegardes/reprise-v1/fichiers/
    sha256      text   NOT NULL,
    byte_size   bigint NOT NULL,
    mime_type   text   NOT NULL,
    PRIMARY KEY (bucket, object_key)
);
COMMENT ON TABLE legacy.fichiers_a_deposer IS
    'Lot 60 : objets déclarés dans media.assets à déposer dans Garage depuis la copie locale des buckets Supabase. Supprimée après le dépôt.';

INSERT INTO legacy.fichiers_a_deposer
SELECT ma.bucket, ma.object_key, o.chemin, ma.checksum_sha256, ma.byte_size, ma.mime_type
FROM l60_asset a
JOIN media.assets ma ON ma.id = a.asset_id
JOIN l60_objet o ON o.objet_id = a.objet_id;

-- Correspondance -----------------------------------------------------------------

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table, target_id, disposition, note)
SELECT 'storage.objects', o.objet_id::text, 'media', 'assets',
       k.asset_id,
       CASE WHEN k.asset_id IS NULL THEN 'skipped'
            WHEN k.objet_porteur = o.objet_id AND k.asset_existant IS NULL THEN 'migrated'
            ELSE 'merged' END,
       CASE WHEN k.asset_id IS NULL THEN COALESCE(
                (SELECT c.note FROM l60_citation c WHERE c.chemin = o.chemin
                 ORDER BY array_position(ARRAY['phase2', 'deja_pourvu', 'sans_porteur', 'vignette'], c.usage), c.source
                 LIMIT 1),
                'non cité')
            WHEN k.asset_existant IS NOT NULL THEN 'contenu déjà présent en v2 (même empreinte)'
            WHEN k.objet_porteur <> o.objet_id THEN 'même contenu (empreinte) qu''un autre objet repris'
       END
FROM l60_objet o
LEFT JOIN l60_contenu k ON k.sha256 = o.sha256
     AND EXISTS (SELECT 1 FROM l60_citation c WHERE c.chemin = o.chemin AND c.usage = 'retenu');

INSERT INTO legacy.id_map (legacy_table, legacy_id, target_schema, target_table, target_id, disposition, note)
SELECT 'public.activity_documents', d.legacy_id, 'programme', 'proposal_documents',
       p.piece_id,
       CASE WHEN p.piece_id IS NULL THEN 'skipped' ELSE 'migrated' END,
       CASE WHEN p.piece_id IS NULL THEN COALESCE(d.note_ecart, 'fichier introuvable') END
FROM legacy.documents_activites d
LEFT JOIN l60_piece p ON p.legacy_id = d.legacy_id;

-- Outbox : `media.asset.uploaded` marqué publié (le lot ne notifie rien) ; les
-- travaux `media.process_asset` restent en file pour le worker.
DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events WHERE correlation_id = 'reprise-v1-60'
               AND event_type <> 'media.asset.uploaded') THEN
        RAISE EXCEPTION 'lot 60 : événement d''outbox inattendu';
    END IF;
END $$;
UPDATE platform.outbox_events SET published_at = now()
WHERE correlation_id = 'reprise-v1-60' AND published_at IS NULL;

-- Compte rendu -------------------------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    FOR r IN
        SELECT s.t, s.extraites,
               count(m.*) FILTER (WHERE m.disposition = 'migrated') AS migrated,
               count(m.*) FILTER (WHERE m.disposition = 'merged')   AS merged,
               count(m.*) FILTER (WHERE m.disposition = 'skipped')  AS skipped
        FROM (VALUES
            ('storage.objects',           (SELECT count(*) FROM v1_storage.objects)),
            ('public.activity_documents', (SELECT count(*) FROM v1.activity_documents))
        ) AS s(t, extraites)
        LEFT JOIN legacy.id_map m ON m.legacy_table = s.t
        GROUP BY s.t, s.extraites ORDER BY s.t
    LOOP
        RAISE NOTICE 'lot 60 : % extraites %, migrated %, merged %, skipped %',
            r.t, r.extraites, r.migrated, r.merged, r.skipped;
        IF r.extraites <> r.migrated + r.merged + r.skipped THEN
            RAISE EXCEPTION 'lot 60 : % ne rend pas compte de toutes ses lignes', r.t;
        END IF;
    END LOOP;

    FOR r IN SELECT note, count(*) AS n FROM legacy.id_map
             WHERE legacy_table = 'storage.objects' AND disposition <> 'migrated'
             GROUP BY note ORDER BY n DESC
    LOOP
        RAISE NOTICE 'lot 60 : objets %, %', r.n, r.note;
    END LOOP;

    FOR r IN SELECT owner_schema || '.' || owner_table || ' ' || role AS k, count(*) AS n
             FROM media.attachments t JOIN l60_contenu c ON c.asset_id = t.asset_id
             GROUP BY 1 ORDER BY 1
    LOOP
        RAISE NOTICE 'lot 60 : rattachements %, %', r.k, r.n;
    END LOOP;

    RAISE NOTICE 'lot 60 : assets créés %, % Mo ; pièces de dossier % ; travaux media.process_asset en file %',
        (SELECT count(*) FROM l60_asset),
        (SELECT round(sum(taille) / 1048576.0, 1) FROM l60_asset),
        (SELECT count(*) FROM l60_piece),
        (SELECT count(*) FROM platform.jobs j JOIN l60_asset a ON j.idempotency_key = a.asset_id::text
         WHERE j.task = 'media.process_asset');

    IF EXISTS (SELECT 1 FROM media.assets a JOIN l60_asset x ON x.asset_id = a.id
               WHERE a.mime_type LIKE 'image/%' AND a.alt_text IS NULL) THEN
        RAISE EXCEPTION 'lot 60 : image sans texte alternatif';
    END IF;
END $$;

-- Fichiers de l'ancien serveur (hors Supabase) : phase 2, comptés seulement.
DO $$
DECLARE v_cit bigint; v_dist bigint;
BEGIN
    WITH u(url) AS (
        SELECT logo_url FROM v1.events
        UNION ALL SELECT banner_low_quality_16_9_url FROM v1.events
        UNION ALL SELECT banner_high_quality_16_9_url FROM v1.events
        UNION ALL SELECT banner_high_quality_32_9_url FROM v1.events
        UNION ALL SELECT file_url FROM v1.negotiation_documents
        UNION ALL SELECT cover_image_url FROM v1.negotiation_documents
        UNION ALL SELECT old_user_data->>'photo_url' FROM v1.migration_logs
    )
    SELECT count(*), count(DISTINCT regexp_replace(url, '^.*/', '')) INTO v_cit, v_dist
    FROM u WHERE NULLIF(btrim(url), '') IS NOT NULL AND pg_temp.chemin_supabase(url) IS NULL;
    RAISE NOTICE 'lot 60 : ancien serveur (phase 2) : % citations, % fichiers distincts', v_cit, v_dist;
END $$;

COMMIT;
