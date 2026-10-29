import type { CoverDesignOptions } from '~/types/programme/cover-design'

/**
 * L'ATELIER DE L'IMAGE DE COUVERTURE (29/10, option A du commanditaire) : les
 * modèles vivent ICI, dans le code du site ; la base ne garde que leur code et
 * les choix de l'organisation. Un modèle est une fonction de dessin sur un
 * canvas de 1600 × 900 : l'aperçu et l'image enregistrée sortent du même
 * dessin, au pixel près. L'image sert aussi d'aperçu quand on partage le lien.
 */

export const COVER_WIDTH = 1600
export const COVER_HEIGHT = 900

export interface CoverColors {
  accent: string
  onAccent: string
  paper: string
  ink: string
  muted: string
  /** `--color-scrim`, opaque : le voile en dose l'opacité. */
  scrim: string
}

export interface CoverSpeaker {
  name: string
  image: CanvasImageSource | null
}

export interface CoverScene {
  title: string
  eyebrow: string
  dateLine: string | null
  placeLine: string | null
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

function sizeOf(image: CanvasImageSource): { width: number; height: number } {
  if (image instanceof HTMLImageElement) return { width: image.naturalWidth || image.width, height: image.naturalHeight || image.height }
  if (image instanceof HTMLCanvasElement || image instanceof ImageBitmap) return { width: image.width, height: image.height }
  return { width: 1, height: 1 }
}

/** Recadre l'image pour remplir le cadre, comme `object-fit: cover`. */
function drawCovering(ctx: CanvasRenderingContext2D, image: CanvasImageSource, x: number, y: number, w: number, h: number): void {
  const { width, height } = sizeOf(image)
  const scale = Math.max(w / width, h / height)
  const sw = w / scale
  const sh = h / scale
  ctx.drawImage(image, (width - sw) / 2, (height - sh) / 2, sw, sh, x, y, w, h)
}

/** Loge l'image dans le cadre sans la couper, comme `object-fit: contain`, centrée. */
function drawContained(ctx: CanvasRenderingContext2D, image: CanvasImageSource, x: number, y: number, w: number, h: number): void {
  const { width, height } = sizeOf(image)
  const scale = Math.min(w / width, h / height)
  const dw = width * scale
  const dh = height * scale
  ctx.drawImage(image, x + (w - dw) / 2, y + (h - dh) / 2, dw, dh)
}

function roundedRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number): void {
  ctx.beginPath()
  ctx.roundRect(x, y, w, h, r)
}

function initialsOfName(name: string): string {
  return name
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0]?.toUpperCase() ?? '')
    .join('')
}

function drawFace(ctx: CanvasRenderingContext2D, scene: CoverScene, speaker: CoverSpeaker, cx: number, cy: number, r: number, ring: string): void {
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

function drawLines(
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
const LOGO_BAR = 120

function drawLogoBar(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const y = COVER_HEIGHT - LOGO_BAR
  ctx.fillStyle = scene.colors.paper
  ctx.fillRect(0, y, COVER_WIDTH, LOGO_BAR)
  if (scene.platformLogo) drawContained(ctx, scene.platformLogo, 48, y + 16, 440, LOGO_BAR - 32)
  if (scene.organizationLogo) drawContained(ctx, scene.organizationLogo, COVER_WIDTH - 48 - 300, y + 18, 300, LOGO_BAR - 36)
}

function drawIcon(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
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
function bottomOf(options: CoverDesignOptions): number {
  return options.show_logos ? COVER_HEIGHT - LOGO_BAR : COVER_HEIGHT
}

function drawChrome(ctx: CanvasRenderingContext2D, scene: CoverScene, options: CoverDesignOptions): void {
  if (options.show_logos) drawLogoBar(ctx, scene)
  if (options.show_event_icon) drawIcon(ctx, scene)
}

/** La photo en plein cadre, ou l'aplat de la couleur choisie sans photo. */
function drawBackdrop(ctx: CanvasRenderingContext2D, scene: CoverScene, bottom: number): void {
  if (scene.photo) drawCovering(ctx, scene.photo, 0, 0, COVER_WIDTH, bottom)
  else {
    ctx.fillStyle = scene.colors.accent
    ctx.fillRect(0, 0, COVER_WIDTH, bottom)
  }
}

/** Un dégradé sur toute la zone utile ; `stops` : position de 0 à 1, couleur, opacité. */
function drawFade(
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
function drawTextBlock(
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

// Dégradés et voiles : seulement sur une photo (exception arbitrée du guide, § « Le verre »).

const fullPhoto: CoverTemplate = {
  code: 'photo-pleine',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    const { accent, scrim, onAccent } = scene.colors
    drawBackdrop(ctx, scene, bottom)
    // La couleur monte de la gauche, le voile du bas : le texte blanc reste lisible sur une photo claire.
    drawFade(ctx, bottom, 'horizontal', [
      [0, accent, 0.7],
      [0.55, accent, 0],
    ])
    drawFade(ctx, bottom, 'vertical', [
      [0, scrim, 0.1],
      [0.4, scrim, 0.15],
      [1, scrim, 0.82],
    ])

    const faces = options.show_speakers ? scene.speakers.slice(0, 4) : []
    const facesWidth = faces.length > 0 ? faces.length * 116 + 40 : 0
    const title = fitText(ctx, scene.title, {
      maxWidth: COVER_WIDTH - 112 - 72 - facesWidth,
      maxLines: 3,
      max: 80,
      min: 40,
      weight: 700,
      family: scene.fontFamily,
    })
    const dateHeight = options.show_date && scene.dateLine ? 30 * 1.3 + 18 : 0
    const blockHeight = 26 * 1.3 + 16 + title.lines.length * title.size * 1.1 + dateHeight
    const top = bottom - 72 - blockHeight

    ctx.fillStyle = accent
    ctx.fillRect(72, top, 12, blockHeight)
    ctx.fillStyle = onAccent
    ctx.font = `600 26px ${scene.fontFamily}`
    let y = drawLines(ctx, [scene.eyebrow.toUpperCase()], 112, top, 26, 1.3) + 16
    ctx.font = `700 ${title.size}px ${scene.fontFamily}`
    y = drawLines(ctx, title.lines, 112, y, title.size, 1.1) + 18
    if (options.show_date && scene.dateLine) {
      ctx.font = `500 30px ${scene.fontFamily}`
      drawLines(ctx, [scene.dateLine], 112, y, 30, 1.3)
    }
    faces.forEach((speaker, index) => {
      const cx = COVER_WIDTH - 72 - 52 - (faces.length - 1 - index) * 116
      drawFace(ctx, scene, speaker, cx, bottom - 72 - 52, 52, onAccent)
    })
    drawChrome(ctx, scene, options)
  },
}

const splitPhoto: CoverTemplate = {
  code: 'texte-photo',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    const { accent, onAccent } = scene.colors
    drawBackdrop(ctx, scene, bottom)
    // L'aplat porte le texte à gauche et se fond dans la photo, sans coupure nette.
    drawFade(ctx, bottom, 'horizontal', [
      [0, accent, 1],
      [0.44, accent, 1],
      [0.74, accent, 0],
    ])
    const y = drawTextBlock(
      ctx,
      scene,
      options,
      { x: 72, y: 64, width: Math.round(COVER_WIDTH * 0.46) - 72, maxTitleLines: 4, titleMax: 76 },
      { title: onAccent, text: onAccent, eyebrow: onAccent },
    )
    if (options.show_speakers) {
      scene.speakers.slice(0, 5).forEach((speaker, index) => {
        drawFace(ctx, scene, speaker, 72 + 46 + index * 104, Math.min(y + 70, bottom - 70), 46, onAccent)
      })
    }
    drawChrome(ctx, scene, options)
  },
}

const faces: CoverTemplate = {
  code: 'visages',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    const { accent, onAccent } = scene.colors
    drawBackdrop(ctx, scene, bottom)
    // La photo reste une texture sous la couleur : les visages portent l'image.
    drawFade(ctx, bottom, 'horizontal', [
      [0, accent, 0.97],
      [0.55, accent, 0.9],
      [1, accent, 0.72],
    ])
    const textWidth = options.show_speakers && scene.speakers.length > 0 ? 820 : COVER_WIDTH - 144
    drawTextBlock(
      ctx,
      scene,
      options,
      { x: 72, y: 64, width: textWidth, maxTitleLines: 4, titleMax: 80 },
      { title: onAccent, text: onAccent, eyebrow: onAccent },
    )
    if (options.show_speakers) {
      const shown = scene.speakers.slice(0, 4)
      const columns = shown.length > 2 ? 2 : shown.length
      const radius = 96
      const middle = bottom / 2
      shown.forEach((speaker, index) => {
        const column = index % 2
        const row = Math.floor(index / 2)
        const rows = Math.ceil(shown.length / columns)
        const cx = 1000 + radius + column * 270
        const cy = middle - ((rows - 1) * 250) / 2 + row * 250 - 20
        drawFace(ctx, scene, speaker, cx, cy, radius, onAccent)
        ctx.fillStyle = onAccent
        ctx.font = `600 24px ${scene.fontFamily}`
        ctx.textAlign = 'center'
        ctx.textBaseline = 'top'
        ctx.fillText(wrapLines(ctx, speaker.name, 250)[0] ?? '', cx, cy + radius + 14)
      })
    }
    drawChrome(ctx, scene, options)
  },
}

const typographic: CoverTemplate = {
  code: 'typographique',
  usesPhoto: false,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    ctx.fillStyle = scene.colors.paper
    ctx.fillRect(0, 0, COVER_WIDTH, bottom)
    ctx.fillStyle = scene.colors.accent
    ctx.fillRect(72, 64, 140, 16)
    const y = drawTextBlock(
      ctx,
      scene,
      options,
      { x: 72, y: 112, width: COVER_WIDTH - 144 - (options.show_event_icon ? 160 : 0), maxTitleLines: 3, titleMax: 104 },
      { title: scene.colors.accent, text: scene.colors.ink, eyebrow: scene.colors.muted },
    )
    if (options.show_speakers && scene.speakers.length > 0 && y < bottom - 60) {
      ctx.fillStyle = scene.colors.muted
      ctx.font = `500 28px ${scene.fontFamily}`
      const names = scene.speakers.map((speaker) => speaker.name).join(', ')
      drawLines(ctx, wrapLines(ctx, names, COVER_WIDTH - 144).slice(0, 2), 72, y + 8, 28, 1.35)
    }
    drawChrome(ctx, scene, options)
  },
}

export const COVER_TEMPLATES: CoverTemplate[] = [fullPhoto, splitPhoto, faces, typographic]

export function findCoverTemplate(code: string | null | undefined): CoverTemplate {
  return COVER_TEMPLATES.find((template) => template.code === code) ?? fullPhoto
}

export function drawCover(ctx: CanvasRenderingContext2D, scene: CoverScene, templateCode: string, options: CoverDesignOptions): void {
  ctx.clearRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
  ctx.save()
  findCoverTemplate(templateCode).draw(ctx, scene, options)
  ctx.restore()
}
