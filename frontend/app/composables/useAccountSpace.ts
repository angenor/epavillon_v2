import type { WorkspaceOverview } from '~/types/organization-workspace'

/**
 * Ce que le menu du compte montre de l'espace organisation : la pastille, ce qui
 * attend la personne, ses dossiers. La même lecture que `/mon-organisation`, sur
 * la première organisation au nom de laquelle elle peut déposer.
 *
 * Côté client seulement, et relue à chaque ouverture du menu : la pastille ne
 * doit pas retarder la page, et une correction envoyée doit la faire tomber.
 */
export function useAccountSpace() {
  const api = useApi()
  const auth = useAuthStore()
  const memberships = useMembershipStore()

  const organization = computed(() => memberships.submittable[0]?.organization ?? null)

  const { data: overview, refresh } = useAsyncData<WorkspaceOverview | null>(
    'account-space',
    async () => {
      const organizationId = organization.value?.id
      const personId = auth.person?.id
      if (!organizationId || !personId) return null
      return api.workspace.overview(organizationId, personId)
    },
    { server: false, lazy: true, default: () => null, watch: [() => organization.value?.id, () => auth.person?.id] },
  )

  return { organization, overview, refresh }
}
