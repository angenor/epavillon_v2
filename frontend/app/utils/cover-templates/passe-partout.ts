import type { CoverScene, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawContained, drawCovering, drawFace, drawLines, fitText, roundedRect, setTracking, sizeOf, wrapLines } from './draw.ts'

/** Passe-partout : la photo encadrée comme un tirage d'exposition sur un filet de couleur, le titre et la date dessous comme un cartel de musée. */

const MARGIN = 56
const FRAME_W = COVER_WIDTH - MARGIN * 2
const FRAME_H = 470
const FRAME_BOTTOM = MARGIN + FRAME_H
const TEXT_W = 1000
const DATE_X = 1140
const DATE_W = COVER_WIDTH - MARGIN - DATE_X
const BOTTOM = COVER_HEIGHT - 48

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

/** La plus grande taille à laquelle le titre tient dans la hauteur donnée, les lignes en plus si la taille baisse. */
function fitInHeight(ctx: CanvasRenderingContext2D, scene: CoverScene, text: string, height: number, max: number, min: number) {
  for (let size = max; size > min; size -= 2) {
    const maxLines = Math.floor(height / (size * 1.05))
    if (maxLines < 1) continue
    ctx.font = `700 ${size}px ${scene.fontFamily}`
    if (wrapLines(ctx, text, TEXT_W).length <= maxLines) {
      return fitText(ctx, text, { maxWidth: TEXT_W, maxLines, max: size, min: size, weight: 700, family: scene.fontFamily })
    }
  }
  const maxLines = Math.max(1, Math.floor(height / (min * 1.05)))
  return fitText(ctx, text, { maxWidth: TEXT_W, maxLines, max: min, min, weight: 700, family: scene.fontFamily })
}

/** Sans photo, le cadre reste un tirage : le sigle de l'édition en grand, la ville et le mois au-dessus. */
function drawEmptyFrame(ctx: CanvasRenderingContext2D, scene: CoverScene, showDate: boolean): void {
  const { accent, accentLight, onAccent } = scene.colors
  ctx.fillStyle = accent
  ctx.fillRect(MARGIN, MARGIN, FRAME_W, FRAME_H)
  ctx.textAlign = 'left'
  ctx.textBaseline = 'alphabetic'
  // Le sigle (« CdP31 ») n'est pas dans la scène à part : c'est la fin du surtitre.
  const acronym = scene.eyebrow.split(' · ').pop() ?? ''
  setTracking(ctx, -8)
  const big = fitText(ctx, acronym, { maxWidth: FRAME_W - 112, maxLines: 1, max: 220, min: 96, weight: 700, family: scene.fontFamily })
  ctx.fillStyle = onAccent
  ctx.fillText(big.lines[0] ?? '', MARGIN + 56, FRAME_BOTTOM - 51)
  setTracking(ctx, 0)
  const context = showDate ? [scene.city, scene.date ? `${scene.date.month} ${scene.date.year}` : null].filter(Boolean).join(' · ') : ''
  if (!context) return
  const line = fitText(ctx, context, { maxWidth: FRAME_W - 280, maxLines: 1, max: 34, min: 26, weight: 500, family: scene.fontFamily })
  ctx.fillStyle = accentLight
  drawLines(ctx, line.lines, MARGIN + 60, MARGIN + 56, line.size, 1.2)
}

function drawDateBlock(ctx: CanvasRenderingContext2D, scene: CoverScene, limit: number): void {
  const { accent, text, muted } = scene.colors
  const date = scene.date
  const rows: { text: string; size: number; weight: number; color: string }[] = []
  if (date) {
    rows.push({ text: `${date.day} ${date.monthShort} ${date.year}`, size: 44, weight: 700, color: accent })
    rows.push({ text: [date.weekday, date.time, date.time ? date.zone : null].filter(Boolean).join(', '), size: 25, weight: 400, color: text })
  } else if (scene.dateLine) {
    rows.push({ text: scene.dateLine, size: 25, weight: 400, color: text })
  }
  if (scene.placeLine) rows.push({ text: scene.placeLine, size: 25, weight: 400, color: muted })
  let y = 616
  for (const row of rows) {
    const lineHeight = row.size > 30 ? 1.05 : 1.25
    const room = Math.min(2, Math.floor((limit - y) / (row.size * lineHeight)))
    if (room < 1) break
    const fit = fitText(ctx, row.text, { maxWidth: DATE_W, maxLines: room, max: row.size, min: row.size - 4, weight: row.weight, family: scene.fontFamily })
    ctx.fillStyle = row.color
    y = drawLines(ctx, fit.lines, DATE_X, y, fit.size, lineHeight) + 6
  }
}

function drawLogos(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const right = COVER_WIDTH - MARGIN
  let x = right
  if (scene.organizationLogo) {
    const w = logoWidth(scene.organizationLogo, 46, 260)
    x -= w
    drawContained(ctx, scene.organizationLogo, x, BOTTOM - 52, w, 46)
    x -= 28
  }
  if (scene.platformLogo) {
    const w = logoWidth(scene.platformLogo, 58, x - DATE_X + 40)
    drawContained(ctx, scene.platformLogo, x - w, BOTTOM - 58, w, 58)
  }
}

export const passePartout: CoverTemplate = {
  code: 'passe-partout',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, paper, text, muted } = scene.colors
    const font = scene.fontFamily

    ctx.fillStyle = paper
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    if (scene.photo) drawCovering(ctx, scene.photo, MARGIN, MARGIN, FRAME_W, FRAME_H)
    else drawEmptyFrame(ctx, scene, options.show_date)
    ctx.fillStyle = accent
    ctx.fillRect(MARGIN, FRAME_BOTTOM, FRAME_W, 8)
    if (options.show_event_icon) iconTile(ctx, scene, 1416, 80, 104, 6, 6)

    // Les portraits chevauchent le bord bas du cadre, alignés à droite.
    const speakers = options.show_speakers ? scene.speakers : []
    const faces = speakers.slice(0, 6)
    faces.forEach((speaker, index) => {
      const cx = COVER_WIDTH - 96 - 56 - (faces.length - 1 - index) * 92
      drawFace(ctx, scene, speaker, cx, 474 + 56, 56, paper, index)
    })

    setTracking(ctx, 2.8)
    const eyebrow = fitText(ctx, scene.eyebrow.toUpperCase(), { maxWidth: TEXT_W, maxLines: 1, max: 20, min: 16, weight: 700, family: font })
    ctx.fillStyle = accent
    drawLines(ctx, eyebrow.lines, MARGIN, 580, eyebrow.size, 1.2)
    setTracking(ctx, 0)

    let floor = BOTTOM
    if (speakers.length > 0) {
      const names = fitText(ctx, speakers.map((speaker) => speaker.name).join(', '), {
        maxWidth: TEXT_W,
        maxLines: 2,
        max: 22,
        min: 19,
        weight: 400,
        family: font,
      })
      const height = names.lines.length * names.size * 1.35
      ctx.fillStyle = muted
      drawLines(ctx, names.lines, MARGIN, floor - height, names.size, 1.35)
      floor -= height + 18
    }

    const titleTop = 580 + 24 + 14
    setTracking(ctx, -0.6)
    const title = fitInHeight(ctx, scene, scene.title, floor - titleTop, 64, 36)
    ctx.fillStyle = text
    drawLines(ctx, title.lines, MARGIN, titleTop, title.size, 1.05)
    setTracking(ctx, 0)

    if (options.show_date) drawDateBlock(ctx, scene, options.show_logos ? BOTTOM - 58 - 16 : BOTTOM)
    if (options.show_logos) drawLogos(ctx, scene)
  },
}
