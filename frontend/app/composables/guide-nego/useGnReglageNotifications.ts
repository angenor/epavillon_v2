/**
 * Les deux réglages de notification : l'accord courriel d'« À propos » et les
 * « Notifications par thématique » du profil (FR-031).
 *
 * Tous deux passent par la file et sont idempotents — le corps porte l'état entier
 * voulu. L'affichage change aussitôt ; la garde `reglage-notifications` le rend à la
 * réouverture sans réseau.
 */
import type { NotificationSettings, NotificationSettingsPayload, ThemeNotificationsPayload } from '~/types/negotiation'
import { ApiRequestError, normalizeApiError } from '~/utils/api-error'
import { ecrireGarde, magasinDesEcritures } from '~/utils/guide-nego/garde'
import {
  CLE_FILE_COURRIEL,
  CLE_FILE_THEMATIQUES_NOTIFIEES,
  CLE_LECTURE_REGLAGE_NOTIFICATIONS,
} from '~/utils/guide-nego/signalements'

interface ReglageLu {
  personne: string
  reglage: NotificationSettings
}

export function useGnReglageNotifications() {
  const api = useApi().negotiationReports
  const session = useGnSession()
  const file = useGnFile()
  const thematiques = useGnThematiques()

  const lecture = useGnLecture<ReglageLu>(CLE_LECTURE_REGLAGE_NOTIFICATIONS, async (garde) => {
    const personne = session.compte.value.id
    if (!personne) throw new Error('sans compte')
    try {
      const reglage = await api.reglageDesNotifications()
      // Une lecture partie avant l'envoi ne défait pas le choix qui attend.
      const enFile = await magasinDesEcritures.lireUne(CLE_FILE_COURRIEL).catch(() => null)
      const voulu = enFile?.personne === personne ? (enFile.corps as NotificationSettingsPayload).email : reglage.email
      return { personne, reglage: { ...reglage, email: voulu } }
    } catch (erreur) {
      // L'API a parlé : ce n'est pas une panne de réseau.
      if (normalizeApiError(erreur) instanceof ApiRequestError) {
        return garde?.personne === personne ? garde : { personne, reglage: { email: true, version: '' } }
      }
      throw erreur
    }
  })

  const lu = computed<ReglageLu | null>(() => {
    const valeur = lecture.etat.value.valeur
    return valeur && valeur.personne === session.compte.value.id ? valeur : null
  })

  file.inscrire(
    CLE_FILE_COURRIEL,
    (intention) => api.reglerLesNotifications((intention.corps as NotificationSettingsPayload).email),
    () => lecture.relire(),
  )
  // L'empreinte des thématiques porte les allumées : la relire évite un `412` au choix suivant.
  file.inscrire(
    CLE_FILE_THEMATIQUES_NOTIFIEES,
    (intention) => api.notifierDesThematiques((intention.corps as ThemeNotificationsPayload).codes),
    () => thematiques.rafraichir(),
  )

  function assurer(): void {
    if (!session.connectee.value) return
    void lecture.rafraichir()
  }

  async function reglerLeCourriel(email: boolean): Promise<void> {
    const personne = session.compte.value.id
    if (!personne) return
    const etat = lecture.etat.value
    const valeur: ReglageLu = { personne, reglage: { email, version: lu.value?.reglage.version ?? '' } }
    lecture.etat.value = { ...etat, valeur, pret: true }
    await ecrireGarde({ cle: CLE_LECTURE_REGLAGE_NOTIFICATIONS, valeur, lu_a: etat.luA ?? new Date().toISOString(), empreinte: null })
    await file.poser(CLE_FILE_COURRIEL, { email } satisfies NotificationSettingsPayload, null)
    await file.partir()
  }

  /** La liste entière des thématiques allumées, parmi les suivies. */
  async function notifierDesThematiques(codes: string[]): Promise<void> {
    const voulues = [...new Set(codes)].filter((c) => thematiques.mesCodes.value.includes(c)).sort()
    await thematiques.appliquerLesNotifiees(voulues)
    await file.poser(CLE_FILE_THEMATIQUES_NOTIFIEES, { codes: voulues } satisfies ThemeNotificationsPayload, null)
    await file.partir()
  }

  return {
    /** Sans accord enregistré : allumé. */
    courriel: computed(() => lu.value?.reglage.email ?? true),
    notifiees: thematiques.notifiees,
    pret: computed(() => lecture.etat.value.pret),
    luA: computed(() => lecture.etat.value.luA),
    assurer,
    reglerLeCourriel,
    notifierDesThematiques,
  }
}
