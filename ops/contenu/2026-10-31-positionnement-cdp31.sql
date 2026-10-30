-- Contenu (31/10) : la CdP31 au stand, d'après le programme du commanditaire
-- (« Programme du Pavillon de la Francophonie COP31 », Word du 30/10 ajusté).
--
-- Chaque dossier rapproché du programme est RETENU (`accepted`) et sa séance
-- PLACÉE au stand, au créneau du programme. RIEN N'EST PUBLIÉ : ni séance
-- (`published_at`, état `scheduled`), ni programme de l'édition, ni journée.
-- Jouer APRÈS 2026-10-30-journees-cdp31.sql.
--
-- MÊMES ÉCRITURES QUE L'APPLICATION, sans l'API :
--   - décision : UPDATE de `status` et `decision_reason` (nul), comme
--     service/transition.rs. Le déclencheur arbitre le graphe, journalise, date
--     et émet `programme.proposal.<état>`. Un dossier déposé passe d'abord en
--     évaluation (submitted -> under_review -> accepted : le graphe n'a pas de
--     raccourci). Décision signée par la personne technique des lots.
--   - naissance des séances d'un dossier retenu qui n'en a pas : la recopie de
--     service/birth.rs (créneau souhaité, à défaut premier jour à l'heure de
--     l'appel, à défaut début de l'édition ; adresse suffixée sur collision ;
--     intervenants, co-organisations, thématiques). Une seule ici : la BAD.
--   - placement : UPDATE de room_id, starts_at, ends_at et event_day_id remis à
--     nul pour que la base redéduise la journée, comme service/planner.rs
--     (le site n'envoie jamais de journée). Jamais refusé pour chevauchement.
--
-- CE QUE CELA DÉCLENCHE (relevé sur le code du worker au 31/10) :
--   - `programme.proposal.under_review` / `.accepted` : aucun consommateur ne
--     les sert (ni avis, ni courriel, ni rappel ; le catalogue d'avis ne porte
--     que `programme.proposal.decided`, qu'aucun événement n'émet). Seule la
--     télémétrie les trace. L'organisation voit en revanche « retenu » et son
--     créneau dans son espace, à sa prochaine visite.
--   - `programme.session.created` (BAD) et `.rescheduled` (séances déplacées) :
--     rappels et réunion Zoom ne concernent que des inscrits ou des séances
--     publiées, il n'y en a aucun ; l'avis de déplacement va aux inscrits, zéro.
--   Le lot s'arrête si une séance touchée a un inscrit ou une réunion Zoom ;
--   le compte rendu compte les événements par type et vérifie qu'aucun travail
--   n'est mis en file.
--
-- REJOUABLE : un dossier déjà retenu n'est pas retouché, une séance déjà à sa
-- place non plus. Une seconde passe n'écrit rien et n'émet rien.
--
-- CE QUI EXISTE EST LU, JAMAIS DEVINÉ : dossier par identifiant, numéro ET
-- début de titre ; séance par identifiant ET heure relevée (sauvegarde
-- epavillon-20261030-173015 + lot du 30/10). Dossier retitré, changé d'état,
-- séance déplacée, publiée, annulée ou dans une autre salle : arrêt avant
-- toute écriture.
--
-- RAPPROCHEMENT, À FAIRE VALIDER PAR LE COMMANDITAIRE
--   1. 66 dossiers retenus, 65 séances au stand (CDP31-00213, retenu et placé à la main le 30/10 à 17:32 UTC au 9/11 11:00, retiré du lot). Trois déplacées : Mission
--      Inclusion (20/11 09:00 -> 17/11 16:45), Initiative Femmes, Eau et
--      Résilience (20/11 10:15 -> 09:00), Enda ECOPOP / PROMOVAL (20/11 14:15 ->
--      10:15). Une créée : BAD, 17/11 14:15-15:15 (le Word donne une heure, le
--      dossier 90 min). Les autres gardent leur heure, qui est celle du Word.
--   2. Titre reformulé dans le Word, dossier unique de l'organisation au même
--      créneau : Agriconsulting (PEID, 11/11), Odysseo (14/11), ministère de
--      l'Agriculture de Tunisie (19/11), ministère québécois (fiche « Gouvernement
--      du Québec », 12/11).
--   3. Le 9/11, une cellule de 09:00 à 11:15 porte l'ouverture, la Fresque du
--      Climat et COP Bike Ride : les deux activités gardent 09:00-11:15. Même
--      salle, même heure : le contrôle avant publication le BLOQUE (règle 3).
--   4. Les cinq pitchs du 14/11 (JVE Centrafrique, Youth Center for Impact,
--      Fédération L'Air et Moi, Youth Voices Cameroon, NRCI) gardent les dix
--      minutes posées le 30/10 ; le Word cite Fondation Green OpenLab en tête,
--      le lot du 30/10 la place en dernier (moment, sans dossier).
--   5. Global Actions : RETENUE, séance NON TOUCHÉE. Le Word la met de 13:00 à
--      14:00 le 17/11, le lot du 30/10 de 13:00 à 15:15 ; elle chevauche alors la
--      BAD (14:15). Sa journée de rattachement est restée au 14/11 (le lot du
--      30/10 a changé l'heure sans remettre event_day_id à nul) : à recaler au
--      planificateur, qui corrige les deux d'un geste. Sans salle, elle bloque
--      aussi la publication.
--   NON TRAITÉS (douteux), ni retenus ni placés :
--     - OCDE, « Événement parallèle... » (13/11 09:15) : le Word n'a qu'un
--       « Panel de partenaires et d'experts » sans organisation ;
--     - AIRF « De l'idée au financement » et CASAD-Bénin « Forêt, femmes,
--       finance... Sakété » (13/11 11:30) : le Word en fait UNE activité,
--       « AIRF et Commune de Sakété : Enjeux de financements pour les régions
--       et les municipalités » ;
--     - IFDD « Intelligence artificielle et environnement » (14/11 10:20) : le
--       Word attribue le hackathon à la Fondation Green OpenLab ;
--     - Commission de l'Union africaine, second dossier ClimSA (déposé, quasi
--       doublon) : seul celui au titre exact du Word est retenu.
--
-- Production :
--   ops/contenu/2026-10-31-positionnement-cdp31.sh flux | ssh <serveur> "cd /opt/epavillon && $COMPOSE exec -T postgres psql -U postgres -d epavillon -X -q -v ON_ERROR_STOP=1"

\set ON_ERROR_STOP on
BEGIN;
SELECT set_config('app.actor_id', '01a00000-0000-7000-8000-00000000f1a1', true),
       set_config('app.request_id', 'contenu-2026-10-31-positionnement-cdp31', true) \g /dev/null

-- Heure d'Antalya : le fuseau de l'édition est vérifié plus bas.
CREATE FUNCTION pg_temp.a(p_jour int, p_heure text) RETURNS timestamptz LANGUAGE sql IMMUTABLE AS
    $$ SELECT (format('2026-11-%s %s', p_jour, p_heure)::timestamp) AT TIME ZONE 'Europe/Istanbul' $$;

-- Garde-fous -----------------------------------------------------------------------

CREATE TEMP TABLE lc_edition ON COMMIT DROP AS
SELECT e.id AS event_id, e.programme_published_at,
       (SELECT r.id FROM event.rooms r JOIN event.venues v ON v.id = r.venue_id
        WHERE v.event_id = e.id AND v.kind = 'pavilion' AND NOT r.is_virtual
          AND r.id = '01a17da1-9b67-7e75-b5c0-d68db6d70feb'
          AND platform.normalize_label(r.name ->> 'fr') = 'stand') AS stand_id
FROM event.events e
WHERE e.slug = 'cdp31-antalya-2026' AND e.timezone = 'Europe/Istanbul' AND e.status <> 'cancelled';

DO $$
BEGIN
    IF (SELECT count(*) FROM lc_edition) <> 1 THEN
        RAISE EXCEPTION 'contenu : édition cdp31-antalya-2026 introuvable, annulée ou hors du fuseau Europe/Istanbul';
    END IF;
    IF (SELECT stand_id FROM lc_edition) IS NULL THEN
        RAISE EXCEPTION 'contenu : salle « Stand » du pavillon de la CdP31 introuvable';
    END IF;
    IF (SELECT programme_published_at FROM lc_edition) IS NOT NULL THEN
        RAISE EXCEPTION 'contenu : programme de la CdP31 déjà publié, à trancher avant de retenir';
    END IF;
    IF to_regclass('event.programme_track_moments') IS NULL THEN
        RAISE EXCEPTION 'contenu : migration 2026-10-30-journees-speciales non jouée';
    END IF;
    IF NOT EXISTS (SELECT 1 FROM identity.people WHERE id = '01a00000-0000-7000-8000-00000000f1a1') THEN
        RAISE EXCEPTION 'contenu : personne technique des lots absente';
    END IF;
END $$;

-- Dossiers : numéro, identifiant, début de titre, séance, état relevé, heure
-- relevée, heure voulue (nulle : séance laissée telle quelle) ---------------------

CREATE TEMP TABLE lc_position (
    reference text PRIMARY KEY, proposal_id uuid NOT NULL UNIQUE, titre text NOT NULL,
    session_id uuid, etat_releve text NOT NULL,
    avant_debut timestamptz, avant_fin timestamptz, debut timestamptz, fin timestamptz
) ON COMMIT DROP;

INSERT INTO lc_position VALUES
    -- 9/11
    ('CDP31-00122', '01a17da1-9bd7-7ae8-8777-ffb4dc0477e1', 'La Fresque du Climat : former, mobiliser,', '01a17da1-9bea-73e0-b3bf-ce02b544070f', 'under_review',
        pg_temp.a(9, '09:00'), pg_temp.a(9, '11:15'), pg_temp.a(9, '09:00'), pg_temp.a(9, '11:15')),
    ('CDP31-00180', '01a17da1-9bda-7d81-b123-b14c35f3c8b7', 'Le vélo au quotidien comme levier abordable et', '01a17da1-9bea-7f38-92f4-e81b507f6d8e', 'under_review',
        pg_temp.a(9, '09:00'), pg_temp.a(9, '11:15'), pg_temp.a(9, '09:00'), pg_temp.a(9, '11:15')),
    ('CDP31-00128', '01a17da1-9bd9-7e53-9289-72365d99ea81', 'La finance circulaire pour le climat -', '01a17da1-9bea-7117-bb78-e1e21a2e339f', 'under_review',
        pg_temp.a(9, '15:30'), pg_temp.a(9, '16:30'), pg_temp.a(9, '15:30'), pg_temp.a(9, '16:30')),
    ('CDP31-00245', '01a17da1-9bd6-7c5f-ac86-0e53f62bac6b', 'Femmes expertes en transparence climatique :', '01a17da1-9bea-7e9b-985c-924734a15d5e', 'under_review',
        pg_temp.a(9, '16:45'), pg_temp.a(9, '17:45'), pg_temp.a(9, '16:45'), pg_temp.a(9, '17:45')),
    -- 10/11
    ('CDP31-00232', '01a17da1-9bd7-7c93-a2fc-7725b487aa79', 'Énergies propres et électrification : comment', '01a17da1-9bea-7234-b694-cb9b39ee0651', 'under_review',
        pg_temp.a(10, '09:00'), pg_temp.a(10, '10:00'), pg_temp.a(10, '09:00'), pg_temp.a(10, '10:00')),
    ('CDP31-00195', '01a17da1-9bda-7d92-95de-bea82a14bca3', 'Atelier sur la mobilité humaine et le', '01a17da1-9bea-761a-80d9-b01c4b7b9f72', 'under_review',
        pg_temp.a(10, '10:15'), pg_temp.a(10, '11:15'), pg_temp.a(10, '10:15'), pg_temp.a(10, '11:15')),
    ('CDP31-00181', '01a17da1-9bd9-752e-9d81-19f3b5d6dbb8', 'Gestion durable des déchets organiques : de la', '01a17da1-9bea-7549-84f6-42139f87733f', 'under_review',
        pg_temp.a(10, '11:30'), pg_temp.a(10, '12:30'), pg_temp.a(10, '11:30'), pg_temp.a(10, '12:30')),
    ('CDP31-00223', '01a17da1-9bda-7f9f-8f42-d43647544f95', 'Bassin du Congo : médias, science, formation et', '01a17da1-9bea-7aaa-a7ce-23bb9184fb84', 'under_review',
        pg_temp.a(10, '13:00'), pg_temp.a(10, '14:00'), pg_temp.a(10, '13:00'), pg_temp.a(10, '14:00')),
    ('CDP31-00172', '01a17da1-9bd9-70b3-8a2c-533e66e82f73', 'Valorisation de l''action climatique des États', '01a17da1-9bea-7507-8597-4f5f06068bf8', 'under_review',
        pg_temp.a(10, '14:15'), pg_temp.a(10, '15:15'), pg_temp.a(10, '14:15'), pg_temp.a(10, '15:15')),
    ('CDP31-00187', '01a17da1-9bda-7c15-afbd-82d9590eb346', 'Agir pour une transition juste : faire', '01a17da1-9bea-73fc-a1fe-510d0bf5b050', 'under_review',
        pg_temp.a(10, '15:30'), pg_temp.a(10, '16:30'), pg_temp.a(10, '15:30'), pg_temp.a(10, '16:30')),
    -- 11/11
    ('CDP31-00214', '01a17da1-9bd9-7923-87b3-42f152585555', 'Adaptation et résilience hydrique : le nexus', '01a17da1-9bea-77be-8ac4-b2e5dc7395c2', 'under_review',
        pg_temp.a(11, '09:00'), pg_temp.a(11, '10:00'), pg_temp.a(11, '09:00'), pg_temp.a(11, '10:00')),
    ('CDP31-00234', '01a17da1-9bd9-7967-8af6-afc1a9e649c6', 'Villes du Sahel : financer l’adaptation au plus', '01a17da1-9bea-7fe0-ac5e-f1b4bbb247ee', 'under_review',
        pg_temp.a(11, '10:15'), pg_temp.a(11, '11:15'), pg_temp.a(11, '10:15'), pg_temp.a(11, '11:15')),
    ('CDP31-00203', '01a17da1-9bd9-7569-b479-93047f8d5015', 'Vivre avec un territoire qui change : pertes et', '01a17da1-9bea-7173-9a83-395eb9ddd3b9', 'under_review',
        pg_temp.a(11, '11:30'), pg_temp.a(11, '12:30'), pg_temp.a(11, '11:30'), pg_temp.a(11, '12:30')),
    ('CDP31-00217', '01a17da1-9bda-7541-a1ab-b018af50837f', 'Cibler les émissions liées aux déchets', '01a17da1-9bea-7714-bc60-7845b62a8d4a', 'under_review',
        pg_temp.a(11, '13:00'), pg_temp.a(11, '14:00'), pg_temp.a(11, '13:00'), pg_temp.a(11, '14:00')),
    ('CDP31-00249', '01a17da1-9bda-7f32-ab90-babc12863c52', 'Les PEID membres de la Francophonie face à la', '01a17da1-9bea-7a21-b11b-631f18b72f4d', 'under_review',
        pg_temp.a(11, '14:15'), pg_temp.a(11, '15:15'), pg_temp.a(11, '14:15'), pg_temp.a(11, '15:15')),
    ('CDP31-00207', '01a17da1-9bda-70d3-8042-554c8f6431f9', 'Favoriser l’adaptation au changement climatique', '01a17da1-9bea-717d-9687-2827a1ec5023', 'under_review',
        pg_temp.a(11, '15:30'), pg_temp.a(11, '16:30'), pg_temp.a(11, '15:30'), pg_temp.a(11, '16:30')),
    ('CDP31-00208', '01a17da1-9bd8-722c-8caf-884ee2364e80', 'Accès à l’énergie en milieu rural : faire des', '01a17da1-9bea-7616-b393-4fe0802ac6fc', 'under_review',
        pg_temp.a(11, '16:45'), pg_temp.a(11, '17:45'), pg_temp.a(11, '16:45'), pg_temp.a(11, '17:45')),
    -- 12/11
    ('CDP31-00184', '01a17da1-9bdc-7d92-872e-cfaccd732ab8', 'Gouvernance, financements et territoires :', '01a17da1-9bea-767a-9a61-10fddc108895', 'under_review',
        pg_temp.a(12, '09:00'), pg_temp.a(12, '10:00'), pg_temp.a(12, '09:00'), pg_temp.a(12, '10:00')),
    ('CDP31-00162', '01a17da1-9bd8-7733-88f1-9ad29d802adb', 'Intégrer adaptation et atténuation : vers des', '01a17da1-9bea-7f93-aaba-451054043a2d', 'under_review',
        pg_temp.a(12, '10:15'), pg_temp.a(12, '11:15'), pg_temp.a(12, '10:15'), pg_temp.a(12, '11:15')),
    ('CDP31-00226', '01a17da1-9bd7-7e46-8bc8-2447c0ec0ff3', 'Panel "« Electrifier maintenant » : 35% d''ici', '01a17da1-9bea-7dff-b5be-34de73e56a0d', 'under_review',
        pg_temp.a(12, '11:30'), pg_temp.a(12, '12:30'), pg_temp.a(12, '11:30'), pg_temp.a(12, '12:30')),
    ('CDP31-00215', '01a17da1-9bdb-70c7-ba81-7dccde662357', 'Circe.med, le réseau méditerranéen de', '01a17da1-9bea-706b-838e-bf8f1b5207c5', 'under_review',
        pg_temp.a(12, '13:00'), pg_temp.a(12, '14:00'), pg_temp.a(12, '13:00'), pg_temp.a(12, '14:00')),
    ('CDP31-00235', '01a17da1-9bd6-7c6d-9860-a370a681504d', 'Miser sur une coopération climatique à échelle', '01a17da1-9bea-7238-985b-95dfb7d060d7', 'under_review',
        pg_temp.a(12, '14:15'), pg_temp.a(12, '15:15'), pg_temp.a(12, '14:15'), pg_temp.a(12, '15:15')),
    ('CDP31-00210', '01a17da1-9bdc-78c7-a394-5df4838eb64c', 'Journée Cameroun – Solutions climatiques pour', '01a17da1-9bea-76cc-85b0-551a64ef4c60', 'under_review',
        pg_temp.a(12, '15:30'), pg_temp.a(12, '16:30'), pg_temp.a(12, '15:30'), pg_temp.a(12, '16:30')),
    ('CDP31-00132', '01a17da1-9bd8-7ff1-9228-f7cc390aebc4', 'Du suivi national de l’adaptation à', '01a17da1-9bea-7648-b879-59797735c1df', 'under_review',
        pg_temp.a(12, '16:45'), pg_temp.a(12, '17:45'), pg_temp.a(12, '16:45'), pg_temp.a(12, '17:45')),
    -- 13/11
    ('CDP31-00161', '01a17da1-9bdb-7b8a-a289-ec5086e89b10', 'Du financement climatique à l’action régionale', '01a17da1-9bea-72b3-83db-40fd3f7f8a23', 'under_review',
        pg_temp.a(13, '10:15'), pg_temp.a(13, '11:15'), pg_temp.a(13, '10:15'), pg_temp.a(13, '11:15')),
    ('CDP31-00196', '01a17da1-9bdc-71ec-aa4d-ca2707d69b54', 'Autorité Nationale de Budgétisation Climat :', '01a17da1-9bea-7939-b396-973fa50cb895', 'under_review',
        pg_temp.a(13, '13:00'), pg_temp.a(13, '14:00'), pg_temp.a(13, '13:00'), pg_temp.a(13, '14:00')),
    ('CDP31-00158', '01a17da1-9bdb-73e3-80c7-e8359e025237', 'Du récif à la résilience : financer et mettre', '01a17da1-9bea-7504-9fd5-d49a5cca34a3', 'under_review',
        pg_temp.a(13, '14:15'), pg_temp.a(13, '15:15'), pg_temp.a(13, '14:15'), pg_temp.a(13, '15:15')),
    ('CDP31-00244', '01a17da1-9bd9-778e-9fa6-47341a902347', '​Rencontre avec les experts : mobiliser les', '01a17da1-9bea-78c9-a645-bd452739a869', 'under_review',
        pg_temp.a(13, '15:30'), pg_temp.a(13, '16:30'), pg_temp.a(13, '15:30'), pg_temp.a(13, '16:30')),
    ('CDP31-00125', '01a17da1-9bd8-75cf-8c55-b8d95e3e8f59', 'Construire des villes africaines résilientes :', '01a17da1-9bea-70b7-baf0-8479c194a4e8', 'under_review',
        pg_temp.a(13, '16:45'), pg_temp.a(13, '17:45'), pg_temp.a(13, '16:45'), pg_temp.a(13, '17:45')),
    -- 14/11
    ('CDP31-00177', '01a17da1-9bd9-7d55-9a12-8ed2f4186aae', 'Présentation de la caravane citoyenne africaine', '01a17da1-9bea-7a44-9d47-61f097d20ff9', 'under_review',
        pg_temp.a(14, '09:20'), pg_temp.a(14, '09:30'), pg_temp.a(14, '09:20'), pg_temp.a(14, '09:30')),
    ('CDP31-00134', '01a17da1-9bd5-7dae-be8b-3d500f3d7017', 'Autonomiser les jeunes et les femmes grâce aux', '01a17da1-9bea-736e-8afe-514fd510dffb', 'under_review',
        pg_temp.a(14, '09:30'), pg_temp.a(14, '09:40'), pg_temp.a(14, '09:30'), pg_temp.a(14, '09:40')),
    ('CDP31-00216', '01a17da1-9bdb-758e-be4e-e3ef4ae4280f', 'LES JEUNES POUR UNE URGENCE CLIMATIQUE', '01a17da1-9bea-7108-b131-f8b5b05a8878', 'under_review',
        pg_temp.a(14, '09:40'), pg_temp.a(14, '09:50'), pg_temp.a(14, '09:40'), pg_temp.a(14, '09:50')),
    ('CDP31-00138', '01a17da1-9bdb-7c85-a9ff-0661d98455d7', 'Agriculture résiliente et énergie propre :', '01a17da1-9bea-7368-91c4-f7f5434ef814', 'under_review',
        pg_temp.a(14, '09:50'), pg_temp.a(14, '10:00'), pg_temp.a(14, '09:50'), pg_temp.a(14, '10:00')),
    ('CDP31-00148', '01a17da1-9bdc-7df0-920c-0ac1bdfaf960', 'Atelier sur l''intelligence artificielle (IA) et', '01a17da1-9bea-77b0-9e28-10961bd0e26f', 'under_review',
        pg_temp.a(14, '10:00'), pg_temp.a(14, '10:10'), pg_temp.a(14, '10:00'), pg_temp.a(14, '10:10')),
    ('CDP31-00188', '01a17da1-9bd8-736d-8352-839ecdf337a6', 'Genre et Climat: Des compétences vertes au', '01a17da1-9bea-7fec-84de-1627d22c8081', 'under_review',
        pg_temp.a(14, '13:00'), pg_temp.a(14, '14:00'), pg_temp.a(14, '13:00'), pg_temp.a(14, '14:00')),
    ('CDP31-00237', '01a17da1-9bdb-7205-87fd-e2e1d7d53f8d', 'Océan, climat et résilience : l’expérience', '01a17da1-9bea-7692-a582-7312d3ba2770', 'under_review',
        pg_temp.a(14, '14:15'), pg_temp.a(14, '15:15'), pg_temp.a(14, '14:15'), pg_temp.a(14, '15:15')),
    ('CDP31-00131', '01a17da1-9bdc-7e57-9540-391d75cadb29', 'Risques climatiques et transition : comment', '01a17da1-9bea-7e49-b0af-0a965e79b633', 'under_review',
        pg_temp.a(14, '15:30'), pg_temp.a(14, '16:30'), pg_temp.a(14, '15:30'), pg_temp.a(14, '16:30')),
    ('CDP31-00163', '01a17da1-9bd9-726f-957c-a8597680360b', 'Transformer les risques en résilience : le', '01a17da1-9bea-7734-a2a0-e06ff34af662', 'under_review',
        pg_temp.a(14, '16:45'), pg_temp.a(14, '17:45'), pg_temp.a(14, '16:45'), pg_temp.a(14, '17:45')),
    -- 16/11
    ('CDP31-00224', '01a17da1-9bdc-7e7d-a674-907a3c2711ac', 'Décloisonner les Conventions de Rio : des', '01a17da1-9bea-7f0e-b430-690bf3eb1022', 'under_review',
        pg_temp.a(16, '09:00'), pg_temp.a(16, '10:00'), pg_temp.a(16, '09:00'), pg_temp.a(16, '10:00')),
    ('CDP31-00255', '01a17da1-9bd9-73ae-81c0-33fab307a12f', 'Pérenniser les acquis : des réalisations de', '01a17da1-9bea-7831-aa9a-c7ed5222735f', 'under_review',
        pg_temp.a(16, '10:15'), pg_temp.a(16, '11:15'), pg_temp.a(16, '10:15'), pg_temp.a(16, '11:15')),
    ('CDP31-00241', '01a17da1-9bd8-7a18-b0f2-1b2d6f50e2f5', 'Vers la mise en place d''un Système National', '01a17da1-9bea-7590-ab33-804e307e089b', 'under_review',
        pg_temp.a(16, '11:30'), pg_temp.a(16, '12:30'), pg_temp.a(16, '11:30'), pg_temp.a(16, '12:30')),
    ('CDP31-00200', '01a17da1-9bd8-7470-a525-ab91eff4740e', 'CDN 3.0 et Vision 2060 : deux cadres', '01a17da1-9bea-7b53-9382-8076687934f1', 'under_review',
        pg_temp.a(16, '15:30'), pg_temp.a(16, '16:30'), pg_temp.a(16, '15:30'), pg_temp.a(16, '16:30')),
    ('CDP31-00173', '01a17da1-9bda-7248-8be3-7bb70cac7c34', 'Atelier sur Innovations et Normes pour des', '01a17da1-9bea-7ad8-a0ee-c04007b1223e', 'under_review',
        pg_temp.a(16, '16:45'), pg_temp.a(16, '17:45'), pg_temp.a(16, '16:45'), pg_temp.a(16, '17:45')),
    -- 17/11
    ('CDP31-00248', '01a17da1-9bd7-7bfe-bd30-0a1557940503', 'Les défis climatiques auxquels Maurice et', '01a17da1-9bea-7d4c-aa3e-a14062fe5ca3', 'under_review',
        pg_temp.a(17, '09:00'), pg_temp.a(17, '10:00'), pg_temp.a(17, '09:00'), pg_temp.a(17, '10:00')),
    ('CDP31-00159', '01a17da1-9bd8-72da-945b-ed83d43ca731', 'De l''engagement à l''action : accompagner', '01a17da1-9bea-7d21-85bb-95a742e78b97', 'under_review',
        pg_temp.a(17, '10:15'), pg_temp.a(17, '11:15'), pg_temp.a(17, '10:15'), pg_temp.a(17, '11:15')),
    ('CDP31-00179', '01a17da1-9bda-715e-9a29-ebe89c34801a', 'Atelier sur: Mettre en œuvre l''Accord de Paris', '01a17da1-9bea-7edc-b85d-7005719a944e', 'under_review',
        pg_temp.a(17, '11:30'), pg_temp.a(17, '12:30'), pg_temp.a(17, '11:30'), pg_temp.a(17, '12:30')),
    ('CDP31-00202', '01a17da1-9bdb-7a50-a781-d9a708811f94', 'Accélérer l''action climatique par le capital', '01a17da1-9bea-7eae-a833-bc3b74d15dca', 'under_review',
        pg_temp.a(17, '13:00'), pg_temp.a(17, '15:15'), NULL, NULL),
    ('CDP31-00257', '01a17da1-9bd8-737e-9a52-cb31f82bfa6a', 'De la restauration aux opportunités : les', NULL, 'submitted',
        NULL, NULL, pg_temp.a(17, '14:15'), pg_temp.a(17, '15:15')),
    ('CDP31-00130', '01a17da1-9bdb-7abb-8c2c-4f72222e54f2', 'Mobiliser les capitaux privés pour le climat :', '01a17da1-9bea-793f-ab0f-8f04a719f70d', 'under_review',
        pg_temp.a(17, '15:30'), pg_temp.a(17, '16:30'), pg_temp.a(17, '15:30'), pg_temp.a(17, '16:30')),
    ('CDP31-00229', '01a17da1-9bd6-7b29-94ed-5db166da2e44', 'Inclusion climatique : financer l’adaptation', '01a17da1-9bea-71f1-bbd0-b78243a28068', 'under_review',
        pg_temp.a(20, '09:00'), pg_temp.a(20, '10:00'), pg_temp.a(17, '16:45'), pg_temp.a(17, '17:45')),
    -- 18/11
    ('CDP31-00191', '01a17da1-9bda-7fc7-90d1-fdae0e9aea49', 'Éduquer pour s’adapter : faire de l’éducation', '01a17da1-9bea-770a-bac0-0ff40b670161', 'under_review',
        pg_temp.a(18, '09:00'), pg_temp.a(18, '10:00'), pg_temp.a(18, '09:00'), pg_temp.a(18, '10:00')),
    ('CDP31-00201', '01a17da1-9bda-799d-aa32-d44626a22928', 'De la vulnérabilité à la résilience: Comment', '01a17da1-9bea-7842-95da-156773495146', 'under_review',
        pg_temp.a(18, '10:15'), pg_temp.a(18, '11:15'), pg_temp.a(18, '10:15'), pg_temp.a(18, '11:15')),
    ('CDP31-00197', '01a17da1-9bda-7aa0-bd1c-ecf5a3029c07', 'Justice climatique en action : financer les', '01a17da1-9bea-7019-a947-3cdcc34205cf', 'under_review',
        pg_temp.a(18, '11:30'), pg_temp.a(18, '12:30'), pg_temp.a(18, '11:30'), pg_temp.a(18, '12:30')),
    ('CDP31-00228', '01a17da1-9bdb-7226-bd94-d27d17f7f4ae', 'Des connaissances à l’action climatique :', '01a17da1-9bea-7228-b9fa-910afb608e9a', 'under_review',
        pg_temp.a(18, '13:00'), pg_temp.a(18, '14:00'), pg_temp.a(18, '13:00'), pg_temp.a(18, '14:00')),
    ('CDP31-00231', '01a17da1-9bdb-780e-8071-5d745ee162f2', 'Comment financer le dernier kilomètre ?', '01a17da1-9bea-7df1-af02-39b42b431fed', 'under_review',
        pg_temp.a(18, '14:15'), pg_temp.a(18, '15:15'), pg_temp.a(18, '14:15'), pg_temp.a(18, '15:15')),
    ('CDP31-00146', '01a17da1-9bdb-73a5-aee9-b9d69e330cef', 'De la donnée climatique à la décision :', '01a17da1-9bea-7c57-8ed0-49d6dc5cf525', 'under_review',
        pg_temp.a(18, '15:30'), pg_temp.a(18, '16:30'), pg_temp.a(18, '15:30'), pg_temp.a(18, '16:30')),
    ('CDP31-00222', '01a17da1-9bd7-7ba5-addd-432fa85b7683', 'Avis Consultatif de la Cour Africaine sur le', '01a17da1-9bea-7d62-99fe-3a320831f237', 'under_review',
        pg_temp.a(18, '16:45'), pg_temp.a(18, '17:45'), pg_temp.a(18, '16:45'), pg_temp.a(18, '17:45')),
    -- 19/11
    ('CDP31-00233', '01a17da1-9bdb-777a-a47e-ee14be2be96b', 'Cuisson propre et résilience climatique dans', '01a17da1-9bea-7195-be88-d251436671c4', 'under_review',
        pg_temp.a(19, '09:00'), pg_temp.a(19, '10:00'), pg_temp.a(19, '09:00'), pg_temp.a(19, '10:00')),
    ('CDP31-00176', '01a17da1-9bda-76e3-85d8-23f1d9948826', 'Renforcer la résilience des territoires ruraux', '01a17da1-9bea-7a8c-b6ce-a4d1963feff8', 'under_review',
        pg_temp.a(19, '10:15'), pg_temp.a(19, '11:15'), pg_temp.a(19, '10:15'), pg_temp.a(19, '11:15')),
    ('CDP31-00250', '01a17da1-9bdb-709f-a144-69bf4202c466', 'Accélérer l’industrialisation verte à travers', '01a17da1-9bea-7f79-92f6-57a064c9d2ca', 'under_review',
        pg_temp.a(19, '11:30'), pg_temp.a(19, '12:30'), pg_temp.a(19, '11:30'), pg_temp.a(19, '12:30')),
    ('CDP31-00190', '01a17da1-9bd8-73dc-9edc-eb3e7e854196', 'Peuples autochtones, savoirs, arts et', '01a17da1-9bea-7cff-b93b-2d214a692c4d', 'under_review',
        pg_temp.a(19, '13:00'), pg_temp.a(19, '14:00'), pg_temp.a(19, '13:00'), pg_temp.a(19, '14:00')),
    ('CDP31-00209', '01a17da1-9bda-78e3-8a09-05152b2d230c', 'Santé globale : conférence-débat sur la santé,', '01a17da1-9bea-726a-8826-7f42de908735', 'under_review',
        pg_temp.a(19, '14:15'), pg_temp.a(19, '15:15'), pg_temp.a(19, '14:15'), pg_temp.a(19, '15:15')),
    ('CDP31-00239', '01a17da1-9bdb-7814-8b61-abfbbefbaab1', 'Mesurer l’effectivité de la politique', '01a17da1-9bea-7df0-b174-f82f5a6283fc', 'under_review',
        pg_temp.a(19, '15:30'), pg_temp.a(19, '16:30'), pg_temp.a(19, '15:30'), pg_temp.a(19, '16:30')),
    ('CDP31-00236', '01a17da1-9bdb-79c0-b1c8-7e1f8ce392df', 'Les énergies renouvelables décentralisées au', '01a17da1-9bea-7bc0-9442-e4ce01c51ee2', 'under_review',
        pg_temp.a(19, '16:45'), pg_temp.a(19, '17:45'), pg_temp.a(19, '16:45'), pg_temp.a(19, '17:45')),
    -- 20/11
    ('CDP31-00149', '01a17da1-9bdb-7c44-933a-d61d22496ceb', 'Irrigation goutte-à-goutte à 5000 DH :', '01a17da1-9bea-7c8d-a51a-64fdefec9794', 'under_review',
        pg_temp.a(20, '10:15'), pg_temp.a(20, '11:15'), pg_temp.a(20, '09:00'), pg_temp.a(20, '10:00')),
    ('CDP31-00192', '01a17da1-9bd9-7c76-bdd2-5ed9b44472f4', 'Approche de résilience des communautés et des', '01a17da1-9bea-79ab-80b8-d45e611dfd95', 'under_review',
        pg_temp.a(20, '14:15'), pg_temp.a(20, '15:15'), pg_temp.a(20, '10:15'), pg_temp.a(20, '11:15'));

DO $$
DECLARE
    r record;
    v_stand uuid := (SELECT stand_id FROM lc_edition);
BEGIN
    FOR r IN
        SELECT x.*, p.id IS NULL AS absent, p.event_id IS DISTINCT FROM ed.event_id AS ailleurs,
               p.reference_code, p.status::text AS etat, p.title ->> 'fr' AS titre_lu,
               (SELECT count(*) FROM programme.sessions s WHERE s.proposal_id = x.proposal_id) AS seances
        FROM lc_position x CROSS JOIN lc_edition ed
        LEFT JOIN programme.proposals p ON p.id = x.proposal_id AND p.deleted_at IS NULL
    LOOP
        IF r.absent OR r.ailleurs OR r.reference_code <> r.reference THEN
            RAISE EXCEPTION 'contenu : dossier % (%) absent de la CdP31', r.reference, r.proposal_id;
        END IF;
        IF platform.normalize_label(r.titre_lu) NOT LIKE platform.normalize_label(r.titre) || '%' THEN
            RAISE EXCEPTION 'contenu : dossier % retitré, « % » attendu, « % » lu', r.reference, r.titre, r.titre_lu;
        END IF;
        IF r.etat <> 'accepted' AND r.etat <> r.etat_releve
           AND NOT (r.etat_releve = 'submitted' AND r.etat = 'under_review') THEN
            RAISE EXCEPTION 'contenu : dossier % à l''état %, % attendu, à trancher', r.reference, r.etat, r.etat_releve;
        END IF;

        IF r.session_id IS NOT NULL THEN
            IF r.seances <> 1 OR NOT EXISTS (
                SELECT 1 FROM programme.sessions s
                WHERE s.id = r.session_id AND s.proposal_id = r.proposal_id
                  AND s.status = 'planned' AND s.published_at IS NULL
                  AND (s.room_id IS NULL OR s.room_id = v_stand)
                  AND ((s.starts_at, s.ends_at) = (r.avant_debut, r.avant_fin)
                       OR (s.starts_at, s.ends_at) IS NOT DISTINCT FROM (r.debut, r.fin))) THEN
                RAISE EXCEPTION 'contenu : séance du dossier % déplacée, publiée ou changée depuis le relevé, à trancher', r.reference;
            END IF;
        -- Séance à naître : aucune encore, ou celle de la première passe, à sa place.
        ELSIF r.seances > 1 OR (r.seances = 1 AND NOT EXISTS (
                SELECT 1 FROM programme.sessions s
                WHERE s.proposal_id = r.proposal_id AND s.sequence_number = 1
                  AND s.status = 'planned' AND s.published_at IS NULL AND s.room_id = v_stand
                  AND (s.starts_at, s.ends_at) = (r.debut, r.fin))) THEN
            RAISE EXCEPTION 'contenu : le dossier % a déjà une séance ailleurs, à trancher', r.reference;
        END IF;
    END LOOP;

    IF EXISTS (SELECT 1 FROM programme.registrations i
               JOIN programme.sessions s ON s.id = i.session_id
               JOIN lc_position x ON x.proposal_id = s.proposal_id) THEN
        RAISE EXCEPTION 'contenu : une séance du lot a des inscrits, qu''un déplacement préviendrait : à trancher';
    END IF;
    IF EXISTS (SELECT 1 FROM live.meetings m
               JOIN programme.sessions s ON s.id = m.session_id
               JOIN lc_position x ON x.proposal_id = s.proposal_id) THEN
        RAISE EXCEPTION 'contenu : une séance du lot a une réunion Zoom, qu''un déplacement recalerait : à trancher';
    END IF;
END $$;

-- Décision ------------------------------------------------------------------------

CREATE TEMP TABLE lc_transition (reference text, vers text) ON COMMIT DROP;

WITH t AS (
    UPDATE programme.proposals p SET status = 'under_review', decision_reason = NULL
    FROM lc_position x WHERE p.id = x.proposal_id AND p.status = 'submitted'
    RETURNING x.reference
)
INSERT INTO lc_transition SELECT reference, 'under_review' FROM t;

WITH t AS (
    UPDATE programme.proposals p SET status = 'accepted', decision_reason = NULL
    FROM lc_position x WHERE p.id = x.proposal_id AND p.status = 'under_review'
    RETURNING x.reference
)
INSERT INTO lc_transition SELECT reference, 'accepted' FROM t;

-- Naissance des séances (service/birth.rs) ------------------------------------------

CREATE TEMP TABLE lc_nee (reference text, session_id uuid) ON COMMIT DROP;

DO $$
DECLARE
    d record;
    v_base text;
    v_voulue text;
    v_adresse text;
    v_id uuid;
BEGIN
    FOR d IN
        SELECT x.reference, p.id, p.event_id, p.slug::text AS slug, p.requested_sessions,
               p.preferred_start_at, e.timezone, e.starts_at AS debut_edition,
               c.id AS call_id, c.daily_start_time, c.default_duration_minutes,
               CASE WHEN p.duration_minutes IS NOT NULL THEN p.duration_minutes
                    ELSE c.default_duration_minutes END AS duree
        FROM lc_position x
        JOIN programme.proposals p ON p.id = x.proposal_id AND p.status = 'accepted'
        JOIN event.events e ON e.id = p.event_id
        LEFT JOIN event.calls_for_proposals c ON c.id = p.call_id
    LOOP
        -- domain/slug.rs : 157 caractères au plus, coupés au dernier tiret.
        v_base := btrim(coalesce(d.slug, ''));
        IF v_base = '' THEN
            v_base := 'dossier';
        ELSIF length(v_base) > 157 THEN
            v_base := left(v_base, 157);
            IF strpos(v_base, '-') > 1 THEN
                v_base := regexp_replace(v_base, '-[^-]*$', '');
            ELSE
                v_base := rtrim(v_base, '-');
            END IF;
        END IF;

        FOR v_rang IN 1 .. greatest(d.requested_sessions, 1) LOOP
            CONTINUE WHEN EXISTS (SELECT 1 FROM programme.sessions s
                                  WHERE s.proposal_id = d.id AND s.sequence_number = v_rang);
            v_voulue := CASE WHEN d.requested_sessions <= 1 THEN v_base ELSE v_base || '-' || v_rang END;
            v_adresse := NULL;
            FOR n IN 0 .. 9 LOOP
                v_adresse := CASE WHEN n = 0 THEN v_voulue ELSE v_voulue || '-' || (n + 1) END;
                EXIT WHEN NOT EXISTS (SELECT 1 FROM programme.sessions s
                                      WHERE s.event_id = d.event_id AND s.slug = v_adresse::platform.slug);
                v_adresse := NULL;
            END LOOP;
            IF v_adresse IS NULL THEN
                RAISE EXCEPTION 'contenu : dix adresses d''URL déjà prises pour la séance du dossier %', d.reference;
            END IF;

            INSERT INTO programme.sessions
                   (event_id, proposal_id, organization_id, sequence_number,
                    title, description, slug, format, timezone, starts_at, ends_at)
            SELECT p.event_id, p.id, p.organization_id, v_rang,
                   p.title, p.detailed_presentation, v_adresse::platform.slug, p.format,
                   d.timezone, debut.instant,
                   debut.instant + make_interval(mins => CASE WHEN d.duree > 0 THEN d.duree ELSE 60 END)
            FROM programme.proposals p
            CROSS JOIN LATERAL (
                SELECT COALESCE(
                    d.preferred_start_at,
                    CASE WHEN d.call_id IS NOT NULL THEN
                        (((d.debut_edition AT TIME ZONE d.timezone)::date::text) || ' '
                         || d.daily_start_time::text)::timestamp AT TIME ZONE d.timezone
                    END,
                    d.debut_edition) AS instant
            ) AS debut
            WHERE p.id = d.id
            ON CONFLICT (proposal_id, sequence_number) DO NOTHING
            RETURNING id INTO v_id;

            CONTINUE WHEN v_id IS NULL;

            -- repo/session_parts.rs et repo/themes.rs ; la ligne du porteur est
            -- posée par le déclencheur.
            INSERT INTO programme.session_speakers
                   (session_id, person_id, role, job_title_snapshot, organization_snapshot, bio,
                    confirmed_at, sort_order, civility, first_name, last_name)
            SELECT v_id, s.person_id, s.role, s.job_title_snapshot, s.organization_snapshot, s.bio,
                   s.confirmed_at, s.sort_order, s.civility, s.first_name, s.last_name
            FROM programme.proposal_speakers s
            WHERE s.proposal_id = d.id
              AND (s.person_id IS NOT NULL OR NOT EXISTS (
                  SELECT 1 FROM programme.session_speakers x
                  WHERE x.session_id = v_id AND x.person_id IS NULL AND x.role = s.role
                    AND x.first_name = s.first_name AND x.last_name = s.last_name))
            ON CONFLICT (session_id, person_id, role) DO NOTHING;

            INSERT INTO programme.session_organizations (session_id, organization_id, role, sort_order)
            SELECT v_id, o.organization_id, o.role, o.sort_order
            FROM programme.proposal_organizations o
            WHERE o.proposal_id = d.id AND o.role <> 'lead'
            ON CONFLICT (session_id, organization_id) DO NOTHING;

            INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, sort_order)
            SELECT 'programme', 'sessions', v_id, src.term_id, src.sort_order
            FROM reference.entity_terms src
            JOIN reference.taxonomy_terms t ON t.id = src.term_id
            WHERE src.entity_schema = 'programme' AND src.entity_table = 'proposals'
              AND src.entity_id = d.id AND t.taxonomy_code = 'activity_theme'
            ON CONFLICT DO NOTHING;

            INSERT INTO lc_nee VALUES (d.reference, v_id);
        END LOOP;
    END LOOP;
END $$;

-- Placement au stand (service/planner.rs) -------------------------------------------

CREATE TEMP TABLE lc_placee (reference text, avant timestamptz, debut timestamptz, fin timestamptz, deplacee boolean)
    ON COMMIT DROP;

WITH maj AS (
    UPDATE programme.sessions s
    SET room_id = ed.stand_id, starts_at = x.debut, ends_at = x.fin, event_day_id = NULL
    FROM lc_position x, lc_edition ed, programme.sessions a
    WHERE x.debut IS NOT NULL
      AND a.id = s.id
      AND s.proposal_id = x.proposal_id AND s.sequence_number = 1
      AND (x.session_id IS NULL OR s.id = x.session_id)
      AND (s.room_id IS DISTINCT FROM ed.stand_id OR (s.starts_at, s.ends_at) <> (x.debut, x.fin))
    RETURNING x.reference, a.starts_at, x.debut, x.fin, (a.starts_at, a.ends_at) <> (x.debut, x.fin)
)
INSERT INTO lc_placee SELECT * FROM maj;

-- Compte rendu ------------------------------------------------------------------------

DO $$
DECLARE r record;
BEGIN
    IF EXISTS (SELECT 1 FROM platform.outbox_events
               WHERE correlation_id = current_setting('app.request_id') AND occurred_at = now()
                 AND event_type NOT IN ('programme.proposal.under_review', 'programme.proposal.accepted',
                                        'programme.session.created', 'programme.session.rescheduled')) THEN
        RAISE EXCEPTION 'contenu : événement d''outbox inattendu (%)',
            (SELECT string_agg(DISTINCT event_type, ', ') FROM platform.outbox_events
             WHERE correlation_id = current_setting('app.request_id') AND occurred_at = now());
    END IF;
    IF EXISTS (SELECT 1 FROM platform.jobs WHERE created_at = now()) THEN
        RAISE EXCEPTION 'contenu : un travail a été mis en file (courriel ?), à vérifier';
    END IF;
    IF EXISTS (SELECT 1 FROM programme.sessions s JOIN lc_position x ON x.proposal_id = s.proposal_id
               WHERE s.published_at IS NOT NULL OR s.status <> 'planned') THEN
        RAISE EXCEPTION 'contenu : une séance du lot est publiée ou programmée';
    END IF;

    FOR r IN SELECT p.reference, to_char(p.avant AT TIME ZONE 'Europe/Istanbul', 'DD/MM HH24:MI') AS avant,
                    to_char(p.debut AT TIME ZONE 'Europe/Istanbul', 'DD/MM HH24:MI') AS debut,
                    to_char(p.fin AT TIME ZONE 'Europe/Istanbul', 'HH24:MI') AS fin
             FROM lc_placee p WHERE p.deplacee ORDER BY p.debut LOOP
        RAISE NOTICE 'contenu : % de % à % - %', r.reference, r.avant, r.debut, r.fin;
    END LOOP;
    RAISE NOTICE 'contenu : % dossiers passés en évaluation, % retenus, % séances créées, % placées au stand (dont % déplacées)',
        (SELECT count(*) FROM lc_transition WHERE vers = 'under_review'),
        (SELECT count(*) FROM lc_transition WHERE vers = 'accepted'),
        (SELECT count(*) FROM lc_nee),
        (SELECT count(*) FROM lc_placee),
        (SELECT count(*) FROM lc_placee WHERE deplacee);
    FOR r IN SELECT event_type, count(*) AS n FROM platform.outbox_events
             WHERE correlation_id = current_setting('app.request_id') AND occurred_at = now()
             GROUP BY 1 ORDER BY 1 LOOP
        RAISE NOTICE 'contenu : outbox % : %', r.event_type, r.n;
    END LOOP;
    RAISE NOTICE 'contenu : au total, % dossiers du lot retenus, % séances au stand, programme non publié',
        (SELECT count(*) FROM programme.proposals p JOIN lc_position x ON x.proposal_id = p.id WHERE p.status = 'accepted'),
        (SELECT count(*) FROM programme.sessions s JOIN lc_position x ON x.proposal_id = s.proposal_id
                                                   JOIN lc_edition ed ON s.room_id = ed.stand_id);
END $$;

COMMIT;
