/**
 * Partager l'adresse de la page : la feuille de partage du téléphone, à défaut
 * le presse-papiers, et `copied` le dit deux secondes et demie.
 */
export function useShareLink(title: () => string) {
  const requestUrl = useRequestURL()
  const copied = ref(false)

  async function share(): Promise<void> {
    const url = requestUrl.href
    if (navigator.share) {
      await navigator.share({ title: title(), url }).catch(() => undefined)
      return
    }
    await navigator.clipboard.writeText(url)
    copied.value = true
    setTimeout(() => (copied.value = false), 2500)
  }

  return { copied, share, url: requestUrl }
}
