import type { CoverTemplate } from './draw.ts'
import { COVER_WIDTH, bottomOf, drawBackdrop, drawChrome, drawFace, drawFade, drawLines, drawTextBlock, fitText, wrapLines } from './draw.ts'

/** Les quatre premiers modèles (29/10). */

// Dégradés et voiles : seulement sur une photo (exception arbitrée du guide, § « Le verre »).

export const fullPhoto: CoverTemplate = {
  code: 'photo-pleine',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    const { accent, scrim, onAccent } = scene.colors
    drawBackdrop(ctx, scene, bottom)
    // La couleur monte de la gauche, le voile du bas : le texte blanc reste lisible sur une photo claire.
    drawFade(ctx, bottom, 'horizontal', [
      [0, accent, 0.7],
      [0.55, accent, 0],
    ])
    drawFade(ctx, bottom, 'vertical', [
      [0, scrim, 0.1],
      [0.4, scrim, 0.15],
      [1, scrim, 0.82],
    ])

    const faces = options.show_speakers ? scene.speakers.slice(0, 4) : []
    const facesWidth = faces.length > 0 ? faces.length * 116 + 40 : 0
    const title = fitText(ctx, scene.title, {
      maxWidth: COVER_WIDTH - 112 - 72 - facesWidth,
      maxLines: 3,
      max: 80,
      min: 40,
      weight: 700,
      family: scene.fontFamily,
    })
    const dateHeight = options.show_date && scene.dateLine ? 30 * 1.3 + 18 : 0
    const blockHeight = 26 * 1.3 + 16 + title.lines.length * title.size * 1.1 + dateHeight
    const top = bottom - 72 - blockHeight

    ctx.fillStyle = accent
    ctx.fillRect(72, top, 12, blockHeight)
    ctx.fillStyle = onAccent
    ctx.font = `600 26px ${scene.fontFamily}`
    let y = drawLines(ctx, [scene.eyebrow.toUpperCase()], 112, top, 26, 1.3) + 16
    ctx.font = `700 ${title.size}px ${scene.fontFamily}`
    y = drawLines(ctx, title.lines, 112, y, title.size, 1.1) + 18
    if (options.show_date && scene.dateLine) {
      ctx.font = `500 30px ${scene.fontFamily}`
      drawLines(ctx, [scene.dateLine], 112, y, 30, 1.3)
    }
    faces.forEach((speaker, index) => {
      const cx = COVER_WIDTH - 72 - 52 - (faces.length - 1 - index) * 116
      drawFace(ctx, scene, speaker, cx, bottom - 72 - 52, 52, onAccent)
    })
    drawChrome(ctx, scene, options)
  },
}

export const splitPhoto: CoverTemplate = {
  code: 'texte-photo',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    const { accent, onAccent } = scene.colors
    drawBackdrop(ctx, scene, bottom)
    // L'aplat porte le texte à gauche et se fond dans la photo, sans coupure nette.
    drawFade(ctx, bottom, 'horizontal', [
      [0, accent, 1],
      [0.44, accent, 1],
      [0.74, accent, 0],
    ])
    const y = drawTextBlock(
      ctx,
      scene,
      options,
      { x: 72, y: 64, width: Math.round(COVER_WIDTH * 0.46) - 72, maxTitleLines: 4, titleMax: 76 },
      { title: onAccent, text: onAccent, eyebrow: onAccent },
    )
    if (options.show_speakers) {
      scene.speakers.slice(0, 5).forEach((speaker, index) => {
        drawFace(ctx, scene, speaker, 72 + 46 + index * 104, Math.min(y + 70, bottom - 70), 46, onAccent)
      })
    }
    drawChrome(ctx, scene, options)
  },
}

export const faces: CoverTemplate = {
  code: 'visages',
  usesPhoto: true,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    const { accent, onAccent } = scene.colors
    drawBackdrop(ctx, scene, bottom)
    // La photo reste une texture sous la couleur : les visages portent l'image.
    drawFade(ctx, bottom, 'horizontal', [
      [0, accent, 0.97],
      [0.55, accent, 0.9],
      [1, accent, 0.72],
    ])
    const textWidth = options.show_speakers && scene.speakers.length > 0 ? 820 : COVER_WIDTH - 144
    drawTextBlock(
      ctx,
      scene,
      options,
      { x: 72, y: 64, width: textWidth, maxTitleLines: 4, titleMax: 80 },
      { title: onAccent, text: onAccent, eyebrow: onAccent },
    )
    if (options.show_speakers) {
      const shown = scene.speakers.slice(0, 4)
      const columns = shown.length > 2 ? 2 : shown.length
      const radius = 96
      const middle = bottom / 2
      shown.forEach((speaker, index) => {
        const column = index % 2
        const row = Math.floor(index / 2)
        const rows = Math.ceil(shown.length / columns)
        const cx = 1000 + radius + column * 270
        const cy = middle - ((rows - 1) * 250) / 2 + row * 250 - 20
        drawFace(ctx, scene, speaker, cx, cy, radius, onAccent)
        ctx.fillStyle = onAccent
        ctx.font = `600 24px ${scene.fontFamily}`
        ctx.textAlign = 'center'
        ctx.textBaseline = 'top'
        ctx.fillText(wrapLines(ctx, speaker.name, 250)[0] ?? '', cx, cy + radius + 14)
      })
    }
    drawChrome(ctx, scene, options)
  },
}

export const typographic: CoverTemplate = {
  code: 'typographique',
  usesPhoto: false,
  draw(ctx, scene, options) {
    const bottom = bottomOf(options)
    ctx.fillStyle = scene.colors.paper
    ctx.fillRect(0, 0, COVER_WIDTH, bottom)
    ctx.fillStyle = scene.colors.accent
    ctx.fillRect(72, 64, 140, 16)
    const y = drawTextBlock(
      ctx,
      scene,
      options,
      { x: 72, y: 112, width: COVER_WIDTH - 144 - (options.show_event_icon ? 160 : 0), maxTitleLines: 3, titleMax: 104 },
      { title: scene.colors.accent, text: scene.colors.ink, eyebrow: scene.colors.muted },
    )
    if (options.show_speakers && scene.speakers.length > 0 && y < bottom - 60) {
      ctx.fillStyle = scene.colors.muted
      ctx.font = `500 28px ${scene.fontFamily}`
      const names = scene.speakers.map((speaker) => speaker.name).join(', ')
      drawLines(ctx, wrapLines(ctx, names, COVER_WIDTH - 144).slice(0, 2), 72, y + 8, 28, 1.35)
    }
    drawChrome(ctx, scene, options)
  },
}

