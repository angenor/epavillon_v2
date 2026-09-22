import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { BLOCS_DE_MA_JOURNEE, jourLisible, memeJour } from '../../app/utils/guide-nego/journee.ts'
import { initialesDe } from '../../app/utils/guide-nego/compte.ts'

const traductions = (locale: string, page: string) =>
  JSON.parse(readFileSync(new URL(`../../i18n/locales/${locale}/pages/guide-nego.${page}.json`, import.meta.url), 'utf8'))

test('les cinq blocs paraissent dans l’ordre fixe (FR-013)', () => {
  assert.deepEqual(BLOCS_DE_MA_JOURNEE, [
    'prochaine-session',
    'changements',
    'trois-agendas',
    'documents',
    'lexique',
  ])
})

/**
 * Chacun des quatre premiers porte **son** état vide, en une ligne, et rien n'y
 * évoque une panne (FR-014) : un bloc vide n'est pas un échec.
 */
test('chaque bloc vide dit ce qui manque, sans un mot de panne', () => {
  for (const locale of ['fr', 'en']) {
    const { blocs } = traductions(locale, 'accueil')
    for (const bloc of BLOCS_DE_MA_JOURNEE.filter((b) => b !== 'lexique')) {
      assert.ok(blocs[bloc]?.titre, `${locale} — ${bloc} : titre`)
      assert.ok(blocs[bloc]?.vide, `${locale} — ${bloc} : ligne vide`)
      assert.doesNotMatch(
        blocs[bloc].vide,
        /erreur|échec|échoué|indisponible|impossible|panne|error|fail|unavailable/i,
        `${locale} — ${bloc}`,
      )
    }
    assert.ok(blocs.lexique?.ouvrir, `${locale} — accès au lexique`)
  }
})

/** « Thème » désigne l'apparence, jamais une thématique ; « Programme » seul est banni. */
test('ni « Thème » ni « Programme » seul dans l’accueil et les thématiques', () => {
  for (const page of ['accueil', 'thematiques']) {
    const texte = JSON.stringify(traductions('fr', page))
    assert.doesNotMatch(texte, /(^|[^\p{L}])th[èe]mes?([^\p{L}]|$)/iu, page)
    assert.doesNotMatch(texte, /(^|[^\p{L}])programmes?([^\p{L}]|$)/iu, page)
  }
})

test('le sous-titre porte le jour, et pas de fuseau', () => {
  const jeudi = new Date(2026, 10, 12, 9, 30)
  assert.equal(jourLisible(jeudi, 'fr'), 'Jeudi 12 novembre')
  assert.equal(jourLisible(jeudi, 'en'), 'Thursday, November 12')
})

/** Une application restée en mémoire ne doit pas ouvrir le lendemain sur la veille. */
test('le jour change à minuit, pas à l’heure qui passe', () => {
  const soir = new Date(2026, 10, 12, 23, 59)
  assert.equal(memeJour(soir, new Date(2026, 10, 12, 0, 1)), true)
  assert.equal(memeJour(soir, new Date(2026, 10, 13, 0, 1)), false)
})

test('l’avatar prend les initiales, accents compris, et rien sans nom lu', () => {
  assert.equal(initialesDe('Awa', 'Diallo'), 'AD')
  assert.equal(initialesDe('émilie', 'traoré'), 'ÉT')
  assert.equal(initialesDe(null, null), '')
  assert.equal(initialesDe('  Awa ', null), 'A')
})
