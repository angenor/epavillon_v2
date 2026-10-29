import type { CoverColors, CoverDate, CoverScene, CoverSpeaker } from '~/utils/cover-studio'

/** Ce que l'atelier sait de l'activité, en texte et en adresses d'images. */
export interface CoverSceneSource {
  title: string
  eyebrow: string
  dateLine: string | null
  placeLine: string | null
  date: CoverDate | null
  room: string | null
  city: string | null
  organizationName: string | null
  speakers: { name: string; imageUrl: string | null }[]
  /** La photo de fond retenue : celle de l'édition ou celle de l'organisation. */
  photoUrl: string | null
  organizationLogoUrl: string | null
  iconUrl: string | null
}

/** Logo juxtaposé OIF et IFDD, version pour fond clair (bandeau de logos). */
const PLATFORM_LOGO = '/logos/svg/oif-ifdd-gris.svg'

/** La police des images, choisie par le commanditaire sur la maquette (29/10) ; Arial là où Helvetica manque. */
const COVER_FONT = "'Helvetica Neue', Helvetica, Arial, sans-serif"

const images = new Map<string, Promise<HTMLImageElement | null>>()

/**
 * Une image chargée une fois par adresse. `crossOrigin` : sans lui, une image
 * dessinée « salit » le canvas et l'image finale ne peut plus être exportée.
 * Les médias sont servis sur l'origine du site (`/media`) ; une image
 * injoignable devient une absence, pas un échec de l'atelier.
 */
function loadImage(url: string | null): Promise<HTMLImageElement | null> {
  if (!url) return Promise.resolve(null)
  const cached = images.get(url)
  if (cached) return cached
  const loading = new Promise<HTMLImageElement | null>((resolve) => {
    const image = new Image()
    image.crossOrigin = 'anonymous'
    image.decoding = 'async'
    image.onload = () => resolve(image)
    image.onerror = () => resolve(null)
    image.src = url
  })
  images.set(url, loading)
  return loading
}

/** Les couleurs de l'image, lues dans les jetons : le canvas ne connaît pas les variables CSS. */
function resolveColors(accentToken: string): CoverColors {
  const style = getComputedStyle(document.documentElement)
  const read = (name: string) => style.getPropertyValue(name).trim()
  return {
    accent: read(accentToken),
    accentLight: read(accentToken.replace(/-\d+$/, '-100')),
    onAccent: read('--ifdd-gris-0'),
    paper: read('--ifdd-gris-0'),
    tint: read('--ifdd-gris-50'),
    rule: read('--ifdd-gris-200'),
    text: read('--ifdd-noir'),
    ink: read('--ifdd-bleu-900'),
    muted: read('--ifdd-gris-700'),
    stripes: [1, 2, 3, 4, 5].map((index) => read(`--color-stripe-${index}`)),
    scrim: read('--color-scrim'),
  }
}

/**
 * LA SCÈNE À DESSINER, recomposée quand la source ou la couleur change. Côté
 * client seulement : ni canvas ni image au rendu serveur.
 */
export function useCoverScene(source: Ref<CoverSceneSource | null>, accentToken: Ref<string>) {
  const scene = shallowRef<CoverScene | null>(null)
  // Le dernier changement gagne : une image lente d'un choix précédent ne doit pas le recouvrir.
  let generation = 0

  watch(
    [source, accentToken],
    async ([current, token]) => {
      const mine = ++generation
      if (!import.meta.client || !current) {
        scene.value = null
        return
      }
      const [photo, platformLogo, organizationLogo, icon, ...faces] = await Promise.all([
        loadImage(current.photoUrl),
        loadImage(PLATFORM_LOGO),
        loadImage(current.organizationLogoUrl),
        loadImage(current.iconUrl),
        ...current.speakers.map((speaker) => loadImage(speaker.imageUrl)),
      ])
      if (mine !== generation) return
      const speakers: CoverSpeaker[] = current.speakers.map((speaker, index) => ({
        name: speaker.name,
        image: faces[index] ?? null,
      }))
      scene.value = {
        title: current.title,
        eyebrow: current.eyebrow,
        dateLine: current.dateLine,
        placeLine: current.placeLine,
        date: current.date,
        room: current.room,
        city: current.city,
        organizationName: current.organizationName,
        speakers,
        photo,
        platformLogo,
        organizationLogo,
        icon,
        colors: resolveColors(token),
        fontFamily: COVER_FONT,
      }
    },
    { immediate: true, deep: true },
  )

  return scene
}
