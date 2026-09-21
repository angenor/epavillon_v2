/**
 * Guide Négo ne change pas la langue du site.
 *
 * `@nuxtjs/i18n` réécrit son cookie de langue à chaque route qu'il résout, sans
 * option pour l'en empêcher : ouvrir Guide Négo — toujours en français — ferait
 * passer en français le site d'une personne qui le lit en anglais. On relève donc
 * le cookie AVANT le module (`enforce: 'pre'`) et on le rétablit après lui, sur les
 * seules routes de Guide Négo.
 */
const COOKIE = 'epavillon_locale'
const UN_AN = 60 * 60 * 24 * 365

function lire(): string | null {
  const trouve = document.cookie.split('; ').find((ligne) => ligne.startsWith(`${COOKIE}=`))
  return trouve ? decodeURIComponent(trouve.slice(COOKIE.length + 1)) : null
}

function retablir(valeur: string | null): void {
  if (lire() === valeur) return
  document.cookie =
    valeur === null
      ? `${COOKIE}=; Path=/; Max-Age=0; SameSite=Lax`
      : `${COOKIE}=${encodeURIComponent(valeur)}; Path=/; Max-Age=${UN_AN}; SameSite=Lax`
}

export default defineNuxtPlugin({
  name: 'guide-nego:langue',
  enforce: 'pre',
  setup(nuxtApp) {
    const estGuideNego = (nom: unknown) => typeof nom === 'string' && /^guide-nego(-|$)/.test(nom)
    const releve = lire()

    nuxtApp.hook('page:finish', () => {
      if (estGuideNego(useRoute().name)) retablir(releve)
    })
  },
})
