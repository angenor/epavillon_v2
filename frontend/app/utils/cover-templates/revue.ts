import type { CoverScene, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawContained, drawCovering, drawFace, drawLines, fitText, roundedRect, setTracking, sizeOf, wrapLines } from './draw.ts'

/** Revue : une page de revue scientifique, grand titre noir sur blanc, date et lieu en colonnes, la photo tient une colonne à droite. */

const LEFT = 80
const WIDTH = 816
const BOTTOM = COVER_HEIGHT - 56
const COLUMN_X = 960
const COLUMN_GAP = 24

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

// Les intitulés « Date » et « Lieu » de la maquette deviennent des pictogrammes : pas de chaîne d'interface dans l'image.
function calendarGlyph(ctx: CanvasRenderingContext2D, x: number, y: number, size: number): void {
  ctx.beginPath()
  ctx.roundRect(x + 1, y + 3, size - 2, size - 4, 3)
  ctx.moveTo(x + 1, y + size * 0.4)
  ctx.lineTo(x + size - 1, y + size * 0.4)
  ctx.moveTo(x + size * 0.3, y)
  ctx.lineTo(x + size * 0.3, y + 6)
  ctx.moveTo(x + size * 0.7, y)
  ctx.lineTo(x + size * 0.7, y + 6)
  ctx.stroke()
}

function pinGlyph(ctx: CanvasRenderingContext2D, x: number, y: number, size: number): void {
  const cx = x + size / 2
  const r = size * 0.36
  ctx.beginPath()
  ctx.arc(cx, y + r + 1, r, Math.PI * 0.82, Math.PI * 2.18)
  ctx.lineTo(cx, y + size)
  ctx.closePath()
  ctx.stroke()
  ctx.beginPath()
  ctx.arc(cx, y + r + 1, r * 0.38, 0, Math.PI * 2)
  ctx.stroke()
}

function oneLine(ctx: CanvasRenderingContext2D, scene: CoverScene, text: string, width: number, size: number, weight: number): string {
  return fitText(ctx, text, { maxWidth: width, maxLines: 1, max: size, min: size - 6, weight, family: scene.fontFamily }).lines[0] ?? ''
}

/** La plus grande taille à laquelle le titre tient dans la hauteur donnée : un titre long gagne des lignes plutôt que de rapetisser. */
function fitInHeight(ctx: CanvasRenderingContext2D, scene: CoverScene, text: string, height: number) {
  const options = { maxWidth: WIDTH, weight: 700, family: scene.fontFamily }
  for (let size = 84; size > 40; size -= 2) {
    const maxLines = Math.floor(height / (size * 1.04))
    if (maxLines < 1) continue
    ctx.font = `700 ${size}px ${scene.fontFamily}`
    if (wrapLines(ctx, text, WIDTH).length <= maxLines) return fitText(ctx, text, { ...options, maxLines, max: size, min: size })
  }
  return fitText(ctx, text, { ...options, maxLines: Math.max(1, Math.floor(height / (40 * 1.04))), max: 40, min: 40 })
}

export const revue: CoverTemplate = {
  code: 'revue',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, onAccent, paper, text, muted, rule } = scene.colors
    const font = scene.fontFamily
    const date = options.show_date ? scene.date : null

    ctx.fillStyle = paper
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    if (scene.photo) {
      drawCovering(ctx, scene.photo, COLUMN_X, 0, COVER_WIDTH - COLUMN_X, COVER_HEIGHT)
    } else {
      ctx.fillStyle = accent
      ctx.fillRect(COLUMN_X, 0, COVER_WIDTH - COLUMN_X, COVER_HEIGHT)
      if (date) {
        ctx.fillStyle = onAccent
        ctx.textAlign = 'left'
        ctx.textBaseline = 'alphabetic'
        setTracking(ctx, -17)
        ctx.font = `700 340px ${font}`
        ctx.fillText(date.day, COLUMN_X + 64, 752)
        setTracking(ctx, 0)
        ctx.fillText(oneLine(ctx, scene, `${date.month} ${date.year}`, 520, 44, 700), COLUMN_X + 64, 834)
      }
    }
    ctx.fillStyle = accent
    ctx.fillRect(COLUMN_X - 16, 0, 16, COVER_HEIGHT)
    if (options.show_event_icon) iconTile(ctx, scene, 1448, 40, 112, 8, 8)

    // Filet de tête : le surtitre à gauche, la date courte à droite, sur la même ligne de base.
    let dateWidth = 0
    if (date) {
      const short = `${date.day} ${date.monthShort} ${date.year}`
      ctx.font = `500 22px ${font}`
      dateWidth = ctx.measureText(short).width
      ctx.fillStyle = text
      ctx.textAlign = 'right'
      ctx.textBaseline = 'alphabetic'
      ctx.fillText(short, LEFT + WIDTH, 84)
    }
    setTracking(ctx, 3)
    ctx.fillStyle = accent
    ctx.textAlign = 'left'
    ctx.textBaseline = 'alphabetic'
    ctx.fillText(oneLine(ctx, scene, scene.eyebrow.toUpperCase(), WIDTH - dateWidth - 32, 22, 700), LEFT, 84)
    setTracking(ctx, 0)
    ctx.fillStyle = text
    ctx.fillRect(LEFT, 108, WIDTH, 3)

    // Le pied se compose du bas vers le haut ; le titre prend la place qui reste.
    let y = BOTTOM
    const logos = options.show_logos ? [scene.platformLogo, scene.organizationLogo] : []
    if (logos.some(Boolean)) {
      const [platform, organization] = logos
      let used = 0
      if (platform) {
        used = logoWidth(platform, 66, 440)
        drawContained(ctx, platform, LEFT, y - 69, used, 66)
      }
      if (organization) {
        const w = logoWidth(organization, 58, WIDTH - used - 40)
        drawContained(ctx, organization, LEFT + WIDTH - w, y - 65, w, 58)
      }
      y -= 72 + 32
    }

    const columnWidth = (WIDTH - COLUMN_GAP) / 2
    const when = date ? [`${date.day} ${date.month} ${date.year}${date.time ? `, ${date.time}` : ''}`, date.time ? date.zone : null] : []
    const where = options.show_date ? (scene.room || scene.city ? [scene.room, scene.city] : [scene.placeLine]) : []
    const columns = [
      { lines: when.filter((line): line is string => Boolean(line)), glyph: calendarGlyph },
      { lines: where.filter((line): line is string => Boolean(line)), glyph: pinGlyph },
    ].filter((column) => column.lines.length > 0)
    if (columns.length > 0) {
      const rows = Math.max(...columns.map((column) => column.lines.length))
      const top = y - (1 + 20 + 22 + 6 + rows * 28 * 1.3)
      ctx.fillStyle = rule
      ctx.fillRect(LEFT, top, WIDTH, 1)
      columns.forEach((column, index) => {
        const x = LEFT + index * (columnWidth + COLUMN_GAP)
        ctx.strokeStyle = accent
        ctx.lineWidth = 2.5
        ctx.lineJoin = 'round'
        column.glyph(ctx, x, top + 21, 22)
        ctx.fillStyle = text
        const lines = column.lines.map((line) => oneLine(ctx, scene, line, columnWidth, 28, 500))
        drawLines(ctx, lines, x, top + 21 + 28, 28, 1.3)
      })
      y = top
    }

    const speakers = options.show_speakers ? scene.speakers.slice(0, 6) : []
    if (speakers.length > 0) {
      const rowTop = y - 28 - 68
      speakers.forEach((speaker, index) => drawFace(ctx, scene, speaker, LEFT + 34 + index * 54, rowTop + 34, 34, paper))
      const namesX = LEFT + 68 + (speakers.length - 1) * 54 + 22
      const names = fitText(ctx, scene.speakers.map((speaker) => speaker.name).join(', '), {
        maxWidth: LEFT + WIDTH - namesX,
        maxLines: 2,
        max: 24,
        min: 20,
        weight: 400,
        family: font,
      })
      ctx.fillStyle = muted
      drawLines(ctx, names.lines, namesX, rowTop + (68 - names.lines.length * names.size * 1.3) / 2, names.size, 1.3)
      y = rowTop
    }

    const titleTop = 151
    setTracking(ctx, -1.2)
    const title = fitInHeight(ctx, scene, scene.title, y - 40 - titleTop)
    ctx.fillStyle = text
    drawLines(ctx, title.lines, LEFT, titleTop, title.size, 1.04)
    setTracking(ctx, 0)
  },
}
