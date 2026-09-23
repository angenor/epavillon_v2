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
import type { AccessRequestView, AccessStateView, RedeemResult } from '~/types/negotiation'
import {
  ACCES_VISITEUSE,
  accesOuvert,
  codeOffertParLeMode,
  demandePossible,
  etatDAcces,
  saisiePossible,
} from '~/utils/guide-nego/acces'
import { relireEtEffacer } from '~/utils/guide-nego/effacements'

export function useGnAcces() {
  const api = useApi()
  const connexion = useGnConnexion()

  // L'accès perdu efface les réservés gardés (FR-034) — sur une réponse seulement.
  const { etat, rafraichir } = useGnLecture<AccessStateView>('acces', () =>
    relireEtEffacer(
      async () => etatDAcces(await api.guideNego.acces()),
      (lu) => !accesOuvert(lu),
      effacerLesCopiesReservees,
    ),
  )

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

  /**
   * **Le mode propose-t-il un code ?** À ne pas confondre avec la ligne
   * au-dessus : celle-ci dit ce qui est faisable maintenant, celle-là ce que le
   * mode offre. Les confondre ferait disparaître le champ à chaque tunnel au
   * lieu de le désactiver en le disant (FR-022, FR-019).
   */
  const codeOffert = computed(() => codeOffertParLeMode(etat.value.valeur))

  /** La demande est-elle proposée ? Elle prend la place du code en « approbation ». */
  const demandePossibleMaintenant = computed(() =>
    demandePossible(etat.value.valeur, connexion.etat.value.enLigne),
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
   * Demander l'accès. L'état est relu ensuite : l'écran d'attente doit montrer
   * la demande qui vient d'être ouverte, pas celle d'avant.
   */
  async function demanderLAcces(message?: string | null): Promise<AccessRequestView> {
    const demandeOuverte = await api.guideNego.demanderLAcces({ message: message || null })
    await rafraichir()
    return demandeOuverte
  }

  /**
   * Retirer sa demande — « **annulée** », son propre fait (FR-026). Ce qui
   * arrive quand on reçoit un code et qu'on entre par lui.
   */
  async function annulerSaDemande(requestId: string): Promise<void> {
    await api.guideNego.annulerSaDemande(requestId)
    await rafraichir()
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
    codeOffert,
    demandePossibleMaintenant,
    assurer,
    rafraichir,
    saisirLeCode,
    demanderLAcces,
    annulerSaDemande,
    relireAuRetourAuPremierPlan,
  }
}
