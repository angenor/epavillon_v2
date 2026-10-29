import type { CoverScene, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, bottomOf, drawContained, drawCovering, drawFace, drawLines, drawLogoBar, fitText, roundedRect, setTracking, sizeOf, withAlpha, wrapLines } from './draw.ts'

/** Éphéméride : une feuille de calendrier détachée à gauche, le jour en chiffre géant ; la photo occupe le reste et porte le titre. */

const LEAF = 520
const PAD = 56
const INNER = LEAF - PAD * 2

function iconTile(ctx: CanvasRenderingContext2D, scene: CoverScene, x: number, y: number, size: number, pad: number, radius: number): void {
  if (!scene.icon) return
  ctx.save()
  roundedRect(ctx, x, y, size, size, radius)
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  roundedRect(ctx, x + pad, y + pad, size - pad * 2, size - pad * 2, radius / 2)
  ctx.clip()
  drawCovering(ctx, scene.icon, x + pad, y + pad, size - pad * 2, size - pad * 2)
  ctx.restore()
}

function logoWidth(image: CanvasImageSource, height: number, max: number): number {
  const { width, height: natural } = sizeOf(image)
  return Math.min((height * width) / natural, max)
}

function oneLine(ctx: CanvasRenderingContext2D, scene: CoverScene, text: string, width: number, size: number, weight: number): string {
  return fitText(ctx, text, { maxWidth: width, maxLines: 1, max: size, min: size - 6, weight, family: scene.fontFamily }).lines[0] ?? ''
}

/** La feuille : surtitre, mois, jour, heure, lieu, logos en pied. */
function drawLeaf(ctx: CanvasRenderingContext2D, scene: CoverScene, showLogos: boolean): void {
  const { accent, paper, text, muted } = scene.colors
  const font = scene.fontFamily
  const date = scene.date
  if (!date) return
  ctx.fillStyle = paper
  ctx.fillRect(0, 0, LEAF, COVER_HEIGHT)
  ctx.textAlign = 'left'

  // « Pavillon de la Francophonie · CdP31 » sur deux lignes, coupé au point médian comme la maquette.
  ctx.fillStyle = muted
  ctx.font = `700 20px ${font}`
  setTracking(ctx, 2.8)
  const eyebrow = scene.eyebrow
    .toUpperCase()
    .split(' · ')
    .flatMap((part) => wrapLines(ctx, part, INNER))
    .slice(0, 3)
  let y = drawLines(ctx, eyebrow, PAD, PAD, 20, 1.4) + 40

  ctx.fillStyle = accent
  setTracking(ctx, 4)
  ctx.textBaseline = 'top'
  ctx.fillText(oneLine(ctx, scene, `${date.month} ${date.year}`.toUpperCase(), INNER, 34, 700), PAD, y)
  y += 41 + 28

  setTracking(ctx, -15)
  ctx.font = `700 300px ${font}`
  ctx.textBaseline = 'alphabetic'
  ctx.fillText(date.day, PAD - 14, y + 228)
  setTracking(ctx, 0)
  y += 234 + 22

  ctx.fillStyle = text
  ctx.font = `500 40px ${font}`
  ctx.textBaseline = 'top'
  ctx.fillText(date.weekday, PAD, y)
  y += 48 + 26
  ctx.fillRect(PAD, y, INNER, 3)
  y += 3 + 24

  if (date.time) {
    ctx.font = `700 56px ${font}`
    ctx.fillText(date.time, PAD, y)
    ctx.fillStyle = muted
    ctx.fillText(oneLine(ctx, scene, date.zone, INNER, 26, 400), PAD, y + 64)
    y += 64 + 31 + 14
  }
  const logosTop = COVER_HEIGHT - 48 - 60
  const room = Math.min(2, Math.floor((logosTop - 16 - y) / (26 * 1.2)))
  if (scene.placeLine && room > 0) {
    const place = fitText(ctx, scene.placeLine, { maxWidth: INNER, maxLines: room, max: 26, min: 22, weight: 400, family: font })
    ctx.fillStyle = text
    drawLines(ctx, place.lines, PAD, y, place.size, 1.2)
  }

  if (!showLogos) return
  let x = PAD
  if (scene.platformLogo) {
    const w = logoWidth(scene.platformLogo, 56, INNER)
    drawContained(ctx, scene.platformLogo, x, logosTop + 2, w, 56)
    x += w + 24
  }
  if (scene.organizationLogo && x < LEAF - PAD - 40) {
    const w = logoWidth(scene.organizationLogo, 46, LEAF - PAD - x)
    drawContained(ctx, scene.organizationLogo, x, logosTop + 7, w, 46)
  }
}

export const ephemeride: CoverTemplate = {
  code: 'ephemeride',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, onAccent, paper, scrim } = scene.colors
    const font = scene.fontFamily
    // Sans date, la feuille n'a plus rien à dire : la photo prend toute la largeur et les logos passent au bandeau.
    const hasLeaf = options.show_date && Boolean(scene.date)
    const left = hasLeaf ? LEAF : 0
    const bottom = hasLeaf ? COVER_HEIGHT : bottomOf(options)

    if (scene.photo) {
      drawCovering(ctx, scene.photo, left, 0, COVER_WIDTH - left, bottom)
      // Le voile monte du bas : le titre blanc reste lisible sur une photo très claire.
      const veil = ctx.createLinearGradient(0, 0, 0, bottom)
      veil.addColorStop(0.18, withAlpha(scrim, 0))
      veil.addColorStop(0.48, withAlpha(scrim, 0.6))
      veil.addColorStop(1, withAlpha(scrim, 0.9))
      ctx.fillStyle = veil
      ctx.fillRect(left, 0, COVER_WIDTH - left, bottom)
    } else {
      ctx.fillStyle = accent
      ctx.fillRect(left, 0, COVER_WIDTH - left, bottom)
    }
    if (hasLeaf) {
      drawLeaf(ctx, scene, options.show_logos)
      ctx.fillStyle = accent
      ctx.fillRect(LEAF, 0, 12, COVER_HEIGHT)
    } else if (options.show_logos) {
      drawLogoBar(ctx, scene)
    }
    if (options.show_event_icon) iconTile(ctx, scene, 1448, 40, 112, 8, 8)

    const x = hasLeaf ? 600 : 80
    const width = COVER_WIDTH - 72 - x
    let y = bottom - 64
    const speakers = options.show_speakers ? scene.speakers.slice(0, 6) : []
    if (speakers.length > 0) {
      const rowTop = y - 84
      speakers.forEach((speaker, index) => drawFace(ctx, scene, speaker, x + 42 + index * 68, rowTop + 42, 42, paper))
      const namesX = x + 84 + (speakers.length - 1) * 68 + 24
      const names = fitText(ctx, scene.speakers.map((speaker) => speaker.name).join(', '), {
        maxWidth: x + width - namesX,
        maxLines: 2,
        max: 24,
        min: 20,
        weight: 400,
        family: font,
      })
      ctx.fillStyle = onAccent
      drawLines(ctx, names.lines, namesX, rowTop + (84 - names.lines.length * names.size * 1.3) / 2, names.size, 1.3)
      y = rowTop - 32
    }

    setTracking(ctx, -0.8)
    const title = fitText(ctx, scene.title, { maxWidth: width, maxLines: 3, max: 76, min: 40, weight: 700, family: font })
    const titleTop = y - title.lines.length * title.size * 1.05
    ctx.fillStyle = onAccent
    drawLines(ctx, title.lines, x, titleTop, title.size, 1.05)
    setTracking(ctx, 0)

    if (!hasLeaf) {
      ctx.font = `700 20px ${font}`
      setTracking(ctx, 2.8)
      const eyebrow = fitText(ctx, scene.eyebrow.toUpperCase(), { maxWidth: width, maxLines: 1, max: 20, min: 16, weight: 700, family: font })
      drawLines(ctx, eyebrow.lines, x, titleTop - 20 - 28, eyebrow.size, 1.4)
      setTracking(ctx, 0)
    }
  },
}
