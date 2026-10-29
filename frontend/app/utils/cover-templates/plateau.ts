import type { CoverScene, CoverTemplate } from './draw.ts'
import {
  COVER_HEIGHT,
  COVER_WIDTH,
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

/** Plateau : les intervenants en bandes verticales à droite, comme sur scène ; sans intervenant, la photo reprend la place. */

const PANEL_X = 726
const PANEL_W = COVER_WIDTH - PANEL_X
const PAD = 64
const TEXT_WIDTH = 720 - PAD * 2

/** Taille des initiales et des noms selon le nombre de bandes : plus elles sont étroites, plus le texte se resserre. */
const INITIALS_SIZE = [0, 260, 220, 160, 120, 92]
const TILE_LIGHTEN = [0, 0.14, 0.3]

/** Recadrage qui garde le haut du portrait : un visage est rarement au centre exact d'une photo en pied. */
function drawPortrait(ctx: CanvasRenderingContext2D, image: CanvasImageSource, x: number, y: number, w: number, h: number): void {
  const { width, height } = sizeOf(image)
  const scale = Math.max(w / width, h / height)
  const sw = w / scale
  const sh = h / scale
  ctx.drawImage(image, (width - sw) / 2, (height - sh) * 0.3, sw, sh, x, y, w, h)
}

function drawSpeakerBands(ctx: CanvasRenderingContext2D, scene: CoverScene): void {
  const { accent, accentLight, paper, onAccent, text } = scene.colors
  const shown = scene.speakers.slice(0, 5)
  const count = shown.length
  const gap = 6
  const bandW = (PANEL_W - gap * (count - 1)) / count
  const nameSize = count <= 3 ? 32 : count === 4 ? 26 : 21
  const pad = count <= 3 ? 32 : 16
  shown.forEach((speaker, index) => {
    const x = PANEL_X + index * (bandW + gap)
    ctx.save()
    ctx.beginPath()
    ctx.rect(x, 0, bandW, COVER_HEIGHT)
    ctx.clip()
    ctx.fillStyle = accent
    ctx.fillRect(x, 0, bandW, COVER_HEIGHT)
    if (speaker.image) {
      drawPortrait(ctx, speaker.image, x, 0, bandW, COVER_HEIGHT)
      // Le voile sous le nom : seulement sur une photo, dont on ignore la clarté.
      const fade = ctx.createLinearGradient(0, COVER_HEIGHT - 360, 0, COVER_HEIGHT)
      fade.addColorStop(0, withAlpha(text, 0))
      fade.addColorStop(1, withAlpha(text, 0.8))
      ctx.fillStyle = fade
      ctx.fillRect(x, COVER_HEIGHT - 360, bandW, 360)
    } else {
      ctx.fillStyle = withAlpha(paper, TILE_LIGHTEN[index % 3] ?? 0)
      ctx.fillRect(x, 0, bandW, COVER_HEIGHT)
      const initials = initialsOfName(speaker.name)
      let size = INITIALS_SIZE[count] ?? 92
      ctx.font = `700 ${size}px ${scene.fontFamily}`
      size = Math.floor(size * Math.min(1, (bandW - 24) / ctx.measureText(initials).width))
      ctx.font = `700 ${size}px ${scene.fontFamily}`
      ctx.fillStyle = accentLight
      setTracking(ctx, -size * 0.03)
      ctx.textAlign = 'center'
      ctx.textBaseline = 'top'
      ctx.fillText(initials, x + bandW / 2, 300)
      setTracking(ctx, 0)
    }
    ctx.fillStyle = onAccent
    ctx.font = `700 ${nameSize}px ${scene.fontFamily}`
    const lines = wrapLines(ctx, speaker.name, bandW - pad * 2).slice(0, 3)
    drawLines(ctx, lines, x + pad, COVER_HEIGHT - 40 - lines.length * nameSize * 1.15, nameSize, 1.15)
    ctx.restore()
  })
}

/** La plus grande taille, de `max` à `min`, à laquelle le titre tient dans la hauteur donnée. */
function fitInBox(ctx: CanvasRenderingContext2D, scene: CoverScene, height: number, max: number, min: number): { lines: string[]; size: number } {
  let size = max
  while (size > min) {
    ctx.font = `700 ${size}px ${scene.fontFamily}`
    if (wrapLines(ctx, scene.title, TEXT_WIDTH).length * size * 1.05 <= height) break
    size -= 4
  }
  const maxLines = Math.max(1, Math.floor(height / (size * 1.05)))
  return fitText(ctx, scene.title, { maxWidth: TEXT_WIDTH, maxLines, max: size, min: size, weight: 700, family: scene.fontFamily })
}

export const plateau: CoverTemplate = {
  code: 'plateau',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, paper, text, muted } = scene.colors
    const family = scene.fontFamily
    ctx.fillStyle = paper
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)

    if (options.show_speakers && scene.speakers.length > 0) drawSpeakerBands(ctx, scene)
    else {
      ctx.fillStyle = accent
      ctx.fillRect(PANEL_X, 0, PANEL_W, COVER_HEIGHT)
      if (scene.photo) drawCovering(ctx, scene.photo, PANEL_X, 0, PANEL_W, COVER_HEIGHT)
    }

    let y = PAD
    const icon = options.show_event_icon ? scene.icon : null
    const eyebrowX = icon ? PAD + 72 + 20 : PAD
    ctx.font = `700 20px ${family}`
    setTracking(ctx, 2.8)
    const eyebrow = scene.eyebrow
      .toUpperCase()
      .split(' · ')
      .flatMap((part) => wrapLines(ctx, part, PAD + TEXT_WIDTH - eyebrowX))
      .slice(0, 3)
    const eyebrowHeight = eyebrow.length * 20 * 1.4
    const headHeight = Math.max(icon ? 72 : 0, eyebrowHeight)
    if (icon) {
      ctx.save()
      roundedRect(ctx, PAD, y + (headHeight - 72) / 2, 72, 72, 4)
      ctx.clip()
      drawCovering(ctx, icon, PAD, y + (headHeight - 72) / 2, 72, 72)
      ctx.restore()
    }
    ctx.fillStyle = accent
    drawLines(ctx, eyebrow, eyebrowX, y + (headHeight - eyebrowHeight) / 2 + 2, 20, 1.4)
    setTracking(ctx, 0)
    y += headHeight + 44

    // Le jour en grand, tel que la scène l'écrit dans la langue du site ; la ligne d'heure seulement si l'activité est programmée.
    const day = options.show_date ? (scene.dateLine?.split(' · ')[0] ?? null) : null
    const when = options.show_date && scene.date?.time ? [scene.date.weekday, scene.date.time, scene.date.zone].join(', ') : null
    const place = options.show_date ? scene.placeLine : null
    const logos = options.show_logos
      ? [
          { image: scene.platformLogo, height: 58, max: 400 },
          { image: scene.organizationLogo, height: 46, max: 260 },
        ].flatMap(({ image, ...size }) => (image ? [{ image, ...size }] : []))
      : []
    let footer = 0
    if (day) footer += 3 + 22 + 38 * 1.1
    if (when) footer += 8 + 26 * 1.2
    if (place) footer += (day ? 4 : 25) + 26 * 1.2
    if (logos.length > 0) footer += (footer > 0 ? 36 : 0) + 58
    const footerTop = COVER_HEIGHT - 56 - footer

    setTracking(ctx, -1)
    const title = fitInBox(ctx, scene, footerTop - 40 - y, 64, 36)
    ctx.fillStyle = text
    ctx.font = `700 ${title.size}px ${family}`
    drawLines(ctx, title.lines, PAD, y, title.size, 1.05)
    setTracking(ctx, 0)

    y = footerTop
    if (day || place) {
      ctx.fillStyle = accent
      ctx.fillRect(PAD, y, 96, 3)
      y += 25
    }
    const line = (value: string, weight: number, size: number, top: number, lineHeight = 1.2) =>
      drawLines(ctx, fitText(ctx, value, { maxWidth: TEXT_WIDTH, maxLines: 1, max: size, min: size, weight, family }).lines, PAD, top, size, lineHeight)
    ctx.fillStyle = text
    if (day) y = line(day, 700, 38, y, 1.1)
    if (when) y = line(when, 400, 26, y + 8)
    ctx.fillStyle = muted
    if (place) y = line(place, 400, 26, y + (day ? 4 : 0))
    if (logos.length > 0) {
      let cursor = PAD
      const top = y + (day || place ? 36 : 0)
      for (const logo of logos) {
        const { width, height } = sizeOf(logo.image)
        const w = Math.min(PAD + TEXT_WIDTH - cursor, logo.max, (logo.height * width) / height)
        drawContained(ctx, logo.image, cursor, top + (58 - logo.height) / 2, w, logo.height)
        cursor += w + 28
      }
    }
  },
}
