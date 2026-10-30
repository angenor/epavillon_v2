import type { ActiveIncident } from '~/types/live'
import type { SessionId } from '~/types/shared'

/**
 * Les messages d'incident d'une activité (page et salle) : globaux, de son
 * édition, de sa journée, d'elle-même, de son organisation. Une panne de la
 * lecture ne coûte jamais la page : un bandeau absent est le cas normal.
 */
export function useSessionIncidents(session: MaybeRefOrGetter<{ id: SessionId } | null>) {
  const api = useApi()
  const id = computed(() => toValue(session)?.id ?? null)

  // Signal à écouter ici : `session:<id>` et `edition:<id>:public:incidents`, puis `refresh()`.
  const { data, refresh } = useLazyAsyncData<ActiveIncident[]>(
    () => `activite-incidents-${id.value ?? 'aucune'}`,
    async () => (id.value ? api.incidents.forSession(id.value).catch(() => []) : []),
    { default: () => [], watch: [id] },
  )

  return { incidents: data, refresh }
}
