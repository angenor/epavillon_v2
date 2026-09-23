import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { zoneFormulaKey, type ZoneFormula } from '../../app/utils/datetime.ts'

const common = JSON.parse(readFileSync(new URL('../../i18n/locales/fr/_common.json', import.meta.url), 'utf8'))

function rendu(formula: ZoneFormula, params: Record<string, string>): string {
  const gabarit: string = zoneFormulaKey(formula, params.zone ?? '')
    .split('.')
    .slice(1) // « common » est le nom du fichier, pas une clé
    .reduce((noeud, cle) => noeud[cle], common)
  return gabarit.replace(/\{(\w+)\}/g, (_, nom: string) => params[nom] ?? '')
}

test('« heure d’ » devant une voyelle', () => {
  for (const ville of ['Antalya', 'Abidjan', 'Istanbul']) {
    assert.equal(rendu('zoneOf', { zone: ville }), `heure d'${ville}`)
    assert.equal(rendu('timeWithZone', { time: '14:30', zone: ville }), `14:30, heure d'${ville}`)
    assert.equal(
      rendu('timeRangeWithZone', { start: '14:30', end: '16:00', zone: ville }),
      `14:30 — 16:00, heure d'${ville}`,
    )
  }
})

test('« heure de » devant une consonne', () => {
  for (const ville of ['Belém', 'Dakar']) {
    assert.equal(rendu('zoneOf', { zone: ville }), `heure de ${ville}`)
    assert.equal(rendu('timeWithZone', { time: '14:30', zone: ville }), `14:30, heure de ${ville}`)
    assert.equal(
      rendu('timeRangeWithZone', { start: '14:30', end: '16:00', zone: ville }),
      `14:30 — 16:00, heure de ${ville}`,
    )
  }
})

test('ni le « h » ni le « y » n’élident, faute de savoir', () => {
  assert.equal(rendu('zoneOf', { zone: 'Hanoï' }), 'heure de Hanoï')
  assert.equal(rendu('zoneOf', { zone: 'Yaoundé' }), 'heure de Yaoundé')
  assert.equal(rendu('zoneOf', { zone: 'Érévan' }), "heure d'Érévan")
})
