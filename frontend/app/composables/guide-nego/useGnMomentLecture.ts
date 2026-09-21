import { momentDeLecture } from '~/utils/guide-nego/connexion'

/**
 * « à 14:05 », « hier à 23:10 », « le 11 nov. à 23:10 » — l'heure du TÉLÉPHONE, sans
 * fuseau (écart 32) : elle se juge contre l'horloge affichée au-dessus. Seules les
 * heures d'événement portent le leur.
 */
export function useGnMomentLecture() {
  const { t, locale } = useI18n()

  function momentLisible(luA: string | null): string | null {
    if (!luA) return null
    const moment = momentDeLecture(luA, new Date(), locale.value)
    return t(`gn-connexion.moment.${moment.quand}`, {
      heure: moment.heure,
      jour: moment.quand === 'avant' ? moment.jour : '',
    })
  }

  return { momentLisible }
}
