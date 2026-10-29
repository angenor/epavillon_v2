import type { CoverDesignOptions } from '~/types/programme/cover-design'

/**
 * LES BRIQUES DE DESSIN DES MODÈLES DE COUVERTURE : la scène, les mesures de
 * texte, les images recadrées, les logos. Un modèle (un fichier de ce dossier)
 * ne dessine qu'avec elles et le contexte 2D ; il n'importe rien d'autre.
 */

export const COVER_WIDTH = 1600
export const COVER_HEIGHT = 900

export interface CoverColors {
  accent: string
  /** La teinte 100 de l'accent : fond clair, lumières d'un duotone. */
  accentLight: string
  onAccent: string
  paper: string
  /** `--ifdd-gris-50`, papier légèrement chaud. */
  tint: string
  /** `--ifdd-gris-200`, filets et bordures. */
  rule: string
  /** `--ifdd-noir`, le texte courant sur papier. */
  text: string
  ink: string
  muted: string
  /** Les cinq couleurs du logo, dans l'ordre du liseré du site. */
  stripes: string[]
  /** `--color-scrim`, opaque : le voile en dose l'opacité. */
  scrim: string
}

export interface CoverSpeaker {
  name: string
  image: CanvasImageSource | null
}

/** La date de la première séance (ou le début de l'édition), en morceaux, dans le fuseau de l'édition. */
export interface CoverDate {
  /** « 10 » */
  day: string
  /** « novembre » */
  month: string
  /** « nov. » */
  monthShort: string
  /** « mardi » */
  weekday: string
  /** « 2026 » */
  year: string
  /** « 14:00 », nulle tant que l'activité n'est pas programmée. */
  time: string | null
  /** « heure de Belém » */
  zone: string
}

export interface CoverScene {
  title: string
  eyebrow: string
  /** « 10 novembre 2026 · 14:00, heure de Belém » */
  dateLine: string | null
  /** « Stand du pavillon, Belém » */
  placeLine: string | null
  date: CoverDate | null
  /** Salle seule, nulle tant que l'activité n'est pas programmée. */
  room: string | null
  city: string | null
  organizationName: string | null
  speakers: CoverSpeaker[]
  /** La photo de fond : celle de l'édition par défaut, ou celle de l'organisation. */
  photo: CanvasImageSource | null
  platformLogo: CanvasImageSource | null
  organizationLogo: CanvasImageSource | null
  icon: CanvasImageSource | null
  colors: CoverColors
  fontFamily: string
}

/** Les couleurs proposées : des teintes de la charte assez sombres pour porter du texte blanc. */
export const COVER_ACCENTS = {
  bleu: '--ifdd-bleu-900',
  cyan: '--ifdd-cyan-700',
  vert: '--ifdd-vert-700',
  violet: '--ifdd-violet-700',
  rouge: '--ifdd-rouge-700',
} as const

export type CoverAccent = keyof typeof COVER_ACCENTS

export const DEFAULT_COVER_OPTIONS: CoverDesignOptions = {
  show_date: true,
  show_speakers: true,
  show_logos: true,
  show_event_banner: true,
  show_event_icon: true,
}

/** Mesure seule : permet de tester le découpage du texte sans navigateur. */
export interface TextMeasurer {
  font: string
  measureText(text: string): { width: number }
}

/** Coupe un texte en lignes qui tiennent dans `maxWidth` ; un mot trop long reste entier sur sa ligne. */
export function wrapLines(ctx: TextMeasurer, text: string, maxWidth: number): string[] {
  const words = text.trim().split(/\s+/).filter(Boolean)
  const lines: string[] = []
  let current = ''
  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word
    if (current && ctx.measureText(candidate).width > maxWidth) {
      lines.push(current)
      current = word
    } else {
      current = candidate
    }
  }
  if (current) lines.push(current)
  return lines
}

/**
 * La plus grande taille, entre `max` et `min`, à laquelle le texte tient en
 * `maxLines` lignes. À la taille minimale, les lignes en trop s'achèvent par
 * « … » : un titre de trois phrases ne doit pas déborder de l'image.
 */
export function fitText(
  ctx: TextMeasurer,
  text: string,
  options: { maxWidth: number; maxLines: number; max: number; min: number; weight: number; family: string },
): { lines: string[]; size: number } {
  for (let size = options.max; size >= options.min; size -= 4) {
    ctx.font = `${options.weight} ${size}px ${options.family}`
    const lines = wrapLines(ctx, text, options.maxWidth)
    if (lines.length <= options.maxLines) return { lines, size }
  }
  ctx.font = `${options.weight} ${options.min}px ${options.family}`
  const lines = wrapLines(ctx, text, options.maxWidth)
  const kept = lines.slice(0, options.maxLines)
  if (lines.length > options.maxLines) kept[kept.length - 1] = `${kept[kept.length - 1]}…`
  return { lines: kept, size: options.min }
}

// ---------------------------------------------------------------------------
// Briques de dessin
// ---------------------------------------------------------------------------

/** `#rrggbb` ou `#rgb` vers `rgba(…)` : les jetons sont opaques, le voile ne l'est pas. */
export function withAlpha(hex: string, alpha: number): string {
  const value = hex.trim().replace('#', '')
  const full = value.length === 3 ? [...value].map((digit) => digit + digit).join('') : value.slice(0, 6)
  const [r, g, b] = [0, 2, 4].map((start) => Number.parseInt(full.slice(start, start + 2), 16))
  return `rgba(${r}, ${g}, ${b}, ${alpha})`
}

export function sizeOf(image: CanvasImageSource): { width: number; height: number } {
  if (image instanceof HTMLImageElement) return { width: image.naturalWidth || image.width, height: image.naturalHeight || image.height }
  if (image instanceof HTMLCanvasElement || image instanceof ImageBitmap) return { width: image.width, height: image.height }
  return { width: 1, height: 1 }
}

/** Recadre l'image pour remplir le cadre, comme `object-fit: cover`. */
export function drawCovering(ctx: CanvasRenderingContext2D, image: CanvasImageSource, x: number, y: number, w: number, h: number): void {
  const { width, height } = sizeOf(image)
  const scale = Math.max(w / width, h / height)
  const sw = w / scale
  const sh = h / scale
  ctx.drawImage(image, (width - sw) / 2, (height - sh) / 2, sw, sh, x, y, w, h)
}

/** Loge l'image dans le cadre sans la couper, comme `object-fit: contain`, centrée. */
export function drawContained(ctx: CanvasRenderingContext2D, image: CanvasImageSource, x: number, y: number, w: number, h: number): void {
  const { width, height } = sizeOf(image)
  const scale = Math.min(w / width, h / height)
  const dw = width * scale
  const dh = height * scale
  ctx.drawImage(image, x + (w - dw) / 2, y + (h - dh) / 2, dw, dh)
}

export function roundedRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  ctx.beginPath()
  ctx.roundRect(x, y, w, h, r)
}

export function initialsOfName(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('')
}

export function drawFace(ctx: CanvasRenderingContext2D, scene: CoverScene, speaker: CoverSpeaker, cx: number, cy: number, r: number, ring: string): void {
  ctx.save()
  ctx.beginPath()
  ctx.arc(cx, cy, r, 0, Math.PI * 2)
  ctx.closePath()
  ctx.fillStyle = ring
  ctx.fill()
  ctx.beginPath()
  ctx.arc(cx, cy, r - 5, 0, Math.PI * 2)
  ctx.clip()
  if (speaker.image) {
    drawCovering(ctx, speaker.image, cx - r, cy - r, r * 2, r * 2)
  } else {
    ctx.fillStyle = scene.colors.muted
    ctx.fillRect(cx - r, cy - r, r * 2, r * 2)
    ctx.fillStyle = scene.colors.onAccent
    ctx.font = `700 ${Math.round(r * 0.7)}px ${scene.fontFamily}`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(initialsOfName(speaker.name), cx, cy)
  }
  ctx.restore()
}

export function drawLines(
  ctx: CanvasRenderingContext2D,
  lines: string[],
  x: number,
  y: number,
  size: number,
  lineHeight: number,
): number {
  ctx.textAlign = 'left'
  ctx.textBaseline = 'top'
  lines.forEach((line, index) => ctx.fillText(line, x, y + index * size * lineHeight))
  return y + lines.length * size * lineHeight
}

/** Bandeau de logos en bas, sur fond clair : les logos sont dessinés pour un fond blanc. */
export const LOGO_BAR = 120

export function drawLogoBar(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const y = COVER_HEIGHT - LOGO_BAR
  ctx.fillStyle = scene.colors.paper
  ctx.fillRect(0, y, COVER_WIDTH, LOGO_BAR)
  if (scene.platformLogo) drawContained(ctx, scene.platformLogo, 48, y + 16, 440, LOGO_BAR - 32)
  if (scene.organizationLogo) drawContained(ctx, scene.organizationLogo, COVER_WIDTH - 48 - 300, y + 18, 300, LOGO_BAR - 36)
}

export function drawIcon(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  if (!scene.icon) return
  const size = 132
  const x = COVER_WIDTH - size - 40
  const y = 40
  ctx.save()
  roundedRect(ctx, x, y, size, size, 20)
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  roundedRect(ctx, x + 10, y + 10, size - 20, size - 20, 14)
  ctx.clip()
  drawCovering(ctx, scene.icon, x + 10, y + 10, size - 20, size - 20)
  ctx.restore()
}

/** Le bas de la zone utile : au-dessus du bandeau de logos quand il est montré. */
export function bottomOf(options: CoverDesignOptions): number {
  return options.show_logos ? COVER_HEIGHT - LOGO_BAR : COVER_HEIGHT
}

export function drawChrome(ctx: CanvasRenderingContext2D, scene: CoverScene, options: CoverDesignOptions): void {
  if (options.show_logos) drawLogoBar(ctx, scene)
  if (options.show_event_icon) drawIcon(ctx, scene)
}

/** La photo en plein cadre, ou l'aplat de la couleur choisie sans photo. */
export function drawBackdrop(ctx: CanvasRenderingContext2D, scene: CoverScene, bottom: number): void {
  if (scene.photo) drawCovering(ctx, scene.photo, 0, 0, COVER_WIDTH, bottom)
  else {
    ctx.fillStyle = scene.colors.accent
    ctx.fillRect(0, 0, COVER_WIDTH, bottom)
  }
}

/** Un dégradé sur toute la zone utile ; `stops` : position de 0 à 1, couleur, opacité. */
export function drawFade(
  ctx: CanvasRenderingContext2D,
  bottom: number,
  direction: 'horizontal' | 'vertical',
  stops: [number, string, number][],
): void {
  const gradient =
    direction === 'horizontal' ? ctx.createLinearGradient(0, 0, COVER_WIDTH, 0) : ctx.createLinearGradient(0, 0, 0, bottom)
  for (const [at, color, alpha] of stops) gradient.addColorStop(at, withAlpha(color, alpha))
  ctx.fillStyle = gradient
  ctx.fillRect(0, 0, COVER_WIDTH, bottom)
}

/** Bloc de texte commun : surtitre, titre, date et lieu, à partir de `y`. */
export function drawTextBlock(
  ctx: CanvasRenderingContext2D,
  scene: CoverScene,
  options: CoverDesignOptions,
  box: { x: number; y: number; width: number; maxTitleLines: number; titleMax: number },
  colors: { title: string; text: string; eyebrow: string },
): number {
  ctx.fillStyle = colors.eyebrow
  ctx.font = `600 26px ${scene.fontFamily}`
  let y = drawLines(ctx, [scene.eyebrow.toUpperCase()], box.x, box.y, 26, 1.3) + 22

  const title = fitText(ctx, scene.title, {
    maxWidth: box.width,
    maxLines: box.maxTitleLines,
    max: box.titleMax,
    min: 40,
    weight: 700,
    family: scene.fontFamily,
  })
  ctx.fillStyle = colors.title
  y = drawLines(ctx, title.lines, box.x, y, title.size, 1.1) + 26

  if (options.show_date) {
    ctx.fillStyle = colors.text
    ctx.font = `500 30px ${scene.fontFamily}`
    for (const line of [scene.dateLine, scene.placeLine]) {
      if (!line) continue
      const wrapped = wrapLines(ctx, line, box.width).slice(0, 2)
      y = drawLines(ctx, wrapped, box.x, y, 30, 1.35) + 6
    }
  }
  return y
}

// ---------------------------------------------------------------------------
// Les modèles
// ---------------------------------------------------------------------------

export interface CoverTemplate {
  code: string
  /** La photo de départ compte-t-elle dans ce modèle ? Sinon l'atelier ne la propose pas. */
  usesPhoto: boolean
  draw(ctx: CanvasRenderingContext2D, scene: CoverScene, options: CoverDesignOptions): void
}


/** Espacement des lettres en pixels ; ignoré par un navigateur qui ne le connaît pas. */
export function setTracking(ctx: CanvasRenderingContext2D, pixels: number): void {
  if ('letterSpacing' in ctx) ctx.letterSpacing = `${pixels}px`
}

/** Le texte centré sur `cx`, ligne par ligne ; rend le bas du bloc. */
export function drawCentered(ctx: CanvasRenderingContext2D, lines: string[], cx: number, y: number, size: number, lineHeight: number): number {
  ctx.textAlign = 'center'
  ctx.textBaseline = 'top'
  lines.forEach((line, index) => ctx.fillText(line, cx, y + index * size * lineHeight))
  ctx.textAlign = 'left'
  return y + lines.length * size * lineHeight
}

/**
 * La photo floutée, recadrée comme `drawCovering`. Safari ignore `ctx.filter` :
 * on réduit l'image puis on l'agrandit, ce qui floute partout.
 */
export function drawBlurred(
  ctx: CanvasRenderingContext2D,
  image: CanvasImageSource,
  x: number,
  y: number,
  w: number,
  h: number,
  strength = 24,
): void {
  const small = document.createElement('canvas')
  small.width = Math.max(1, Math.round(w / strength))
  small.height = Math.max(1, Math.round(h / strength))
  const smallCtx = small.getContext('2d')
  if (!smallCtx) return drawCovering(ctx, image, x, y, w, h)
  smallCtx.imageSmoothingQuality = 'high'
  drawCovering(smallCtx, image, 0, 0, small.width, small.height)
  ctx.save()
  ctx.imageSmoothingEnabled = true
  ctx.imageSmoothingQuality = 'high'
  ctx.drawImage(small, x, y, w, h)
  ctx.restore()
}

/**
 * La photo en deux tons de `dark` (ombres) et `light` (lumières), recadrée comme
 * `drawCovering`. Par modes de fusion, pas par `ctx.filter`, que Safari ignore.
 */
export function drawDuotone(
  ctx: CanvasRenderingContext2D,
  image: CanvasImageSource,
  x: number,
  y: number,
  w: number,
  h: number,
  dark: string,
  light: string,
): void {
  const layer = document.createElement('canvas')
  layer.width = Math.round(w)
  layer.height = Math.round(h)
  const lc = layer.getContext('2d')
  if (!lc) return drawCovering(ctx, image, x, y, w, h)
  drawCovering(lc, image, 0, 0, layer.width, layer.height)
  // Niveaux de gris : la couleur seule, sans teinte, prise par `saturation` sur un gris neutre.
  lc.globalCompositeOperation = 'saturation'
  lc.fillStyle = '#808080'
  lc.fillRect(0, 0, layer.width, layer.height)
  // Les lumières prennent la teinte claire, les ombres la foncée.
  lc.globalCompositeOperation = 'multiply'
  lc.fillStyle = light
  lc.fillRect(0, 0, layer.width, layer.height)
  lc.globalCompositeOperation = 'screen'
  lc.fillStyle = dark
  lc.fillRect(0, 0, layer.width, layer.height)
  ctx.drawImage(layer, x, y)
}

/** Les deux logos côte à côte sur une ligne de hauteur `h` ; rend la largeur occupée. */
export function drawLogoRow(ctx: CanvasRenderingContext2D, scene: CoverScene, x: number, y: number, h: number, gap = 40): number {
  let cursor = x
  for (const logo of [scene.platformLogo, scene.organizationLogo]) {
    if (!logo) continue
    const { width, height } = sizeOf(logo)
    const w = Math.min(h * (width / height), 420)
    drawContained(ctx, logo, cursor, y, w, h)
    cursor += w + gap
  }
  return cursor - x - (cursor > x ? gap : 0)
}
