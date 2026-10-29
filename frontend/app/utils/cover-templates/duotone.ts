import type { CoverScene, CoverSpeaker, CoverTemplate } from './draw.ts'
import {
  COVER_HEIGHT,
  COVER_WIDTH,
  drawContained,
  drawCovering,
  drawDuotone,
  drawLines,
  fitText,
  initialsOfName,
  roundedRect,
  setTracking,
  sizeOf,
  withAlpha,
  wrapLines,
} from './draw.ts'

/** Duotone : la photo passe aux deux tons de l'accent, n'importe quelle image s'accorde à la charte et porte le texte blanc. */

const LEFT = 96
const TEXT_WIDTH = 1000

/** Les pastilles sans portrait alternent trois nuances de l'accent, éclaircies par le papier. */
const TILE_LIGHTEN = [0, 0.14, 0.3]

function drawDisc(ctx: CanvasRenderingContext2D, scene: CoverScene, speaker: CoverSpeaker, cx: number, cy: number, r: number, index: number): void {
  const { accent, paper, onAccent } = scene.colors
  ctx.save()
  ctx.beginPath()
  ctx.arc(cx, cy, r, 0, Math.PI * 2)
  ctx.fillStyle = paper
  ctx.fill()
  ctx.beginPath()
  ctx.arc(cx, cy, r - 5, 0, Math.PI * 2)
  ctx.clip()
  if (speaker.image) drawCovering(ctx, speaker.image, cx - r, cy - r, r * 2, r * 2)
  else {
    ctx.fillStyle = accent
    ctx.fillRect(cx - r, cy - r, r * 2, r * 2)
    ctx.fillStyle = withAlpha(paper, TILE_LIGHTEN[index % 3] ?? 0)
    ctx.fillRect(cx - r, cy - r, r * 2, r * 2)
    ctx.fillStyle = onAccent
    ctx.font = `700 ${Math.round(r * 0.63)}px ${scene.fontFamily}`
    ctx.textAlign = 'center'
    ctx.textBaseline = 'middle'
    ctx.fillText(initialsOfName(speaker.name), cx, cy)
  }
  ctx.restore()
}

/** Le cartouche blanc des logos, ancré en haut à droite : les logos sont dessinés pour un fond clair. */
function drawLogoCartouche(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const logos = [
    { image: scene.platformLogo, height: 56, max: 420 },
    { image: scene.organizationLogo, height: 46, max: 300 },
  ].flatMap(({ image, height, max }) => {
    if (!image) return []
    const { width, height: natural } = sizeOf(image)
    return [{ image, height, width: Math.min(max, (height * width) / natural) }]
  })
  if (logos.length === 0) return
  const gap = 28
  const inner = logos.reduce((sum, logo) => sum + logo.width, 0) + (logos.length - 1) * (gap * 2 + 1)
  const boxW = inner + 56
  const boxH = 56 + 36
  const x = COVER_WIDTH - 56 - boxW
  const y = 56
  roundedRect(ctx, x, y, boxW, boxH, 8)
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  let cursor = x + 28
  logos.forEach((logo, index) => {
    if (index > 0) {
      ctx.fillStyle = scene.colors.rule
      ctx.fillRect(cursor + gap, y + (boxH - 52) / 2, 1, 52)
      cursor += gap * 2 + 1
    }
    drawContained(ctx, logo.image, cursor, y + (boxH - logo.height) / 2, logo.width, logo.height)
    cursor += logo.width
  })
}

export const duotone: CoverTemplate = {
  code: 'duotone',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, accentLight, paper, onAccent } = scene.colors
    const family = scene.fontFamily
    ctx.fillStyle = accent
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    if (scene.photo) {
      // Lumières en teinte claire, ramenées vers une nuance moyenne : le blanc du texte garde son contraste.
      drawDuotone(ctx, scene.photo, 0, 0, COVER_WIDTH, COVER_HEIGHT, accent, accentLight)
      ctx.fillStyle = withAlpha(accent, 0.55)
      ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
      const fade = ctx.createLinearGradient(0, 0, COVER_WIDTH, 0)
      fade.addColorStop(0, withAlpha(accent, 0.92))
      fade.addColorStop(0.45, withAlpha(accent, 0.75))
      fade.addColorStop(0.85, withAlpha(accent, 0))
      ctx.fillStyle = fade
      ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    }

    const icon = options.show_event_icon ? scene.icon : null
    const eyebrowX = icon ? LEFT + 80 + 22 : LEFT
    setTracking(ctx, 3.2)
    const eyebrow = fitText(ctx, scene.eyebrow.toUpperCase(), {
      maxWidth: LEFT + TEXT_WIDTH - eyebrowX,
      maxLines: 1,
      max: 23,
      min: 18,
      weight: 700,
      family,
    })
    setTracking(ctx, -1.8)
    const title = fitText(ctx, scene.title, { maxWidth: TEXT_WIDTH, maxLines: 3, max: 92, min: 44, weight: 700, family })
    setTracking(ctx, 0)
    const titleHeight = title.lines.length * title.size * 1.03

    const dateLine = options.show_date ? scene.dateLine : null
    const placeLine = options.show_date ? scene.placeLine : null
    ctx.font = `400 24px ${family}`
    const names = options.show_speakers ? scene.speakers.map((speaker) => speaker.name).join(', ') : ''
    const nameLines = names ? fitText(ctx, names, { maxWidth: TEXT_WIDTH, maxLines: 2, max: 24, min: 24, weight: 400, family }).lines : []

    const headHeight = icon ? 80 : 23 * 1.2
    const height =
      headHeight +
      36 +
      titleHeight +
      (dateLine ? 40 + 60 : 0) +
      (placeLine ? 14 + 27 * 1.2 : 0) +
      (nameLines.length > 0 ? 12 + nameLines.length * 24 * 1.35 : 0)
    let y = Math.max(48, (COVER_HEIGHT - height) / 2)

    if (icon) {
      ctx.save()
      roundedRect(ctx, LEFT, y, 80, 80, 6)
      ctx.fillStyle = paper
      ctx.fill()
      roundedRect(ctx, LEFT + 5, y + 5, 70, 70, 3)
      ctx.clip()
      drawCovering(ctx, icon, LEFT + 5, y + 5, 70, 70)
      ctx.restore()
    }
    ctx.fillStyle = onAccent
    ctx.font = `700 ${eyebrow.size}px ${family}`
    ctx.textBaseline = 'middle'
    setTracking(ctx, 3.2)
    ctx.fillText(eyebrow.lines[0] ?? '', eyebrowX, y + headHeight / 2)
    y += headHeight + 36

    setTracking(ctx, -1.8)
    ctx.font = `700 ${title.size}px ${family}`
    y = drawLines(ctx, title.lines, LEFT, y, title.size, 1.03)
    setTracking(ctx, 0)

    if (dateLine) {
      y += 40
      const pill = fitText(ctx, dateLine, { maxWidth: TEXT_WIDTH - 44, maxLines: 1, max: 30, min: 22, weight: 700, family })
      ctx.font = `700 ${pill.size}px ${family}`
      const text = pill.lines[0] ?? ''
      roundedRect(ctx, LEFT, y, ctx.measureText(text).width + 44, 60, 6)
      ctx.fillStyle = paper
      ctx.fill()
      ctx.fillStyle = accent
      ctx.textBaseline = 'middle'
      ctx.fillText(text, LEFT + 22, y + 31)
      y += 60
    }
    if (placeLine) {
      ctx.fillStyle = onAccent
      ctx.font = `400 27px ${family}`
      y = drawLines(ctx, wrapLines(ctx, placeLine, TEXT_WIDTH).slice(0, 1), LEFT, y + 14, 27, 1.2)
    }
    if (nameLines.length > 0) {
      ctx.fillStyle = accentLight
      ctx.font = `400 24px ${family}`
      drawLines(ctx, nameLines, LEFT, y + 12, 24, 1.35)
    }

    if (options.show_logos) drawLogoCartouche(ctx, scene)

    if (options.show_speakers) {
      const shown = scene.speakers.slice(0, 5)
      const r = 54
      const step = r * 2 - 20
      const first = COVER_WIDTH - 72 - r - (shown.length - 1) * step
      shown.forEach((speaker, index) => drawDisc(ctx, scene, speaker, first + index * step, COVER_HEIGHT - 72 - r, r, index))
    }
  },
}
