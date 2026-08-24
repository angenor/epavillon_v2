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
 *
 * ── UNE ÉCRITURE SIMULÉE N'EST PAS UNE LECTURE SIMULÉE ──────────────────────
 *
 * Une lecture d'exemple affiche des données fausses ; une ÉCRITURE d'exemple ne
 * conserve rien du tout. L'écran répond « c'est fait », l'ordre change sous les
 * yeux, et la modification disparaît au rechargement. C'est arrivé sur la
 * vitrine, où retirer une épingle du panneau semblait sans effet : personne ne
 * pouvait le deviner du bandeau, qui ne parlait que des données lues. Les
 * chemins d'écriture sont donc suivis à part, et le bandeau le dit franchement.
 */
export function useMockData() {
  const paths = useState<string[]>('api:mock-paths', () => [])
  const writes = useState<string[]>('api:mock-writes', () => [])

  return {
    paths: readonly(paths),
    /** Vrai dès qu'un bloc de l'écran courant vient des données simulées. */
    active: computed(() => paths.value.length > 0),
    /** Vrai dès qu'une écriture de l'écran courant n'a été enregistrée nulle part. */
    hasWrites: computed(() => writes.value.length > 0),
    mark(path: string, kind: 'read' | 'write' = 'read') {
      if (!paths.value.includes(path)) paths.value = [...paths.value, path]
      if (kind === 'write' && !writes.value.includes(path)) writes.value = [...writes.value, path]
    },
    reset() {
      if (paths.value.length > 0) paths.value = []
      if (writes.value.length > 0) writes.value = []
    },
  }
}
