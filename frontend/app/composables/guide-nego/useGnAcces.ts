/**
 * L'état d'accès dans l'application — **une lecture, et elle sert trois écrans**.
 *
 * Le parcours d'entrée, le verrou d'un module réservé et « Mon accès » lisent
 * `GET /negotiation/me/access` et rien d'autre. Chacun y trouve tout : le mode
 * d'admission courant, l'état dérivé du RBAC, ce que l'accès ouvre, les
 * appartenances de réseau et la dernière demande.
 *
 * ⚠ **À ne pas confondre avec `useGnSession`**, qui dit l'état du COMPTE, ni
 * avec `useGnConnexion`, qui dit l'état du RÉSEAU. Trois choses distinctes :
 * être connectée, avoir du réseau, et avoir l'accès.
 *
 * **Hors connexion, l'état lu s'affiche avec l'heure de sa lecture** (FR-034,
 * principe XI), et la saisie d'un code se refuse en le disant (FR-019) : un
 * accès ne se met pas en file, on ne peut pas l'annoncer avant de l'avoir
 * obtenu.
 */
import type { AccessStateView, RedeemResult } from '~/types/negotiation'
import {
  ACCES_VISITEUSE,
  accesOuvert,
  etatDAcces,
  saisiePossible,
} from '~/utils/guide-nego/acces'

export function useGnAcces() {
  const api = useApi()
  const connexion = useGnConnexion()

  const { etat, rafraichir } = useGnLecture<AccessStateView>('acces', async () => {
    const lu = await api.guideNego.acces()
    return etatDAcces(lu)
  })

  const acces = computed<AccessStateView>(() => etat.value.valeur ?? ACCES_VISITEUSE)
  const ouvert = computed(() => accesOuvert(etat.value.valeur))
  const mode = computed(() => acces.value.admission_mode)
  const reseaux = computed(() => acces.value.networks)
  const demande = computed(() => acces.value.request)
  const luA = computed(() => etat.value.luA)
  const pret = computed(() => etat.value.pret)

  /** La saisie exige le réseau — et le mode doit l'accepter. */
  const saisieDeCodePossible = computed(() =>
    saisiePossible(etat.value.valeur, connexion.etat.value.enLigne),
  )

  async function assurer(): Promise<void> {
    if (!pret.value) await rafraichir()
  }

  /**
   * Saisir le code. **Le résultat est rendu tel quel à l'écran** : les neuf
   * issues sortent en 200, et leur message est composé par l'API.
   *
   * L'état est relu après une issue qui a pu le changer — un accès ouvert, une
   * appartenance gagnée, une demande ouverte —, pour que l'écran suivant
   * n'attende pas la prochaine visite pour s'en apercevoir.
   */
  async function saisirLeCode(code: string): Promise<RedeemResult> {
    const resultat = await api.guideNego.saisirLeCode(code)

    if (
      resultat.issue === 'accepted' ||
      resultat.issue === 'already_granted' ||
      resultat.issue === 'pending_approval'
    ) {
      await rafraichir()
    }

    return resultat
  }

  /**
   * Relit au retour au premier plan : un accès accordé ou retiré depuis le
   * back-office se voit à la réouverture, sans que rien ne soit à toucher
   * (FR-041).
   */
  function relireAuRetourAuPremierPlan(): void {
    if (typeof document === 'undefined') return

    const relire = () => {
      if (document.visibilityState === 'visible') void rafraichir()
    }
    document.addEventListener('visibilitychange', relire)
    onScopeDispose(() => document.removeEventListener('visibilitychange', relire))
  }

  return {
    acces,
    ouvert,
    mode,
    reseaux,
    demande,
    luA,
    pret,
    saisieDeCodePossible,
    assurer,
    rafraichir,
    saisirLeCode,
    relireAuRetourAuPremierPlan,
  }
}
