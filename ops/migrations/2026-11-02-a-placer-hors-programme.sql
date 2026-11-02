-- Une séance « à placer » est hors du programme (02/11). Rejouable.
-- Modèle : docs/database/075_programme_sessions.sql § 1 (tg_sessions_unplaced)
-- et § 7 (detect_conflicts, publication_readiness).
--
-- « Retirer du calendrier » ne posait que room_id = NULL : la séance restait
-- publiée sans lieu, en direct, et bloquait la publication. Désormais :
--   - quitter la grille retire la séance du site public (planned, non publiée)
--     et coupe son direct ; une séance à placer n'est jamais publiée ;
--   - une séance à placer n'entre dans aucun conflit et ne retient plus la
--     publication (« Séance sans lieu ni précision de lieu » disparaît).
-- Aucune donnée touchée : aucune séance publiée n'est à placer en production
-- (vérifié le 02/11 ; les 61 séances terminées de la CdP30 sans salle ne sont
-- pas concernées, le déclencheur ne vise que planned et scheduled).
-- Les deux fonctions remplacées étaient identiques en production à celles du
-- dépôt avant ce changement.
--
-- Jouer AVANT de déployer l'API et le worker (l'ordre inverse marche aussi : la
-- base refuse déjà de publier une séance à placer, seul le décompte annoncé
-- différerait de l'effet).
-- Usage : psql -v ON_ERROR_STOP=1 -d epavillon -f ce-fichier
BEGIN;

-- Copie de docs/database/075_programme_sessions.sql.
CREATE OR REPLACE FUNCTION programme.tg_sessions_unplaced()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    IF NEW.room_id IS NOT NULL OR NEW.location_note IS NOT NULL
       OR NEW.status NOT IN ('planned', 'scheduled') THEN
        RETURN NEW;
    END IF;

    IF OLD.room_id IS NOT NULL OR OLD.location_note IS NOT NULL THEN
        NEW.published_at := NULL;
        NEW.status := 'planned';
        NEW.is_streamed := false;
        NEW.broadcast_channel_id := NULL;
    ELSIF OLD.published_at IS NULL AND NEW.published_at IS NOT NULL THEN
        NEW.published_at := NULL;
        NEW.status := OLD.status;
    END IF;
    RETURN NEW;
END;
$$;

DROP TRIGGER IF EXISTS tg_sessions_unplaced ON programme.sessions;
CREATE TRIGGER tg_sessions_unplaced
    BEFORE UPDATE OF room_id, location_note, published_at ON programme.sessions
    FOR EACH ROW EXECUTE FUNCTION programme.tg_sessions_unplaced();

CREATE OR REPLACE FUNCTION programme.detect_conflicts(p_event_id uuid)
RETURNS TABLE (
    severity      text,
    conflict_kind text,
    subject_id    uuid,
    subject_label text,
    session_a     uuid,
    session_a_title text,
    session_b     uuid,
    session_b_title text,
    overlap       tstzrange
)
LANGUAGE sql
STABLE
AS $$
    WITH active AS (
        SELECT s.*, platform.t(s.title) AS label
        FROM programme.sessions s
        WHERE s.status IN ('planned', 'scheduled', 'live')
          AND (s.room_id IS NOT NULL OR s.location_note IS NOT NULL)
    )
    -- 1. Un seul stand : deux activités du même événement OCCUPANT UNE SALLE
    --    PHYSIQUE en même temps. Voir l'en-tête : une séance en salle virtuelle
    --    ou pas encore installée n'occupe pas le stand.
    --
    --    Les deux séances dans la MÊME salle sont écartées ici : la branche 3 le
    --    dit mieux, en nommant la salle. Sans cette exclusion, chaque double
    --    réservation était comptée deux fois dans le bandeau du planificateur.
    SELECT 'blocking', 'venue_capacity',
           a.event_id, 'Stand unique de l''événement',
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM active a
    JOIN active b ON b.event_id = a.event_id AND b.id > a.id AND a.time_range && b.time_range
    WHERE a.event_id = p_event_id
      AND a.enforce_room_exclusivity
      AND b.enforce_room_exclusivity
      AND a.room_id IS DISTINCT FROM b.room_id

    UNION ALL

    -- 2. Un seul direct, tous événements confondus.
    SELECT 'blocking', 'broadcast',
           a.broadcast_channel_id, 'Canal de diffusion occupé',
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM active a
    JOIN active b ON b.id > a.id
                 AND b.is_streamed
                 AND b.broadcast_channel_id IS NOT DISTINCT FROM a.broadcast_channel_id
                 AND a.time_range && b.time_range
    WHERE a.is_streamed
      AND a.broadcast_channel_id IS NOT NULL
      AND (a.event_id = p_event_id OR b.event_id = p_event_id)

    UNION ALL

    -- 3. Deux sessions dans la même salle physique.
    SELECT 'blocking', 'room',
           a.room_id, platform.t(r.name),
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM active a
    JOIN active b ON b.room_id = a.room_id AND b.id > a.id AND a.time_range && b.time_range
    JOIN event.rooms r ON r.id = a.room_id
    WHERE a.event_id = p_event_id
      AND a.room_id IS NOT NULL
      AND NOT r.is_virtual

    UNION ALL

    -- 4. Un intervenant attendu à deux endroits.
    SELECT 'warning', 'speaker',
           p.id, p.display_name,
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM programme.session_speakers sa
    JOIN programme.session_speakers sb ON sb.person_id = sa.person_id AND sb.session_id <> sa.session_id
    JOIN active a ON a.id = sa.session_id
    JOIN active b ON b.id = sb.session_id AND b.id > a.id AND a.time_range && b.time_range
    JOIN identity.people p ON p.id = sa.person_id
    WHERE a.event_id = p_event_id

    UNION ALL

    -- 5. Une organisation programmée deux fois sur le même créneau, en tenant
    --    compte des co-organisations.
    SELECT 'warning', 'organization',
           o.id, o.legal_name,
           a.id, a.label, b.id, b.label, a.time_range * b.time_range
    FROM programme.session_organizations oa
    JOIN programme.session_organizations ob ON ob.organization_id = oa.organization_id
                                           AND ob.session_id <> oa.session_id
    JOIN active a ON a.id = oa.session_id
    JOIN active b ON b.id = ob.session_id AND b.id > a.id AND a.time_range && b.time_range
    JOIN org.organizations o ON o.id = oa.organization_id
    WHERE a.event_id = p_event_id;
$$;

CREATE OR REPLACE FUNCTION programme.publication_readiness(p_event_id uuid)
RETURNS TABLE (severity text, issue text, detail text, session_id uuid, occurs_at timestamptz)
LANGUAGE sql
STABLE
AS $$
    -- NE CONTRÔLE QUE CE QUI SERA PUBLIC (28/10 2026). Un dossier en évaluation
    -- ou rejeté peut porter un créneau pressenti (repris de la v1) : la
    -- publication le laisse privé, il ne doit donc pas la retenir. Est retenue
    -- une séance déjà publique, sans dossier (programmée par l'IFDD) ou de
    -- dossier accepté : le prédicat même de la publication (module event,
    -- `seances_a_publier`), plus ce qui est déjà en ligne. Le planificateur,
    -- lui, garde tous les conflits par `detect_conflicts()`.
    --
    -- Une séance à placer (ni salle ni précision de lieu) n'est jamais publiée
    -- (`tg_sessions_unplaced`, 02/11/2026) : elle ne retient donc rien.
    WITH retenue AS (
        SELECT s.*
        FROM programme.sessions s
        LEFT JOIN programme.proposals p ON p.id = s.proposal_id
        WHERE (s.room_id IS NOT NULL OR s.location_note IS NOT NULL)
          AND (s.published_at IS NOT NULL
               OR s.proposal_id IS NULL
               OR p.status = 'accepted')
    )
    SELECT c.severity,
           CASE c.conflict_kind
               WHEN 'venue_capacity' THEN 'Deux activités simultanées sur un stand unique'
               WHEN 'broadcast'      THEN 'Deux diffusions en direct simultanées'
               WHEN 'room'           THEN 'Salle réservée deux fois'
               WHEN 'speaker'        THEN 'Intervenant attendu à deux endroits'
               ELSE 'Organisation programmée deux fois'
           END,
           format('%s ↔ %s', c.session_a_title, c.session_b_title),
           c.session_a,
           lower(c.overlap)
    FROM programme.detect_conflicts(p_event_id) c
    WHERE c.session_a IN (SELECT r.id FROM retenue r)
      AND c.session_b IN (SELECT r.id FROM retenue r)

    UNION ALL

    SELECT 'blocking', 'Session sans créneau valide',
           platform.t(s.title), s.id, s.starts_at
    FROM retenue s
    WHERE s.event_id = p_event_id
      AND s.status IN ('planned', 'scheduled')
      AND (s.starts_at IS NULL OR s.ends_at <= s.starts_at)

    UNION ALL

    -- « Séance sans lieu ni précision de lieu », bloquante du 18/08 au
    -- 02/11/2026, a disparu : c'est la définition même d'une séance à placer,
    -- que la publication laisse désormais privée au lieu d'être retenue par elle.
    SELECT 'warning', 'Session diffusée sans canal assigné',
           platform.t(s.title), s.id, s.starts_at
    FROM retenue s
    WHERE s.event_id = p_event_id
      AND s.is_streamed
      AND s.broadcast_channel_id IS NULL
      AND s.status IN ('planned', 'scheduled')

    UNION ALL

    SELECT 'warning', 'Session sans intervenant déclaré',
           platform.t(s.title), s.id, s.starts_at
    FROM retenue s
    WHERE s.event_id = p_event_id
      AND s.status IN ('planned', 'scheduled')
      AND NOT EXISTS (SELECT 1 FROM programme.session_speakers sp WHERE sp.session_id = s.id);
$$;

COMMIT;
