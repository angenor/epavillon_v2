import {
  apresEchec,
  apresLectureGardee,
  apresReussite,
  connexionInitiale,
  type EtatConnexion,
} from '~/utils/guide-nego/connexion'

/**
 * Un Wi-Fi sans issue dit « en ligne » à tort : le résultat réel des lectures corrige
 * ce que le navigateur annonce.
 */
export function useGnConnexion() {
  const etat = useState<EtatConnexion>('gn-connexion', () => connexionInitiale(navigator.onLine))
  const ecoute = useState('gn-connexion-ecoute', () => false)

  if (!ecoute.value) {
    ecoute.value = true
    window.addEventListener('offline', () => (etat.value = apresEchec(etat.value)))
  }

  return {
    etat: readonly(etat),
    noterReussite: (luA: string) => (etat.value = apresReussite(etat.value, luA)),
    noterEchec: () => (etat.value = apresEchec(etat.value)),
    noterLecture: (luA: string) => (etat.value = apresLectureGardee(etat.value, luA)),
    marquerBandeauVu: () => (etat.value = { ...etat.value, bandeauVu: true }),
  }
}
