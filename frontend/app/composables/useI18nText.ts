import type { MaybeRefOrGetter } from 'vue'
import type { I18nText } from '~/types/shared'

/**
 * Résolution d'un texte multilingue de la BASE dans la locale active.
 *
 * Ne pas confondre avec `t()`, qui sert aux libellés d'interface. Ici on affiche
 * une donnée saisie au back-office : titre d'activité, libellé de thématique,
 * nom de salle, intitulé de critère d'évaluation. Recopier l'une de ces valeurs
 * dans un fichier i18n est le défaut exact de la v1.
 */
export function useI18nText() {
  const { locale } = useI18n()

  return {
    /** Le texte dans la locale active, avec repli sur le français. */
    tr: (value: I18nText | null | undefined) => resolveI18nText(value, locale.value),
    /** Version réactive, pour un affichage lié à une donnée qui change. */
    trRef: (value: MaybeRefOrGetter<I18nText | null | undefined>) =>
      computed(() => resolveI18nText(toValue(value), locale.value)),
    /** La traduction existe-t-elle vraiment, ou affiche-t-on le repli français ? */
    hasTranslation: (value: I18nText | null | undefined) =>
      hasI18nTranslation(value, locale.value),
  }
}
