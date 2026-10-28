-- Envois de courriels du back-office (28/10). Rejouable.
-- Modèle : docs/database/110_engagement.sql § 12 (mailings, mailing_recipients,
-- mailing_audience(), réglage de la copie de l'équipe, permission
-- engagement.mailing.send accordée au rôle admin).
--
-- Demande du commanditaire : l'équivalent de l'envoi de courriels de la v1,
-- avec un historique, la boîte d'envoi ne se consultant pas.
--
-- Additive : deux tables, deux types, une fonction, un réglage, une
-- permission ; pièces jointes : une colonne de media.attachable_roles, son
-- contrôle dans media.tg_validate_attachment(), une ligne de la table blanche. Sans limite d'envoi (décision du commanditaire, 28/10) : le
-- réglage de cadence d'une version antérieure de ce fichier est retiré s'il
-- a été posé. Déployer avec l'API et le worker, qui s'en servent.
BEGIN;
DO $$
BEGIN
    IF to_regtype('engagement.mailing_status') IS NULL THEN
        CREATE TYPE engagement.mailing_status AS ENUM ('draft', 'queued', 'sent', 'cancelled');
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS engagement.mailings (
    id              uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    -- RÈGLE MÉTIER N° 8 : un envoi appartient à une édition, et un
    -- administrateur de cette seule édition n'écrit qu'à ses participants.
    event_id        uuid        NOT NULL CONSTRAINT xmod_fk_mailings_event
                                REFERENCES event.events(id) ON DELETE RESTRICT,
    status          engagement.mailing_status NOT NULL DEFAULT 'draft',

    subject         text        NOT NULL DEFAULT '',
    -- HTML assaini à l'écriture (même liste blanche que les modèles de
    -- messages). Variables `{{prenom}}`, `{{nom}}`, `{{organisation}}`,
    -- `{{email}}`, `{{edition}}`, résolues par destinataire au moment de l'envoi.
    body_html       text        NOT NULL DEFAULT '',

    -- Critères nommés et combinables (union), évalués par mailing_audience() :
    -- [{"kind": "submitters" | "speakers" | "registrants" | "roles",
    --   "proposal_statuses": [...], "registration_statuses": [...],
    --   "role_codes": [...], "session_id": "..."}]
    audience        jsonb       NOT NULL DEFAULT '[]'::jsonb,
    -- Adresses saisies à la main, en plus des critères.
    extra_addresses platform.email[] NOT NULL DEFAULT '{}',

    -- COPIE : posée sur CHAQUE message, comme en v1. Le destinataire qui répond
    -- à tous écrit donc aussi à l'équipe, qui voit les réponses : c'était la
    -- stratégie de l'équipe faute de pouvoir lire la boîte d'envoi. Le prix est
    -- écrit à l'écran : 300 destinataires, 300 copies par adresse en copie.
    cc              platform.email[] NOT NULL DEFAULT '{}',
    bcc             platform.email[] NOT NULL DEFAULT '{}',
    reply_to        platform.email,

    duplicated_from uuid        REFERENCES engagement.mailings(id) ON DELETE SET NULL,
    created_by      uuid        CONSTRAINT xmod_fk_mailings_author
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    queued_by       uuid        CONSTRAINT xmod_fk_mailings_sender
                                REFERENCES identity.people(id) ON DELETE SET NULL,
    queued_at       timestamptz,
    completed_at    timestamptz,
    cancelled_at    timestamptz,
    created_at      timestamptz NOT NULL DEFAULT now(),
    updated_at      timestamptz NOT NULL DEFAULT now(),

    CONSTRAINT ck_mailings_audience_shape CHECK (jsonb_typeof(audience) = 'array'),
    CONSTRAINT ck_mailings_queued CHECK ((status = 'draft') = (queued_at IS NULL)),
    -- Un envoi parti a un objet et un corps : la base le garantit, pas l'écran.
    CONSTRAINT ck_mailings_ready CHECK (
        status = 'draft'
        OR (length(btrim(subject)) > 0 AND length(btrim(body_html)) > 0)),
    -- Au-delà, la copie n'est plus une copie mais une liste de diffusion cachée,
    -- et chaque adresse multiplie le nombre de messages comptés par le serveur.
    CONSTRAINT ck_mailings_copies CHECK (cardinality(cc) <= 10 AND cardinality(bcc) <= 10),
    CONSTRAINT ck_mailings_extra CHECK (cardinality(extra_addresses) <= 500)
);

CREATE INDEX IF NOT EXISTS ix_mailings_event ON engagement.mailings (event_id, created_at DESC);

DROP TRIGGER IF EXISTS tg_mailings_updated_at ON engagement.mailings;
CREATE TRIGGER tg_mailings_updated_at
    BEFORE UPDATE ON engagement.mailings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

DROP TRIGGER IF EXISTS tg_mailings_audit ON engagement.mailings;
CREATE TRIGGER tg_mailings_audit
    AFTER INSERT OR UPDATE OR DELETE ON engagement.mailings
    FOR EACH ROW EXECUTE FUNCTION platform.tg_audit();

COMMENT ON TABLE engagement.mailings IS
    'Courriels écrits depuis le back-office à des participants d''une édition. Brouillon, puis envoi figé et historisé.';
COMMENT ON COLUMN engagement.mailings.audience IS
    'Critères de destinataires combinés par union ; évalués par engagement.mailing_audience(), figés dans mailing_recipients à l''envoi.';

DO $$
BEGIN
    IF to_regtype('engagement.mailing_recipient_status') IS NULL THEN
        CREATE TYPE engagement.mailing_recipient_status AS ENUM ('queued', 'sending', 'sent', 'failed', 'suppressed', 'cancelled');
    END IF;
END
$$;

CREATE TABLE IF NOT EXISTS engagement.mailing_recipients (
    id          uuid        PRIMARY KEY DEFAULT platform.uuid_v7(),
    mailing_id  uuid        NOT NULL REFERENCES engagement.mailings(id) ON DELETE CASCADE,
    email       platform.email NOT NULL,
    person_id   uuid        CONSTRAINT xmod_fk_mailing_recipients_person
                            REFERENCES identity.people(id) ON DELETE SET NULL,
    -- Valeurs des variables, figées à l'envoi : l'historique montre ce qui est
    -- parti, même si la personne change de nom ensuite.
    variables   jsonb       NOT NULL DEFAULT '{}'::jsonb,
    -- Les critères qui ont produit cette adresse ('submitters', 'addresses'…).
    sources     text[]      NOT NULL DEFAULT '{}',
    status      engagement.mailing_recipient_status NOT NULL DEFAULT 'queued',
    last_error  text,
    sent_at     timestamptz,
    updated_at  timestamptz NOT NULL DEFAULT now(),
    -- citext : la même adresse en deux casses reste un seul destinataire.
    CONSTRAINT ux_mailing_recipients UNIQUE (mailing_id, email)
);

CREATE INDEX IF NOT EXISTS ix_mailing_recipients_queued
    ON engagement.mailing_recipients (mailing_id, id) WHERE status = 'queued';

DROP TRIGGER IF EXISTS tg_mailing_recipients_updated_at ON engagement.mailing_recipients;
CREATE TRIGGER tg_mailing_recipients_updated_at
    BEFORE UPDATE ON engagement.mailing_recipients
    FOR EACH ROW EXECUTE FUNCTION platform.tg_set_updated_at();

COMMENT ON TABLE engagement.mailing_recipients IS
    'Destinataires figés d''un envoi du back-office, un par adresse, avec leur état. Remplace la boîte d''envoi illisible.';

-- La liste des destinataires d'un envoi, telle qu'elle partirait maintenant.
--
-- Lit hors de son schéma (programme, identity, org) parce que la question porte
-- sur les participants d'une édition, comme schedule_session_reminders(). Ne
-- regarde QUE l'édition passée : un critère qui citerait la séance d'une autre
-- édition ne rend rien (règle métier n° 8, y compris sur requête forgée).
--
-- Une ligne par adresse (citext, donc sans égard à la casse), avec les critères
-- qui l'ont produite. Les comptes non actifs (suspendus, bloqués, anonymisés)
-- sont écartés ; une adresse libre est gardée telle quelle, rattachée à sa
-- personne si elle en est l'adresse principale.
CREATE OR REPLACE FUNCTION engagement.mailing_audience(
    p_event_id        uuid,
    p_criteria        jsonb,
    p_extra_addresses platform.email[] DEFAULT '{}'
)
RETURNS TABLE (
    email         text,
    person_id     uuid,
    first_name    text,
    last_name     text,
    organization  text,
    sources       text[],
    is_suppressed boolean
)
LANGUAGE sql
STABLE
AS $$
    WITH crit AS (
        SELECT c,
               c ->> 'kind' AS kind,
               NULLIF(c ->> 'session_id', '')::uuid AS session_id,
               ARRAY(SELECT jsonb_array_elements_text(COALESCE(c -> 'proposal_statuses', '[]'))) AS proposal_statuses,
               ARRAY(SELECT jsonb_array_elements_text(COALESCE(c -> 'registration_statuses', '[]'))) AS registration_statuses,
               ARRAY(SELECT jsonb_array_elements_text(COALESCE(c -> 'role_codes', '[]'))) AS role_codes
        FROM jsonb_array_elements(
                 CASE WHEN jsonb_typeof(p_criteria) = 'array' THEN p_criteria ELSE '[]'::jsonb END) AS t(c)
    ),
    -- Les dossiers visés : de l'édition, non supprimés, au statut demandé (tous
    -- sauf les brouillons si aucun n'est précisé), réduits au dossier d'une
    -- séance quand le critère en nomme une.
    dossiers AS (
        SELECT crit.kind, crit.session_id, p.id AS proposal_id, p.submitted_by,
               p.contact_person_id, p.organization_id
        FROM crit
        JOIN programme.proposals p
          ON p.event_id = p_event_id AND p.deleted_at IS NULL
        WHERE crit.kind IN ('submitters', 'speakers')
          AND (CASE WHEN cardinality(crit.proposal_statuses) = 0 THEN p.status <> 'draft'
                    ELSE p.status::text = ANY (crit.proposal_statuses) END)
          AND (crit.session_id IS NULL OR EXISTS (
                   SELECT 1 FROM programme.sessions s
                   WHERE s.id = crit.session_id AND s.event_id = p_event_id
                     AND s.proposal_id = p.id))
    ),
    candidats AS (
        -- Déposant et personne à contacter de chaque dossier.
        SELECT x.pid AS person_id, d.organization_id, 'submitters' AS source
        FROM dossiers d
        CROSS JOIN LATERAL unnest(ARRAY[d.submitted_by, d.contact_person_id]) AS x(pid)
        WHERE d.kind = 'submitters' AND x.pid IS NOT NULL

        UNION ALL
        -- Intervenants des dossiers. Un intervenant sans adresse (sans
        -- personne) n'est joignable par aucun courriel.
        SELECT ps.person_id, COALESCE(ps.organization_id, d.organization_id), 'speakers'
        FROM dossiers d
        JOIN programme.proposal_speakers ps ON ps.proposal_id = d.proposal_id
        WHERE d.kind = 'speakers' AND ps.person_id IS NOT NULL

        UNION ALL
        -- Intervenants d'une séance programmée sans dossier (l'IFDD la porte).
        SELECT ss.person_id, NULL::uuid, 'speakers'
        FROM crit
        JOIN programme.sessions s
          ON s.id = crit.session_id AND s.event_id = p_event_id AND s.proposal_id IS NULL
        JOIN programme.session_speakers ss ON ss.session_id = s.id
        WHERE crit.kind = 'speakers' AND ss.person_id IS NOT NULL

        UNION ALL
        -- Inscrits : par défaut ceux qui ont une place ou l'attendent, ou y
        -- sont venus. Un désinscrit n'est écrit que si on le demande.
        SELECT r.person_id, r.organization_id, 'registrants'
        FROM crit
        JOIN programme.sessions s
          ON s.event_id = p_event_id
         AND (crit.session_id IS NULL OR s.id = crit.session_id)
        JOIN programme.registrations r ON r.session_id = s.id
        WHERE crit.kind = 'registrants'
          AND (CASE WHEN cardinality(crit.registration_statuses) = 0
                    THEN r.status IN ('registered', 'waitlisted', 'attended')
                    ELSE r.status::text = ANY (crit.registration_statuses) END)

        UNION ALL
        -- L'équipe de l'édition : rôles attribués sur cette édition, en cours.
        SELECT ra.person_id, NULL::uuid, 'roles'
        FROM crit
        JOIN identity.role_assignments ra
          ON ra.scope_type = 'event' AND ra.scope_id = p_event_id
         AND ra.revoked_at IS NULL
         AND ra.valid_from <= now()
         AND (ra.valid_until IS NULL OR ra.valid_until > now())
        WHERE crit.kind = 'roles'
          AND (cardinality(crit.role_codes) = 0 OR ra.role_code = ANY (crit.role_codes))
    ),
    adresses AS (
        SELECT pe.primary_email::text AS email, pe.id AS person_id, pe.first_name,
               pe.last_name, COALESCE(c.organization_id, pe.primary_organization_id) AS organization_id,
               c.source
        FROM candidats c
        JOIN identity.people pe ON pe.id = c.person_id AND pe.status = 'active'

        UNION ALL
        SELECT a::text, pe.id, pe.first_name, pe.last_name, pe.primary_organization_id, 'addresses'
        FROM unnest(COALESCE(p_extra_addresses, '{}')) AS a
        LEFT JOIN identity.people pe ON pe.primary_email = a AND pe.status = 'active'
    )
    SELECT lower(a.email),
           (array_agg(a.person_id ORDER BY a.person_id) FILTER (WHERE a.person_id IS NOT NULL))[1],
           (array_agg(a.first_name) FILTER (WHERE a.first_name IS NOT NULL))[1],
           (array_agg(a.last_name) FILTER (WHERE a.last_name IS NOT NULL))[1],
           (array_agg(COALESCE(o.acronym, o.legal_name)) FILTER (WHERE o.id IS NOT NULL))[1],
           array_agg(DISTINCT a.source ORDER BY a.source),
           engagement.is_email_suppressed(lower(a.email)::platform.email)
    FROM adresses a
    LEFT JOIN org.organizations o ON o.id = a.organization_id
    GROUP BY lower(a.email)
    ORDER BY 4 NULLS LAST, 3 NULLS LAST, 1;
$$;

COMMENT ON FUNCTION engagement.mailing_audience IS
    'Destinataires d''un envoi du back-office : critères d''une édition (union) et adresses libres, dédoublonnés, liste de suppression signalée.';

-- Copie de l'équipe proposée par défaut, réglable sans redéploiement. Aucune
-- limite d'envoi : décision du commanditaire le 28/10.
INSERT INTO platform.settings (key, value, description) VALUES
    ('engagement.mailing.team_cc', '[]',
     'Adresses de l''équipe proposées en copie de chaque nouvel envoi, en plus de celle de la personne qui écrit.')
ON CONFLICT (key) DO NOTHING;

INSERT INTO identity.permissions (code, label, module_code) VALUES
    ('engagement.mailing.send', '{"fr":"Écrire aux participants d''une édition","en":"Email the participants of an edition"}', 'engagement')
ON CONFLICT (code) DO NOTHING;

INSERT INTO identity.role_permissions (role_code, permission_code) VALUES
    ('admin', 'engagement.mailing.send')
ON CONFLICT DO NOTHING;

DELETE FROM platform.settings WHERE key = 'engagement.mailing.hourly_limit';


-- Pièces jointes (28/10). Modèle : 050_media.sql (colonne max_total_byte_size,
-- contrôle du poids cumulé dans tg_validate_attachment) et 110_engagement.sql
-- (ligne engagement.mailings / attachment).
ALTER TABLE media.attachable_roles
    ADD COLUMN IF NOT EXISTS max_total_byte_size bigint
        CHECK (max_total_byte_size IS NULL OR max_total_byte_size > 0);

-- Copie de docs/database/050_media.sql.
CREATE OR REPLACE FUNCTION media.tg_validate_attachment()
RETURNS trigger
LANGUAGE plpgsql
AS $$
DECLARE
    v_rule  media.attachable_roles%ROWTYPE;
    v_asset media.assets%ROWTYPE;
BEGIN
    SELECT * INTO v_rule FROM media.attachable_roles
    WHERE owner_schema = NEW.owner_schema AND owner_table = NEW.owner_table AND role = NEW.role;

    IF NOT FOUND OR NOT v_rule.is_active THEN
        RAISE EXCEPTION 'Rattachement refusé : le rôle « % » n''est pas déclaré pour %.%.',
            NEW.role, NEW.owner_schema, NEW.owner_table
            USING ERRCODE = 'integrity_constraint_violation',
                  HINT = 'Déclarer la combinaison dans media.attachable_roles avant de l''utiliser.';
    END IF;

    SELECT * INTO v_asset FROM media.assets WHERE id = NEW.asset_id;
    IF v_asset.deleted_at IS NOT NULL THEN
        RAISE EXCEPTION 'Rattachement refusé : l''objet % est supprimé.', NEW.asset_id
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;
    IF v_asset.status = 'quarantined' THEN
        RAISE EXCEPTION 'Rattachement refusé : l''objet % est en quarantaine.', NEW.asset_id
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    -- Contrôle du type MIME attendu pour ce rôle.
    IF cardinality(v_rule.allowed_mime_prefixes) > 0
       AND NOT EXISTS (
           SELECT 1 FROM unnest(v_rule.allowed_mime_prefixes) AS p
           WHERE v_asset.mime_type LIKE replace(p, '*', '%')
       ) THEN
        RAISE EXCEPTION 'Rattachement refusé : type % non accepté pour le rôle « % » (attendu : %).',
            v_asset.mime_type, NEW.role, array_to_string(v_rule.allowed_mime_prefixes, ', ')
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    IF v_rule.max_byte_size IS NOT NULL AND v_asset.byte_size > v_rule.max_byte_size THEN
        RAISE EXCEPTION 'Rattachement refusé : % octets dépassent la limite de % pour le rôle « % ».',
            v_asset.byte_size, v_rule.max_byte_size, NEW.role
            USING ERRCODE = 'integrity_constraint_violation';
    END IF;

    -- CONTRÔLE DE LA FORME. Ne s'applique QU'AUX OBJETS MESURÉS : `width` et
    -- `height` sont nuls pour un document, et un PDF n'a pas de ratio à
    -- respecter. Un objet image sans dimensions relevées passe donc — c'est le
    -- relevé qui a échoué, pas le fichier qui est mal cadré, et refuser ici
    -- transformerait une panne de traitement en refus de téléversement.
    IF v_rule.expected_aspect_ratio IS NOT NULL
       AND v_asset.width IS NOT NULL AND v_asset.height IS NOT NULL
       AND v_asset.height > 0
       AND abs(v_asset.width::numeric / v_asset.height - v_rule.expected_aspect_ratio)
           > v_rule.expected_aspect_ratio * v_rule.aspect_ratio_tolerance THEN
        RAISE EXCEPTION 'Rattachement refusé : % × % ne respecte pas la forme attendue du rôle « % » (rapport %, attendu % à % près).',
            v_asset.width, v_asset.height, NEW.role,
            round(v_asset.width::numeric / v_asset.height, 4),
            v_rule.expected_aspect_ratio,
            to_char(v_rule.aspect_ratio_tolerance * 100, 'FM990.0') || ' %'
            USING ERRCODE = 'integrity_constraint_violation',
                  HINT = 'Recadrer le fichier avant de le téléverser : la déclinaison est choisie à la main, jamais rognée par le navigateur.';
    END IF;

    -- POIDS CUMULÉ (28/10) : les objets déjà rattachés à cette entité pour ce
    -- rôle, plus celui-ci, ne dépassent pas le plafond déclaré.
    IF v_rule.max_total_byte_size IS NOT NULL
       AND v_asset.byte_size + COALESCE((
           SELECT sum(o.byte_size)
           FROM media.attachments a
           JOIN media.assets o ON o.id = a.asset_id
           WHERE a.owner_schema = NEW.owner_schema AND a.owner_table = NEW.owner_table
             AND a.owner_id = NEW.owner_id AND a.role = NEW.role
             AND a.id IS DISTINCT FROM NEW.id AND a.asset_id <> NEW.asset_id), 0)
           > v_rule.max_total_byte_size THEN
        RAISE EXCEPTION 'Rattachement refusé : le rôle « % » ne dépasse pas % octets au total par entité.',
            NEW.role, v_rule.max_total_byte_size
            USING ERRCODE = 'check_violation';
    END IF;

    NEW.is_exclusive := NOT v_rule.is_multiple;

    -- Rôle unique déjà pourvu : message explicite plutôt qu'une violation d'index.
    IF NEW.is_exclusive AND EXISTS (
        SELECT 1 FROM media.attachments a
        WHERE a.owner_schema = NEW.owner_schema AND a.owner_table = NEW.owner_table
          AND a.owner_id = NEW.owner_id AND a.role = NEW.role
          AND a.id IS DISTINCT FROM NEW.id
    ) THEN
        RAISE EXCEPTION 'Rattachement refusé : le rôle « % » n''accepte qu''un seul objet pour %.% (%).',
            NEW.role, NEW.owner_schema, NEW.owner_table, NEW.owner_id
            USING ERRCODE = 'unique_violation',
                  HINT = 'Remplacer le rattachement existant plutôt qu''en ajouter un second.';
    END IF;

    RETURN NEW;
END;
$$;

-- PIÈCES JOINTES d'un envoi (28/10, demande du commanditaire) : plusieurs
-- fichiers, PDF, documents bureautiques, texte et images, toujours privés (la
-- garde du module media l'impose : un fichier joint n'est jamais servi au
-- public). 10 Mo au total par envoi : le serveur d'envoi de l'ancien
-- hébergement borne la taille d'un message, de valeur inconnue. Réglable :
-- UPDATE media.attachable_roles SET max_total_byte_size = … .
INSERT INTO media.attachable_roles
    (owner_schema, owner_table, role, label, is_multiple, allowed_mime_prefixes,
     max_byte_size, max_total_byte_size) VALUES
    ('engagement', 'mailings', 'attachment', '{"fr":"Pièce jointe","en":"Attachment"}', true,
     '{application/pdf,application/msword,application/vnd.*,application/rtf,text/plain,text/csv,image/*}',
     10485760, 10485760)
ON CONFLICT DO NOTHING;

COMMIT;
