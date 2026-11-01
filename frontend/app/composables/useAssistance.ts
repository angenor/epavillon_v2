import type { ProposalId } from '~/types/shared'

/**
 * La fenêtre d'assistance, posée par le gabarit sur chaque écran, et ce que
 * l'écran ouvert lui dit : le dossier qu'il montre (un nouveau sujet le
 * propose d'office), la place du bouton parmi les siens, et l'ouverture
 * demandée par un lien (`?vue=exchanges`). `revision` avance quand un sujet
 * change l'état d'un dossier (demande de correction résolue) : la page la suit.
 */
export function useAssistance() {
  const open = useState('assistance-open', () => false)
  const focusProposalId = useState<ProposalId | null>('assistance-focus', () => null)
  const stack = useState('assistance-stack', () => 0)
  const revision = useState('assistance-revision', () => 0)

  return {
    open,
    focusProposalId,
    stack,
    revision,
    /** Le temps de l'écran : à son départ, la fenêtre reprend sa place et oublie le dossier. */
    attach(options: { proposalId?: () => ProposalId | null; stack?: number }): void {
      if (import.meta.server) return
      const place = options.stack ?? 0
      let focus: ProposalId | null = null
      watch(
        () => options.proposalId?.() ?? null,
        (id) => {
          focus = id
          focusProposalId.value = id
        },
        { immediate: true },
      )
      stack.value = place
      // L'écran suivant a pu se poser avant le départ de celui-ci : on ne défait que sa propre trace.
      onBeforeUnmount(() => {
        if (focusProposalId.value === focus) focusProposalId.value = null
        if (stack.value === place) stack.value = 0
      })
    },
  }
}
