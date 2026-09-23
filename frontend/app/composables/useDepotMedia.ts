import type { UploadPayload, UploadedAsset } from '~/types/media'

/**
 * Le dépôt d'un fichier vers `POST /media/assets`, partagé par `MediaImageField`
 * et `MediaFileField` : le type contrôlé avant l'envoi, l'envoi, l'état et
 * l'erreur. Un seul geste, un seul comportement.
 *
 * `motifs` suit la règle du rôle (`allowed_mime_prefixes`), où `*` vaut
 * « n'importe quoi » comme en base ; sans règle, `repli` s'applique.
 */
export function useDepotMedia(options: {
  motifs: MaybeRefOrGetter<readonly string[]>
  repli: string
}) {
  const { t } = useI18n()
  const api = useApi()

  const enCours = ref(false)
  const echec = ref<string | null>(null)

  const motifs = computed<readonly string[]>(() => {
    const declares = toValue(options.motifs)
    return declares.length > 0 ? declares : [options.repli]
  })

  /** La valeur de l'attribut `accept` du champ de fichier. */
  const accept = computed(() => motifs.value.join(','))

  function accepte(mime: string): boolean {
    return motifs.value.some((motif) => {
      const [tete, queue] = motif.split('*')
      if (queue === undefined) return mime === motif
      return mime.startsWith(tete ?? '') && mime.endsWith(queue)
    })
  }

  /** Rend l'objet déposé, ou `null` : le motif du refus est alors dans `echec`. */
  async function deposer(payload: UploadPayload): Promise<UploadedAsset | null> {
    enCours.value = true
    echec.value = null
    try {
      return await api.media.upload(payload)
    } catch (thrown) {
      echec.value = apiErrorMessage(thrown, t)
      return null
    } finally {
      enCours.value = false
    }
  }

  return { enCours, echec, accept, accepte, deposer }
}
