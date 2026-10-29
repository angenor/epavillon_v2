import type { CoverDate, CoverScene, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawContained, drawCovering, drawFace, drawLines, fitText, roundedRect, setTracking, sizeOf, wrapLines } from './draw.ts'

/**
 * Encart : la photo intacte en plein cadre, tout le texte dans une carte blanche
 * posée en bas à gauche ; les intervenants, nom en étiquette, en colonne à droite.
 */

const CARD_X = 64
const CARD_WIDTH = 1000
const PAD_X = 52
const INNER = CARD_WIDTH - PAD_X * 2

function font(scene: CoverScene, weight: number, size: number): string {
  return `${weight} ${size}px ${scene.fontFamily}`
}

function capitalized(text: string): string {
  return text.charAt(0).toLocaleUpperCase() + text.slice(1)
}

/** Deux colonnes, chacune une ligne forte et une ligne discrète : quand, puis où. */
function infoColumns(scene: CoverScene): { main: string; sub: string | null }[] {
  const date: CoverDate | null = scene.date
  const when = date
    ? { main: `${capitalized(date.weekday)} ${date.day} ${date.month} ${date.year}${date.time ? `, ${date.time}` : ''}`, sub: date.time ? date.zone : null }
    : scene.dateLine
      ? { main: scene.dateLine, sub: null }
      : null
  const place = scene.room ?? scene.city
  const where = place ? { main: place, sub: scene.room ? scene.city : null } : null
  return [when, where].filter((column): column is { main: string; sub: string | null } => column !== null)
}

/** La plus grande taille, de `max` à 36, à laquelle le titre tient dans la boîte ; à 36, des points de suspension. */
function fitTitle(ctx: CanvasRenderingContext2D, scene: CoverScene, width: number, height: number, max: number): { lines: string[]; size: number } {
  for (let size = max; size >= 36; size -= 4) {
    const maxLines = Math.min(4, Math.max(1, Math.floor(height / (size * 1.05))))
    setTracking(ctx, -0.01 * size)
    const fit = fitText(ctx, scene.title, { maxWidth: width, maxLines, max: size, min: size, weight: 700, family: scene.fontFamily })
    if (size === 36 || wrapLines(ctx, scene.title, width).length <= maxLines) return fit
  }
  return { lines: [], size: 36 }
}

function drawSpeakers(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const { paper, text } = scene.colors
  const right = COVER_WIDTH - 64
  scene.speakers.slice(0, 5).forEach((speaker, index) => {
    const cy = 64 + 44 + index * 104
    drawFace(ctx, scene, speaker, right - 44, cy, 44, paper)
    ctx.font = font(scene, 700, 24)
    const name = fitText(ctx, speaker.name, { maxWidth: 310, maxLines: 1, max: 24, min: 24, weight: 700, family: scene.fontFamily }).lines[0] ?? ''
    const width = ctx.measureText(name).width + 32
    const x = right - 88 - 14 - width
    roundedRect(ctx, x, cy - 22, width, 44, 6)
    ctx.fillStyle = paper
    ctx.fill()
    ctx.fillStyle = text
    ctx.textBaseline = 'middle'
    ctx.fillText(name, x + 16, cy + 1)
  })
}

export const encart: CoverTemplate = {
  code: 'encart',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, paper, rule, text, muted } = scene.colors
    ctx.fillStyle = accent
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    if (scene.photo) drawCovering(ctx, scene.photo, 0, 0, COVER_WIDTH, COVER_HEIGHT)

    const icon = options.show_event_icon ? scene.icon : null
    if (icon) {
      ctx.save()
      roundedRect(ctx, 64, 64, 104, 104, 8)
      ctx.fillStyle = paper
      ctx.fill()
      roundedRect(ctx, 70, 70, 92, 92, 4)
      ctx.clip()
      drawCovering(ctx, icon, 70, 70, 92, 92)
      ctx.restore()
    }
    if (options.show_speakers) drawSpeakers(ctx, scene)

    // La carte se mesure avant d'être dessinée : elle est calée par le bas.
    const columns = options.show_date ? infoColumns(scene) : []
    const columnWidth = columns.length > 1 ? (INNER - 65) / 2 : INNER
    const infoHeight = columns.length > 0 ? 26 + 35 + (columns.some((column) => column.sub) ? 30 : 0) : 0
    const logos = options.show_logos ? [scene.platformLogo, scene.organizationLogo].filter((logo) => logo !== null) : []
    const logosHeight = logos.length > 0 ? 24 + 1 + 24 + 54 : 0
    const fixed = 10 + 40 + 24 + 14 + infoHeight + logosHeight + 34
    const top = icon ? 64 + 104 + 32 : 64
    const title = fitTitle(ctx, scene, INNER, COVER_HEIGHT - 64 - top - fixed, 64)
    const cardHeight = fixed + title.lines.length * title.size * 1.05
    const cardY = COVER_HEIGHT - 64 - cardHeight

    // Liseré et corps en deux formes : un liseré découpé dans le blanc laisserait un filet clair sur l'aplat.
    ctx.beginPath()
    ctx.roundRect(CARD_X, cardY, CARD_WIDTH, 10, [8, 8, 0, 0])
    ctx.fillStyle = accent
    ctx.fill()
    ctx.beginPath()
    ctx.roundRect(CARD_X, cardY + 10, CARD_WIDTH, cardHeight - 10, [0, 0, 8, 8])
    ctx.fillStyle = paper
    ctx.fill()

    const x = CARD_X + PAD_X
    let y = cardY + 50
    ctx.fillStyle = accent
    ctx.font = font(scene, 700, 20)
    setTracking(ctx, 2.8)
    const eyebrow = fitText(ctx, scene.eyebrow.toLocaleUpperCase(), { maxWidth: INNER, maxLines: 1, max: 20, min: 20, weight: 700, family: scene.fontFamily })
    y = drawLines(ctx, eyebrow.lines, x, y, 20, 1.2) + 14

    ctx.fillStyle = text
    ctx.font = font(scene, 700, title.size)
    setTracking(ctx, -0.01 * title.size)
    y = drawLines(ctx, title.lines, x, y, title.size, 1.05)
    setTracking(ctx, 0)

    if (columns.length > 0) {
      y += 26
      columns.forEach((column, index) => {
        const cx = x + index * (columnWidth + 65)
        if (index > 0) {
          ctx.fillStyle = rule
          ctx.fillRect(cx - 33, y, 1, infoHeight - 26)
        }
        ctx.fillStyle = text
        ctx.font = font(scene, 700, 29)
        const main = fitText(ctx, column.main, { maxWidth: columnWidth, maxLines: 1, max: 29, min: 22, weight: 700, family: scene.fontFamily })
        drawLines(ctx, main.lines, cx, y, main.size, 1.2)
        if (column.sub) {
          ctx.fillStyle = muted
          ctx.font = font(scene, 400, 22)
          drawLines(ctx, fitText(ctx, column.sub, { maxWidth: columnWidth, maxLines: 1, max: 22, min: 22, weight: 400, family: scene.fontFamily }).lines, cx, y + 39, 22, 1.2)
        }
      })
      y += infoHeight - 26
    }

    if (logos.length > 0) {
      ctx.fillStyle = rule
      ctx.fillRect(x, y + 24, INNER, 1)
      const row = y + 49
      if (scene.platformLogo) drawContained(ctx, scene.platformLogo, x, row, Math.min(54 * (sizeOf(scene.platformLogo).width / sizeOf(scene.platformLogo).height), 440), 54)
      if (scene.organizationLogo) {
        const { width, height } = sizeOf(scene.organizationLogo)
        const w = Math.min(44 * (width / height), 320)
        drawContained(ctx, scene.organizationLogo, x + INNER - w, row + 5, w, 44)
      }
    }
  },
}
