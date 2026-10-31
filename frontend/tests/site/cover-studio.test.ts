import { test } from 'node:test'
import assert from 'node:assert/strict'
import { CUSTOM_ACCENT, findCoverTemplate, fitText, hexToHsl, hslToHex, whiteContrast, withAlpha, wrapLines } from '../../app/utils/cover-studio.ts'

/** Un mesureur sans navigateur : chaque caractère vaut un dixième de la taille de police. */
function measurer() {
  return {
    font: '400 40px sans-serif',
    measureText(text: string) {
      const size = Number(/(\d+)px/.exec(this.font)?.[1] ?? 40)
      return { width: text.length * size * 0.5 }
    },
  }
}

test('les lignes se coupent entre les mots, un mot trop long reste entier', () => {
  const ctx = measurer()
  ctx.font = '400 20px sans-serif'
  assert.deepEqual(wrapLines(ctx, 'Restauration des mangroves du golfe', 200), ['Restauration des', 'mangroves du golfe'])
  assert.deepEqual(wrapLines(ctx, 'Anticonstitutionnellement', 50), ['Anticonstitutionnellement'])
})

test('le titre prend la plus grande taille qui tient, sinon il s’achève par des points de suspension', () => {
  const ctx = measurer()
  const short = fitText(ctx, 'Mangroves', { maxWidth: 800, maxLines: 2, max: 100, min: 40, weight: 700, family: 'sans-serif' })
  assert.equal(short.size, 100)
  assert.deepEqual(short.lines, ['Mangroves'])

  const long = fitText(ctx, 'un deux trois quatre cinq six sept huit neuf dix onze douze', {
    maxWidth: 200,
    maxLines: 2,
    max: 80,
    min: 40,
    weight: 700,
    family: 'sans-serif',
  })
  assert.equal(long.size, 40)
  assert.equal(long.lines.length, 2)
  assert.ok(long.lines[1]?.endsWith('…'))
})

test('le voile dose l’opacité d’un jeton opaque', () => {
  assert.equal(withAlpha('#231f20', 0.5), 'rgba(35, 31, 32, 0.5)')
  assert.equal(withAlpha('#fff', 1), 'rgba(255, 255, 255, 1)')
})

test('un code de modèle inconnu retombe sur le premier modèle du catalogue', () => {
  assert.equal(findCoverTemplate('disparu').code, 'photo-pleine')
  assert.equal(findCoverTemplate('visages').code, 'visages')
})

test('une couleur libre fait l’aller-retour teinte, intensité, clarté sans dériver', () => {
  for (const hex of ['#1b2a6b', '#a3172e', '#4e6b1f', '#000000']) {
    const { hue, saturation, lightness } = hexToHsl(hex)
    const back = hslToHex(hue, saturation, lightness)
    const gap = [1, 3, 5].map((i) => Math.abs(Number.parseInt(back.slice(i, i + 2), 16) - Number.parseInt(hex.slice(i, i + 2), 16)))
    assert.ok(Math.max(...gap) <= 3, `${hex} → ${back}`)
    assert.match(back, CUSTOM_ACCENT)
  }
})

test('le texte blanc est jugé lisible sur une couleur foncée, pas sur une claire', () => {
  assert.ok(whiteContrast('#1b2a6b') >= 4.5)
  assert.ok(whiteContrast('#f5d76e') < 4.5)
  assert.equal(Math.round(whiteContrast('#000000')), 21)
})
