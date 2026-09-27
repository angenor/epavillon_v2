/**
 * Proposer un terme au lexique (récit 7), ouvert à tout compte. La proposition part par
 * la file de 0c sous une référence choisie ici : sans réseau, elle part au retour, une
 * fois (R8). Sans compte, la saisie est gardée le temps de se connecter.
 */
import type { ProposalInput } from '~/types/negotiation-savoir'
import { retourAGarder } from '~/utils/guide-nego/parcours'
import { saisieAGarder, saisieALire, type SaisieDeTerme } from '~/utils/guide-nego/proposition'
import { CLE_RETOUR_APRES_CONNEXION, CLE_TERME_PROPOSE, lireCle, poserCle } from '~/utils/guide-nego/stockage'

const PREFIXE = 'savoir.terme.'

export function useGnTermesProposes() {
  const api = useApi().guideNegoSavoir
  const session = useGnSession()
  const file = useGnFile()

  file.inscrireFamille(PREFIXE, (intention) => api.proposerUnTerme(intention.corps as ProposalInput))

  /** Rend `false` sans compte : rien n'est parti. */
  async function proposer(saisie: SaisieDeTerme): Promise<boolean> {
    if (!session.connectee.value) return false
    const entree: ProposalInput = {
      client_ref: crypto.randomUUID(),
      term: saisie.terme.trim(),
      context: saisie.contexte.trim() || null,
    }
    await file.poser(`${PREFIXE}${entree.client_ref}`, entree, null)
    await file.partir()
    return true
  }

  function garder(chemin: string, saisie: SaisieDeTerme): void {
    const maintenant = Date.now()
    poserCle(CLE_TERME_PROPOSE, saisieAGarder(chemin, saisie, maintenant))
    poserCle(CLE_RETOUR_APRES_CONNEXION, retourAGarder(chemin, maintenant))
  }

  /** La saisie gardée pour cet écran, une fois connecté ; elle ne sert qu'une fois. */
  async function reprendre(chemin: string): Promise<SaisieDeTerme | null> {
    const saisie = saisieALire(lireCle(CLE_TERME_PROPOSE), chemin, Date.now())
    if (!saisie) return null
    await session.assurer()
    if (!session.connectee.value) return null
    poserCle(CLE_TERME_PROPOSE, '')
    return saisie
  }

  return { connectee: computed(() => session.connectee.value), proposer, garder, reprendre }
}
