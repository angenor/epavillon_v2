import {
  DRAPEAU_APPLICATION,
  echangesOuverts as resoudreEchanges,
  extraireDrapeaux,
  fusionner,
  resoudreDrapeau,
  type DrapeauxGardes,
} from '~/utils/guide-nego/drapeaux'

/**
 * Les deux drapeaux de Guide Négo, lus par l'API et gardés sur le téléphone.
 *
 * Le verdict vaut AUSSITÔT d'après la garde ; la réponse de l'API s'applique à son
 * arrivée, dans les deux sens. Seule la toute première ouverture attend le réseau.
 */
export function useGnDrapeaux() {
  const api = useApi()

  const { etat, rafraichir } = useGnLecture<DrapeauxGardes>('drapeaux', async (garde) => {
    const reponse = extraireDrapeaux(await api.platform.featureFlags())
    if (reponse === null) throw new Error('réponse illisible')
    return fusionner(reponse, garde)
  })

  const ouverte = computed(() => resoudreDrapeau(null, etat.value.valeur, DRAPEAU_APPLICATION))
  const echangesOuverts = computed(() => resoudreEchanges(null, etat.value.valeur))
  const pret = computed(() => etat.value.pret)

  /** Rend la main dès qu'un verdict existe — la garde, ou à défaut le réseau. */
  async function assurer(): Promise<void> {
    if (pret.value) {
      void rafraichir()
      return
    }
    const lecture = rafraichir()
    await new Promise<void>((resolve) => {
      const arret = watch(pret, (estPret) => estPret && (arret(), resolve()), { immediate: true, flush: 'sync' })
    })
    void lecture
  }

  return { ouverte, echangesOuverts, pret, assurer, rafraichir }
}
