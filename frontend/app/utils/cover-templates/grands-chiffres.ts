import type { CoverScene, CoverSpeaker, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawContained, drawCovering, drawFace, drawLines, fitText, roundedRect, setTracking, sizeOf, withAlpha, wrapLines } from './draw.ts'

/**
 * Grands chiffres : la date en caractères géants, tronqués par le haut du cadre,
 * la photo visible seulement à travers les lettres. Le texte courant repose sur
 * l'aplat : la photo ne touche jamais sa lisibilité.
 */

const LEFT = 72
const RIGHT = COVER_WIDTH - 72
const BOTTOM = COVER_HEIGHT - 56
const DIGITS_SIZE = 430
const DIGITS_BASELINE = 310

function font(scene: CoverScene, weight: number, size: number): string {
  return `${weight} ${size}px ${scene.fontFamily}`
}

function drawDigits(ctx: CanvasRenderingContext2D, scene: CoverScene, text: string): void {
  const { accent, accentLight, paper } = scene.colors
  const tracking = -0.03 * DIGITS_SIZE
  const write = (target: CanvasRenderingContext2D, mode: 'fill' | 'stroke') => {
    target.font = font(scene, 700, DIGITS_SIZE)
    target.textAlign = 'left'
    target.textBaseline = 'alphabetic'
    setTracking(target, tracking)
    if (mode === 'fill') target.fillText(text, 44, DIGITS_BASELINE)
    else target.strokeText(text, 44, DIGITS_BASELINE)
    setTracking(target, 0)
  }
  if (!scene.photo) {
    ctx.save()
    // L'accent éclairci, sans photo : les chiffres restent un motif, pas un texte à lire.
    ctx.fillStyle = withAlpha(accentLight, 0.22)
    write(ctx, 'fill')
    ctx.restore()
    return
  }
  // La photo ne passe qu'à travers les lettres : texte d'abord, photo en `source-in` sur un calque.
  const layer = document.createElement('canvas')
  layer.width = COVER_WIDTH
  layer.height = COVER_HEIGHT
  const lc = layer.getContext('2d')
  if (lc) {
    lc.fillStyle = accent
    write(lc, 'fill')
    lc.font = font(scene, 700, DIGITS_SIZE)
    setTracking(lc, tracking)
    const width = lc.measureText(text).width
    setTracking(lc, 0)
    lc.globalCompositeOperation = 'source-in'
    drawCovering(lc, scene.photo, 44, -64, Math.max(width, 1), DIGITS_SIZE)
    ctx.drawImage(layer, 0, 0)
  }
  ctx.save()
  ctx.strokeStyle = paper
  ctx.lineWidth = 3
  ctx.lineJoin = 'round'
  write(ctx, 'stroke')
  ctx.restore()
}

/** La plus grande taille, de `max` à 40, à laquelle le titre tient dans la boîte ; à 40, des points de suspension. */
function fitTitle(ctx: CanvasRenderingContext2D, scene: CoverScene, width: number, height: number, max: number): { lines: string[]; size: number } {
  for (let size = max; size >= 40; size -= 4) {
    const maxLines = Math.min(4, Math.max(1, Math.floor(height / (size * 1.06))))
    setTracking(ctx, -0.01 * size)
    const fit = fitText(ctx, scene.title, { maxWidth: width, maxLines, max: size, min: size, weight: 700, family: scene.fontFamily })
    if (size === 40 || wrapLines(ctx, scene.title, width).length <= maxLines) return fit
  }
  return { lines: [], size: 40 }
}

/** Logos OIF-IFDD et de l'organisation dans une capsule blanche, calée en bas à droite ; rend sa largeur. */
function drawLogoCapsule(ctx: CanvasRenderingContext2D, scene: CoverScene, right: number, bottom: number): number {
  const logos = [
    { image: scene.platformLogo, h: 54 },
    { image: scene.organizationLogo, h: 44 },
  ].flatMap(({ image, h }) => (image ? [{ image, h, w: Math.min(h * (sizeOf(image).width / sizeOf(image).height), 300) }] : []))
  if (logos.length === 0) return 0
  const inner = logos.reduce((sum, logo) => sum + logo.w, 0) + (logos.length - 1) * 49
  const width = inner + 48
  const height = 86
  const x = right - width
  const y = bottom - height
  ctx.save()
  roundedRect(ctx, x, y, width, height, 8)
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  let cursor = x + 24
  logos.forEach((logo, index) => {
    if (index > 0) {
      ctx.fillStyle = scene.colors.rule
      ctx.fillRect(cursor + 24, y + 19, 1, 48)
      cursor += 49
    }
    drawContained(ctx, logo.image, cursor, y + (height - logo.h) / 2, logo.w, logo.h)
    cursor += logo.w
  })
  ctx.restore()
  return width
}

function drawFaceStack(ctx: CanvasRenderingContext2D, scene: CoverScene, speakers: CoverSpeaker[], x: number, cy: number): number {
  speakers.forEach((speaker, index) => drawFace(ctx, scene, speaker, x + 32 + index * 52, cy, 32, scene.colors.paper))
  return speakers.length > 0 ? 64 + (speakers.length - 1) * 52 : 0
}

function drawIconTile(ctx: CanvasRenderingContext2D, scene: CoverScene, icon: CanvasImageSource, x: number, y: number): void {
  ctx.save()
  roundedRect(ctx, x, y, 72, 72, 6)
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  roundedRect(ctx, x + 4, y + 4, 64, 64, 3)
  ctx.clip()
  drawCovering(ctx, icon, x + 4, y + 4, 64, 64)
  ctx.restore()
}

export const grandsChiffres: CoverTemplate = {
  code: 'grands-chiffres',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, accentLight, onAccent } = scene.colors
    ctx.fillStyle = accent
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    const date = options.show_date ? scene.date : null
    if (date) drawDigits(ctx, scene, `${date.day} ${date.monthShort.replace(/\.$/, '').toLocaleUpperCase()}`)

    // Pied, du bas vers le haut : lieu, date, intervenants ; les logos à droite.
    const capsule = options.show_logos ? drawLogoCapsule(ctx, scene, RIGHT, BOTTOM) : 0
    const columnWidth = RIGHT - LEFT - (capsule > 0 ? capsule + 48 : 0)
    let y = BOTTOM
    if (options.show_date) {
      if (scene.placeLine) {
        ctx.fillStyle = accentLight
        ctx.font = font(scene, 400, 26)
        const place = fitText(ctx, scene.placeLine, { maxWidth: columnWidth, maxLines: 1, max: 26, min: 20, weight: 400, family: scene.fontFamily })
        y -= place.size * 1.2
        drawLines(ctx, place.lines, LEFT, y, place.size, 1.2)
        y -= 6
      }
      if (scene.dateLine) {
        ctx.fillStyle = onAccent
        const line = fitText(ctx, scene.dateLine, { maxWidth: columnWidth, maxLines: 1, max: 32, min: 22, weight: 700, family: scene.fontFamily })
        y -= line.size * 1.2
        drawLines(ctx, line.lines, LEFT, y, line.size, 1.2)
      }
    }
    const speakers = options.show_speakers ? scene.speakers.slice(0, 5) : []
    if (speakers.length > 0) {
      const stackWidth = 64 + (speakers.length - 1) * 52
      const names = fitText(ctx, scene.speakers.map((speaker) => speaker.name).join(', '), {
        maxWidth: Math.min(620, columnWidth - stackWidth - 20),
        maxLines: 2,
        max: 22,
        min: 22,
        weight: 400,
        family: scene.fontFamily,
      }).lines
      const rowHeight = Math.max(64, names.length * 22 * 1.3)
      y -= (y < BOTTOM ? 18 : 0) + rowHeight
      drawFaceStack(ctx, scene, speakers, LEFT, y + rowHeight / 2)
      ctx.fillStyle = accentLight
      ctx.font = font(scene, 400, 22)
      drawLines(ctx, names, LEFT + stackWidth + 20, y + (rowHeight - names.length * 22 * 1.3) / 2, 22, 1.3)
    }
    const footTop = Math.min(y, capsule > 0 ? BOTTOM - 86 : y)

    // Sous les chiffres, le titre prend la plus grande taille qui tient ; sans eux, le bloc descend sur le pied.
    const icon = options.show_event_icon ? scene.icon : null
    const headHeight = (icon ? 72 : 26) + 22
    const headTop = date ? 312 : 72
    const title = fitTitle(ctx, scene, RIGHT - LEFT, footTop - 28 - headTop - headHeight, date ? 72 : 96)
    const top = date ? headTop : Math.max(headTop, footTop - 28 - title.lines.length * title.size * 1.06 - headHeight)
    if (icon) drawIconTile(ctx, scene, icon, LEFT, top)
    const eyebrowX = icon ? LEFT + 92 : LEFT
    ctx.fillStyle = onAccent
    ctx.font = font(scene, 700, 22)
    setTracking(ctx, 3.08)
    const eyebrow = fitText(ctx, scene.eyebrow.toLocaleUpperCase(), { maxWidth: RIGHT - eyebrowX, maxLines: 1, max: 22, min: 22, weight: 700, family: scene.fontFamily })
    ctx.textBaseline = 'middle'
    ctx.fillText(eyebrow.lines[0] ?? '', eyebrowX, top + (icon ? 36 : 13))

    ctx.font = font(scene, 700, title.size)
    setTracking(ctx, -0.01 * title.size)
    drawLines(ctx, title.lines, LEFT, top + headHeight, title.size, 1.06)
    setTracking(ctx, 0)
  },
}
