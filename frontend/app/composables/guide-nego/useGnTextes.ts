/**
 * Les textes qui engagent, **gardés un par un** : une fois lu, un texte se relit
 * sans réseau, avec l'heure de sa lecture (FR-033).
 */
import type { LegalText, LegalTextKey } from '~/types/platform'

export function useGnTexte(cle: LegalTextKey) {
  const api = useApi()
  const { etat, rafraichir } = useGnLecture<LegalText>(`texte-${cle}`, () => api.guideNego.texte(cle))

  const texte = computed(() => etat.value.valeur)
  const pret = computed(() => etat.value.pret)
  const luA = computed(() => etat.value.luA)
  /** Lu une fois au moins : sépare « pas encore reçu » de « jamais reçu ». */
  const lu = computed(() => etat.value.source !== 'aucune')

  return { texte, pret, luA, lu, rafraichir }
}
