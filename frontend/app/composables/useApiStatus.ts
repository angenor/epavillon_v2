/**
 * L'état de la liaison avec l'API, partagé par toute l'application.
 *
 * MODE DÉGRADÉ. Une API injoignable n'est pas l'erreur d'un écran : c'est un
 * état de la plateforme. Sans cet état partagé, une page qui charge six données
 * afficherait six messages d'erreur identiques, et la personne en conclurait que
 * six choses sont cassées. Un seul bandeau le dit une fois, et chaque bloc garde
 * son état d'erreur pour ce qui le concerne vraiment.
 *
 * La valeur est optimiste au départ : tant qu'aucun appel n'a échoué, l'API est
 * réputée reachable. Un appel qui aboutit rétablit l'état — c'est ce qui fait
 * disparaître le bandeau tout seul quand le service revient.
 */
import type { ApiUnreachableReason } from '~/utils/api-error'

export function useApiStatus() {
  const reachable = useState('api:reachable', () => true)
  const reason = useState<ApiUnreachableReason | null>('api:reason', () => null)

  return {
    reachable: readonly(reachable),
    reason: readonly(reason),
    reportOutage(cause: ApiUnreachableReason) {
      reachable.value = false
      reason.value = cause
    },
    reportRecovery() {
      if (reachable.value) return
      reachable.value = true
      reason.value = null
    },
  }
}

/**
 * Témoin de session — un cookie ORDINAIRE, sans rien de secret dedans.
 *
 * Il ne dit qu'une chose : quelqu'un s'est connecté depuis ce navigateur. Il
 * n'autorise rien, et le falsifier ne donne accès à rien — les deux vrais jetons
 * sont `HttpOnly`, signés, et posés par l'API seule.
 *
 * À QUOI IL SERT. Le jeton d'accès dure un quart d'heure et le rendu serveur ne
 * peut pas le renouveler : le cookie de rafraîchissement est limité au path
 * `/api/auth` et n'atteint donc jamais le serveur Nuxt. Au premier affichage
 * après cette limite, la page se rend déconnectée. Le témoin dit au navigateur
 * qu'une rotate vaut la peine d'être tentée — et, pour un visiteur qui ne
 * s'est jamais connecté, qu'elle n'en vaut pas la peine.
 */
/** Nom du témoin. Écrit à deux endroits — ici pour la lecture, dans le store pour sa durée. */
export const SESSION_WITNESS_COOKIE = 'epavillon_session'

export function useSessionWitness() {
  return useCookie<string | null>(SESSION_WITNESS_COOKIE, {
    default: () => null,
    sameSite: 'lax',
    path: '/',
  })
}
