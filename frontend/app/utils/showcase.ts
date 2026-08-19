import type { AttachedImage } from '~/types/media'
import type { HighlightId } from '~/types/content'
import type { ShowcaseRow } from '~/types/views'

/**
 * LE BANDEAU D'OUVERTURE, SA PART DE LOGIQUE PURE (A15).
 *
 * Tout ce qui se décide sans DOM vit ici : quel fond rendre, combien de temps le
 * laisser, quelle vignette montrer, où aller ensuite. Le composant ne garde que
 * ce qui a besoin du navigateur — poser les minuteries et écouter les
 * préférences de mouvement.
 *
 * ── LE FOND SE DÉCIDE D'APRÈS LE MÉDIA REÇU, JAMAIS D'APRÈS UN DRAPEAU ──────
 *
 * `content.v_showcase` sert `background_video` par le même mécanisme que
 * `background_image` (`media.attached_image()`), et ne sert QUE des objets
 * `ready` : une vidéo encore en traitement arrive `null`. Le front n'a donc
 * aucune colonne « a une vidéo » à consulter — il regarde ce qu'il a reçu et
 * descend l'échelle des replis :
 *
 *     vidéo → image → aplat `background_color_hex` → surface institutionnelle
 *
 * C'est la même échelle que celle du modèle, dans le même ordre, et elle est
 * écrite UNE FOIS.
 *
 * ── UNE VIDÉO QUI NE CHARGE PAS N'EST PAS UNE DIAPOSITIVE PERDUE ────────────
 *
 * La v1 passait à la suivante en cas d'erreur de lecture, et c'était le bon
 * geste : un cadre noir de quinze secondes est pire qu'une diapositive sautée.
 * On y ajoute la mémoire de l'échec (`skipVideo`) — sans elle, un bandeau de
 * deux diapositives dont l'une a une vidéo cassée tournerait indéfiniment sur
 * la même erreur au lieu de se rabattre sur l'image.
 */

// ---------------------------------------------------------------------------
// Durées
// ---------------------------------------------------------------------------

/**
 * Sept secondes pour une diapositive fixe : le temps de lire une citation
 * courte et son attribution, pas celui de s'impatienter.
 */
export const SHOWCASE_STILL_MS = 7_000

/**
 * Quinze secondes quand un fond animé tourne. Une vidéo coupée au bout de sept
 * secondes n'a rien montré ; c'est la seule raison de cette valeur, et elle ne
 * s'applique pas quand la vidéo a été écartée.
 */
export const SHOWCASE_VIDEO_MS = 15_000

// ---------------------------------------------------------------------------
// Le fond
// ---------------------------------------------------------------------------

/**
 * Ce que la diapositive doit peindre derrière son texte. Quatre formes, et le
 * composant en rend exactement une : pas de `v-if` en cascade dans le gabarit,
 * pas de branche oubliée.
 */
export type ShowcaseBackground =
  /** `<video autoplay muted loop playsinline>`, affichée par la vignette. */
  | { kind: 'video'; video: AttachedImage; poster: AttachedImage | null }
  /** `<img>` sur le fond photographique. */
  | { kind: 'image'; image: AttachedImage }
  /** Aplat saisi au back-office — une DONNÉE, pas un jeton de style. */
  | { kind: 'color'; color: string }
  /** Rien de rien : la surface institutionnelle du bandeau reste seule. */
  | { kind: 'none' }

export interface ShowcaseBackgroundOptions {
  /**
   * La vidéo de cette diapositive a déjà échoué : on ne la redemande pas.
   * Le repli devient l'image, puis l'aplat.
   */
  skipVideo?: boolean
}

/** Le fond à peindre, replis compris. */
export function showcaseBackground(
  slide: ShowcaseRow,
  options: ShowcaseBackgroundOptions = {},
): ShowcaseBackground {
  if (!options.skipVideo && slide.background_video) {
    return {
      kind: 'video',
      video: slide.background_video,
      // La vignette sert d'affiche : c'est ce qu'on voit pendant le chargement.
      poster: slide.thumbnail ?? slide.background_image,
    }
  }
  if (slide.background_image) return { kind: 'image', image: slide.background_image }
  if (slide.background_color_hex) return { kind: 'color', color: slide.background_color_hex }
  return { kind: 'none' }
}

/**
 * La vignette du rail : celle qui est prévue pour, à défaut le fond
 * photographique. Sans l'une ni l'autre, le rail retombe sur l'aplat — c'est au
 * composant de le peindre, il n'y a pas d'image à rendre.
 */
export function showcaseThumbnail(slide: ShowcaseRow): AttachedImage | null {
  return slide.thumbnail ?? slide.background_image
}

/** Combien de temps laisser cette diapositive à l'écran. */
export function showcaseDurationMs(
  slide: ShowcaseRow,
  options: ShowcaseBackgroundOptions = {},
): number {
  return showcaseBackground(slide, options).kind === 'video'
    ? SHOWCASE_VIDEO_MS
    : SHOWCASE_STILL_MS
}

// ---------------------------------------------------------------------------
// Le parcours
// ---------------------------------------------------------------------------

/**
 * L'index atteint en avançant de `step` diapositives, en boucle.
 *
 * Le modulo de JavaScript rend un reste négatif pour un opérande négatif : sans
 * le second `+ total`, le bouton « précédent » sur la première diapositive
 * renverrait `-1`. C'est le genre de détail qui ne se voit qu'au clavier.
 */
export function showcaseIndexAfter(index: number, total: number, step: number): number {
  if (total <= 0) return 0
  return (((index + step) % total) + total) % total
}

/** Borne un index dans la liste — une liste qui rétrécit ne laisse pas un index mort. */
export function clampShowcaseIndex(index: number, total: number): number {
  if (total <= 0) return 0
  if (index < 0) return 0
  return index > total - 1 ? total - 1 : index
}

/** La diapositive d'un index, ou `null` — `noUncheckedIndexedAccess` oblige. */
export function showcaseAt(slides: ShowcaseRow[], index: number): ShowcaseRow | null {
  return slides[index] ?? null
}

/** Les identifiants dont la vidéo a échoué, sous une forme que Vue suit. */
export type FailedVideoIds = ReadonlySet<HighlightId>

// ---------------------------------------------------------------------------
// Préférence de mouvement
// ---------------------------------------------------------------------------

/**
 * `prefers-reduced-motion: reduce` — LU EN JAVASCRIPT, ET C'EST INDISPENSABLE.
 *
 * `main.css` neutralise déjà toutes les animations et transitions CSS. Un
 * autodéfilement n'en est pas une : c'est une minuterie, que rien dans la
 * feuille de style ne peut arrêter. Sans cette lecture, une personne qui a
 * demandé « moins d'animations » verrait quand même la page changer sous ses
 * yeux toutes les sept secondes.
 *
 * ELLE ARRÊTE, ELLE N'ACCÉLÈRE PAS. La tentation est de raccourcir la durée ;
 * c'est l'inverse de ce qui est demandé.
 *
 * Rend `false` au rendu serveur, où la préférence est inconnue : le composant
 * ne démarre de toute façon aucune minuterie avant `onMounted`.
 */
export function reducedMotionQuery(): MediaQueryList | null {
  if (typeof window === 'undefined' || typeof window.matchMedia !== 'function') return null
  return window.matchMedia('(prefers-reduced-motion: reduce)')
}

export function prefersReducedMotion(): boolean {
  return reducedMotionQuery()?.matches ?? false
}

/**
 * Suit la préférence et rend la fonction de désabonnement. Elle peut changer
 * pendant la visite — un réglage système modifié dans un autre onglet.
 */
export function onReducedMotionChange(handler: (reduced: boolean) => void): () => void {
  const query = reducedMotionQuery()
  if (!query) return () => {}
  const listener = (event: MediaQueryListEvent): void => handler(event.matches)
  query.addEventListener('change', listener)
  return () => query.removeEventListener('change', listener)
}
