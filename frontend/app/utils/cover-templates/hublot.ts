import type { CoverScene, CoverTemplate } from './draw.ts'
import { COVER_HEIGHT, COVER_WIDTH, drawContained, drawCovering, drawFace, drawLines, fitText, roundedRect, setTracking, sizeOf, withAlpha, wrapLines } from './draw.ts'

/**
 * Hublot : la photo dans un grand disque cerclé de blanc, à droite, les
 * intervenants posés sur son bord ; le texte sur l'aplat à gauche. Sans photo,
 * le disque devient un médaillon de date.
 */

const CX = 1330
const CY = 450
const RADIUS = 450
const LEFT = 80
const WIDTH = 720

function font(scene: CoverScene, weight: number, size: number): string {
  return `${weight} ${size}px ${scene.fontFamily}`
}

function capitalized(text: string): string {
  return text.charAt(0).toLocaleUpperCase() + text.slice(1)
}

function drawPorthole(ctx: CanvasRenderingContext2D, scene: CoverScene, showDate: boolean): void {
  const { paper, accent, accentLight, onAccent, scrim } = scene.colors
  ctx.save()
  ctx.beginPath()
  ctx.arc(CX, CY, RADIUS + 12, 0, Math.PI * 2)
  ctx.fillStyle = paper
  ctx.fill()
  ctx.beginPath()
  ctx.arc(CX, CY, RADIUS, 0, Math.PI * 2)
  ctx.clip()
  if (scene.photo) {
    drawCovering(ctx, scene.photo, CX - RADIUS, CY - RADIUS, RADIUS * 2, RADIUS * 2)
  } else {
    // L'accent assombri : le médaillon se détache de l'aplat sans quitter la couleur choisie.
    ctx.fillStyle = accent
    ctx.fillRect(CX - RADIUS, 0, RADIUS * 2, COVER_HEIGHT)
    ctx.fillStyle = withAlpha(scrim, 0.28)
    ctx.fillRect(CX - RADIUS, 0, RADIUS * 2, COVER_HEIGHT)
    const date = scene.date
    if (showDate && date) {
      // Centré sur la partie visible du disque, que le bord droit du cadre coupe.
      const mid = CX - RADIUS + 360
      ctx.textAlign = 'center'
      ctx.textBaseline = 'alphabetic'
      ctx.fillStyle = onAccent
      ctx.font = font(scene, 700, 300)
      setTracking(ctx, -15)
      ctx.fillText(date.day, mid, 517)
      ctx.fillStyle = accentLight
      ctx.font = font(scene, 700, 44)
      setTracking(ctx, 3.5)
      ctx.fillText(`${date.monthShort.toLocaleUpperCase()} ${date.year}`, mid, 605)
      setTracking(ctx, 0)
    }
  }
  ctx.restore()
}

/** Les deux logos dans une capsule blanche ; rend sa hauteur, 0 sans logo. */
function drawLogoCapsule(ctx: CanvasRenderingContext2D, scene: CoverScene, x: number, bottom: number): number {
  const logos = [
    { image: scene.platformLogo, h: 50 },
    { image: scene.organizationLogo, h: 40 },
  ].flatMap(({ image, h }) => (image ? [{ image, h, w: Math.min(h * (sizeOf(image).width / sizeOf(image).height), 280) }] : []))
  if (logos.length === 0) return 0
  const width = logos.reduce((sum, logo) => sum + logo.w, 0) + (logos.length - 1) * 45 + 44
  const height = 78
  const y = bottom - height
  ctx.save()
  roundedRect(ctx, x, y, width, height, 8)
  ctx.fillStyle = scene.colors.paper
  ctx.fill()
  let cursor = x + 22
  logos.forEach((logo, index) => {
    if (index > 0) {
      ctx.fillStyle = scene.colors.rule
      ctx.fillRect(cursor + 22, y + 17, 1, 44)
      cursor += 45
    }
    drawContained(ctx, logo.image, cursor, y + (height - logo.h) / 2, logo.w, logo.h)
    cursor += logo.w
  })
  ctx.restore()
  return height
}

export const hublot: CoverTemplate = {
  code: 'hublot',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const { accent, accentLight, onAccent, paper } = scene.colors
    ctx.fillStyle = accent
    ctx.fillRect(0, 0, COVER_WIDTH, COVER_HEIGHT)
    drawPorthole(ctx, scene, options.show_date)

    // Sur le bord gauche du disque, en éventail autour de l'horizontale.
    const speakers = options.show_speakers ? scene.speakers.slice(0, 7) : []
    speakers.forEach((speaker, index) => {
      const angle = ((180 + (index - (speakers.length - 1) / 2) * 21) * Math.PI) / 180
      drawFace(ctx, scene, speaker, CX + RADIUS * Math.cos(angle), CY + RADIUS * Math.sin(angle), 50, paper, index)
    })

    // En-tête : icône de l'édition et surtitre.
    const icon = options.show_event_icon ? scene.icon : null
    if (icon) {
      ctx.save()
      roundedRect(ctx, LEFT, 72, 72, 72, 6)
      ctx.fillStyle = paper
      ctx.fill()
      roundedRect(ctx, LEFT + 4, 76, 64, 64, 3)
      ctx.clip()
      drawCovering(ctx, icon, LEFT + 4, 76, 64, 64)
      ctx.restore()
    }
    const eyebrowX = icon ? LEFT + 92 : LEFT
    ctx.fillStyle = onAccent
    ctx.font = font(scene, 700, 21)
    setTracking(ctx, 2.94)
    const eyebrow = fitText(ctx, scene.eyebrow.toLocaleUpperCase(), { maxWidth: LEFT + WIDTH - eyebrowX, maxLines: 2, max: 21, min: 21, weight: 700, family: scene.fontFamily })
    const eyebrowHeight = eyebrow.lines.length * 21 * 1.4
    drawLines(ctx, eyebrow.lines, eyebrowX, icon ? 108 - eyebrowHeight / 2 : 72, 21, 1.4)
    setTracking(ctx, 0)
    const headBottom = 72 + Math.max(icon ? 72 : 0, eyebrowHeight)

    const footTop = options.show_logos ? COVER_HEIGHT - 64 - drawLogoCapsule(ctx, scene, LEFT, COVER_HEIGHT - 64) : COVER_HEIGHT - 64

    // Bloc central : titre, date, lieu, noms ; centré entre l'en-tête et les logos.
    const date = scene.date
    const dateMain = date
      ? `${capitalized(date.weekday)} ${date.day} ${date.month} ${date.year}${date.time ? ` · ${date.time}` : ''}`
      : scene.dateLine
    const dateSub = [date?.time ? date.zone : null, scene.placeLine].filter(Boolean).join(' · ')
    ctx.font = font(scene, 400, 22)
    const names = speakers.length > 0 ? fitText(ctx, speakers.map((speaker) => speaker.name).join(', '), { maxWidth: WIDTH, maxLines: 2, max: 22, min: 22, weight: 400, family: scene.fontFamily }).lines : []
    const details =
      (options.show_date && dateMain ? 30 + 36 : 0) +
      (options.show_date && dateSub ? 6 + 30 : 0) +
      (names.length > 0 ? 14 + names.length * 22 * 1.35 : 0)
    const room = footTop - headBottom - 48 - details
    let title = { lines: [] as string[], size: 36 }
    for (let size = 64; size >= 36; size -= 4) {
      const maxLines = Math.min(4, Math.max(1, Math.floor(room / (size * 1.05))))
      setTracking(ctx, -0.015 * size)
      title = fitText(ctx, scene.title, { maxWidth: WIDTH, maxLines, max: size, min: size, weight: 700, family: scene.fontFamily })
      if (wrapLines(ctx, scene.title, WIDTH).length <= maxLines) break
    }
    const blockHeight = title.lines.length * title.size * 1.05 + details
    let y = headBottom + Math.max(24, (footTop - headBottom - blockHeight) / 2)
    ctx.fillStyle = onAccent
    y = drawLines(ctx, title.lines, LEFT, y, title.size, 1.05)
    setTracking(ctx, 0)

    if (options.show_date && dateMain) {
      ctx.font = font(scene, 700, 30)
      const fit = fitText(ctx, dateMain, { maxWidth: WIDTH, maxLines: 1, max: 30, min: 22, weight: 700, family: scene.fontFamily })
      y = drawLines(ctx, fit.lines, LEFT, y + 30, fit.size, 1.2)
    }
    ctx.fillStyle = accentLight
    if (options.show_date && dateSub) {
      ctx.font = font(scene, 400, 25)
      const fit = fitText(ctx, dateSub, { maxWidth: WIDTH, maxLines: 1, max: 25, min: 20, weight: 400, family: scene.fontFamily })
      y = drawLines(ctx, fit.lines, LEFT, y + 6, fit.size, 1.2)
    }
    if (names.length > 0) {
      ctx.font = font(scene, 400, 22)
      drawLines(ctx, names, LEFT, y + 14, 22, 1.35)
    }
  },
}
