import type { SpaceVisitPayload } from '~/types/admin-organizations'

/**
 * Déclare l'ouverture d'une page de l'espace d'une organisation, une fois par
 * cible et par affichage, côté navigateur seulement. Envoyée avant que les
 * données de la page arrivent : une page qui échoue à charger compte aussi
 * comme visitée, c'est précisément le cas à vérifier.
 */
export function useSpaceVisit(target: () => SpaceVisitPayload | null): void {
  const api = useApi()
  const auth = useAuthStore()
  const sent = new Set<string>()

  onMounted(() => {
    watch(
      () => (auth.person ? target() : null),
      (payload) => {
        if (!payload) return
        const key = payload.organization_id ?? payload.proposal_id ?? ''
        if (!key || sent.has(key)) return
        sent.add(key)
        api.workspace.recordVisit(payload).catch(() => {})
      },
      { immediate: true },
    )
  })
}
