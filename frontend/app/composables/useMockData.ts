/**
 * Ce que l'écran courant n'a PAS reçu de l'API.
 *
 * Trois écrans du jalon lisent encore des données simulées alors même que l'API
 * est configurée : les messages d'incident, l'accueil public et sa vitrine
 * administrable. Leurs données existent en base — schémas `live` et `content` —
 * mais aucun crate Rust ne les sert à ce jour.
 *
 * ILS LE DISENT. Servir des exemples sans le signaler ferait prendre une
 * programmation fictive pour la vraie, et c'est le genre de méprise qui se
 * découvre en réunion. Un bandeau l'annonce, et nomme les routes attendues :
 * la dette est visible sur l'écran de la personne qui l'utilise, pas seulement
 * dans un fichier de suivi.
 *
 * LA LISTE SE VIDE À CHAQUE NAVIGATION (`middleware/mock-data.global.ts`) : elle
 * décrit l'écran affiché, pas l'historique de la session.
 */
export function useMockData() {
  const paths = useState<string[]>('api:mock-paths', () => [])

  return {
    paths: readonly(paths),
    /** Vrai dès qu'un bloc de l'écran courant vient des données simulées. */
    active: computed(() => paths.value.length > 0),
    mark(path: string) {
      if (!paths.value.includes(path)) paths.value = [...paths.value, path]
    },
    reset() {
      if (paths.value.length > 0) paths.value = []
    },
  }
}
