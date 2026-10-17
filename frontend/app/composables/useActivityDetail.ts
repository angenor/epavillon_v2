/**
 * L'édition et la fiche d'une activité, d'après les slugs de l'adresse. Partagé
 * par la page de l'activité et sa salle Zoom : même clé, donc un seul appel quand
 * on passe de l'une à l'autre.
 */
export function useActivityDetail() {
  const route = useRoute()
  const api = useApi()
  const editionSlug = computed(() => String(route.params.edition))
  const sessionSlug = computed(() => String(route.params.session))

  return useAsyncData(
    () => `activite-${editionSlug.value}-${sessionSlug.value}`,
    async () => {
      const edition = (await api.events.publicList()).find((entry) => entry.slug === editionSlug.value)
      if (!edition) return null
      const detail = await api.pavillon.activite(edition.id, sessionSlug.value)
      return detail ? { edition, detail } : null
    },
    { lazy: true },
  )
}
