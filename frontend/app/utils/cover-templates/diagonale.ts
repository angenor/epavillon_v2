import type { CoverScene, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawContained, drawCovering, drawFace, drawLines, fitText, roundedRect, setTracking, sizeOf, withAlpha } from './draw.ts'

/** Diagonale : un pan de couleur coupé en biais, doublé d'un liseré blanc, porte le titre ; la photo ne passe jamais sous le texte. */

const LEFT = 80
const WIDTH = 790
/** Le biais : la coupe va de `TOP_X` en haut à `TOP_X - SLANT` en bas ; l'étiquette de date et le cartouche suivent le même angle. */
const TOP_X = 1118
const SLANT = 240
const LISERE = 34
const SKEW = Math.tan((15 * Math.PI) / 180)

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

function slab(ctx: CanvasRenderingContext2D, color: string, topX: number): void {
  ctx.beginPath()
  ctx.moveTo(0, 0)
  ctx.lineTo(topX, 0)
  ctx.lineTo(topX - SLANT, COVER_HEIGHT)
  ctx.lineTo(0, COVER_HEIGHT)
  ctx.closePath()
  ctx.fillStyle = color
  ctx.fill()
}

/** Étiquette blanche inclinée comme la coupe, le texte droit dedans ; rend le haut de l'étiquette. */
function drawDateTag(ctx: CanvasRenderingContext2D, scene: CoverScene, label: string, bottom: number): number {
  setTracking(ctx, 0.6)
  const fit = fitText(ctx, label, { maxWidth: WIDTH - 60, maxLines: 1, max: 32, min: 24, weight: 700, family: scene.fontFamily })
  const textWidth = ctx.measureText(fit.lines[0] ?? '').width
  const height = Math.round(fit.size * 1.2) + 28
  const top = bottom - height
  const lean = (SKEW * height) / 2
  ctx.beginPath()
  ctx.moveTo(LEFT + lean, top)
  ctx.lineTo(LEFT + textWidth + 60 + lean, top)
  ctx.lineTo(LEFT + textWidth + 60 - lean, bottom)
  ctx.lineTo(LEFT - lean, bottom)
  ctx.closePath()
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  ctx.fillStyle = scene.colors.accent
  ctx.textAlign = 'left'
  ctx.textBaseline = 'middle'
  ctx.fillText(fit.lines[0] ?? '', LEFT + 30, top + height / 2 + 1)
  setTracking(ctx, 0)
  return top
}

function drawLogoPlate(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const { platformLogo, organizationLogo } = scene
  if (!platformLogo && !organizationLogo) return
  const top = 770
  ctx.beginPath()
  ctx.moveTo(1035, top)
  ctx.lineTo(COVER_WIDTH, top)
  ctx.lineTo(COVER_WIDTH, COVER_HEIGHT)
  ctx.lineTo(1000, COVER_HEIGHT)
  ctx.closePath()
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  const start = 1076
  const end = COVER_WIDTH - 56
  let used = 0
  if (platformLogo) {
    used = logoWidth(platformLogo, 64, end - start)
    drawContained(ctx, platformLogo, start, top + 33, used, 64)
  }
  if (organizationLogo && end - start - used - 28 > 60) {
    const w = logoWidth(organizationLogo, 52, end - start - used - 28)
    drawContained(ctx, organizationLogo, end - w, top + 39, w, 52)
  }
}

export const diagonale: CoverTemplate = {
  code: 'diagonale',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, accentLight, onAccent, paper, scrim } = scene.colors
    const font = scene.fontFamily

    if (scene.photo) {
      drawCovering(ctx, scene.photo, 0, 0, COVER_WIDTH, COVER_HEIGHT)
    } else {
      // Sans photo, l'accent assombri tient la place de l'image : deux tons d'une même couleur.
      ctx.fillStyle = accent
      ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
      ctx.fillStyle = withAlpha(scrim, 0.3)
      ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    }
    slab(ctx, paper, TOP_X + LISERE)
    slab(ctx, accent, TOP_X)

    const showIcon = options.show_event_icon && Boolean(scene.icon)
    if (showIcon) iconTile(ctx, scene, LEFT, 72, 88, 6, 6)
    const eyebrowX = showIcon ? LEFT + 88 + 22 : LEFT
    const rowBottom = showIcon ? 72 + 88 : 72 + 27
    setTracking(ctx, 3)
    const eyebrow = fitText(ctx, scene.eyebrow.toUpperCase(), {
      maxWidth: LEFT + WIDTH - eyebrowX,
      maxLines: 1,
      max: 22,
      min: 16,
      weight: 700,
      family: font,
    })
    ctx.fillStyle = onAccent
    ctx.textAlign = 'left'
    ctx.textBaseline = 'middle'
    ctx.fillText(eyebrow.lines[0] ?? '', eyebrowX, showIcon ? 72 + 44 : 72 + 14)
    setTracking(ctx, 0)

    let floor = COVER_HEIGHT - 72
    const date = scene.date
    if (options.show_date && (date || scene.placeLine)) {
      const detail = [date?.time ? date.zone : null, scene.placeLine].filter(Boolean).join(' · ')
      if (detail) {
        const line = fitText(ctx, detail, { maxWidth: WIDTH, maxLines: 1, max: 25, min: 20, weight: 400, family: font })
        ctx.fillStyle = onAccent
        drawLines(ctx, line.lines, LEFT, floor - 30, line.size, 1.2)
        floor -= 30 + 16
      }
      if (date) {
        const label = `${date.weekday} ${date.day} ${date.monthShort} ${date.year}${date.time ? ` · ${date.time}` : ''}`
        floor = drawDateTag(ctx, scene, label.toUpperCase(), floor)
      }
    }

    const titleTop = rowBottom + 48
    setTracking(ctx, -0.7)
    const title = fitText(ctx, scene.title, { maxWidth: WIDTH, maxLines: 4, max: 72, min: 40, weight: 700, family: font })
    ctx.fillStyle = onAccent
    let y = drawLines(ctx, title.lines, LEFT, titleTop, title.size, 1.05)
    setTracking(ctx, 0)

    const speakers = options.show_speakers ? scene.speakers : []
    if (speakers.length > 0) {
      y += 30
      const room = Math.min(2, Math.floor((floor - 32 - y) / (25 * 1.35)))
      if (room > 0) {
        const names = fitText(ctx, speakers.map((speaker) => speaker.name).join(', '), {
          maxWidth: 700,
          maxLines: room,
          max: 25,
          min: 21,
          weight: 400,
          family: font,
        })
        ctx.fillStyle = accentLight
        drawLines(ctx, names.lines, LEFT, y, names.size, 1.35)
      }
      speakers.slice(0, 5).forEach((speaker, index) => {
        drawFace(ctx, scene, speaker, COVER_WIDTH - 64 - 52, 64 + 52 + index * 120, 52, paper)
      })
    }

    if (options.show_logos) drawLogoPlate(ctx, scene)
  },
}
