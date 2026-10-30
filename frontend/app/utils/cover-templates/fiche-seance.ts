import type { CoverScene, CoverSpeaker, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawContained, drawCovering, drawFace, drawLines, fitText, roundedRect, setTracking, sizeOf, withAlpha, wrapLines } from './draw.ts'

/**
 * Fiche de séance : un bandeau photo coupé net par le liseré aux cinq couleurs,
 * le titre sur papier, puis un tableau date, heure, lieu, intervenants qu'on lit
 * comme une fiche de programme. Chaque colonne s'annonce par un pictogramme.
 */

const LEFT = 72
const WIDTH = COVER_WIDTH - 144
const BAND = 300
const TABLE_TOP = 604

type Glyph = 'calendar' | 'clock' | 'pin' | 'people'
type Cell = { glyph: Glyph; lines: string[] } | { glyph: 'people'; speakers: CoverSpeaker[] }

function font(scene: CoverScene, weight: number, size: number): string {
  return `${weight} ${size}px ${scene.fontFamily}`
}

function capitalized(text: string): string {
  return text.charAt(0).toLocaleUpperCase() + text.slice(1)
}

/** La photo recadrée comme `drawCovering`, mais centrée à 40 % de sa hauteur : les visages sont rarement au milieu. */
function drawCoveringHigh(ctx: CanvasRenderingContext2D, image: CanvasImageSource, w: number, h: number): void {
  const { width, height } = sizeOf(image)
  const scale = Math.max(w / width, h / height)
  const sw = w / scale
  const sh = h / scale
  ctx.drawImage(image, (width - sw) / 2, (height - sh) * 0.4, sw, sh, 0, 0, w, h)
}

/** Pictogramme au trait de 24 px, à la place des intitulés de colonne. */
function drawGlyph(ctx: CanvasRenderingContext2D, glyph: Glyph, x: number, y: number, color: string): void {
  ctx.save()
  ctx.strokeStyle = color
  ctx.lineWidth = 2.5
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  ctx.beginPath()
  if (glyph === 'calendar') {
    ctx.roundRect(x + 1, y + 3, 22, 20, 3)
    ctx.moveTo(x + 1, y + 10)
    ctx.lineTo(x + 23, y + 10)
    ctx.moveTo(x + 7, y)
    ctx.lineTo(x + 7, y + 5)
    ctx.moveTo(x + 17, y)
    ctx.lineTo(x + 17, y + 5)
  } else if (glyph === 'clock') {
    ctx.arc(x + 12, y + 12, 11, 0, Math.PI * 2)
    ctx.moveTo(x + 12, y + 6)
    ctx.lineTo(x + 12, y + 12)
    ctx.lineTo(x + 16, y + 15)
  } else if (glyph === 'pin') {
    ctx.moveTo(x + 12, y + 23)
    ctx.bezierCurveTo(x + 5, y + 15, x + 3, y + 12, x + 3, y + 9)
    ctx.arc(x + 12, y + 9, 9, Math.PI, 0)
    ctx.bezierCurveTo(x + 21, y + 12, x + 19, y + 15, x + 12, y + 23)
    ctx.moveTo(x + 15, y + 9)
    ctx.arc(x + 12, y + 9, 3, 0, Math.PI * 2)
  } else {
    ctx.arc(x + 9, y + 7, 4.5, 0, Math.PI * 2)
    ctx.moveTo(x + 1, y + 22)
    ctx.arc(x + 9, y + 22, 8, Math.PI, 0)
    ctx.moveTo(x + 21.5, y + 8)
    ctx.arc(x + 18, y + 8, 3.5, 0, Math.PI * 2)
    ctx.moveTo(x + 19, y + 14)
    ctx.quadraticCurveTo(x + 23, y + 15, x + 23, y + 21)
  }
  ctx.stroke()
  ctx.restore()
}

function cellsOf(scene: CoverScene, showDate: boolean, showSpeakers: boolean): Cell[] {
  const cells: Cell[] = []
  if (showDate) {
    const date = scene.date
    if (date) cells.push({ glyph: 'calendar', lines: [capitalized(date.weekday), `${date.day} ${date.month} ${date.year}`] })
    else if (scene.dateLine) cells.push({ glyph: 'calendar', lines: [scene.dateLine] })
    if (date?.time) cells.push({ glyph: 'clock', lines: [date.time, date.zone] })
    const place = scene.room ?? scene.city
    if (place) cells.push({ glyph: 'pin', lines: scene.room && scene.city ? [scene.room, scene.city] : [place] })
  }
  if (showSpeakers && scene.speakers.length > 0) cells.push({ glyph: 'people', speakers: scene.speakers })
  return cells
}

/** Visages qui se chevauchent ; au-delà de ce que la colonne loge, un disque « +N ». */
function drawFaceStack(ctx: CanvasRenderingContext2D, scene: CoverScene, speakers: CoverSpeaker[], x: number, cy: number, width: number): number {
  const { accent, paper, onAccent } = scene.colors
  const fits = Math.max(1, Math.floor((width - 60) / 48) + 1)
  const shown = speakers.length > fits ? speakers.slice(0, fits - 1) : speakers
  shown.forEach((speaker, index) => drawFace(ctx, scene, speaker, x + 30 + index * 48, cy, 30, paper, index))
  const rest = speakers.length - shown.length
  if (rest > 0) {
    const cx = x + 30 + shown.length * 48
    ctx.beginPath()
    ctx.arc(cx, cy, 30, 0, Math.PI * 2)
    ctx.fillStyle = paper
    ctx.fill()
    ctx.beginPath()
    ctx.arc(cx, cy, 26, 0, Math.PI * 2)
    ctx.fillStyle = accent
    ctx.fill()
    ctx.fillStyle = onAccent
    ctx.font = font(scene, 700, 20)
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(`+${rest}`, cx, cy + 1)
    ctx.textAlign = 'left'
  }
  return 60 + (shown.length + (rest > 0 ? 1 : 0) - 1) * 48
}

export const ficheSeance: CoverTemplate = {
  code: 'fiche-seance',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, paper, onAccent, text, rule, scrim, stripes } = scene.colors
    ctx.fillStyle = paper
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    ctx.fillStyle = accent
    ctx.fillRect(0, 0, COVER_WIDTH, BAND)
    if (scene.photo) {
      drawCoveringHigh(ctx, scene.photo, COVER_WIDTH, BAND)
      // Voile du bas seulement : il porte le surtitre blanc posé sur la photo.
      const fade = ctx.createLinearGradient(0, 0, 0, BAND)
      fade.addColorStop(0.25, withAlpha(scrim, 0))
      fade.addColorStop(1, withAlpha(scrim, 0.75))
      ctx.fillStyle = fade
      ctx.fillRect(0, 0, COVER_WIDTH, BAND)
    }
    stripes.forEach((color, index) => {
      ctx.fillStyle = color
      const x = Math.round((index * COVER_WIDTH) / stripes.length)
      ctx.fillRect(x, BAND, Math.round(((index + 1) * COVER_WIDTH) / stripes.length) - x, 12)
    })

    const icon = options.show_event_icon ? scene.icon : null
    if (icon) {
      const x = COVER_WIDTH - 72 - 104
      ctx.save()
      roundedRect(ctx, x, 40, 104, 104, 8)
      ctx.fillStyle = paper
      ctx.fill()
      roundedRect(ctx, x + 6, 46, 92, 92, 4)
      ctx.clip()
      drawCovering(ctx, icon, x + 6, 46, 92, 92)
      ctx.restore()
    }
    ctx.fillStyle = onAccent
    ctx.font = font(scene, 700, 24)
    setTracking(ctx, 3.36)
    const eyebrow = fitText(ctx, scene.eyebrow.toLocaleUpperCase(), { maxWidth: WIDTH, maxLines: 1, max: 24, min: 24, weight: 700, family: scene.fontFamily })
    drawLines(ctx, eyebrow.lines, LEFT, 234, 24, 1.2)

    const cells = cellsOf(scene, options.show_date, options.show_speakers)
    const titleBottom = cells.length > 0 ? TABLE_TOP - 28 : options.show_logos ? 740 : 840
    let title = { lines: [] as string[], size: 40 }
    for (let size = cells.length > 0 ? 68 : 96; size >= 40; size -= 4) {
      const maxLines = Math.max(1, Math.floor((titleBottom - 358) / (size * 1.05)))
      setTracking(ctx, -0.015 * size)
      title = fitText(ctx, scene.title, { maxWidth: WIDTH, maxLines, max: size, min: size, weight: 700, family: scene.fontFamily })
      if (wrapLines(ctx, scene.title, WIDTH).length <= maxLines) break
    }
    ctx.fillStyle = text
    drawLines(ctx, title.lines, LEFT, 358, title.size, 1.05)
    setTracking(ctx, 0)

    if (cells.length > 0) {
      ctx.fillStyle = text
      ctx.fillRect(LEFT, TABLE_TOP, WIDTH, 3)
      const columnWidth = WIDTH / cells.length
      cells.forEach((cell, index) => {
        const x = LEFT + index * columnWidth
        const padLeft = index === 0 ? 0 : 22
        const inner = columnWidth - padLeft - (index === cells.length - 1 ? 0 : 22)
        if (index > 0) {
          ctx.fillStyle = rule
          ctx.fillRect(x, TABLE_TOP + 3, 1, 128)
        }
        drawGlyph(ctx, cell.glyph, x + padLeft, TABLE_TOP + 19, accent)
        const top = TABLE_TOP + 51
        if ('speakers' in cell) {
          const stack = drawFaceStack(ctx, scene, cell.speakers, x + padLeft, top + 32, inner > 600 ? inner / 2 : inner)
          const free = inner - stack - 16
          if (free >= 180) {
            ctx.fillStyle = text
            ctx.font = font(scene, 500, 24)
            const names = fitText(ctx, cell.speakers.map((speaker) => speaker.name).join(', '), { maxWidth: free, maxLines: 2, max: 24, min: 24, weight: 500, family: scene.fontFamily }).lines
            drawLines(ctx, names, x + padLeft + stack + 16, top + 32 - (names.length * 24 * 1.3) / 2, 24, 1.3)
          }
          return
        }
        ctx.fillStyle = text
        cell.lines.forEach((line, row) => {
          ctx.font = font(scene, 500, 27)
          const fit = fitText(ctx, line, { maxWidth: inner, maxLines: 1, max: 27, min: 20, weight: 500, family: scene.fontFamily })
          drawLines(ctx, fit.lines, x + padLeft, top + row * 35, fit.size, 1.3)
        })
      })
    }

    if (options.show_logos) {
      const y = COVER_HEIGHT - 44 - 56
      ctx.fillStyle = rule
      ctx.fillRect(LEFT, y - 22, WIDTH, 1)
      if (scene.platformLogo) {
        const { width, height } = sizeOf(scene.platformLogo)
        drawContained(ctx, scene.platformLogo, LEFT, y, Math.min(56 * (width / height), 460), 56)
      }
      if (scene.organizationLogo) {
        const { width, height } = sizeOf(scene.organizationLogo)
        const w = Math.min(46 * (width / height), 340)
        drawContained(ctx, scene.organizationLogo, LEFT + WIDTH - w, y + 5, w, 46)
      }
    }
  },
}
