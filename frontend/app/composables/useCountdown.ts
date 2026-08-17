/**
 * Compte à rebours jusqu'à une échéance — l'échéance d'un appel à propositions.
 *
 * POURQUOI UN REBOURS PLUTÔT QU'UNE DATE SEULE. « Clôture le 30 septembre » se
 * lit sans s'évaluer : il faut compter les jours de tête, et on s'y trompe.
 * « Il reste 6 jours » se comprend d'un coup, et c'est ce chiffre-là qui décide
 * une organisation à déposer aujourd'hui plutôt que « bientôt ». La date reste
 * affichée à côté : le rebours dit l'urgence, la date dit le fait.
 *
 * DEUX PRÉCAUTIONS QUE CE COMPOSABLE PORTE, ET QUE L'ÉCRAN N'A PAS À REFAIRE :
 *
 *   · LE RENDU SERVEUR NE TICOTE PAS. La valeur est calculée une fois au rendu,
 *     puis rafraîchie côté client seulement. Sans cela, l'heure du serveur et
 *     celle du navigateur diffèrent d'une seconde et Vue signale une
 *     désynchronisation d'hydratation à chaque chargement.
 *   · LA CADENCE SUIT CE QU'ON AFFICHE : une fois par minute tant qu'il reste
 *     plus d'une heure, une fois par seconde dans la dernière heure. Un
 *     `setInterval` d'une seconde qui tourne pendant six semaines pour animer un
 *     nombre de jours ne sert personne.
 */

import type { DateInput } from '~/utils/datetime'

export interface CountdownParts {
  /** L'échéance est-elle déjà passée ? */
  expired: boolean
  /** Millisecondes restantes, plancher à zéro. */
  totalMs: number
  days: number
  hours: number
  minutes: number
  seconds: number
  /** Reste-t-il moins de 48 h ? L'encart passe alors en avertissement. */
  imminent: boolean
}

const HOUR_MS = 3_600_000

function partsFrom(targetMs: number, nowMs: number): CountdownParts {
  const totalMs = Math.max(0, targetMs - nowMs)
  const totalSeconds = Math.floor(totalMs / 1000)
  return {
    expired: totalMs <= 0,
    totalMs,
    days: Math.floor(totalSeconds / 86_400),
    hours: Math.floor(totalSeconds / 3_600) % 24,
    minutes: Math.floor(totalSeconds / 60) % 60,
    seconds: totalSeconds % 60,
    imminent: totalMs > 0 && totalMs <= 48 * HOUR_MS,
  }
}

/**
 * @param target instant d'échéance ; `null` tant qu'il n'est pas connu — la
 *               valeur rendue est alors `null`, et l'écran n'affiche rien.
 */
export function useCountdown(target: MaybeRefOrGetter<DateInput | null | undefined>) {
  const now = ref(Date.now())
  let timer: ReturnType<typeof setTimeout> | null = null

  const targetMs = computed(() => {
    const date = toDate(toValue(target))
    return date ? date.getTime() : null
  })

  const parts = computed<CountdownParts | null>(() =>
    targetMs.value === null ? null : partsFrom(targetMs.value, now.value),
  )

  function stop(): void {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }

  /** Se replanifie à chaque tour : la cadence dépend du temps restant. */
  function schedule(): void {
    stop()
    if (targetMs.value === null) return
    const remaining = targetMs.value - now.value
    if (remaining <= 0) return
    timer = setTimeout(() => {
      now.value = Date.now()
      schedule()
    }, remaining > HOUR_MS ? 60_000 : 1_000)
  }

  onMounted(() => {
    now.value = Date.now()
    schedule()
  })
  watch(targetMs, () => schedule())
  onScopeDispose(stop)

  return parts
}
