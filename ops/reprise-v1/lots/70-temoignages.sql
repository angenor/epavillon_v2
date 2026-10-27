-- Lot 70 (phase 2) : les témoignages de l'accueil de la v1.
--
-- Ils ne sont pas dans Supabase : la v1 les écrit dans son composant d'accueil
-- (`src/components/HeroSection.vue`, `mockWrittenTestimonials`) et les affiche
-- faute de témoignage mis en avant en base. Repris : les quatre affichés, avec
-- leur photo (`public/images/temoignages/`), en diapositives publiées de la
-- vitrine (content.highlights, rôle `banner`). Écartés : le cinquième (non mis
-- en avant, photo de banque d'images) et l'unique ligne de
-- public.user_testimonials (texte d'essai).
--
-- La v1 n'avait ni titre ni ordre : le titre reprend les mots du témoin, l'ordre
-- celui du composant ; la fonction sous le nom est le texte que la v1 affichait.
-- Le bandeau rogne une citation au-delà de cinq lignes : les deux textes longs
-- s'affichent par une phrase entière du témoin, l'intégral va dans `body`.
-- Les identifiants d'asset sont fixés ici : la clé d'objet en dépend, et elle
-- doit être la même sur le clone qui dépose les photos et en production.
--
-- Joué par 70-temoignages.sh, qui charge avant lui `lot70_photo` (fichier,
-- taille, type, sha256) et, sur un clone local, pose `-v depot=1`.

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'reprise-v1-70', true);

CREATE TEMP TABLE l70 ON COMMIT DROP AS
SELECT t.*, platform.uuid_v7() AS highlight_id
FROM (VALUES
    (1, 'mock-written-biligua', 'temoignage-Biligua-Koivogui.jpg',
     '01a17df3-3711-7f4c-9795-a4b2dd7347d8'::uuid, 'temoignage-biligua-koivogui.jpg',
     'Trois participantes souriantes, badge au cou, dans un pavillon de la conférence',
     'Des outils pratiques et des simulations',
     $t$Ma participation aux formations de l'IFDD sur les négociations climatiques a été transformatrice pour ma carrière de négociatrice.$t$,
     $t$Ma participation aux formations de l'IFDD sur les négociations climatiques a été transformatrice pour ma carrière de négociatrice. Grâce aux outils pratiques et aux simulations de négociations, j'ai développé une meilleure compréhension des enjeux complexes du financement climatique et de l'adaptation.$t$,
     'Biligua Koivogui', 'Négociatrice francophone', '#1E40AF'),
    (2, 'mock-written-1', 'Antoine_Faye.jpg',
     '01a17df3-3713-71c3-b6d7-0a0c8b5c83f9'::uuid, 'antoine-faye.jpg',
     'Antoine Faye en entretien à la COP25 de Madrid',
     'Le pavillon, notre bouée de sauvetage',
     NULL,
     $t$En effet, pour celles et ceux parmi nous qui participent annuellement aux Conférences des Parties (CdP), le Pavillon de la Francophonie nous sert toujours de « bouée de sauvetage », lorsqu'en plein dans les négociations, les textes en anglais deviennent un peu confus.$t$,
     'Antoine Faye', 'Consultant indépendant', NULL),
    (3, 'mock-written-kaully', 'Mme_Kaully_Tirouvi_Maurice.png',
     '01a17df3-3713-72dd-b14b-dd00df7c39b7'::uuid, 'mme-kaully-tirouvi-maurice.jpg',
     'Kaully Tirouvi devant l''installation #COP30 à Belém',
     'Des simulations fidèles aux réalités de la CdP30',
     NULL,
     $t$Les sessions de formation que j'ai suivies à Maurice et au Ghana se sont révélées particulièrement pertinentes et efficaces, les simulations de négociation reflétant les réalités rencontrées lors de la CdP30.$t$,
     'Kaully Tirouvi', 'Négociatrice bénéficiaire, île Maurice', NULL),
    (4, 'mock-written-constance', 'Mme_Constance_Constance_Genevee.jpg',
     '01a17df3-3713-7e36-a4a1-8e88ed864af0'::uuid, 'mme-constance-genevee.jpg',
     'Constance Genevée, micro en main, lors d''une prise de parole au pavillon',
     'Porter la voix des communautés vulnérables',
     $t$Dans un contexte largement dominé par l'anglais, le pavillon de l'OIF offre aux francophones un espace essentiel d'expression et de visibilité.$t$,
     $t$Cette année encore, l'OIF a permis à ma délégation à la COP 30, de porter la voix des communautés vulnérables face aux changements climatiques. Je suis fière d'avoir été impliquée dans l'organisation de la Journée de la Jeunesse qui a connu un franc succès, ainsi que d'avoir pris part à des temps forts tels que la Journée de la Finance et le Bilan de la première semaine de négociations. Dans un contexte largement dominé par l'anglais, le pavillon de l'OIF offre aux francophones un espace essentiel d'expression et de visibilité.$t$,
     'Constance Genevée', 'Délégation du Bénin', NULL)
) AS t(ordre, legacy_id, fichier, asset_id, nom_objet, alt, titre, extrait, citation, auteur, fonction, fond);

DO $$
DECLARE
    v_seau text := (SELECT value #>> '{}' FROM platform.settings WHERE key = 'media.default_bucket');
BEGIN
    PERFORM set_config('lot70.seau', v_seau, true);
    IF (SELECT count(*) FROM l70 JOIN lot70_photo USING (fichier)) <> 4 THEN
        RAISE EXCEPTION 'lot 70 : photos attendues 4, trouvées %',
            (SELECT count(*) FROM l70 JOIN lot70_photo USING (fichier));
    END IF;
    -- Une photo déjà au bucket serait à réutiliser, pas à doubler : décision à écrire.
    IF EXISTS (SELECT 1 FROM l70 JOIN lot70_photo p USING (fichier)
               WHERE media.find_by_checksum(p.sha256, v_seau) IS NOT NULL) THEN
        RAISE EXCEPTION 'lot 70 : une photo de témoignage est déjà au bucket %', v_seau;
    END IF;
END $$;

INSERT INTO media.assets (id, bucket, object_key, checksum_sha256, mime_type, byte_size,
                          original_filename, owner_person_id, visibility, status, alt_text)
SELECT l.asset_id, current_setting('lot70.seau'),
       '2026/10/' || replace(l.asset_id::text, '-', '') || '/' || l.nom_objet,
       p.sha256, p.mime, p.taille, l.fichier,
       '01a00000-0000-7000-8000-00000000f1a1', 'public', 'uploaded',
       jsonb_build_object('fr', l.alt)
FROM l70 l JOIN lot70_photo p USING (fichier);

INSERT INTO content.highlights (id, placement, status, nature_code, sort_order, title, quote, body,
                                author_name, author_title, background_color_hex, created_by)
SELECT highlight_id, 'home_hero', 'published', 'testimonial', ordre * 10,
       jsonb_build_object('fr', titre), jsonb_build_object('fr', COALESCE(extrait, citation)),
       CASE WHEN extrait IS NOT NULL THEN jsonb_build_object('fr', citation) END,
       auteur, jsonb_build_object('fr', fonction), fond,
       '01a00000-0000-7000-8000-00000000f1a1'
FROM l70;

INSERT INTO media.attachments (owner_schema, owner_table, owner_id, asset_id, role, created_by)
SELECT 'content', 'highlights', highlight_id, asset_id, 'banner',
       '01a00000-0000-7000-8000-00000000f1a1'
FROM l70;

-- Correspondances ----------------------------------------------------------------

SELECT legacy.remember('code.herosection', legacy_id, 'content', 'highlights', highlight_id)
FROM l70 \g /dev/null
SELECT legacy.remember('code.images-temoignages', fichier, 'media', 'assets', asset_id)
FROM l70 \g /dev/null
SELECT legacy.remember('code.herosection', 'mock-written-2', 'content', 'highlights', NULL,
                       'skipped', 'non mis en avant dans la v1, photo de banque d''images') \g /dev/null
SELECT legacy.remember('public.user_testimonials', '19c340b1-feb2-426b-9431-aa3d663c0159',
                       'content', 'highlights', NULL, 'skipped', 'texte d''essai (« Lorem ipsum »)') \g /dev/null

-- Sa photo, laissée au lot 60 pour la phase 2, suit le témoignage.
UPDATE legacy.id_map
SET note = 'photo du témoignage d''essai, écarté au lot 70'
WHERE legacy_table = 'storage.objects' AND note = 'phase 2 : témoignage (content.highlights)';

-- Compte rendu ---------------------------------------------------------------------

DO $$
DECLARE
    v_temoignages int := (SELECT count(*) FROM content.highlights
                          WHERE id IN (SELECT highlight_id FROM l70) AND status = 'published');
BEGIN
    IF v_temoignages <> 4
       OR (SELECT count(*) FROM media.attachments WHERE owner_id IN (SELECT highlight_id FROM l70)) <> 4 THEN
        RAISE EXCEPTION 'lot 70 : 4 diapositives publiées et 4 photos attendues';
    END IF;
    RAISE NOTICE 'lot 70 : code.herosection 5 extraits, migrated 4, skipped 1 ; public.user_testimonials 1, skipped 1';
    RAISE NOTICE 'lot 70 : photos 4, % Ko ; travaux media.process_asset en file %',
        (SELECT sum(byte_size)::bigint / 1024 FROM media.assets WHERE id IN (SELECT asset_id FROM l70)),
        (SELECT count(*) FROM platform.jobs WHERE task = 'media.process_asset' AND status = 'queued'
           AND payload->>'asset_id' IN (SELECT asset_id::text FROM l70));
END $$;

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
SELECT a.bucket, a.object_key, l.fichier, a.checksum_sha256, a.byte_size, a.mime_type
FROM l70 l JOIN media.assets a ON a.id = l.asset_id;
\endif

COMMIT;
