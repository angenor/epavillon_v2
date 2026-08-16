-- =============================================================================
-- ePavillon v2 — 090_publications.sql
-- Module Publications : articles rédigés par les organisations, cycle éditorial,
-- historique des versions, pièces jointes et QUOTA DE PUBLICATION.
--
-- Dépend de : 000_bootstrap, 010_platform, 020_reference, 030_identity,
--             040_organizations, 050_media, 060_events,
--             070_programme_proposals, 075_programme_sessions
--
-- DÉCISION STRUCTURANTE — LE QUOTA EST UNE RÈGLE DE BASE, PAS UNE RÈGLE D'ÉCRAN
-- Citation du cadrage : « Les organisations peuvent publier des articles. Vu que
-- nous avons un espace de stockage assez limité, certaines organisations ne
-- seront autorisées à publier qu'une fois par semaine, d'autres plusieurs fois
-- par semaine, par mois, par jour. Pour chaque organisation, il faudra pouvoir
-- régler le nombre/fréquence de publication dans le back-office. »
--
-- Trois conséquences de conception :
--
--   Q1. LA FRÉQUENCE EST UNE DONNÉE, PAS DU CODE.
--       `publishing_policies` porte le couple (quota, période) par organisation,
--       administrable depuis le back-office, historisé et daté. Aucun
--       redéploiement pour accorder une dérogation à une organisation.
--
--   Q2. LA RÈGLE EST APPLIQUÉE PAR LA BASE.
--       Un contrôle posé dans l'API laisse passer l'import en masse, le script
--       d'administration et la future file de publication programmée. Le trigger
--       `tg_enforce_publishing_quota` s'interpose sur TOUT chemin d'écriture :
--       il n'existe aucune porte dérobée. Un verrou consultatif par organisation
--       neutralise la course entre deux publications simultanées.
--
--   Q3. LE QUOTA A DEUX DIMENSIONS.
--       Le nombre d'articles borne le flux éditorial ; `max_storage_bytes` borne
--       l'empreinte disque, qui est la contrainte réellement citée (Garage/S3
--       auto-hébergé). Les deux sont vérifiées au même point de passage : le
--       franchissement vers un état publiquement visible.
--
-- Autre choix assumé : les COMMENTAIRES publics sont délégués au module
-- `engagement` (une seule modération, un seul fil transverse aux activités et
-- aux articles) ; seules les RÉACTIONS légères restent ici, car elles portent un
-- compteur dénormalisé lu à chaque affichage de liste et ne justifient pas un
-- aller-retour inter-module.
-- =============================================================================

-- -----------------------------------------------------------------------------
-- 1. Politiques de publication
--
-- Une politique = « cette organisation peut publier N fois par <période>, dans
-- la limite de M octets ». La ligne dont `organization_id IS NULL` est la
-- politique PAR DÉFAUT : elle s'applique à toute organisation sans règle propre,
-- ce qui évite d'avoir à créer une ligne pour chacune des centaines de
-- structures inscrites.
-- -----------------------------------------------------------------------------
CREATE TYPE publication.quota_period AS ENUM ('day', 'week', 'month', 'year');

CREATE OR REPLACE FUNCTION publication.period_interval(p_period publication.quota_period)
RETURNS interval
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT CASE p_period
               WHEN 'day'   THEN interval '1 day'
               WHEN 'week'  THEN interval '7 days'
               WHEN 'month' THEN interval '1 month'
               WHEN 'year'  THEN interval '1 year'
           END;
$$;

-- Libellé français de la période : les messages d'erreur remontent tels quels
-- dans le back-office et l'espace organisation, ils ne peuvent pas afficher la
-- valeur brute de l'ENUM.
CREATE OR REPLACE FUNCTION publication.period_label(p_period publication.quota_period)
RETURNS text
LANGUAGE sql
IMMUTABLE
PARALLEL SAFE
AS $$
    SELECT CASE p_period
               WHEN 'day'   THEN 'jour'
               WHEN 'week'  THEN 'semaine'
               WHEN 'month' THEN 'mois'
               WHEN 'year'  THEN 'an'
           END;
$$;

CREATE TABLE publication.publishing_policies (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    -- NULL = politique par défaut de la plateforme.
    organization_id   uuid        CONSTRAINT xmod_fk_publishing_policies_organization
                                  REFERENCES org.organizations(id) ON DELETE CASCADE,

    -- NULL = illimité (réservé à l'IFDD et aux partenaires institutionnels).
    quota             integer     CHECK (quota IS NULL OR quota >= 0),
    period            publication.quota_period NOT NULL DEFAULT 'week',

    -- Empreinte disque maximale, toutes pièces confondues. NULL = illimité.
    max_storage_bytes bigint      CHECK (max_storage_bytes IS NULL OR max_storage_bytes > 0),

    -- Suspension : coupe la publication sans détruire la politique ni son
    -- historique (abus, litige, campagne de modération).
    is_suspended      boolean     NOT NULL DEFAULT false,
    suspension_reason text,

    -- Fenêtre de validité : une dérogation ponctuelle (« 5 par jour pendant la
    -- COP30 ») s'exprime en posant une politique bornée dans le temps.
    valid_from        timestamptz NOT NULL DEFAULT now(),
    valid_until       timestamptz,

    granted_by        uuid        CONSTRAINT xmod_fk_publishing_policies_grantor
                                  REFERENCES identity.people(id) ON DELETE SET NULL,
    note              text,
    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_publishing_policies_window
        CHECK (valid_until IS NULL OR valid_until > valid_from),
    CONSTRAINT ck_publishing_policies_suspension_reason
        CHECK (NOT is_suspended OR suspension_reason IS NOT NULL),

    -- Deux politiques ne peuvent se chevaucher pour une même organisation :
    -- la résolution d'une politique effective est donc toujours déterministe.
    CONSTRAINT ex_publishing_policies_org_overlap
        EXCLUDE USING gist (
            organization_id WITH =,
            tstzrange(valid_from, valid_until, '[)') WITH &&
        ) WHERE (organization_id IS NOT NULL),

    -- Même garantie pour la politique par défaut : une seule à un instant donné.
    CONSTRAINT ex_publishing_policies_default_overlap
        EXCLUDE USING gist (
            tstzrange(valid_from, valid_until, '[)') WITH &&
        ) WHERE (organization_id IS NULL)
);

CREATE INDEX ix_publishing_policies_org
    ON publication.publishing_policies (organization_id, valid_from DESC)
    WHERE organization_id IS NOT NULL;

CREATE TRIGGER tg_publishing_policies_updated_at
    BEFORE UPDATE ON publication.publishing_policies
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

-- Toute modification de quota est une décision administrative : elle se trace.
CREATE TRIGGER tg_publishing_policies_audit
    AFTER INSERT OR UPDATE OR DELETE ON publication.publishing_policies
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE publication.publishing_policies IS
    'Quota éditorial par organisation (nombre d''articles par période et empreinte disque). La ligne organization_id IS NULL est la politique par défaut.';
COMMENT ON COLUMN publication.publishing_policies.quota IS
    'Nombre d''articles publiables sur la période glissante. NULL = illimité.';
COMMENT ON COLUMN publication.publishing_policies.max_storage_bytes IS
    'Empreinte disque maximale des médias de l''organisation (couvertures + pièces jointes). NULL = illimité.';

-- Politique applicable à une organisation à l'instant présent : règle propre si
-- elle existe, politique par défaut sinon.
CREATE OR REPLACE FUNCTION publication.effective_policy(p_organization_id uuid)
RETURNS publication.publishing_policies
LANGUAGE sql
STABLE
AS $$
    SELECT pp.*
    FROM publication.publishing_policies pp
    WHERE (pp.organization_id = p_organization_id OR pp.organization_id IS NULL)
      AND pp.valid_from <= now()
      AND (pp.valid_until IS NULL OR pp.valid_until > now())
    -- Une règle propre l'emporte toujours sur la politique par défaut.
    ORDER BY (pp.organization_id IS NOT NULL) DESC, pp.valid_from DESC
    LIMIT 1;
$$;

COMMENT ON FUNCTION publication.effective_policy(uuid) IS
    'Politique de publication applicable à une organisation maintenant (règle propre, à défaut politique par défaut).';

-- -----------------------------------------------------------------------------
-- 2. Articles
--
-- Le contenu est multilingue par nature (plateforme francophone ouverte sur
-- l'international) : titre, chapeau et corps sont des `platform.i18n_text`,
-- pas des colonnes dupliquées `_fr`/`_en` comme en v1.
-- -----------------------------------------------------------------------------
CREATE TYPE publication.article_status AS ENUM (
    'draft',       -- rédaction en cours, visible de la seule organisation
    'submitted',   -- soumis à l'IFDD pour revue
    'in_review',   -- pris en charge par un éditeur
    'scheduled',   -- accepté, publication différée (consomme déjà un quota)
    'published',   -- visible publiquement
    'rejected',    -- refusé, motif dans editorial_reviews
    'archived'     -- retiré de la vue publique, conservé
);

CREATE TABLE publication.articles (
    id                uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),

    organization_id   uuid        NOT NULL CONSTRAINT xmod_fk_articles_organization
                                  REFERENCES org.organizations(id) ON DELETE RESTRICT,
    author_id         uuid        CONSTRAINT xmod_fk_articles_author
                                  REFERENCES identity.people(id) ON DELETE SET NULL,

    title             platform.i18n_text NOT NULL,
    summary           platform.i18n_text,          -- chapeau : listes, cartes, meta description
    body              platform.i18n_text NOT NULL, -- corps rédactionnel (HTML assaini côté API)
    slug              platform.slug NOT NULL,      -- segment d'URL stable, socle du SEO Nuxt

    status            publication.article_status NOT NULL DEFAULT 'draft',
    published_at      timestamptz,
    scheduled_for     timestamptz,

    cover_asset_id    uuid        CONSTRAINT xmod_fk_articles_cover
                                  REFERENCES media.assets(id) ON DELETE SET NULL,

    -- Rattachements facultatifs : compte rendu d'une session, article lié à une
    -- édition d'événement. Nullables : un article peut être totalement autonome.
    related_event_id  uuid        CONSTRAINT xmod_fk_articles_event
                                  REFERENCES event.events(id) ON DELETE SET NULL,
    related_session_id uuid       CONSTRAINT xmod_fk_articles_session
                                  REFERENCES programme.sessions(id) ON DELETE SET NULL,

    -- Comptabilité de stockage, dénormalisée volontairement : le module média
    -- publie le poids au rattachement, le quota se calcule sans lecture
    -- inter-schéma (donc sans appel réseau le jour où `media` est extrait).
    cover_bytes       bigint      NOT NULL DEFAULT 0 CHECK (cover_bytes >= 0),
    attachments_bytes bigint      NOT NULL DEFAULT 0 CHECK (attachments_bytes >= 0),
    storage_bytes     bigint      GENERATED ALWAYS AS (cover_bytes + attachments_bytes) STORED,

    view_count        integer     NOT NULL DEFAULT 0 CHECK (view_count >= 0),
    reaction_count    integer     NOT NULL DEFAULT 0 CHECK (reaction_count >= 0),
    -- Projection alimentée par le module engagement (handler d'événement
    -- `engagement.comment.published`) : la lecture d'une liste d'articles ne
    -- déclenche aucun appel au module de commentaires.
    comment_count     integer     NOT NULL DEFAULT 0 CHECK (comment_count >= 0),

    -- Recherche plein texte bilingue. Titre et chapeau seulement : le corps
    -- ferait dépasser la limite de 1 Mo d'un tsvector sur les articles longs, et
    -- n'apporte rien au filtrage de liste.
    search_vector     tsvector    GENERATED ALWAYS AS (
        to_tsvector('french',
            coalesce(title ->> 'fr', '') || ' ' || coalesce(summary ->> 'fr', ''))
        || to_tsvector('english',
            coalesce(title ->> 'en', '') || ' ' || coalesce(summary ->> 'en', ''))
    ) STORED,

    -- Suppression douce : une URL diffusée ne doit jamais renvoyer une erreur
    -- serveur, et la corbeille reste restaurable par l'organisation.
    deleted_at        timestamptz,
    deleted_by        uuid        CONSTRAINT xmod_fk_articles_deleter
                                  REFERENCES identity.people(id) ON DELETE SET NULL,

    created_at        timestamptz NOT NULL DEFAULT now(),
    updated_at        timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_articles_publication_shape CHECK (
        (status = 'published' AND published_at IS NOT NULL)
     OR (status = 'scheduled' AND scheduled_for IS NOT NULL)
     OR (status NOT IN ('published', 'scheduled'))
    )
);

-- Unicité du slug sur les seuls articles vivants : une suppression libère l'URL.
CREATE UNIQUE INDEX ux_articles_slug ON publication.articles (slug) WHERE deleted_at IS NULL;

CREATE INDEX ix_articles_public
    ON publication.articles (published_at DESC)
    WHERE status = 'published' AND deleted_at IS NULL;
CREATE INDEX ix_articles_organization
    ON publication.articles (organization_id, status, created_at DESC)
    WHERE deleted_at IS NULL;
CREATE INDEX ix_articles_review_queue
    ON publication.articles (status, created_at)
    WHERE status IN ('submitted', 'in_review');
CREATE INDEX ix_articles_due
    ON publication.articles (scheduled_for)
    WHERE status = 'scheduled' AND deleted_at IS NULL;
CREATE INDEX ix_articles_search   ON publication.articles USING gin (search_vector);
CREATE INDEX ix_articles_author   ON publication.articles (author_id) WHERE author_id IS NOT NULL;
-- Fenêtre de quota : sert le comptage du trigger sans balayer la table.
CREATE INDEX ix_articles_quota_window
    ON publication.articles (organization_id, (coalesce(published_at, scheduled_for)) DESC)
    WHERE status IN ('published', 'scheduled') AND deleted_at IS NULL;

CREATE TRIGGER tg_articles_updated_at
    BEFORE UPDATE ON publication.articles
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

CREATE TRIGGER tg_articles_audit
    AFTER INSERT OR UPDATE OR DELETE ON publication.articles
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE publication.articles IS
    'Articles publiés par les organisations. Le passage en ligne est soumis au quota de publication.';
COMMENT ON COLUMN publication.articles.slug IS
    'Segment d''URL. Généré depuis le titre français via platform.slugify() si non fourni, avec suffixe en cas de collision.';
COMMENT ON COLUMN publication.articles.storage_bytes IS
    'Empreinte disque de l''article (couverture + pièces jointes), agrégée par organisation pour le quota de stockage.';
COMMENT ON COLUMN publication.articles.comment_count IS
    'Compteur projeté depuis le module engagement, qui détient les commentaires publics et leur modération.';

-- Rattachement thématique : AUCUNE table de liaison dédiée. Les thèmes et
-- secteurs d'un article vivent dans reference.entity_terms, comme pour les
-- propositions d'activité. Exemples :
--   INSERT INTO reference.entity_terms (entity_schema, entity_table, entity_id, term_id, role)
--   SELECT 'publication', 'articles', :article_id, t.id, 'primary'
--   FROM reference.taxonomy_terms t
--   WHERE t.taxonomy_code = 'activity_theme' AND t.code = 'adaptation';
--   -- lecture : SELECT reference.terms_of('publication','articles', :id, 'activity_theme');

-- Génération du slug quand l'application ne le fournit pas, et déduplication.
-- Le référencement Nuxt repose sur cette valeur : elle ne peut pas être laissée
-- à l'initiative de chaque appelant.
CREATE OR REPLACE FUNCTION publication.tg_articles_ensure_slug()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_base   text;
    v_slug   text;
    v_suffix integer := 1;
BEGIN
    IF NEW.slug IS NOT NULL THEN
        RETURN NEW;
    END IF;

    v_base := left(coalesce(platform.slugify(NEW.title ->> 'fr'), 'article'), 150);
    -- Le domaine platform.slug exige au moins deux caractères.
    IF length(v_base) < 2 THEN
        v_base := 'article-' || v_base;
    END IF;
    v_slug := v_base;

    WHILE EXISTS (
        SELECT 1 FROM publication.articles a
        WHERE a.slug = v_slug AND a.deleted_at IS NULL AND a.id IS DISTINCT FROM NEW.id
    ) LOOP
        v_suffix := v_suffix + 1;
        v_slug   := v_base || '-' || v_suffix;
    END LOOP;

    NEW.slug := v_slug;
    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_articles_ensure_slug
    BEFORE INSERT ON publication.articles
    FOR EACH ROW EXECUTE FUNCTION publication.tg_articles_ensure_slug();

-- -----------------------------------------------------------------------------
-- 3. Consommation du quota
--
-- Fenêtre GLISSANTE et non calendaire : « une fois par semaine » signifie « pas
-- deux fois en sept jours », pas « une fois entre lundi et dimanche » — sinon
-- publier le dimanche puis le lundi respecterait la règle tout en la violant.
-- Un article programmé consomme sa place dès la programmation : réserver un
-- créneau futur est une publication engagée.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE FUNCTION publication.storage_usage(
    p_organization_id     uuid,
    p_exclude_article_id  uuid DEFAULT NULL
)
RETURNS bigint
LANGUAGE sql
STABLE
AS $$
    SELECT coalesce(sum(a.storage_bytes), 0)::bigint
    FROM publication.articles a
    WHERE a.organization_id = p_organization_id
      AND a.deleted_at IS NULL
      AND a.id IS DISTINCT FROM p_exclude_article_id;
$$;

CREATE OR REPLACE FUNCTION publication.remaining_quota(p_organization_id uuid)
RETURNS TABLE (
    quota              integer,
    period             publication.quota_period,
    consumed           integer,
    remaining          integer,
    window_started_at  timestamptz,
    resets_at          timestamptz,
    is_suspended       boolean,
    max_storage_bytes  bigint,
    storage_used_bytes bigint
)
LANGUAGE sql
STABLE
AS $$
    WITH pol AS (
        SELECT p.*, now() - publication.period_interval(p.period) AS window_start
        FROM publication.effective_policy(p_organization_id) p
    ),
    consumption AS (
        SELECT
            count(*)::integer                                       AS used,
            min(coalesce(a.published_at, a.scheduled_for))          AS oldest
        FROM publication.articles a
        CROSS JOIN pol w
        WHERE a.organization_id = p_organization_id
          AND a.deleted_at IS NULL
          AND a.status IN ('published', 'scheduled')
          AND coalesce(a.published_at, a.scheduled_for) >= w.window_start
    )
    SELECT
        w.quota,
        w.period,
        c.used,
        CASE WHEN w.quota IS NULL THEN NULL ELSE greatest(w.quota - c.used, 0) END,
        w.window_start,
        -- Instant où la plus ancienne publication sort de la fenêtre : c'est là
        -- qu'un créneau se libère. Message affiché tel quel dans le back-office.
        CASE WHEN c.oldest IS NULL THEN now()
             ELSE c.oldest + publication.period_interval(w.period) END,
        coalesce(w.is_suspended, false),
        w.max_storage_bytes,
        publication.storage_usage(p_organization_id)
    FROM pol w CROSS JOIN consumption c;
$$;

COMMENT ON FUNCTION publication.remaining_quota(uuid) IS
    'État du quota d''une organisation : autorisé, consommé sur la fenêtre glissante, restant, date de réinitialisation et empreinte disque.';

-- Verrou de la règle. SQLSTATE dédiés (classe PQ, hors classes réservées) :
--   PQ001 quota d'articles épuisé  -> l'API répond 429 avec resets_at
--   PQ002 publication suspendue    -> l'API répond 403 avec le motif
--   PQ003 quota de stockage épuisé -> l'API répond 507
CREATE OR REPLACE FUNCTION publication.tg_enforce_publishing_quota()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_state record;
BEGIN
    -- Seul le franchissement vers un état publiquement engageant est contrôlé.
    IF NEW.status NOT IN ('published', 'scheduled') THEN
        RETURN NEW;
    END IF;

    -- scheduled -> published : le créneau a déjà été décompté, ne pas le compter
    -- deux fois. Idem pour une simple correction de contenu d'un article en ligne.
    IF TG_OP = 'UPDATE' AND OLD.status IN ('published', 'scheduled') THEN
        RETURN NEW;
    END IF;

    -- Horodatage posé par la base : sans lui, la fenêtre glissante serait
    -- incalculable pour cet article.
    IF NEW.status = 'published' AND NEW.published_at IS NULL THEN
        NEW.published_at := now();
    END IF;

    -- Sérialisation par organisation : deux publications concurrentes ne peuvent
    -- pas lire le même compteur et passer toutes les deux.
    PERFORM pg_advisory_xact_lock(hashtextextended('publication.quota:' || NEW.organization_id::text, 0));

    SELECT * INTO v_state FROM publication.remaining_quota(NEW.organization_id);

    IF v_state.is_suspended THEN
        RAISE EXCEPTION
            'Publication suspendue pour cette organisation. Contacter l''IFDD.'
            USING ERRCODE = 'PQ002',
                  DETAIL  = format('organization_id=%s', NEW.organization_id);
    END IF;

    IF v_state.quota IS NOT NULL AND v_state.consumed >= v_state.quota THEN
        RAISE EXCEPTION
            'Quota de publication atteint : % article(s) par %. Prochaine publication possible le %.',
            v_state.quota, publication.period_label(v_state.period),
            to_char(v_state.resets_at, 'DD/MM/YYYY à HH24:MI')
            USING ERRCODE = 'PQ001',
                  DETAIL  = format('organization_id=%s consumed=%s quota=%s resets_at=%s',
                                   NEW.organization_id, v_state.consumed, v_state.quota, v_state.resets_at),
                  HINT    = 'Programmer l''article après la date de réinitialisation ou demander une révision du quota.';
    END IF;

    -- NEW.storage_bytes est une colonne générée : elle n'est pas encore calculée
    -- dans un trigger BEFORE. On additionne donc ses deux termes sources.
    IF v_state.max_storage_bytes IS NOT NULL
       AND publication.storage_usage(NEW.organization_id, NEW.id)
           + NEW.cover_bytes + NEW.attachments_bytes > v_state.max_storage_bytes THEN
        RAISE EXCEPTION
            'Quota de stockage atteint : % octets autorisés, % octets utilisés.',
            v_state.max_storage_bytes, v_state.storage_used_bytes
            USING ERRCODE = 'PQ003',
                  HINT    = 'Alléger les pièces jointes ou archiver d''anciens articles.';
    END IF;

    RETURN NEW;
END;
$$;

CREATE TRIGGER tg_articles_enforce_quota
    BEFORE INSERT OR UPDATE OF status ON publication.articles
    FOR EACH ROW EXECUTE FUNCTION publication.tg_enforce_publishing_quota();

COMMENT ON FUNCTION publication.tg_enforce_publishing_quota() IS
    'Applique le quota éditorial et le quota de stockage au passage en published/scheduled, quel que soit le chemin d''écriture.';

-- Événements de domaine : l'outbox porte les notifications (accusé de dépôt,
-- avis de mise en ligne) sans coupler ce module à `engagement`.
CREATE OR REPLACE FUNCTION publication.tg_articles_emit_events()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'UPDATE' AND OLD.status IS NOT DISTINCT FROM NEW.status THEN
        RETURN NULL;
    END IF;

    IF NEW.status IN ('submitted', 'published', 'rejected') THEN
        PERFORM platform.emit_event(
            'publication', 'article', NEW.id,
            'publication.article.' || NEW.status::text,
            jsonb_build_object(
                'organization_id', NEW.organization_id,
                'author_id',       NEW.author_id,
                'slug',            NEW.slug,
                'title',           NEW.title,
                'published_at',    NEW.published_at
            )
        );
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_articles_emit_events
    AFTER INSERT OR UPDATE OF status ON publication.articles
    FOR EACH ROW EXECUTE FUNCTION publication.tg_articles_emit_events();

-- Mise en ligne des articles programmés, appelée périodiquement par le worker.
CREATE OR REPLACE FUNCTION publication.publish_due_articles(p_limit integer DEFAULT 100)
RETURNS SETOF uuid
LANGUAGE sql
AS $$
    WITH due AS (
        SELECT a.id FROM publication.articles a
        WHERE a.status = 'scheduled' AND a.deleted_at IS NULL AND a.scheduled_for <= now()
        ORDER BY a.scheduled_for
        LIMIT p_limit
        FOR UPDATE SKIP LOCKED
    )
    UPDATE publication.articles a
    SET status = 'published', published_at = coalesce(a.published_at, a.scheduled_for)
    FROM due
    WHERE a.id = due.id
    RETURNING a.id;
$$;

-- -----------------------------------------------------------------------------
-- 4. Historique des versions
--
-- Chaque écriture du contenu archive la version remplacée : la modération sait
-- ce qui a changé après validation, et l'organisation peut revenir en arrière.
-- La v1 n'offrait rien de tel : une mauvaise manipulation était définitive.
-- -----------------------------------------------------------------------------
CREATE TABLE publication.article_revisions (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    article_id      uuid        NOT NULL REFERENCES publication.articles(id) ON DELETE CASCADE,
    revision_number integer     NOT NULL CHECK (revision_number > 0),
    -- Instantané {title, summary, body} de la version remplacée.
    content         jsonb       NOT NULL,
    author_id       uuid        CONSTRAINT xmod_fk_article_revisions_author
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    change_note     text,
    created_at      timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_article_revisions UNIQUE (article_id, revision_number)
);

CREATE INDEX ix_article_revisions_article
    ON publication.article_revisions (article_id, revision_number DESC);

COMMENT ON TABLE publication.article_revisions IS
    'Versions successives du contenu d''un article. Alimenté automatiquement à chaque modification du texte.';

CREATE OR REPLACE FUNCTION publication.tg_capture_article_revision()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    INSERT INTO publication.article_revisions (article_id, revision_number, content, author_id)
    SELECT
        OLD.id,
        coalesce(max(r.revision_number), 0) + 1,
        jsonb_build_object('title', OLD.title, 'summary', OLD.summary, 'body', OLD.body),
        platform.current_actor_id()
    FROM publication.article_revisions r
    WHERE r.article_id = OLD.id;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_articles_capture_revision
    AFTER UPDATE OF title, summary, body ON publication.articles
    FOR EACH ROW
    WHEN (OLD.title IS DISTINCT FROM NEW.title
       OR OLD.summary IS DISTINCT FROM NEW.summary
       OR OLD.body IS DISTINCT FROM NEW.body)
    EXECUTE FUNCTION publication.tg_capture_article_revision();

-- Restauration : réécrit le contenu de l'article, ce qui archive au passage la
-- version courante. Aucune perte, quel que soit le sens du déplacement.
CREATE OR REPLACE FUNCTION publication.restore_revision(p_article_id uuid, p_revision_number integer)
RETURNS uuid
LANGUAGE plpgsql
AS $$
DECLARE
    v_content jsonb;
BEGIN
    SELECT r.content INTO v_content
    FROM publication.article_revisions r
    WHERE r.article_id = p_article_id AND r.revision_number = p_revision_number;

    IF v_content IS NULL THEN
        RAISE EXCEPTION 'Révision % introuvable pour l''article %.', p_revision_number, p_article_id
            USING ERRCODE = 'no_data_found';
    END IF;

    -- NULLIF sur le littéral JSON `null` : un chapeau absent au moment de
    -- l'instantané doit redevenir un NULL SQL, pas un jsonb 'null' que le
    -- domaine platform.i18n_text rejetterait.
    UPDATE publication.articles
    SET title   = (nullif(v_content -> 'title',   'null'::jsonb))::platform.i18n_text,
        summary = (nullif(v_content -> 'summary', 'null'::jsonb))::platform.i18n_text,
        body    = (nullif(v_content -> 'body',    'null'::jsonb))::platform.i18n_text
    WHERE id = p_article_id;

    RETURN p_article_id;
END;
$$;

-- -----------------------------------------------------------------------------
-- 5. Revue éditoriale
--
-- Décisions successives d'un article : la table est un journal, pas un statut.
-- Le motif de refus est obligatoire — la v1 laissait l'organisation sans
-- explication, ce qui générait autant de courriels que de refus.
-- -----------------------------------------------------------------------------
CREATE TYPE publication.review_decision AS ENUM ('approved', 'rejected', 'changes_requested');

CREATE TABLE publication.editorial_reviews (
    id           uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    article_id   uuid        NOT NULL REFERENCES publication.articles(id) ON DELETE CASCADE,
    reviewer_id  uuid        CONSTRAINT xmod_fk_editorial_reviews_reviewer
                             REFERENCES identity.people(id) ON DELETE SET NULL,
    decision     publication.review_decision NOT NULL,
    reason       text,
    -- Note interne non transmise à l'organisation (coordination de l'équipe).
    internal_note text,
    decided_at   timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ck_editorial_reviews_reason
        CHECK (decision = 'approved' OR (reason IS NOT NULL AND length(btrim(reason)) >= 10))
);

CREATE INDEX ix_editorial_reviews_article  ON publication.editorial_reviews (article_id, decided_at DESC);
CREATE INDEX ix_editorial_reviews_reviewer ON publication.editorial_reviews (reviewer_id, decided_at DESC);

COMMENT ON TABLE publication.editorial_reviews IS
    'Journal des décisions de modération d''un article. Un refus impose un motif communicable à l''organisation.';

-- -----------------------------------------------------------------------------
-- 6. Pièces jointes et comptabilité de stockage
-- -----------------------------------------------------------------------------
CREATE TABLE publication.article_attachments (
    id                 uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    article_id         uuid        NOT NULL REFERENCES publication.articles(id) ON DELETE CASCADE,
    asset_id           uuid        NOT NULL CONSTRAINT xmod_fk_article_attachments_asset
                                   REFERENCES media.assets(id) ON DELETE RESTRICT,
    label              platform.i18n_text,
    -- reference.taxonomy_terms.code de la taxonomie `document_type`.
    document_type_code text,
    -- Poids recopié depuis media.assets au rattachement : le quota se calcule
    -- localement, sans dépendre de la disponibilité du module média.
    byte_size          bigint      NOT NULL DEFAULT 0 CHECK (byte_size >= 0),
    sort_order         smallint    NOT NULL DEFAULT 0,
    created_by         uuid        CONSTRAINT xmod_fk_article_attachments_creator
                                   REFERENCES identity.people(id) ON DELETE SET NULL,
    created_at         timestamptz NOT NULL DEFAULT now(),
    CONSTRAINT ux_article_attachments UNIQUE (article_id, asset_id)
);

CREATE INDEX ix_article_attachments_article ON publication.article_attachments (article_id, sort_order);

COMMENT ON TABLE publication.article_attachments IS
    'Documents joints à un article (media.assets). Leur poids alimente le quota de stockage de l''organisation.';

CREATE OR REPLACE FUNCTION publication.tg_sync_attachments_bytes()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_article uuid := coalesce(NEW.article_id, OLD.article_id);
BEGIN
    UPDATE publication.articles a
    SET attachments_bytes = (
        SELECT coalesce(sum(t.byte_size), 0)
        FROM publication.article_attachments t
        WHERE t.article_id = v_article
    )
    WHERE a.id = v_article;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_article_attachments_sync_bytes
    AFTER INSERT OR UPDATE OF byte_size, article_id OR DELETE ON publication.article_attachments
    FOR EACH ROW EXECUTE FUNCTION publication.tg_sync_attachments_bytes();

-- -----------------------------------------------------------------------------
-- 7. Réactions
--
-- Réactions gardées dans ce module (compteur dénormalisé lu à chaque liste) ;
-- les COMMENTAIRES relèvent du module `engagement`, qui centralise fil de
-- discussion et modération pour toutes les entités de la plateforme.
-- -----------------------------------------------------------------------------
CREATE TYPE publication.reaction_kind AS ENUM ('like', 'love', 'insightful', 'useful');

CREATE TABLE publication.article_reactions (
    article_id uuid        NOT NULL REFERENCES publication.articles(id) ON DELETE CASCADE,
    person_id  uuid        NOT NULL CONSTRAINT xmod_fk_article_reactions_person
                           REFERENCES identity.people(id) ON DELETE CASCADE,
    kind       publication.reaction_kind NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (article_id, person_id, kind)
);

CREATE INDEX ix_article_reactions_person ON publication.article_reactions (person_id, created_at DESC);

COMMENT ON TABLE publication.article_reactions IS
    'Réactions d''une personne à un article (une par type). Les commentaires publics sont gérés par le module engagement.';

CREATE OR REPLACE FUNCTION publication.tg_sync_reaction_count()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        UPDATE publication.articles SET reaction_count = reaction_count + 1 WHERE id = NEW.article_id;
    ELSE
        UPDATE publication.articles SET reaction_count = greatest(reaction_count - 1, 0) WHERE id = OLD.article_id;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER tg_article_reactions_sync_count
    AFTER INSERT OR DELETE ON publication.article_reactions
    FOR EACH ROW EXECUTE FUNCTION publication.tg_sync_reaction_count();

-- -----------------------------------------------------------------------------
-- 8. Vue publique
--
-- Contrat de lecture unique pour Nuxt et le pré-rendu SEO : un article n'est
-- public que s'il est publié, daté dans le passé et non supprimé. Cette
-- condition n'est donc écrite qu'une fois, pas dans chaque requête d'API.
-- -----------------------------------------------------------------------------
CREATE OR REPLACE VIEW publication.public_articles AS
SELECT
    a.id, a.slug, a.title, a.summary, a.body,
    a.organization_id, a.author_id, a.cover_asset_id,
    a.related_event_id, a.related_session_id,
    a.published_at, a.view_count, a.reaction_count, a.comment_count,
    reference.terms_of('publication', 'articles', a.id, 'activity_theme')     AS theme_codes,
    reference.terms_of('publication', 'articles', a.id, 'application_sector') AS sector_codes
FROM publication.articles a
WHERE a.status = 'published'
  AND a.deleted_at IS NULL
  AND a.published_at <= now();

COMMENT ON VIEW publication.public_articles IS
    'Articles réellement visibles du public, thématiques résolues. Source unique du site Nuxt et du pré-rendu SEO.';

-- -----------------------------------------------------------------------------
-- 9. Raccordements et amorçage
-- -----------------------------------------------------------------------------

-- Fusion d'organisations : les articles suivent la fiche absorbante ; la
-- politique de quota de la fiche absorbée disparaît (la cible garde la sienne,
-- et deux politiques ouvertes sur la même organisation violeraient l'exclusion).
INSERT INTO org.organization_references (ref_schema, ref_table, ref_column, strategy) VALUES
    ('publication', 'articles',            'organization_id', 'reassign'),
    ('publication', 'publishing_policies', 'organization_id', 'delete')
ON CONFLICT DO NOTHING;

-- Politique par défaut : une publication par semaine, 200 Mio de médias.
-- Valeur volontairement prudente au regard de l'espace disque limité ; le
-- back-office relève le plafond organisation par organisation.
INSERT INTO publication.publishing_policies (organization_id, quota, period, max_storage_bytes, note)
SELECT NULL, 1, 'week', 209715200,
       'Politique par défaut de la plateforme, applicable à toute organisation sans règle propre.'
WHERE NOT EXISTS (
    SELECT 1 FROM publication.publishing_policies WHERE organization_id IS NULL
);
