-- Contenu (30/10) : les trois pages de fil de la CdP31 (Antalya, 2026).
--
-- Atelier préparatoire francophone (8/11, hôtel), Journée finance durable
-- (13/11) et Journée action jeunesse pour le climat (14/11), selon le modèle
-- du 30/10 (060_events.sql § 3 bis et § 4 ter, 075 § 2 bis ; migration
-- ops/migrations/2026-10-30-journees-speciales.sql, à jouer AVANT ce lot).
-- Textes repris de frontend/app/mocks/track-pages.ts (maquettes retenues,
-- docs/maquettes/journees-speciales/2-bandeau-et-billet/).
--
-- REJOUABLE : identifiants des lignes créées fixés ici ; tout s'écrit par
-- ON CONFLICT DO NOTHING, une séance ne bouge que si elle est encore à son
-- heure d'avant. Une seconde passe ne change rien, et ne défait pas ce que
-- l'équipe aurait retouché au back-office entre-temps.
--
-- CE QUI EXISTE EST LU, JAMAIS DEVINÉ : édition par son slug, séances par
-- leur identifiant (relevé sur la sauvegarde epavillon-20261029-234452) ET le
-- début de leur titre ; une séance absente, d'une autre édition, annulée,
-- retitrée ou déjà déplacée ailleurs arrête le lot avant toute écriture.
--
-- CHOIX, À FAIRE VALIDER PAR LE COMMANDITAIRE
--   1. Lieu « Hôtel DoubleTree by Hilton », genre `other`, sans salle (l'atelier
--      n'a aucune activité). Coordonnées relevées sur OpenStreetMap (objet way
--      432156890) ; les maquettes en portaient d'autres, à 2 km de là.
--   2. Organisations créées en `candidate`, faute de fiche (règle n° 1 : noms,
--      sigles et variantes cherchés dans org.organization_names avant) : OIF,
--      GIZ, Fonds pour l'adaptation, Fonds vert pour le climat (un seul fonds,
--      « GCF » et « Fonds vert climat » en dénominations), Québec jeunesse monde.
--      Une fiche existante au même nom est reprise ; une qui ne partage que le
--      sigle arrête le lot (à trancher). Fondation Green OpenLab et les Fonds
--      d'investissement climatiques (Banque mondiale) ne sont pas créés : aucun
--      fil ne les cite en partenaire, et le lot ne touche pas aux organisations
--      des activités.
--   3. Atelier (side_programme, publié) : uniquement des MOMENTS, ce ne sont pas
--      des activités du pavillon. Deux parties, Matin et Après-midi. Les noms
--      des présentateurs du Guide ne sont pas écrits ici (aucun nom de personne
--      dans le dépôt) : à ajouter au back-office si on les veut.
--   4. Journée finance (non publiée : ses activités ne le sont pas, la page
--      serait vide ; elle se publiera avec le programme). Trois parties ;
--      moments accueil, ouverture (trois puces), clôture. L'activité de l'OCDE
--      PASSE DE 09:00 À 09:15 (fin inchangée, 10:00) : elle porte le panel. Les
--      sept autres restent à leur heure ; AIRF et Sakété, toutes deux à 11:30,
--      se chevauchent déjà (signalé, jamais bloqué : règles 2 et 3).
--   5. Journée jeunesse (non publiée, même raison). Trois parties, celles que
--      les maquettes proposent (le programme ne les nomme pas). « Les 10
--      minutes qui inspirent » : les cinq pitchs, DIX MINUTES CHACUN de 09:20 à
--      10:10, dans l'ordre du programme (JVE Centrafrique, Youth Center for
--      Impact, Fédération L'Air et Moi, Youth Voices Cameroon, NRCI), puis
--      Fondation Green OpenLab, sans activité, en moment de 10:10 à 10:20.
--      Hackathon : l'activité « Intelligence artificielle et environnement » de
--      l'IFDD, de 10:20 à 11:20, temps fort. « Genre et Climat » à 13:00,
--      inchangée. Global Actions n'est pas rattachée (voir 7).
--   6. Changer l'heure d'une séance émet `programme.session.rescheduled`, comme
--      le planificateur : réunion Zoom recalée, rappels déplacés, inscrits
--      prévenus. Aucun des trois n'existe pour ces séances au 29/10 ; le compte
--      rendu dit combien d'événements partent.
--   7. Global Actions (« Accélérer l'action climatique par le capital humain »)
--      quitte le 14/11 pour le MARDI 17/11 À 13:00, à la demande du
--      commanditaire (30/10). Faute d'autre information, elle GARDE SA DURÉE
--      (2 h 15) : fin à 15:15. Hors de tout fil. Un chevauchement le 17/11 est
--      signalé au planificateur, jamais bloqué (règle 2).
--
-- Production :
--   ops/contenu/2026-10-30-journees-cdp31.sh flux | ssh <serveur> "cd /opt/epavillon && $COMPOSE exec -T postgres psql -U postgres -d epavillon -X -q -v ON_ERROR_STOP=1"

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'contenu-2026-10-30-journees-cdp31', true) \g /dev/null

CREATE FUNCTION pg_temp.fr(p text) RETURNS jsonb LANGUAGE sql IMMUTABLE STRICT AS $$ SELECT jsonb_build_object('fr', p) $$;
-- Heure d'Antalya : le fuseau de l'édition est vérifié plus bas.
CREATE FUNCTION pg_temp.a(p_jour int, p_heure text) RETURNS timestamptz LANGUAGE sql IMMUTABLE AS
    $$ SELECT (format('2026-11-%s %s', p_jour, p_heure)::timestamp) AT TIME ZONE 'Europe/Istanbul' $$;

-- Garde-fous -----------------------------------------------------------------------

CREATE TEMP TABLE lc_edition ON COMMIT DROP AS
SELECT e.id AS event_id,
       (SELECT v.id FROM event.venues v WHERE v.event_id = e.id AND v.kind = 'pavilion'
        ORDER BY v.created_at, v.id LIMIT 1) AS pavillon_id
FROM event.events e
WHERE e.slug = 'cdp31-antalya-2026' AND e.timezone = 'Europe/Istanbul' AND e.status <> 'cancelled';

DO $$
BEGIN
    IF (SELECT count(*) FROM lc_edition) <> 1 THEN
        RAISE EXCEPTION 'contenu : édition cdp31-antalya-2026 introuvable, annulée ou hors du fuseau Europe/Istanbul';
    END IF;
    IF to_regclass('event.programme_track_moments') IS NULL
       OR (SELECT count(*) FROM reference.taxonomy_terms WHERE taxonomy_code = 'track_partner_role') < 4 THEN
        RAISE EXCEPTION 'contenu : migration 2026-10-30-journees-speciales non jouée';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM identity.people WHERE id = '01a00000-0000-7000-8000-00000000f1a1') THEN
        RAISE EXCEPTION 'contenu : personne technique des lots absente';
    END IF;
END $$;

-- Séances : identifiant, début de titre, heure actuelle, heure voulue ----------------

CREATE TEMP TABLE lc_seance (
    -- `fil` nul : séance déplacée sans être rattachée.
    session_id uuid PRIMARY KEY, fil text, partie text, rang smallint NOT NULL,
    temps_fort boolean NOT NULL, titre text NOT NULL,
    avant_debut timestamptz NOT NULL, avant_fin timestamptz NOT NULL,
    debut timestamptz NOT NULL, fin timestamptz NOT NULL
) ON COMMIT DROP;

INSERT INTO lc_seance VALUES
    -- Finance durable, 13/11
    ('01a17da1-9bea-7bbd-a04f-8f8fdd871249', 'finance', 'p1', 10, false, 'Événement parallele sur le travail conjoint OCDE-OIF/IFDD',
        pg_temp.a(13, '09:00'), pg_temp.a(13, '10:00'), pg_temp.a(13, '09:15'), pg_temp.a(13, '10:00')),
    ('01a17da1-9bea-72b3-83db-40fd3f7f8a23', 'finance', 'p2', 20, false, 'Du financement climatique à l''action régionale',
        pg_temp.a(13, '10:15'), pg_temp.a(13, '11:15'), pg_temp.a(13, '10:15'), pg_temp.a(13, '11:15')),
    ('01a17da1-9bea-7ada-aec5-8d97f1584311', 'finance', 'p2', 30, false, 'De l''idée au financement',
        pg_temp.a(13, '11:30'), pg_temp.a(13, '12:30'), pg_temp.a(13, '11:30'), pg_temp.a(13, '12:30')),
    ('01a17da1-9bea-7d75-93ee-8312895f5dc8', 'finance', 'p2', 40, false, 'Forêt, femmes, finance',
        pg_temp.a(13, '11:30'), pg_temp.a(13, '12:30'), pg_temp.a(13, '11:30'), pg_temp.a(13, '12:30')),
    ('01a17da1-9bea-7939-b396-973fa50cb895', 'finance', 'p2', 50, false, 'Autorité Nationale de Budgétisation Climat',
        pg_temp.a(13, '13:00'), pg_temp.a(13, '14:00'), pg_temp.a(13, '13:00'), pg_temp.a(13, '14:00')),
    ('01a17da1-9bea-7504-9fd5-d49a5cca34a3', 'finance', 'p2', 60, false, 'Du récif à la résilience',
        pg_temp.a(13, '14:15'), pg_temp.a(13, '15:15'), pg_temp.a(13, '14:15'), pg_temp.a(13, '15:15')),
    ('01a17da1-9bea-78c9-a645-bd452739a869', 'finance', 'p3', 70, false, 'Rencontre avec les experts',
        pg_temp.a(13, '15:30'), pg_temp.a(13, '16:30'), pg_temp.a(13, '15:30'), pg_temp.a(13, '16:30')),
    ('01a17da1-9bea-70b7-baf0-8479c194a4e8', 'finance', 'p3', 80, false, 'Construire des villes africaines résilientes',
        pg_temp.a(13, '16:45'), pg_temp.a(13, '17:45'), pg_temp.a(13, '16:45'), pg_temp.a(13, '17:45')),
    -- Jeunesse, 14/11 : cinq pitchs de dix minutes, puis le hackathon
    ('01a17da1-9bea-7a44-9d47-61f097d20ff9', 'jeunesse', 'inspirer', 10, false, 'Présentation de la caravane citoyenne africaine',
        pg_temp.a(14, '10:15'), pg_temp.a(14, '12:30'), pg_temp.a(14, '09:20'), pg_temp.a(14, '09:30')),
    ('01a17da1-9bea-736e-8afe-514fd510dffb', 'jeunesse', 'inspirer', 20, false, 'Autonomiser les jeunes et les femmes',
        pg_temp.a(14, '10:15'), pg_temp.a(14, '11:15'), pg_temp.a(14, '09:30'), pg_temp.a(14, '09:40')),
    ('01a17da1-9bea-7108-b131-f8b5b05a8878', 'jeunesse', 'inspirer', 30, false, 'Les jeunes pour une urgence climatique',
        pg_temp.a(14, '10:15'), pg_temp.a(14, '11:15'), pg_temp.a(14, '09:40'), pg_temp.a(14, '09:50')),
    ('01a17da1-9bea-7368-91c4-f7f5434ef814', 'jeunesse', 'inspirer', 40, false, 'Agriculture résiliente et énergie propre',
        pg_temp.a(14, '10:15'), pg_temp.a(14, '12:30'), pg_temp.a(14, '09:50'), pg_temp.a(14, '10:00')),
    ('01a17da1-9bea-77b0-9e28-10961bd0e26f', 'jeunesse', 'inspirer', 50, false, 'Atelier sur l''intelligence artificielle (IA) et le renforcement',
        pg_temp.a(14, '10:15'), pg_temp.a(14, '12:30'), pg_temp.a(14, '10:00'), pg_temp.a(14, '10:10')),
    ('01a17da1-9bea-7da2-af56-7c4e6a731f1c', 'jeunesse', 'creer', 60, true, 'Intelligence artificielle et environnement',
        pg_temp.a(14, '10:15'), pg_temp.a(14, '12:30'), pg_temp.a(14, '10:20'), pg_temp.a(14, '11:20')),
    ('01a17da1-9bea-7fec-84de-1627d22c8081', 'jeunesse', 'agir', 70, false, 'Genre et Climat',
        pg_temp.a(14, '13:00'), pg_temp.a(14, '14:00'), pg_temp.a(14, '13:00'), pg_temp.a(14, '14:00')),
    -- Hors journée jeunesse : déplacée au 17/11, durée gardée
    ('01a17da1-9bea-7eae-a833-bc3b74d15dca', NULL, NULL, 0, false, 'Accélérer l''action climatique par le capital humain',
        pg_temp.a(14, '10:15'), pg_temp.a(14, '12:30'), pg_temp.a(17, '13:00'), pg_temp.a(17, '15:15'));

DO $$
DECLARE r record;
BEGIN
    FOR r IN
        SELECT x.session_id, x.titre, s.id IS NULL AS absente, s.event_id IS DISTINCT FROM ed.event_id AS ailleurs,
               s.status, s.title ->> 'fr' AS titre_lu,
               (s.starts_at, s.ends_at) IN ((x.avant_debut, x.avant_fin), (x.debut, x.fin)) AS heure_connue
        FROM lc_seance x CROSS JOIN lc_edition ed
        LEFT JOIN programme.sessions s ON s.id = x.session_id
    LOOP
        IF r.absente OR r.ailleurs THEN
            RAISE EXCEPTION 'contenu : séance % (« % ») absente de la CdP31', r.session_id, r.titre;
        END IF;
        IF platform.normalize_label(r.titre_lu) NOT LIKE platform.normalize_label(r.titre) || '%' THEN
            RAISE EXCEPTION 'contenu : séance % retitrée, « % » attendu, « % » lu', r.session_id, r.titre, r.titre_lu;
        END IF;
        IF r.status IN ('cancelled', 'postponed', 'completed') THEN
            RAISE EXCEPTION 'contenu : séance « % » à l''état %, à trancher', r.titre_lu, r.status;
        END IF;
        IF NOT r.heure_connue THEN
            RAISE EXCEPTION 'contenu : séance « % » déplacée depuis le relevé, à trancher', r.titre_lu;
        END IF;
    END LOOP;
END $$;

-- Organisations partenaires ---------------------------------------------------------
-- `noms` : dénominations qui reprennent une fiche existante ; `sigle` seul ne
-- suffit pas à reprendre (deux fiches peuvent porter le même), il arrête le lot.

CREATE TEMP TABLE lc_org (
    cle text PRIMARY KEY, id_fixe uuid NOT NULL, nom text NOT NULL, sigle text, type_code text NOT NULL,
    pays text, site text, noms jsonb NOT NULL, org_id uuid
) ON COMMIT DROP;

INSERT INTO lc_org (cle, id_fixe, nom, sigle, type_code, pays, site, noms) VALUES
    ('oif', '01a18dc9-5da9-7bd9-9a7c-46022b3e602c', 'Organisation internationale de la Francophonie', 'OIF',
     'international_organization', 'FR', 'https://www.francophonie.org',
     '[["International Organisation of La Francophonie", "translation", "en"]]'),
    ('giz', '01a18dc9-5da9-7e03-a7ce-b3e6a16d14b8', 'Deutsche Gesellschaft für Internationale Zusammenarbeit', 'GIZ',
     'public_national_institution', 'DE', 'https://www.giz.de',
     '[["Agence allemande de coopération internationale", "translation", "fr"]]'),
    ('fonds_adaptation', '01a18dc9-5da9-78f6-b8c9-5e29fd474a24', 'Fonds pour l''adaptation', NULL,
     'international_organization', 'US', 'https://www.adaptation-fund.org',
     '[["Adaptation Fund", "translation", "en"]]'),
    ('fonds_vert', '01a18dc9-5da9-76e1-ae8c-dd5a4138232c', 'Fonds vert pour le climat', 'GCF',
     'international_organization', 'KR', 'https://www.greenclimate.fund',
     '[["Green Climate Fund", "translation", "en"], ["Fonds vert climat", "short", "fr"]]'),
    ('qjm', '01a18dc9-5da9-7f2c-acd5-1bf556d1d1da', 'Québec jeunesse monde', 'QJM',
     'ngo_association', 'CA', NULL, '[]');

DO $$
DECLARE
    r record;
    v_trouvees uuid[];
    v_sigle uuid[];
BEGIN
    FOR r IN SELECT * FROM lc_org LOOP
        SELECT array_agg(DISTINCT o.id) INTO v_trouvees
        FROM org.organization_names n
        JOIN org.organizations o ON o.id = n.organization_id AND o.status IN ('candidate', 'active')
        WHERE n.kind <> 'acronym'
          AND n.name_normalized IN (
              SELECT platform.normalize_label(r.nom)
              UNION ALL
              SELECT platform.normalize_label(v ->> 0) FROM jsonb_array_elements(r.noms) v);

        IF cardinality(v_trouvees) > 1 THEN
            RAISE EXCEPTION 'contenu : « % » désigne plusieurs fiches (%), à fusionner d''abord', r.nom, v_trouvees;
        ELSIF cardinality(v_trouvees) = 1 THEN
            UPDATE lc_org SET org_id = v_trouvees[1] WHERE cle = r.cle;
            IF v_trouvees[1] <> r.id_fixe THEN
                RAISE NOTICE 'contenu : « % » reprend la fiche existante %', r.nom, v_trouvees[1];
            END IF;
            CONTINUE;
        END IF;

        IF r.sigle IS NOT NULL THEN
            SELECT array_agg(DISTINCT o.id) INTO v_sigle
            FROM org.organization_names n
            JOIN org.organizations o ON o.id = n.organization_id AND o.status IN ('candidate', 'active')
            WHERE n.kind = 'acronym' AND n.name_normalized = platform.normalize_label(r.sigle);
            IF v_sigle IS NOT NULL THEN
                RAISE EXCEPTION 'contenu : le sigle % est déjà porté par % sous un autre nom que « % », à trancher',
                    r.sigle, v_sigle, r.nom;
            END IF;
        END IF;
        IF EXISTS (SELECT 1 FROM org.organizations WHERE slug = platform.slugify(r.nom)) THEN
            RAISE EXCEPTION 'contenu : adresse %, prise par une fiche fusionnée ou rejetée, à trancher', platform.slugify(r.nom);
        END IF;

        INSERT INTO org.organizations (id, legal_name, acronym, slug, organization_type_code, country_id,
                                       website, status, created_by)
        VALUES (r.id_fixe, r.nom, r.sigle, platform.slugify(r.nom), r.type_code,
                (SELECT c.id FROM reference.countries c WHERE c.iso2 = r.pays),
                r.site, 'candidate', current_setting('app.actor_id')::uuid);
        INSERT INTO org.organization_names (organization_id, name, kind, locale, is_confirmed, created_by)
        SELECT r.id_fixe, v ->> 0, (v ->> 1)::org.name_kind, v ->> 2, true, current_setting('app.actor_id')::uuid
        FROM jsonb_array_elements(r.noms) v
        ON CONFLICT DO NOTHING;
        UPDATE lc_org SET org_id = r.id_fixe WHERE cle = r.cle;
        RAISE NOTICE 'contenu : fiche créée, « % »', r.nom;
    END LOOP;
END $$;

-- Lieu de l'atelier -----------------------------------------------------------------

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM event.venues v JOIN lc_edition ed ON ed.event_id = v.event_id
               WHERE platform.normalize_label(v.name ->> 'fr') LIKE '%doubletree%'
                 AND v.id <> '01a18dc9-5da8-7756-868b-13c95dd46916') THEN
        RAISE EXCEPTION 'contenu : un lieu DoubleTree existe déjà pour la CdP31, à trancher';
    END IF;
END $$;

INSERT INTO event.venues (id, event_id, name, kind, address, map_url, access_note, latitude, longitude)
SELECT '01a18dc9-5da8-7756-868b-13c95dd46916', ed.event_id,
       pg_temp.fr('Hôtel DoubleTree by Hilton'), 'other',
       'Etiler, Adnan Menderes Blv. No:61, 07010 Muratpaşa / Antalya, Turquie',
       'https://www.google.com/maps/search/?api=1&query=DoubleTree+by+Hilton+Antalya+City+Centre',
       pg_temp.fr('En ville, hors de la zone bleue. La CdP31 ouvre le lendemain.'),
       36.896540, 30.710629
FROM lc_edition ed
ON CONFLICT (id) DO NOTHING;

-- Fils ------------------------------------------------------------------------------

DO $$
BEGIN
    IF EXISTS (SELECT 1 FROM event.programme_tracks t JOIN lc_edition ed ON ed.event_id = t.event_id
               WHERE (t.code IN ('atelier_preparatoire', 'journee_finance_durable', 'journee_jeunesse_climat')
                      OR t.slug IN ('atelier-preparatoire', 'journee-finance-durable', 'journee-jeunesse-climat'))
                 AND t.id NOT IN ('01a18dc9-5da9-7f16-876a-663a2e08683c', '01a18dc9-5da9-7bc8-bbc3-f56510428bf5',
                                  '01a18dc9-5da9-7b2a-a15a-0b6917f228ec')) THEN
        RAISE EXCEPTION 'contenu : un fil de même code ou de même adresse existe déjà pour la CdP31, à trancher';
    END IF;
END $$;

INSERT INTO event.programme_tracks (
    id, event_id, code, slug, kind, title, subtitle, description, tagline, starts_on, ends_on, color_hex,
    published_at, sort_order, target_audiences, support_note,
    feature_kicker, feature_title, feature_body, feature_link_label, feature_link_url, venue_id)
SELECT x.id, ed.event_id, x.code, x.slug, x.kind::event.track_kind, x.title, x.subtitle, x.description, x.tagline,
       x.jour, x.jour, x.couleur, CASE WHEN x.publie THEN now() END, x.rang, x.publics, x.soutien,
       x.f_surtitre, x.f_titre, x.f_texte, x.f_lien_libelle, x.f_lien, x.lieu
FROM lc_edition ed, (VALUES
    ('01a18dc9-5da9-7f16-876a-663a2e08683c'::uuid, 'atelier_preparatoire', 'atelier-preparatoire', 'side_programme',
     pg_temp.fr('Atelier préparatoire francophone des négociateurs-trices et des représentant(e)s de la société civile'),
     pg_temp.fr('Une synthèse actualisée des enjeux de la CdP31, la veille de l''ouverture.'),
     pg_temp.fr(E'Permettre aux négociateurs et autres parties prenantes francophones à la CdP31 d''avoir une synthèse actualisée des enjeux : les principaux enjeux des négociations, la mise en œuvre de l''Accord de Paris, les positions des principales Parties.\n\nUn temps d''échange entre négociateurs, délégués, ONG, secteur privé, collectivités et groupes régionaux.'),
     NULL::jsonb, date '2026-11-08', NULL::text, true, 1::smallint,
     ARRAY[pg_temp.fr('Négociateurs'), pg_temp.fr('Points focaux de la CCNUCC'), pg_temp.fr('Décideurs politiques'),
           pg_temp.fr('Experts'), pg_temp.fr('ONG'), pg_temp.fr('Secteur privé'),
           pg_temp.fr('Collectivités territoriales'), pg_temp.fr('Organisations internationales et régionales')]::platform.i18n_text[],
     NULL::jsonb,
     pg_temp.fr('Présenté pendant l''atelier'), pg_temp.fr('Guide des négociations de la CdP31'),
     pg_temp.fr('Le Guide et son Résumé sont présentés l''après-midi par leurs auteurs. Le Guide se lit aussi dans l''application Guide Négo, au fil des sessions.'),
     pg_temp.fr('Ouvrir dans Guide Négo'), '/guide-nego', '01a18dc9-5da8-7756-868b-13c95dd46916'::uuid),
    ('01a18dc9-5da9-7bc8-bbc3-f56510428bf5', 'journee_finance_durable', 'journee-finance-durable', 'special_day',
     pg_temp.fr('Journée finance durable en Francophonie'),
     pg_temp.fr('Opportunités et partenariats pour l''accès à la finance durable en Francophonie'),
     pg_temp.fr(E'L''OIF, ses partenaires financiers et des porteurs de projets francophones se retrouvent au pavillon pour rapprocher les guichets de la finance climat de ceux qui en ont besoin.\n\nLancement de l''appel à manifestation d''intérêt des États membres pour l''accompagnement à l''accès à la finance durable, présentation de la plateforme francophone de la finance durable et de l''étude OIF-OCDE, puis des cas concrets : Sahel, collectivités, budgétisation de la CDN, littoral insulaire, villes africaines.'),
     NULL, date '2026-11-13', '#00A1E4', false, 2,
     '{}'::platform.i18n_text[], NULL, NULL, NULL, NULL, NULL, NULL, NULL),
    ('01a18dc9-5da9-7b2a-a15a-0b6917f228ec', 'journee_jeunesse_climat', 'journee-jeunesse-climat', 'special_day',
     pg_temp.fr('Journée action jeunesse pour le climat en Francophonie'),
     pg_temp.fr('Innovations et engagement des jeunes pour accélérer l''action climatique en Francophonie'),
     pg_temp.fr('Une matinée portée par les jeunes francophones : pitchs d''organisations, hackathon sur l''intelligence artificielle appliquée au climat, jeu concours et scène artistique, puis un temps sur les compétences vertes et le leadership.'),
     pg_temp.fr('Hackathon, pitchs, jeu, scène'), date '2026-11-14', '#8FBF2F', false, 3,
     '{}'::platform.i18n_text[],
     pg_temp.fr('Avec le soutien financier du gouvernement du Québec, dans le cadre du Plan pour une économie verte 2030.'),
     NULL, NULL, NULL, NULL, NULL, NULL)
) AS x(id, code, slug, kind, title, subtitle, description, tagline, jour, couleur, publie, rang, publics, soutien,
       f_surtitre, f_titre, f_texte, f_lien_libelle, f_lien, lieu)
ON CONFLICT (id) DO NOTHING;

-- Parties ---------------------------------------------------------------------------

CREATE TEMP TABLE lc_partie (cle text, fil text, id uuid, PRIMARY KEY (fil, cle)) ON COMMIT DROP;
INSERT INTO lc_partie VALUES
    ('matin',      'atelier',  '01a18dc9-5da9-7d0a-a6b9-fc9240dd11c2'),
    ('apres-midi', 'atelier',  '01a18dc9-5da9-7822-bb37-18395a875865'),
    ('p1',         'finance',  '01a18dc9-5da9-7a36-b7c3-8e545da6b01a'),
    ('p2',         'finance',  '01a18dc9-5da9-7ae8-a8d8-ee8d9617fe72'),
    ('p3',         'finance',  '01a18dc9-5da9-74c0-809c-c47bfeba7d74'),
    ('inspirer',   'jeunesse', '01a18dc9-5da9-7100-94f8-3583ed99a30b'),
    ('creer',      'jeunesse', '01a18dc9-5da9-74a1-a3bd-609bf24531ac'),
    ('agir',       'jeunesse', '01a18dc9-5da9-7a17-a529-260157affbe3');

CREATE TEMP TABLE lc_fil (fil text PRIMARY KEY, track_id uuid NOT NULL) ON COMMIT DROP;
INSERT INTO lc_fil VALUES
    ('atelier',  '01a18dc9-5da9-7f16-876a-663a2e08683c'),
    ('finance',  '01a18dc9-5da9-7bc8-bbc3-f56510428bf5'),
    ('jeunesse', '01a18dc9-5da9-7b2a-a15a-0b6917f228ec');

INSERT INTO event.programme_track_parts (id, track_id, label, title, summary, sort_order)
SELECT p.id, f.track_id, x.label, x.title, x.summary, x.rang
FROM (VALUES
    ('atelier', 'matin', 1, pg_temp.fr('Matin'), NULL::jsonb, pg_temp.fr('Où en sont les négociations, thème par thème')),
    ('atelier', 'apres-midi', 2, pg_temp.fr('Après-midi'), NULL, pg_temp.fr('Le Guide des négociations et le Programme de la Francophonie')),
    ('finance', 'p1', 1, NULL, pg_temp.fr('Opportunités de financement durable et synergies d''action en Francophonie'),
        pg_temp.fr('Opportunités de financement et synergies d''action')),
    ('finance', 'p2', 2, NULL, pg_temp.fr('Initiatives pertinentes nationales, études de cas et témoignages'),
        pg_temp.fr('Initiatives nationales, études de cas et témoignages')),
    ('finance', 'p3', 3, NULL, pg_temp.fr('Partage entre les pairs et partenaires, réseautage'),
        pg_temp.fr('Partage entre pairs et partenaires, réseautage')),
    ('jeunesse', 'inspirer', 1, pg_temp.fr('S''inspirer'), NULL, pg_temp.fr('Ouverture et dix minutes qui inspirent')),
    ('jeunesse', 'creer', 2, pg_temp.fr('Créer et jouer'), NULL, pg_temp.fr('Hackathon, Info ou Intox, ClimArtistes')),
    ('jeunesse', 'agir', 3, pg_temp.fr('S''engager'), NULL, pg_temp.fr('Compétences vertes et leadership'))
) AS x(fil, cle, rang, label, title, summary)
JOIN lc_fil f USING (fil)
JOIN lc_partie p USING (fil, cle)
ON CONFLICT (id) DO NOTHING;

-- Moments (ni activité, ni inscription, ni conflit) ------------------------------------
-- Une précision par ligne : la page en fait des puces.

INSERT INTO event.programme_track_moments (id, track_id, part_id, starts_at, ends_at, label, description)
SELECT x.id::uuid, f.track_id, p.id, x.debut, x.fin, pg_temp.fr(x.libelle), pg_temp.fr(x.precisions)
FROM (VALUES
    ('01a18dc9-5da9-7d99-9d44-ac75e66d3b5f', 'atelier', 'matin', pg_temp.a(8, '09:00'), pg_temp.a(8, '09:15'),
     'Ouverture de l''atelier', E'Mot de l''OIF\nMot du Secrétariat de la CCNUCC'),
    ('01a18dc9-5da9-7fc1-8778-525c3dc3170a', 'atelier', 'matin', pg_temp.a(8, '09:45'), pg_temp.a(8, '12:00'),
     'Présentations des avancées par thématique',
     E'Atténuation\nAdaptation\nPertes et préjudices\nMécanismes de coopération\nFinance climat'),
    ('01a18dc9-5da9-7e21-a080-2108cd1fa189', 'atelier', 'matin', pg_temp.a(8, '12:00'), pg_temp.a(8, '12:30'),
     'Questions transversales', E'Cadre de la transparence\nTransfert de technologies\nAgriculture\nGenre et climat'),
    ('01a18dc9-5da9-780c-9f2b-89e3c5db8d83', 'atelier', 'matin', pg_temp.a(8, '12:30'), pg_temp.a(8, '13:30'),
     'Déjeuner', NULL),
    ('01a18dc9-5da9-7ab6-88a5-8c870ded9f06', 'atelier', 'apres-midi', pg_temp.a(8, '14:15'), pg_temp.a(8, '14:50'),
     'Présentation du Guide des négociations et du Programme de la Francophonie à la CdP31',
     E'Présentation du Guide des négociations\nLe Programme de la Francophonie à la CdP31'),
    ('01a18dc9-5da9-7488-a8d5-610b76db23c2', 'atelier', 'apres-midi', pg_temp.a(8, '14:50'), pg_temp.a(8, '15:00'),
     'Clôture', NULL),

    ('01a18dc9-5da9-7d6c-be1b-044d054df086', 'finance', 'p1', pg_temp.a(13, '08:00'), pg_temp.a(13, '08:45'),
     'Accueil et installation', NULL),
    ('01a18dc9-5da9-7624-b28e-4d122eb76fbb', 'finance', 'p1', pg_temp.a(13, '09:00'), pg_temp.a(13, '09:15'),
     'Ouverture de la journée',
     E'Mot d''ouverture, OIF\nLancement officiel de l''appel à manifestation d''intérêt des États membres de l''OIF pour l''accompagnement à l''accès à la finance durable\nPrésentation de la plateforme francophone de la finance durable'),
    ('01a18dc9-5da9-776b-8457-7b4e4ca743b1', 'finance', 'p3', pg_temp.a(13, '18:00'), NULL,
     'Clôture', NULL),

    ('01a18dc9-5da9-73b0-a24f-09446075b19d', 'jeunesse', 'inspirer', pg_temp.a(14, '08:00'), pg_temp.a(14, '09:00'),
     'Arrivée des jeunes et des participant-e-s, animation', NULL),
    ('01a18dc9-5da9-73ba-811a-b241fe15b71d', 'jeunesse', 'inspirer', pg_temp.a(14, '09:00'), pg_temp.a(14, '09:20'),
     'Ouverture officielle', NULL),
    ('01a18dc9-5da9-740e-a282-eca19d75768b', 'jeunesse', 'inspirer', pg_temp.a(14, '10:10'), pg_temp.a(14, '10:20'),
     '« Les 10 minutes qui inspirent » : Fondation Green OpenLab', NULL),
    ('01a18dc9-5da9-76fd-9d11-c7f0f6ff3eef', 'jeunesse', 'creer', pg_temp.a(14, '11:20'), pg_temp.a(14, '12:00'),
     'Jeu concours « Info ou Intox Climat » : qui sera le Champion ou la Championne CdP 31 / OIF ?', NULL),
    ('01a18dc9-5da9-7e57-a92c-4449efcf83b4', 'jeunesse', 'creer', pg_temp.a(14, '12:00'), pg_temp.a(14, '13:00'),
     'Les ClimArtistes en scène', 'Expression artistique')
) AS x(id, fil, cle, debut, fin, libelle, precisions)
JOIN lc_fil f USING (fil)
JOIN lc_partie p USING (fil, cle)
ON CONFLICT (id) DO NOTHING;

-- Partenaires -----------------------------------------------------------------------

INSERT INTO event.programme_track_partners (track_id, organization_id, role_term_id, sort_order)
SELECT f.track_id, o.org_id, t.id, x.rang
FROM (VALUES
    ('atelier',  'oif',              'organizer',         1),
    ('finance',  'oif',              'organizer',         1),
    ('finance',  'giz',              'partner',           2),
    ('finance',  'fonds_adaptation', 'partner',           3),
    ('finance',  'fonds_vert',       'partner',           4),
    ('jeunesse', 'oif',              'organizer',         1),
    ('jeunesse', 'quebec',           'financial_support', 2),
    ('jeunesse', 'qjm',              'partner',           3)
) AS x(fil, cle, role, rang)
JOIN lc_fil f USING (fil)
JOIN (SELECT cle, org_id FROM lc_org
      UNION ALL
      -- Fiche existante, retrouvée par son nom (règle n° 1).
      SELECT 'quebec', (SELECT o.id FROM org.organizations o
                        WHERE o.legal_name_normalized = platform.normalize_label('Gouvernement du Québec')
                          AND o.status IN ('candidate', 'active'))) o USING (cle)
JOIN reference.taxonomy_terms t ON t.taxonomy_code = 'track_partner_role' AND t.code = x.role
ON CONFLICT (track_id, organization_id) DO NOTHING;

DO $$
BEGIN
    IF (SELECT count(*) FROM event.programme_track_partners p JOIN lc_fil f ON f.track_id = p.track_id) < 8 THEN
        RAISE EXCEPTION 'contenu : partenaire manquant (Gouvernement du Québec introuvable ?)';
    END IF;
END $$;

-- Séances : heure, puis rattachement ----------------------------------------------------

CREATE TEMP TABLE lc_deplacee (id uuid, titre text, avant_debut timestamptz, debut timestamptz, fin timestamptz)
    ON COMMIT DROP;
WITH maj AS (
    UPDATE programme.sessions s
    SET starts_at = x.debut, ends_at = x.fin
    FROM lc_seance x
    WHERE s.id = x.session_id
      AND (s.starts_at, s.ends_at) = (x.avant_debut, x.avant_fin)
      AND (x.debut, x.fin) <> (x.avant_debut, x.avant_fin)
    RETURNING s.id, x.titre, x.avant_debut, x.debut, x.fin
)
INSERT INTO lc_deplacee SELECT * FROM maj;

INSERT INTO programme.session_tracks (session_id, track_id, part_id, sort_order, is_highlight, added_by)
SELECT x.session_id, f.track_id, p.id, x.rang, x.temps_fort, current_setting('app.actor_id')::uuid
FROM lc_seance x
JOIN lc_fil f USING (fil)
JOIN lc_partie p ON p.fil = x.fil AND p.cle = x.partie
ON CONFLICT (session_id, track_id) DO NOTHING;

-- Compte rendu ------------------------------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events
               WHERE correlation_id = current_setting('app.request_id')
                 AND occurred_at = now() AND event_type <> 'programme.session.rescheduled') THEN
        RAISE EXCEPTION 'contenu : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = current_setting('app.request_id') AND occurred_at = now());
    END IF;
    FOR r IN SELECT to_char(avant_debut AT TIME ZONE 'Europe/Istanbul', 'DD/MM HH24:MI') AS avant,
                    to_char(debut AT TIME ZONE 'Europe/Istanbul', 'DD/MM HH24:MI') AS debut,
                    to_char(fin AT TIME ZONE 'Europe/Istanbul', 'HH24:MI') AS fin, titre
             FROM lc_deplacee ORDER BY debut LOOP
        RAISE NOTICE 'contenu : « % » de % à % - %', r.titre, r.avant, r.debut, r.fin;
    END LOOP;
    FOR r IN SELECT t.code, t.published_at IS NOT NULL AS publie,
                    (SELECT count(*) FROM event.programme_track_parts p WHERE p.track_id = t.id) AS parties,
                    (SELECT count(*) FROM event.programme_track_moments m WHERE m.track_id = t.id) AS moments,
                    (SELECT count(*) FROM event.programme_track_partners p WHERE p.track_id = t.id) AS partenaires,
                    (SELECT count(*) FROM programme.session_tracks s WHERE s.track_id = t.id) AS activites
             FROM event.programme_tracks t JOIN lc_fil f ON f.track_id = t.id ORDER BY t.sort_order LOOP
        RAISE NOTICE 'contenu : % (%) : % parties, % moments, % partenaires, % activités',
            r.code, CASE WHEN r.publie THEN 'publié' ELSE 'non publié' END,
            r.parties, r.moments, r.partenaires, r.activites;
    END LOOP;
    RAISE NOTICE 'contenu : % séances déplacées, % événements d''outbox (séance déplacée), % inscriptions sur les séances du lot',
        (SELECT count(*) FROM lc_deplacee),
        (SELECT count(*) FROM platform.outbox_events
         WHERE correlation_id = current_setting('app.request_id') AND occurred_at = now()),
        (SELECT count(*) FROM programme.registrations i JOIN lc_seance x ON x.session_id = i.session_id);
END $$;

COMMIT;
