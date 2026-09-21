/**
 * L'état du compte dans l'application — **et ce qu'il devient sans réseau**.
 *
 * ⚠ **À ne pas confondre avec `useGnConnexion`**, qui dit l'état du RÉSEAU.
 * Celui-ci dit l'état du COMPTE. Les deux se lisent ensemble, et c'est
 * précisément leur croisement qui porte la règle la plus facile à trahir.
 *
 * **La règle : hors connexion, l'écran ne se vide pas** (FR-006 ter).
 * Le geste réflexe d'un client web — jeton périmé, on renvoie à la connexion —
 * transforme une salle de négociation sans réseau en application inutilisable.
 * Une reconnexion sans réseau est impossible ; la réclamer est une impasse. On
 * garde donc ce qui a été lu, avec l'heure de sa lecture, et l'on ne demande la
 * reconnexion **qu'au retour du réseau**.
 *
 * Ce composable **enveloppe le store `auth` du site** et n'ouvre aucun second
 * système d'identité : un compte, une session, deux clients. Il **ne nomme aucun
 * cookie** — la session vit dans deux cookies `HttpOnly` que le navigateur
 * renvoie seul.
 */
import type { LoginResult } from '~/types/auth'
import { appareilDeclare } from '~/utils/guide-nego/appareil'
import { ecrireGarde } from '~/utils/guide-nego/garde'
import {
  COMPTE_DECONNECTE,
  etatDuCompte,
  reconnexionAReclamer,
  type EtatDuCompte,
} from '~/utils/guide-nego/compte'

export function useGnSession() {
  const api = useApi()
  const auth = useAuthStore()
  const connexion = useGnConnexion()

  // La lecture passe par la primitive de 0a : garde en IndexedDB, heure de
  // lecture, et **aucune exception** quand le réseau manque.
  const { etat, rafraichir } = useGnLecture<EtatDuCompte>('compte', async () => {
    const moi = await api.auth.session(auth.person?.id ?? null)
    return etatDuCompte(moi)
  })

  const compte = computed<EtatDuCompte>(() => etat.value.valeur ?? COMPTE_DECONNECTE)
  const connectee = computed(() => compte.value.connectee)
  const luA = computed(() => etat.value.luA)
  const pret = computed(() => etat.value.pret)

  /**
   * **Le seul endroit qui décide de renvoyer à la connexion.** Aucun écran ne
   * refait ce calcul : celui qui l'oublierait viderait sa page au premier tunnel.
   */
  const reconnexionDemandee = computed(() =>
    reconnexionAReclamer(etat.value.valeur, connexion.etat.value.enLigne, pret.value),
  )

  /**
   * Relit au retour au premier plan. C'est ce qui fait qu'une personne revenue
   * de son courriel — adresse confirmée sur iPhone, dans une autre session —
   * voit la suite **sans rien avoir à toucher**.
   *
   * Le même mécanisme que 0a emploie au retour du réseau : aucune attente
   * active, aucune interrogation répétée.
   */
  function relireAuRetourAuPremierPlan(): void {
    if (typeof document === 'undefined') return

    const relire = () => {
      if (document.visibilityState === 'visible') void rafraichir()
    }
    document.addEventListener('visibilitychange', relire)
    onScopeDispose(() => document.removeEventListener('visibilitychange', relire))
  }

  async function assurer(): Promise<void> {
    if (!pret.value) await rafraichir()
  }

  /**
   * Se connecter depuis l'application. **L'objet `client` se joint ici**, en un
   * seul endroit : un écran qui l'oublierait ouvrirait une session de douze
   * heures marquée « site », et rien ne le signalerait.
   *
   * Le store du site tient la session — un compte, une session, deux clients.
   */
  async function connecter(adresse: string, motDePasse: string): Promise<LoginResult> {
    const issue = await auth.signIn({
      email: adresse,
      password: motDePasse,
      // L'écran de l'application n'a pas de case à cocher : la durée longue
      // vient du client, pas d'un choix qu'on n'a pas offert.
      remember_me: false,
      client: appareilDeclare(),
    })
    if (issue.status === 'authenticated') await rafraichir()
    return issue
  }

  /**
   * Ferme **cette** session, et elle seule : les autres appareils restent
   * ouverts. L'état gardé est réécrit tout de suite — sans cela, la prochaine
   * ouverture hors connexion afficherait un compte dont on vient de sortir.
   */
  async function deconnecter(): Promise<void> {
    await auth.signOut()
    const maintenant = new Date().toISOString()
    etat.value = {
      ...etat.value,
      valeur: COMPTE_DECONNECTE,
      luA: maintenant,
      source: 'reseau',
      pret: true,
    }
    await ecrireGarde({
      cle: 'compte',
      valeur: COMPTE_DECONNECTE,
      lu_a: maintenant,
      empreinte: null,
    })
  }

  return {
    compte,
    connectee,
    luA,
    pret,
    reconnexionDemandee,
    assurer,
    connecter,
    rafraichir,
    relireAuRetourAuPremierPlan,
    deconnecter,
  }
}
