-- =============================================================================
-- ePavillon v2 — 115_content.sql
-- Module Contenu : les CONTENUS MIS EN AVANT de la vitrine publique — témoignages,
-- innovations, bonnes pratiques, annonces — leur ordre, leur fenêtre de diffusion
-- et leur emplacement à l'écran.
--
-- Dépend de : 000_bootstrap, 010_platform, 020_reference, 030_identity,
--             040_organizations, 050_media, 060_events, 075_programme_sessions
--
-- -----------------------------------------------------------------------------
-- POURQUOI UN MODULE, ET POURQUOI PAS AILLEURS
--
-- La page d'accueil fait défiler des contenus choisis par l'IFDD. En v1, ces
-- contenus étaient de trois natures et vivaient à trois endroits : deux tables
-- Supabase divergentes (`user_testimonials` et `video_testimonials`, qui ne
-- partageaient ni leur colonne de contexte, ni leur état de modération, ni leur
-- titre), une table `innovations_practices` pour les bonnes pratiques, et CINQ
-- COMPOSANTS VUE dans lesquels les annonces — journée jeunesse, séminaire de
-- Chypre, webinaire PACO — étaient écrites en dur avec leurs dates, leurs
-- images et leurs identifiants. Changer l'accueil demandait un redéploiement.
--
-- Le besoin est donc un seul : « mettre en avant quelque chose, à tel endroit,
-- dans cet ordre, pendant cette période ». Ce n'est ni de la publication
-- (`publication.articles` porte un cycle éditorial, un quota et des révisions,
-- et son module est fermé), ni de l'engagement (`engagement` porte la relation
-- aux personnes : notifications, courriels, commentaires, messagerie). C'est un
-- objet éditorial de la vitrine, et il a son schéma.
--
-- Le module est délibérément petit. Il ne dépend d'aucun autre module pour
-- FONCTIONNER : ses liens vers `event`, `programme`, `org` et `identity` sont
-- tous facultatifs et tous nommés `xmod_fk_`. Une diapositive qui n'attribue
-- rien et ne pointe nulle part reste une diapositive valide.
--
-- -----------------------------------------------------------------------------
-- DÉCISIONS DE CONCEPTION
--
-- C1. UNE SEULE TABLE, PAS UNE PAR NATURE DE CONTENU.
--     La v1 avait deux tables de témoignages pour un seul écran, et le frontend
--     les fusionnait à la main en leur inventant un champ `type` qui n'existait
--     dans aucune des deux. La nature du contenu — témoignage, innovation,
--     bonne pratique, annonce, chiffre clé — est un VOCABULAIRE OUVERT : elle
--     vit dans `reference.taxonomy_terms` (taxonomie `highlight_nature`), avec
--     son libellé traduit et sa couleur, modifiables au back-office. Ajouter
--     « Portrait de négociatrice » à la vitrine ne demande aucune migration.
--
--     C'est aussi ce qui corrige le défaut n° 1 de la v1 : la v1 affichait
--     « Innovation/Bonne pratique » et « Témoignage négociat(eur)rice
--     Francophone » en chaînes françaises figées dans le composant du hero.
--
-- C2. L'ORDRE EST UNE DONNÉE.
--     La v1 n'avait aucune colonne d'ordre : le carrousel suivait `created_at
--     DESC`, et l'IFDD ne pouvait pas décider ce qui passe en premier.
--     `sort_order` existe, il est administrable, et l'index public le porte.
--
-- C3. LA MISE EN AVANT EST UN ACTE DE BACK-OFFICE, PAS UNE CASE COCHÉE PAR
--     L'AUTEUR. En v1, `featured` était renseigné par la personne qui déposait
--     son propre témoignage, aucun écran d'administration ne permettait de
--     l'inverser, et les politiques RLS l'interdisaient même à un administrateur.
--     Ici, la table entière est un objet de back-office : elle porte sa
--     permission (`content.highlight.manage`) et son périmètre (`event_id`).
--
-- C4. LA FENÊTRE DE DIFFUSION EST DANS LA BASE.
--     Les cinq widgets d'annonce de la v1 apparaissaient et disparaissaient
--     selon des dates comparées en JavaScript, dans le composant. `starts_at` /
--     `ends_at` font ce travail en base : l'annonce d'une journée spéciale
--     s'éteint toute seule le lendemain, sans que personne y pense.
--
-- C5. AUCUNE URL DE MÉDIA EN BASE (ADR-08).
--     Le fond d'une diapositive — vidéo ou photographie — est un objet de
--     `media.assets` rattaché par `media.attachments`. Trois rôles sont déclarés
--     pour cette table : `banner` (fond image), `video` (fond vidéo) et `cover`
--     (vignette du rail, qui sert aussi d'affiche à la vidéo). La v1 stockait
--     `photo_url`, `video_url` et `thumbnail_url` en texte libre — et
--     `thumbnail_url` était presque toujours nul, les vignettes étant fabriquées
--     dans le navigateur du visiteur.
--
--     Le rôle `video` A ÉTÉ AJOUTÉ à `media.attachment_role` pour ce module
--     (voir `050_media.sql` § 1) : l'énumération n'avait que des rôles d'image
--     et de document, alors que la plateforme stocke déjà des enregistrements de
--     séance. Le détourner sur `attachment` aurait rendu le rôle illisible.
--
-- C6. LE LIEN VERS UNE ÉDITION OU UNE SÉANCE EST TYPÉ, PAS UNE URL.
--     `event_id` et `session_id` sont de vraies clés étrangères. Une séance
--     déplacée entraîne sa diapositive ; une édition supprimée emporte ses
--     annonces. `link_url` reste disponible pour ce qui sort de la plateforme.
--
-- C7. LE PÉRIMÈTRE D'ADMINISTRATION PASSE PAR `event_id` (ADR-14, règle n° 8).
--     Une diapositive rattachée à une édition n'est visible et modifiable que
--     par qui administre cette édition. Une diapositive sans édition parle de la
--     plateforme entière : elle demande la portée globale.
-- =============================================================================

CREATE SCHEMA IF NOT EXISTS content;

COMMENT ON SCHEMA content IS
    'Contenus éditoriaux de la vitrine publique : ce que l''IFDD choisit de mettre en avant, où, dans quel ordre et pendant combien de temps.';

INSERT INTO platform.modules (code, schema_name, display_name, depends_on) VALUES
    ('content', 'content', '{"fr":"Contenu éditorial","en":"Editorial content"}',
     '{identity,org,media,event,programme}')
ON CONFLICT (code) DO NOTHING;

-- -----------------------------------------------------------------------------
-- 1. Types du module
--
-- Rappel de la règle du projet : un ENUM est réservé aux MACHINES À ÉTATS et aux
-- vocabulaires fermés DONT DÉPEND DU CODE. Les deux qui suivent en relèvent —
-- l'emplacement décide quel composant affiche la diapositive, l'état décide si
-- elle sort. La NATURE du contenu, elle, est ouverte : elle est en taxonomie.
-- -----------------------------------------------------------------------------

-- Où la diapositive s'affiche. Deux emplacements aujourd'hui, tous deux servis
-- par la page d'accueil :
--   · `home_hero`  — le bandeau d'ouverture, plein cadre, qui défile ;
--   · `home_aside` — le panneau latéral, sous les prochaines échéances. C'est
--     lui qui remplace les cinq widgets d'annonce codés en dur de la v1.
-- Ajouter un emplacement (page d'une édition, pied de page) demande un ALTER
-- TYPE et un composant : c'est voulu, un emplacement sans rendu n'existe pas.
CREATE TYPE content.highlight_placement AS ENUM ('home_hero', 'home_aside');

-- Cycle de vie éditorial. `archived` n'est pas une suppression : une diapositive
-- retirée de la vitrine reste consultable au back-office et réutilisable
-- l'année suivante — un témoignage de COP30 se remet en avant à la COP31.
CREATE TYPE content.highlight_status AS ENUM ('draft', 'published', 'archived');

-- -----------------------------------------------------------------------------
-- 2. Les contenus mis en avant
-- -----------------------------------------------------------------------------
CREATE TABLE content.highlights (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    placement          content.highlight_placement NOT NULL DEFAULT 'home_hero',
    status             content.highlight_status    NOT NULL DEFAULT 'draft',

    -- Nature éditoriale : code d'un terme de la taxonomie `highlight_nature`.
    -- Code sans clé étrangère, comme `org.organizations.organization_type_code`
    -- et `event.event_series.track_code` : le module reste extractible, et le
    -- libellé comme la couleur se résolvent par jointure à l'affichage.
    nature_code        text        NOT NULL CHECK (nature_code ~ '^[a-z][a-z0-9_]*$'),

    -- L'ordre CHOISI par l'IFDD. Croissant : le plus petit passe en premier.
    sort_order         smallint    NOT NULL DEFAULT 0,

    -- Ce qui s'affiche.
    --   `title` — toujours présent : c'est le repère du back-office autant que
    --             le titre de la diapositive (« Le pavillon, notre bouée de
    --             sauvetage »).
    --   `quote` — le texte porté en grand sur le fond. Pour un témoignage,
    --             l'EXTRAIT choisi : une citation de quatre cents mots ne tient
    --             pas à l'écran, et c'est à l'éditeur de couper, pas au CSS.
    --   `body`  — le texte long, pour la page de détail. Facultatif.
    title              platform.i18n_text NOT NULL,
    quote              platform.i18n_text,
    body               platform.i18n_text,

    -- Attribution. Facultative : une annonce n'a pas d'auteur.
    --
    -- `author_name` est un texte et non une jointure obligatoire : beaucoup de
    -- témoignages viennent de personnes qui n'ont pas de compte sur la
    -- plateforme (une négociatrice croisée au pavillon, un ministre). Quand la
    -- personne EST inscrite, `person_id` la désigne et la vue préfère son nom
    -- d'affichage : une correction de patronyme dans le profil se répercute.
    person_id          uuid        CONSTRAINT xmod_fk_highlights_person
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    author_name        text        CHECK (author_name IS NULL OR length(btrim(author_name)) > 0),
    -- « Négociatrice, délégation du Bénin » — la fonction telle qu'elle doit
    -- s'afficher sous la citation, traduisible.
    author_title       platform.i18n_text,

    -- Organisation citée. RÈGLE MÉTIER N° 1 : on référence le répertoire plutôt
    -- que de retaper un nom, sinon « IFDD » et « Institut de la Francophonie
    -- pour le développement durable » redeviennent deux organisations.
    -- `organization_label` n'existe que pour le cas où l'organisation n'est PAS
    -- au répertoire, et le CHECK interdit de renseigner les deux.
    organization_id    uuid        CONSTRAINT xmod_fk_highlights_organization
                                   REFERENCES org.organizations(id) ON DELETE SET NULL,
    organization_label text        CHECK (organization_label IS NULL OR length(btrim(organization_label)) > 0),

    country_id         uuid        REFERENCES reference.countries(id) ON DELETE SET NULL,

    -- Rattachement et périmètre d'administration.
    --
    -- `event_id` a deux rôles indissociables : il situe éditorialement la
    -- diapositive (« ce témoignage est de la COP30 ») ET il porte le filtre du
    -- back-office (ADR-14). NULL = contenu de plateforme, portée globale.
    event_id           uuid        CONSTRAINT xmod_fk_highlights_event
                                   REFERENCES event.events(id) ON DELETE CASCADE,
    -- La séance mise en avant. Le trigger en dérive `event_id` s'il manque, et
    -- refuse l'incohérence s'il est renseigné autrement.
    session_id         uuid        CONSTRAINT xmod_fk_highlights_session
                                   REFERENCES programme.sessions(id) ON DELETE SET NULL,

    -- Sortie de plateforme (page de l'IFDD, vidéo hébergée ailleurs). Le libellé
    -- du lien est traduisible : « Voir le témoignage complet » n'est pas une
    -- chaîne d'interface, il change d'une diapositive à l'autre.
    link_url           platform.url,
    link_label         platform.i18n_text,

    -- Repli de fond quand aucun média n'est rattaché — le hero doit rester
    -- lisible pendant que le worker fabrique les variantes. Couleur DE DONNÉE,
    -- au même titre que celle d'une thématique : elle n'a rien à faire dans la
    -- feuille de style.
    background_color_hex text      CHECK (background_color_hex IS NULL
                                          OR background_color_hex ~ '^#[0-9a-fA-F]{6}$'),

    -- Fenêtre de diffusion. NULL des deux côtés = sans limite de temps.
    starts_at          timestamptz,
    ends_at            timestamptz,

    -- Posé par le trigger au passage en `published`, conservé ensuite : il dit
    -- depuis quand ce contenu est public, ce que `updated_at` ne dit pas.
    published_at       timestamptz,

    created_by         uuid        CONSTRAINT xmod_fk_highlights_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    updated_at         timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_highlights_window
        CHECK (starts_at IS NULL OR ends_at IS NULL OR ends_at > starts_at),
    -- Une organisation se désigne, ou se nomme, jamais les deux.
    CONSTRAINT ck_highlights_organization_shape
        CHECK (organization_id IS NULL OR organization_label IS NULL),
    -- Publié sans date de publication serait un état que rien ne date.
    CONSTRAINT ck_highlights_published_dated
        CHECK (status <> 'published' OR published_at IS NOT NULL),
    -- Un libellé de lien sans lien ne mène nulle part.
    CONSTRAINT ck_highlights_link_shape
        CHECK (link_label IS NULL OR link_url IS NOT NULL)
);

COMMENT ON TABLE content.highlights IS
    'Contenus mis en avant sur la vitrine publique : témoignages, innovations, bonnes pratiques, annonces. Un objet, une nature en taxonomie, un emplacement, un ordre et une fenêtre de diffusion.';
COMMENT ON COLUMN content.highlights.nature_code IS
    'Code d''un terme de reference.taxonomy_terms, taxonomie highlight_nature. Le libellé et la couleur viennent de la base — jamais d''un fichier i18n.';
COMMENT ON COLUMN content.highlights.quote IS
    'Texte porté en grand sur le fond. Pour un témoignage, l''EXTRAIT choisi par l''éditeur : couper appartient à la rédaction, pas à la mise en page.';
COMMENT ON COLUMN content.highlights.sort_order IS
    'Ordre de défilement, croissant. La v1 n''en avait aucun : le carrousel suivait la date de création, et l''IFDD ne décidait pas ce qui passe en premier.';
COMMENT ON COLUMN content.highlights.event_id IS
    'Édition de rattachement. Porte AUSSI le périmètre d''administration (ADR-14) : NULL = contenu de plateforme, réservé à la portée globale.';
COMMENT ON COLUMN content.highlights.starts_at IS
    'Début de diffusion. NULL = immédiate. Remplace les comparaisons de dates faites en JavaScript dans les widgets d''annonce de la v1.';
COMMENT ON COLUMN content.highlights.background_color_hex IS
    'Fond de repli quand aucun média n''est rattaché ou que les variantes ne sont pas prêtes. Couleur de donnée, saisie au back-office.';

-- Index de la lecture publique : un seul emplacement, déjà trié. La clause
-- partielle reprend EXACTEMENT le filtre de `v_showcase` — sans quoi elle ne
-- servirait pas.
CREATE INDEX ix_highlights_public
    ON content.highlights (placement, sort_order, id)
    WHERE status = 'published';

-- Périmètre d'administration et listes du back-office.
CREATE INDEX ix_highlights_event   ON content.highlights (event_id, placement, sort_order);
CREATE INDEX ix_highlights_session ON content.highlights (session_id) WHERE session_id IS NOT NULL;
CREATE INDEX ix_highlights_nature  ON content.highlights (nature_code, sort_order);

CREATE TRIGGER tg_highlights_updated_at
    BEFORE UPDATE ON content.highlights
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_highlights_audit
    AFTER INSERT OR UPDATE OR DELETE ON content.highlights
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

-- -----------------------------------------------------------------------------
-- 3. Cohérence : l'édition se déduit de la séance, la publication se date
--
-- Deux règles qu'aucun chemin d'écriture ne doit pouvoir contourner. Les poser
-- dans l'API laisserait passer l'import en masse et le script d'administration.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION content.tg_highlight_normalize()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_session_event uuid;
BEGIN
    -- Une diapositive qui met en avant une séance appartient à l'édition de
    -- cette séance. On la dérive quand elle manque ; on refuse la contradiction
    -- quand elle est renseignée autrement — une annonce classée sous la COP30
    -- alors qu'elle pointe une séance de la COP31 échapperait au périmètre
    -- d'administration des deux.
    IF NEW.session_id IS NOT NULL THEN
        SELECT s.event_id INTO v_session_event
        FROM programme.sessions s WHERE s.id = NEW.session_id;

        IF NEW.event_id IS NULL THEN
            NEW.event_id := v_session_event;
        ELSIF NEW.event_id <> v_session_event THEN
            RAISE EXCEPTION
                'Contenu mis en avant incohérent : la séance % appartient à l''édition %, pas à %',
                NEW.session_id, v_session_event, NEW.event_id
                USING ERRCODE = 'integrity_constraint_violation';
        END IF;
    END IF;

    -- La date de publication se pose au premier passage en `published` et ne se
    -- rejoue pas : republier après archivage ne réécrit pas l'histoire.
    IF NEW.status = 'published' AND NEW.published_at IS NULL THEN
        NEW.published_at := now();
    END IF;

    RETURN NEW;
END;
$$;

COMMENT ON FUNCTION content.tg_highlight_normalize() IS
    'Dérive l''édition depuis la séance mise en avant, refuse la contradiction, et date la première publication.';

CREATE TRIGGER tg_highlights_normalize
    BEFORE INSERT OR UPDATE ON content.highlights
    FOR EACH ROW EXECUTE FUNCTION content.tg_highlight_normalize();

-- Publication annoncée au reste de la plateforme : la vitrine publique est
-- mise en cache, et c'est cet événement qui l'invalide. Format imposé par
-- `ck_outbox_event_type_format` : trois segments en minuscules.
CREATE OR REPLACE FUNCTION content.tg_highlight_emit_published()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.status = 'published' AND (TG_OP = 'INSERT' OR OLD.status <> 'published') THEN
        PERFORM platform.emit_event(
            'content', 'highlight', NEW.id, 'content.highlight.published',
            jsonb_build_object('placement', NEW.placement, 'event_id', NEW.event_id)
        );
    ELSIF TG_OP = 'UPDATE' AND OLD.status = 'published' AND NEW.status <> 'published' THEN
        PERFORM platform.emit_event(
            'content', 'highlight', NEW.id, 'content.highlight.withdrawn',
            jsonb_build_object('placement', NEW.placement, 'event_id', NEW.event_id)
        );
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_highlights_emit_published
    AFTER INSERT OR UPDATE ON content.highlights
    FOR EACH ROW EXECUTE FUNCTION content.tg_highlight_emit_published();

-- -----------------------------------------------------------------------------
-- 4. La vitrine, prête à afficher
--
-- Une requête pour tout le bandeau d'accueil. Le principe est celui de
-- `programme.v_public_schedule` : ce que l'écran doit montrer est résolu EN BASE
-- — libellé et couleur de la nature, nom de l'auteur, nom et sigle de
-- l'organisation, pays, fond, vignette, vidéo. Une colonne qui manque ici coûte
-- une requête par diapositive, ou un renoncement d'affichage.
--
-- LE FILTRE TEMPOREL EST DANS LA VUE. Le laisser à l'appelant, c'était la v1 :
-- chaque composant comparait les dates à sa façon, et une annonce périmée
-- survivait à l'endroit où quelqu'un avait oublié la comparaison.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW content.v_showcase AS
SELECT
    h.id,
    h.placement,
    h.sort_order,

    h.nature_code,
    n.label      AS nature_label,
    n.color_hex  AS nature_color,
    n.icon       AS nature_icon,

    h.title,
    h.quote,
    h.body,

    -- Le nom d'affichage du profil prime : une correction de patronyme faite
    -- dans le profil se répercute sur la vitrine sans réédition.
    COALESCE(p.display_name, h.author_name) AS author_name,
    h.author_title,
    h.person_id,

    h.organization_id,
    COALESCE(o.legal_name, h.organization_label) AS organization_name,
    o.acronym    AS organization_acronym,

    -- Deux colonnes, deux usages, comme partout dans le modèle : le code ISO
    -- filtre et porte le drapeau, le nom est multilingue et s'affiche.
    c.iso2       AS country_code,
    c.name       AS country_name,

    h.event_id,
    e.slug       AS event_slug,
    e.title      AS event_title,

    h.session_id,
    s.slug       AS session_slug,
    s.title      AS session_title,
    s.starts_at  AS session_starts_at,
    s.ends_at    AS session_ends_at,
    s.timezone   AS session_timezone,

    h.link_url,
    h.link_label,

    -- Les trois médias possibles d'une diapositive. Rendus par
    -- `media.attached_image()` plutôt que par une jointure recopiée : le
    -- rattachement est polymorphe, et deux vues qui le déroulent à la main sont
    -- deux occasions de diverger.
    --
    -- `background_video` sort du même mécanisme bien qu'il ne s'agisse pas d'une
    -- image : la fonction ne fait aucune hypothèse sur le type MIME, et le
    -- frontend décide de rendre une balise `video` ou `img` d'après ce qu'il
    -- reçoit — jamais d'après une colonne qui prétendrait le savoir.
    media.attached_image('content', 'highlights', h.id, 'banner') AS background_image,
    media.attached_image('content', 'highlights', h.id, 'video')  AS background_video,
    media.attached_image('content', 'highlights', h.id, 'cover')  AS thumbnail,
    h.background_color_hex,

    -- Thématiques, deux fois et pour deux usages : filtrer, puis afficher.
    reference.terms_of('content', 'highlights', h.id, 'activity_theme')     AS theme_codes,
    reference.term_badges('content', 'highlights', h.id, 'activity_theme')  AS themes,

    h.starts_at,
    h.ends_at,
    h.published_at
FROM content.highlights h
LEFT JOIN reference.taxonomy_terms n
       ON n.taxonomy_code = 'highlight_nature' AND n.code = h.nature_code AND n.is_active
LEFT JOIN identity.people p     ON p.id = h.person_id
LEFT JOIN org.organizations o   ON o.id = h.organization_id
LEFT JOIN reference.countries c ON c.id = COALESCE(h.country_id, o.country_id)
LEFT JOIN event.events e        ON e.id = h.event_id
LEFT JOIN programme.sessions s  ON s.id = h.session_id
WHERE h.status = 'published'
  AND (h.starts_at IS NULL OR h.starts_at <= now())
  AND (h.ends_at   IS NULL OR h.ends_at   >  now());

COMMENT ON VIEW content.v_showcase IS
    'La vitrine publique prête à l''affichage : nature, attribution, médias et rattachements résolus en base, fenêtre de diffusion déjà appliquée. Trier par placement puis sort_order.';

-- -----------------------------------------------------------------------------
-- 5. Rôles de média
--
-- Sans ces trois lignes, tout `INSERT INTO media.attachments` visant cette table
-- lève « Rattachement refusé : le rôle X n'est pas déclaré » : la table blanche
-- de `050_media.sql` § 4 rend le polymorphisme déclaratif.
--
-- Les plafonds : 15 Mio pour un fond photographique (le même que la bannière
-- d'une édition), 5 Mio pour une vignette de rail, 200 Mio pour une vidéo de
-- fond. Ce dernier chiffre est haut pour la plateforme et bas pour de la vidéo :
-- une boucle de bandeau se monte à quinze ou vingt secondes, muette, et n'a
-- aucune raison de peser davantage. La v1 imposait 60 secondes par un CHECK en
-- base ; la limite est ici une limite de POIDS, qui est le vrai coût.
-- -----------------------------------------------------------------------------
INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes, max_byte_size) VALUES
    ('content', 'highlights', 'banner', '{"fr":"Fond photographique","en":"Background image"}',
     false, '{image/*}',  15728640),
    ('content', 'highlights', 'video',  '{"fr":"Fond vidéo","en":"Background video"}',
     false, '{video/*}', 209715200),
    ('content', 'highlights', 'cover',  '{"fr":"Vignette","en":"Thumbnail"}',
     false, '{image/*}',   5242880)
ON CONFLICT DO NOTHING;

-- -----------------------------------------------------------------------------
-- 6. Permission et vocabulaire
-- -----------------------------------------------------------------------------

-- Une permission, testée par `identity.has_permission` avec sa portée — jamais
-- un nom de rôle. Accordée à `admin` : sa portée suit celle de l'administrateur,
-- et un compte détaché sur la COP31 ne pourra toucher qu'aux contenus de la
-- COP31. Les contenus de plateforme (`event_id IS NULL`) restent au global.
INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('content.highlight.manage',
     '{"fr":"Gérer la vitrine et les contenus mis en avant","en":"Manage the showcase and featured content"}',
     'content')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin',      'content.highlight.manage'),
    ('super_admin','content.highlight.manage')
ON CONFLICT DO NOTHING;

-- La NATURE d'un contenu mis en avant : vocabulaire ouvert, donc taxonomie.
-- La v1 figeait ces libellés dans le composant du hero, en français seulement.
-- Les couleurs reprennent la sémantique de la charte : cyan pour l'information,
-- vert pour ce qui est acquis, violet pour ce qui relève du récit personnel.
INSERT INTO reference.taxonomies (code, label, description, is_multi_select, is_hierarchical, is_system) VALUES
    ('highlight_nature', '{"fr":"Natures de contenu mis en avant","en":"Featured content kinds"}',
     '{"fr":"Ce qu''est la diapositive : témoignage, innovation, annonce…","en":"What the slide is: testimonial, innovation, announcement…"}',
     false, false, false)
ON CONFLICT (code) DO NOTHING;

INSERT INTO reference.taxonomy_terms (taxonomy_code, code, label, color_hex, sort_order) VALUES
    ('highlight_nature', 'testimonial',
     '{"fr":"Témoignage","en":"Testimonial"}', '#732f85', 10),
    ('highlight_nature', 'negotiator_voice',
     '{"fr":"Parole de négociateur","en":"Negotiator''s voice"}', '#732f85', 20),
    ('highlight_nature', 'innovation',
     '{"fr":"Innovation","en":"Innovation"}', '#00a1e4', 30),
    ('highlight_nature', 'best_practice',
     '{"fr":"Bonne pratique","en":"Best practice"}', '#8fbf2f', 40),
    ('highlight_nature', 'announcement',
     '{"fr":"Annonce","en":"Announcement"}', '#00a1e4', 50),
    ('highlight_nature', 'key_figure',
     '{"fr":"Chiffre clé","en":"Key figure"}', '#1d1a5b', 60)
ON CONFLICT (taxonomy_code, code) DO NOTHING;

-- Les contenus mis en avant peuvent porter des thématiques, comme une activité
-- ou un article : `reference.entity_terms('content', 'highlights', <id>)`.
-- Aucune déclaration n'est nécessaire — le rattachement générique est ouvert.
