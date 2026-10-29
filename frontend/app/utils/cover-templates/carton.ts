import type { CoverScene, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawCentered, drawContained, drawCovering, fitText, roundedRect, setTracking, sizeOf, wrapLines } from './draw.ts'

/** Carton d'invitation : double cadre fin, tout centré, la typographie seule ; aucune photo. */

const CX = COVER_WIDTH / 2
const TOP = 92
const BOTTOM = COVER_HEIGHT - 76
const TITLE_WIDTH = 1240
const NOTES_WIDTH = 1100

/** Le cartouche des logos, centré en pied ; rend sa hauteur, nulle sans logo. */
function drawLogoCartouche(ctx: CanvasRenderingContext2D, scene: CoverScene, bottom: number): number {
  const logos = [
    { image: scene.platformLogo, height: 52, max: 420 },
    { image: scene.organizationLogo, height: 42, max: 300 },
  ].flatMap(({ image, height, max }) => {
    if (!image) return []
    const { width, height: natural } = sizeOf(image)
    return [{ image, height, width: Math.min(max, (height * width) / natural) }]
  })
  if (logos.length === 0) return 0
  const gap = 26
  const boxW = logos.reduce((sum, logo) => sum + logo.width, 0) + (logos.length - 1) * (gap * 2 + 1) + 52
  const boxH = 52 + 28
  const x = CX - boxW / 2
  const y = bottom - boxH
  roundedRect(ctx, x, y, boxW, boxH, 6)
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  let cursor = x + 26
  logos.forEach((logo, index) => {
    if (index > 0) {
      ctx.fillStyle = scene.colors.rule
      ctx.fillRect(cursor + gap, y + (boxH - 46) / 2, 1, 46)
      cursor += gap * 2 + 1
    }
    drawContained(ctx, logo.image, cursor, y + (boxH - logo.height) / 2, logo.width, logo.height)
    cursor += logo.width
  })
  return boxH
}

function capitalize(text: string): string {
  return text.charAt(0).toLocaleUpperCase() + text.slice(1)
}

export const carton: CoverTemplate = {
  code: 'carton',
  usesPhoto: false,
  draw(ctx, scene, options) {
    const { accent, paper, text, muted } = scene.colors
    const family = scene.fontFamily
    ctx.fillStyle = paper
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    ctx.strokeStyle = accent
    ctx.lineWidth = 4
    ctx.strokeRect(42, 42, COVER_WIDTH - 84, COVER_HEIGHT - 84)
    ctx.lineWidth = 1
    ctx.strokeRect(56.5, 56.5, COVER_WIDTH - 113, COVER_HEIGHT - 113)

    const cartouche = options.show_logos ? drawLogoCartouche(ctx, scene, BOTTOM) : 0

    // La date d'une séance programmée prend son jour de la semaine, comme sur un carton ; sinon la plage de l'édition, telle quelle.
    const dateLine = options.show_date && scene.dateLine
      ? scene.date?.time
        ? `${capitalize(scene.date.weekday)} ${scene.dateLine}`
        : scene.dateLine
      : null
    const placeLine = options.show_date ? scene.placeLine : null
    const names = options.show_speakers ? scene.speakers.map((speaker) => speaker.name).join(', ') : ''

    const nameLines = names ? fitText(ctx, names, { maxWidth: NOTES_WIDTH, maxLines: 2, max: 24, min: 24, weight: 400, family }).lines : []
    ctx.font = `500 32px ${family}`
    const dateLines = dateLine ? wrapLines(ctx, dateLine, TITLE_WIDTH).slice(0, 2) : []
    const notesHeight =
      dateLines.length * 32 * 1.2 +
      (placeLine ? (dateLines.length > 0 ? 8 : 0) + 26 * 1.2 : 0) +
      (nameLines.length > 0 ? 16 + nameLines.length * 24 * 1.35 : 0)

    const icon = options.show_event_icon ? scene.icon : null
    const headHeight = (icon ? 84 + 26 : 0) + 21 * 1.2 + 59
    const reserved = (notesHeight > 0 ? 59 + notesHeight : 0) + (cartouche > 0 ? cartouche + 32 : 0)
    const titleBox = BOTTOM - reserved - TOP - headHeight
    setTracking(ctx, -0.7)
    let size = 68
    while (size > 32) {
      ctx.font = `700 ${size}px ${family}`
      if (wrapLines(ctx, scene.title, TITLE_WIDTH).length * size * 1.08 <= titleBox) break
      size -= 4
    }
    const title = fitText(ctx, scene.title, {
      maxWidth: TITLE_WIDTH,
      maxLines: Math.max(1, Math.floor(titleBox / (size * 1.08))),
      max: size,
      min: size,
      weight: 700,
      family,
    })
    setTracking(ctx, 0)
    // Le texte part du haut comme sur la maquette ; quand des options l'ont vidé, il redescend vers le milieu du carton.
    const slack = titleBox - title.lines.length * title.size * 1.08
    let y = TOP + Math.max(0, slack - 120) / 2

    if (icon) {
      ctx.save()
      roundedRect(ctx, CX - 42, y, 84, 84, 4)
      ctx.clip()
      drawCovering(ctx, icon, CX - 42, y, 84, 84)
      ctx.restore()
      y += 84 + 26
    }

    ctx.fillStyle = accent
    setTracking(ctx, 5)
    const eyebrow = fitText(ctx, scene.eyebrow.toUpperCase(), { maxWidth: TITLE_WIDTH, maxLines: 1, max: 21, min: 16, weight: 700, family })
    // L'espacement s'ajoute aussi après la dernière lettre : le décalage d'une demi-espace recentre la ligne.
    drawCentered(ctx, eyebrow.lines, CX + 2.5, y, eyebrow.size, 1.2)
    setTracking(ctx, 0)
    y += 21 * 1.2

    const ruleAt = (top: number) => {
      ctx.fillStyle = accent
      ctx.fillRect(CX - 36, top + 28, 72, 3)
      return top + 28 + 3 + 28
    }
    y = ruleAt(y)

    setTracking(ctx, -0.7)
    ctx.font = `700 ${title.size}px ${family}`
    ctx.fillStyle = text
    y = drawCentered(ctx, title.lines, CX, y, title.size, 1.08)
    setTracking(ctx, 0)

    if (notesHeight === 0) return
    y = ruleAt(y)
    if (dateLines.length > 0) {
      ctx.fillStyle = text
      ctx.font = `500 32px ${family}`
      y = drawCentered(ctx, dateLines, CX, y, 32, 1.2)
    }
    if (placeLine) {
      ctx.fillStyle = muted
      ctx.font = `400 26px ${family}`
      y = drawCentered(ctx, wrapLines(ctx, placeLine, TITLE_WIDTH).slice(0, 1), CX, y + (dateLines.length > 0 ? 8 : 0), 26, 1.2)
    }
    if (nameLines.length > 0) {
      ctx.fillStyle = muted
      ctx.font = `italic 400 24px ${family}`
      drawCentered(ctx, nameLines, CX, y + 16, 24, 1.35)
    }
  },
}
