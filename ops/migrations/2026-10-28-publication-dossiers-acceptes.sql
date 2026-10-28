-- Le contrôle avant publication ne regarde que ce qui sera public (28/10). Rejouable.
-- Modèle : docs/database/075_programme_sessions.sql § 7, programme.publication_readiness().
--
-- La publication du programme ne rend plus publiques que les séances sans
-- dossier ou de dossier accepté (API, même livraison). Un créneau pressenti
-- d'un dossier en évaluation ou rejeté reste privé : il ne doit plus retenir
-- la publication (lieu manquant, conflit entre deux dossiers non tranchés).
--
-- Déployer avec l'API : sans elle, la publication rendrait encore publics des
-- créneaux que ce contrôle n'a pas vérifiés.
BEGIN;

-- Copie de docs/database/075_programme_sessions.sql.
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
    WITH retenue AS (
        SELECT s.*
        FROM programme.sessions s
        LEFT JOIN programme.proposals p ON p.id = s.proposal_id
        WHERE s.published_at IS NOT NULL
           OR s.proposal_id IS NULL
           OR p.status = 'accepted'
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

    -- SANS LIEU, NI MÊME UNE PRÉCISION DE LIEU (ajouté le 18/08 2026, en
    -- écrivant le planificateur). Une séance retenue vit d'abord sans salle :
    -- c'est l'état normal tant que l'équipe n'a pas arbitré, et c'est justement
    -- ce que le panneau « à placer » du planificateur montre. Rien ne l'empêche
    -- ni ne doit l'empêcher — mais une programmation rendue publique sans dire
    -- OÙ se tient l'activité envoie le visiteur chercher une salle qui n'a pas
    -- de nom. `location_note` suffit quand il n'y a pas de salle en base : une
    -- séance en ligne y porte son mode d'accès, une séance hors pavillon
    -- l'adresse de son hôte.
    SELECT 'blocking', 'Séance sans lieu ni précision de lieu',
           platform.t(s.title), s.id, s.starts_at
    FROM retenue s
    WHERE s.event_id = p_event_id
      AND s.status IN ('planned', 'scheduled')
      AND s.room_id IS NULL
      AND s.location_note IS NULL

    UNION ALL

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
