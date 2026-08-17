/**
 * Délai avant de pouvoir redemander un courriel.
 *
 * POURQUOI SOIXANTE SECONDES, ET PAS ZÉRO. Un bouton « Renvoyer » toujours actif
 * est cliqué trois fois de suite par quelqu'un qui ne voit rien arriver ; la
 * personne reçoit alors trois courriels dont deux jetons déjà périmés — et le
 * dernier reçu n'est pas forcément le dernier émis. Le rebours dit à la fois
 * « c'est parti » et « laisse au courriel le temps d'arriver ».
 *
 * POURQUOI PAS UNE SIMPLE DÉSACTIVATION. Un bouton grisé sans explication se lit
 * comme une panne. Le compte à rebours est ÉCRIT dans le libellé — « Renvoyer le
 * lien (43 s) » — et annoncé poliment aux lecteurs d'écran par l'écran appelant.
 *
 * CÔTÉ CLIENT SEULEMENT : `setInterval` n'a pas de sens au rendu serveur, et le
 * rebours ne démarre qu'après un clic, donc jamais avant l'hydratation.
 */

export interface ResendCountdown {
  /** Secondes restantes ; `0` quand le renvoi est de nouveau possible. */
  remaining: Readonly<Ref<number>>
  canResend: ComputedRef<boolean>
  /** Démarre ou redémarre le rebours. */
  start: () => void
  /** Remet à zéro — utile quand l'envoi a échoué : inutile de faire attendre. */
  reset: () => void
}

export function useResendCountdown(seconds = 60): ResendCountdown {
  const remaining = ref(0)
  let timer: ReturnType<typeof setInterval> | null = null

  function stop(): void {
    if (timer !== null) {
      clearInterval(timer)
      timer = null
    }
  }

  function start(): void {
    if (!import.meta.client) return
    stop()
    remaining.value = seconds
    timer = setInterval(() => {
      remaining.value -= 1
      if (remaining.value <= 0) stop()
    }, 1000)
  }

  function reset(): void {
    stop()
    remaining.value = 0
  }

  // Un intervalle qui survit à la page continue de tourner pour rien et retient
  // le composant en mémoire.
  onScopeDispose(stop)

  return {
    remaining: readonly(remaining),
    canResend: computed(() => remaining.value <= 0),
    start,
    reset,
  }
}
