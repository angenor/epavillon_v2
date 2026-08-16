import type { SessionId } from '~/types/shared'

/**
 * LE REPÈRE « EN DIRECT » — règle métier n° 4 : UN SEUL DIRECT À LA FOIS, tous
 * événements confondus. Une seule équipe technique, un seul flux.
 *
 * Ce composable existe pour que la règle soit tenue par CONSTRUCTION plutôt que
 * par discipline. Sans lui, chaque carte déciderait seule d'afficher son repère
 * à partir de `session.status === 'live'`, et deux cartes clignoteraient dès
 * qu'une séance resterait marquée « live » après son heure de fin — cas
 * parfaitement banal, personne ne ferme une séance à la seconde près.
 *
 * `UiLiveBadge` et `UiSessionCard` ne rendent le repère que pour la séance
 * déclarée ici. Toutes les autres, quel que soit leur statut en base, affichent
 * leur état temporel ordinaire.
 *
 * CÔTÉ MODÈLE, la garantie porte sur le CANAL, pas sur la plateforme :
 * `event.broadcast_channels` est une ressource réservable et `detect_conflicts()`
 * remonte un conflit `broadcast` quand deux séances se disputent le même canal.
 * Si deux éditions ouvrent chacune le leur, deux directs simultanés redeviennent
 * possibles en base — le point est en arbitrage (voir `docs/PROGRESSION.md`,
 * « Portée de la règle un seul direct »). L'interface, elle, tranche déjà : une
 * seule carte porte le repère.
 *
 * L'état est partagé par `useState()`, donc unique pour toute l'application et
 * transmis du serveur au client sans second rendu.
 */
export function useLiveSession() {
  const liveSessionId = useState<SessionId | null>('live-session', () => null)

  return {
    /** La séance actuellement en direct, ou `null` — l'état ordinaire. */
    liveSessionId,

    /** Cette séance est-elle CELLE qui est en direct ? */
    isLive: (sessionId: SessionId | null | undefined): boolean =>
      sessionId !== null && sessionId !== undefined && liveSessionId.value === sessionId,

    /**
     * Déclare la séance en direct. Toute déclaration remplace la précédente :
     * c'est le seul comportement compatible avec la règle. Passer `null` éteint
     * le repère.
     */
    setLive: (sessionId: SessionId | null): void => {
      liveSessionId.value = sessionId
    },

    /** Éteint le repère si c'est bien cette séance qui le portait. */
    clearLive: (sessionId: SessionId): void => {
      if (liveSessionId.value === sessionId) liveSessionId.value = null
    },
  }
}
