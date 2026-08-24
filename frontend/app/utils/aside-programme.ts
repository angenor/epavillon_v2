import type { EventId, IsoDateTime, TimeZoneName } from '~/types/shared'
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { DateInput } from '~/utils/datetime'

/**
 * LE PANNEAU « À VENIR », SA PART DE LOGIQUE PURE.
 *
 * Grouper les séances par journée, dire de combien de jours un rendez-vous est
 * encore éloigné, retrouver la prochaine séance d'une édition. Trois gestes sans
 * DOM, écrits une fois — même raisonnement que `utils/edition-history.ts`.
 *
 * ── LE JOUR EST CELUI DE LA SÉANCE, PAS CELUI DU VISITEUR ───────────────────
 *
 * Le panneau mêle les éditions : une séance de Belém et un webinaire à l'heure
 * de Montréal s'y suivent. Grouper sur l'horloge du navigateur ferait basculer
 * une séance du 17 au 18 novembre selon l'endroit d'où on regarde la page. La
 * clé de journée se calcule donc dans le fuseau de la SÉANCE, et « aujourd'hui »
 * se compare dans ce même fuseau.
 *
 * ── « MAINTENANT » VIENT DE LA RÉPONSE, PAS DE LA MACHINE ───────────────────
 *
 * Toutes ces fonctions reçoivent l'instant en paramètre — `HomeScreen.generated_at`,
 * l'horloge qui fait autorité. Lire `Date.now()` ici donnerait deux résultats
 * différents au rendu serveur et à l'hydratation, et l'écran se redessinerait
 * sous les yeux du visiteur pour cause de décalage d'une seconde.
 */

/** Une journée de la frise : son en-tête, et ce qui s'y tient. */
export interface ProgrammeDay {
  /** Clé de journée civile, `AAAA-MM-JJ`, dans le fuseau de la première séance. */
  key: string
  /** Début de la première séance du jour — ce qui date l'en-tête. */
  startsAt: IsoDateTime
  /** Fuseau qui a servi à dater la journée : l'en-tête ne ment pas sur son heure. */
  timezone: TimeZoneName
  /** `0` pour aujourd'hui, `1` pour demain, au-delà pour le reste. */
  daysAhead: number
  sessions: PublicScheduleRow[]
}

/**
 * Écart en JOURNÉES CIVILES entre deux instants, vu depuis un fuseau donné.
 *
 * Ni une division du nombre de millisecondes, ni un arrondi : une séance qui
 * commence dans vingt heures peut être « demain » comme « aujourd'hui », selon
 * l'heure qu'il est. Ce sont les jours du calendrier qui comptent, et c'est ce
 * que le lecteur attend d'un « dans 3 jours ».
 */
export function daysBetweenInZone(
  from: DateInput,
  to: DateInput,
  timeZone: TimeZoneName,
): number {
  const start = Date.parse(`${dayKeyInZone(from, timeZone)}T00:00:00Z`)
  const end = Date.parse(`${dayKeyInZone(to, timeZone)}T00:00:00Z`)
  if (Number.isNaN(start) || Number.isNaN(end)) return 0
  return Math.round((end - start) / 86_400_000)
}

/**
 * Les séances groupées par journée, dans l'ordre du temps.
 *
 * RIEN N'EST FILTRÉ : l'API rend déjà des séances à venir ou en cours, annulations
 * exclues, et bornées. Rejouer ce filtre ici, c'est se donner deux endroits où
 * la règle peut diverger.
 */
export function groupProgrammeDays(
  sessions: PublicScheduleRow[],
  now: DateInput,
): ProgrammeDay[] {
  const days = new Map<string, ProgrammeDay>()

  for (const session of [...sessions].sort((a, b) => a.starts_at.localeCompare(b.starts_at))) {
    const key = dayKeyInZone(session.starts_at, session.timezone)
    const day = days.get(key)
    if (day) {
      day.sessions.push(session)
      continue
    }
    days.set(key, {
      key,
      startsAt: session.starts_at,
      timezone: session.timezone,
      daysAhead: daysBetweenInZone(now, session.starts_at, session.timezone),
      sessions: [session],
    })
  }

  return [...days.values()]
}

/**
 * L'édition commune à toutes les séances, s'il y en a une.
 *
 * Le bloc s'intitule « Au programme — COP31 » quand la frise ne montre qu'une
 * édition, et « Au programme » quand elle en mêle plusieurs — auquel cas c'est
 * chaque carte qui porte le nom de la sienne. Nommer une édition au-dessus de
 * séances qui viennent de trois éditions différentes serait faux.
 */
export function commonEditionId(sessions: PublicScheduleRow[]): EventId | null {
  const first = sessions[0]
  if (!first) return null
  return sessions.every((session) => session.event_id === first.event_id) ? first.event_id : null
}

/**
 * La prochaine séance d'une édition donnée, parmi celles déjà en main.
 *
 * `null` est un cas ordinaire : une édition annoncée dont le programme n'est pas
 * publié n'a aucune séance, et une édition lointaine n'apparaît pas dans les
 * quelques séances que l'accueil reçoit.
 */
export function nextSessionOfEdition(
  sessions: PublicScheduleRow[],
  eventId: EventId,
): PublicScheduleRow | null {
  return (
    [...sessions]
      .filter((session) => session.event_id === eventId)
      .sort((a, b) => a.starts_at.localeCompare(b.starts_at))[0] ?? null
  )
}

/**
 * Le nombre de journées que couvre une édition, bornes comprises.
 *
 * Bornes COMPRISES : une édition qui ouvre le 9 et ferme le 20 dure douze jours,
 * pas onze. C'est le décompte qu'annonce le pavillon, et la soustraction nue
 * donne l'autre.
 */
export function editionDayCount(edition: PublicEditionRow): number {
  return daysBetweenInZone(edition.starts_at, edition.ends_at, edition.timezone) + 1
}
