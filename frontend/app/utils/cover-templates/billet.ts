import type { CoverScene, CoverSpeaker, CoverTemplate } from './draw.ts'
import {
  COVER_HEIGHT,
  COVER_WIDTH,
  drawBlurred,
  drawContained,
  drawCovering,
  drawLines,
  fitText,
  initialsOfName,
  roundedRect,
  setTracking,
  sizeOf,
  withAlpha,
  wrapLines,
} from './draw.ts'

/** Billet : un ticket d'entrée posé sur la photo floutée, la date en grand sur le talon détachable. */

const TICKET = { x: 120, y: 130, w: 1360, h: 640, r: 16 }
/** Le talon commence à l'axe des encoches. */
const STUB_X = TICKET.x + 1000
const NOTCH = 28
const LEFT = TICKET.x + 56
const TEXT_WIDTH = 1000 - 56 * 2
const TILE_LIGHTEN = [0, 0.14, 0.3]

function ticketPath(ctx: CanvasRenderingContext2D): void {
  const { x, y, w, h, r } = TICKET
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.lineTo(STUB_X - NOTCH, y)
  ctx.arc(STUB_X, y, NOTCH, Math.PI, 0, true)
  ctx.lineTo(x + w - r, y)
  ctx.arcTo(x + w, y, x + w, y + r, r)
  ctx.lineTo(x + w, y + h - r)
  ctx.arcTo(x + w, y + h, x + w - r, y + h, r)
  ctx.lineTo(STUB_X + NOTCH, y + h)
  ctx.arc(STUB_X, y + h, NOTCH, 0, Math.PI, true)
  ctx.lineTo(x + r, y + h)
  ctx.arcTo(x, y + h, x, y + h - r, r)
  ctx.lineTo(x, y + r)
  ctx.arcTo(x, y, x + r, y, r)
  ctx.closePath()
}

function drawDisc(ctx: CanvasRenderingContext2D, scene: CoverScene, speaker: CoverSpeaker, cx: number, cy: number, r: number, index: number): void {
  const { accent, paper, onAccent } = scene.colors
  ctx.save()
  ctx.beginPath()
  ctx.arc(cx, cy, r, 0, Math.PI * 2)
  ctx.fillStyle = paper
  ctx.fill()
  ctx.beginPath()
  ctx.arc(cx, cy, r - 4, 0, Math.PI * 2)
  ctx.clip()
  if (speaker.image) drawCovering(ctx, speaker.image, cx - r, cy - r, r * 2, r * 2)
  else {
    ctx.fillStyle = accent
    ctx.fillRect(cx - r, cy - r, r * 2, r * 2)
    ctx.fillStyle = withAlpha(paper, TILE_LIGHTEN[index % 3] ?? 0)
    ctx.fillRect(cx - r, cy - r, r * 2, r * 2)
    ctx.fillStyle = onAccent
    ctx.font = `700 ${Math.round(r * 0.66)}px ${scene.fontFamily}`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(initialsOfName(speaker.name), cx, cy)
  }
  ctx.restore()
}

/** Le talon : jour de la semaine, quantième, mois et année, heure, puis le lieu en pied. */
function drawStub(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const { accentLight, onAccent } = scene.colors
  const family = scene.fontFamily
  const x = STUB_X + 48
  const width = TICKET.x + TICKET.w - 40 - x
  const top = TICKET.y + 44
  const bottom = TICKET.y + TICKET.h - 40
  const line = (text: string, weight: number, size: number, y: number, lineHeight = 1.2) =>
    drawLines(ctx, fitText(ctx, text, { maxWidth: width, maxLines: 1, max: size, min: Math.min(size, 20), weight, family }).lines, x, y, size, lineHeight)

  let y = top
  const date = scene.date
  if (date) {
    ctx.fillStyle = accentLight
    // Le jour de la semaine n'a de sens que pour une séance programmée ; sinon c'est celui de l'ouverture de l'édition.
    if (date.time) y = line(date.weekday, 400, 26, y)
    ctx.fillStyle = onAccent
    setTracking(ctx, -9.5)
    ctx.font = `700 190px ${family}`
    // Cale sur la ligne de base : une boîte de 0,8 em, comme la maquette, que le chiffre déborde à peine.
    ctx.textBaseline = 'alphabetic'
    ctx.fillText(date.day, x - 8, y + 14 + 146)
    setTracking(ctx, 0)
    y += 14 + 190 * 0.8
    setTracking(ctx, 2.4)
    y = line(`${date.monthShort} ${date.year}`.toUpperCase(), 700, 30, y + 18)
    setTracking(ctx, 0)
    if (date.time) {
      ctx.fillStyle = accentLight
      ctx.fillRect(x, y + 26, width, 2)
      ctx.fillStyle = onAccent
      y = line(date.time, 700, 58, y + 54, 1)
      ctx.fillStyle = accentLight
      y = line(date.zone, 400, 22, y + 6)
    }
  }

  const place = [scene.room, scene.city].filter((part): part is string => Boolean(part))
  if (place.length === 0 && scene.placeLine) place.push(scene.placeLine)
  ctx.fillStyle = onAccent
  ctx.font = `400 22px ${family}`
  const lines = place.flatMap((part) => wrapLines(ctx, part, width)).slice(0, 3)
  const placeTop = bottom - lines.length * 22 * 1.35
  if (placeTop > y + 16) drawLines(ctx, lines, x, placeTop, 22, 1.35)
}

function drawLogos(ctx: CanvasRenderingContext2D, scene: CoverScene, y: number): void {
  const right = LEFT + TEXT_WIDTH
  const slots = [
    { image: scene.platformLogo, height: 54, max: 440, align: 'left' },
    { image: scene.organizationLogo, height: 44, max: 320, align: 'right' },
  ]
  for (const { image, height, max, align } of slots) {
    if (!image) continue
    const { width, height: natural } = sizeOf(image)
    const w = Math.min(max, (height * width) / natural)
    drawContained(ctx, image, align === 'left' ? LEFT : right - w, y + (56 - height) / 2, w, height)
  }
}

export const billet: CoverTemplate = {
  code: 'billet',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, paper, text, muted, rule } = scene.colors
    const family = scene.fontFamily
    ctx.fillStyle = accent
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    if (scene.photo) {
      drawBlurred(ctx, scene.photo, 0, 0, COVER_WIDTH, COVER_HEIGHT, 28)
      ctx.fillStyle = withAlpha(accent, 0.5)
      ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    } else {
      // Sans photo, le fond fonce d'un cran : le talon, de la couleur d'accent, s'en détache.
      ctx.fillStyle = withAlpha(text, 0.25)
      ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    }

    ctx.save()
    ticketPath(ctx)
    ctx.clip()
    ctx.fillStyle = paper
    ctx.fillRect(TICKET.x, TICKET.y, TICKET.w, TICKET.h)
    ctx.fillStyle = accent
    ctx.fillRect(STUB_X, TICKET.y, TICKET.x + TICKET.w - STUB_X, TICKET.h)
    if (options.show_date) drawStub(ctx, scene)
    ctx.restore()

    let y = TICKET.y + 52
    const icon = options.show_event_icon ? scene.icon : null
    const eyebrowX = icon ? LEFT + 64 + 20 : LEFT
    if (icon) {
      ctx.save()
      roundedRect(ctx, LEFT, y, 64, 64, 4)
      ctx.clip()
      drawCovering(ctx, icon, LEFT, y, 64, 64)
      ctx.restore()
    }
    setTracking(ctx, 2.8)
    const eyebrow = fitText(ctx, scene.eyebrow.toUpperCase(), { maxWidth: LEFT + TEXT_WIDTH - eyebrowX, maxLines: 1, max: 20, min: 16, weight: 700, family })
    ctx.fillStyle = accent
    ctx.textBaseline = 'middle'
    ctx.fillText(eyebrow.lines[0] ?? '', eyebrowX, y + (icon ? 32 : 12))
    setTracking(ctx, 0)
    y += (icon ? 64 : 24) + 32

    const speakers = options.show_speakers ? scene.speakers.slice(0, 5) : []
    const logos = options.show_logos && (scene.platformLogo || scene.organizationLogo)
    let bottom = TICKET.y + TICKET.h - 44
    const logosY = bottom - 56
    if (logos) bottom = logosY - 22 - 1 - 22
    const speakersY = bottom - 64
    if (speakers.length > 0) bottom = speakersY

    setTracking(ctx, -0.6)
    const titleBox = bottom - 28 - y
    let size = 60
    while (size > 32) {
      ctx.font = `700 ${size}px ${family}`
      if (wrapLines(ctx, scene.title, TEXT_WIDTH).length * size * 1.06 <= titleBox) break
      size -= 4
    }
    const title = fitText(ctx, scene.title, {
      maxWidth: TEXT_WIDTH,
      maxLines: Math.max(1, Math.floor(titleBox / (size * 1.06))),
      max: size,
      min: size,
      weight: 700,
      family,
    })
    ctx.fillStyle = text
    drawLines(ctx, title.lines, LEFT, y, title.size, 1.06)
    setTracking(ctx, 0)

    if (speakers.length > 0) {
      const r = 32
      const step = r * 2 - 12
      speakers.forEach((speaker, index) => drawDisc(ctx, scene, speaker, LEFT + r + index * step, speakersY + r, r, index))
      const namesX = LEFT + (speakers.length - 1) * step + r * 2 + 20
      const names = fitText(ctx, scene.speakers.map((speaker) => speaker.name).join(', '), {
        maxWidth: LEFT + TEXT_WIDTH - namesX,
        maxLines: 2,
        max: 22,
        min: 22,
        weight: 400,
        family,
      })
      ctx.fillStyle = muted
      drawLines(ctx, names.lines, namesX, speakersY + 32 - (names.lines.length * 22 * 1.3) / 2, 22, 1.3)
    }
    if (logos) {
      ctx.fillStyle = rule
      ctx.fillRect(LEFT, logosY - 23, TEXT_WIDTH, 1)
      drawLogos(ctx, scene, logosY)
    }
  },
}
