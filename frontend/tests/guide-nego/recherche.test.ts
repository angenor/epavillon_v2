import { test } from 'node:test'
import assert from 'node:assert/strict'
import type { DocumentReading, ReadingPage } from '../../app/types/negotiation-documents.ts'
import {
  chercherDansLeDocument,
  passageAReperer,
  passageDeLaPage,
  pagesCherchees,
  surligner,
} from '../../app/utils/guide-nego/lecteur.ts'
import { repererPassage } from '../../app/utils/guide-nego/pdf/reperer.ts'
import { replier } from '../../app/utils/guide-nego/repli.ts'

const PHRASE =
  "La CdP30 représente une étape stratégique pour finaliser les éléments clés permettant un suivi robuste des progrès collectifs en matière d'adaptation."

function lecture(pages: ReadingPage[]): DocumentReading {
  return {
    id: 'guide',
    version: '1.0',
    mode: 'reflow',
    page_count: pages.length,
    outline: [
      {
        title: '3.6. Adaptation',
        level: 2,
        page_index: 2,
        children: [{ title: '3.6.1. GGA', level: 3, page_index: 2, children: [] }],
      },
    ],
    pages,
  }
}

const page = (index: number, blocks: ReadingPage['blocks']): ReadingPage => ({ index, label: String(index + 57), blocks })

test('« progres collectifs » sans accent trouve « progrès collectifs », avec sa page et sa section', () => {
  const doc = lecture([
    page(1, [{ kind: 'paragraph', spans: [{ text: 'Rien ici.' }] }]),
    page(2, [
      { kind: 'heading', level: 3, spans: [{ text: '3.6.1. GGA' }] },
      { kind: 'paragraph', spans: [{ text: PHRASE }] },
    ]),
  ])
  const [passage, ...autres] = chercherDansLeDocument(doc, '  PROGRES   collectifs ')
  assert.equal(autres.length, 0)
  assert.equal(passage?.etiquette, '59')
  assert.equal(passage?.section, '3.6.1. GGA', 'un passage dit sa sous-partie')
  assert.equal(passage?.extrait.trouve, 'progrès collectifs', 'le surlignage porte sur le texte d’origine')
  assert.match(passage?.extrait.avant ?? '', /^… .*suivi robuste des $/)
  assert.equal(pagesCherchees(doc), 2)
})

test('les positions renvoient au texte d’origine, même quand le repli change sa longueur', () => {
  const texte = 'Œuvre  commune'
  const { replie, origine } = replier(texte)
  assert.equal(replie, 'oeuvre commune', 'deux blancs n’en font qu’un')
  assert.equal(origine[0], 0)
  assert.equal(origine[1], 0, 'les deux lettres de « œ » renvoient à la même')
  assert.equal(origine[2], 1)
})

test('une expression coupée entre deux segments se trouve et se surligne des deux côtés', () => {
  const doc = lecture([
    page(1, [{ kind: 'paragraph', spans: [{ text: 'le ' }, { text: 'global goal', italic: true, term: true }, { text: ' on adaptation' }] }]),
  ])
  const [p] = chercherDansLeDocument(doc, 'goal on')
  assert.ok(p)
  const coupes = surligner(doc.pages[0]?.blocks[0]?.kind === 'paragraph' ? doc.pages[0].blocks[0].spans : [], [
    { debut: p.debut, fin: p.fin, courant: true },
  ])
  assert.deepEqual(
    coupes.map((s) => [s.text, s.surlignage ?? null, s.term ?? false, s.source]),
    [
      ['le ', null, false, 0],
      ['global ', null, true, 1],
      ['goal', 'courant', true, 1],
      [' on', 'courant', false, 2],
      [' adaptation', null, false, 2],
    ],
  )
  assert.equal(coupes.map((s) => s.text).join(''), 'le global goal on adaptation', 'le texte ne change jamais')
})

test('le texte d’un tableau se cherche, dans son champ', () => {
  const doc = lecture([page(1, [{ kind: 'origin', reason: 'table', text: [{ text: 'Indicateurs de progrès collectifs' }] }])])
  const [p] = chercherDansLeDocument(doc, 'progres')
  assert.equal(p?.champ, 'text')
})

test('« vous êtes ici » : le premier passage de la page en cours', () => {
  const doc = lecture([
    page(1, [{ kind: 'paragraph', spans: [{ text: 'progrès, puis progrès' }] }]),
    page(2, [{ kind: 'paragraph', spans: [{ text: 'progrès' }] }]),
  ])
  const passages = chercherDansLeDocument(doc, 'progres')
  assert.equal(passages.length, 3)
  assert.equal(passageDeLaPage(passages, 2), 2)
  assert.equal(passageDeLaPage(passages, 5), -1)
  assert.deepEqual(chercherDansLeDocument(doc, '   '), [])
})

test('cent pages se cherchent en moins d’une seconde (SC-009)', () => {
  const pages = Array.from({ length: 100 }, (_, i) =>
    page(
      i + 1,
      Array.from({ length: 12 }, () => ({ kind: 'paragraph' as const, spans: [{ text: PHRASE }, { text: ' Texte de remplissage.' }] })),
    ),
  )
  const doc = lecture(pages)
  const debut = performance.now()
  const passages = chercherDansLeDocument(doc, 'progres collectifs')
  const duree = performance.now() - debut
  assert.equal(passages.length, 1200)
  assert.ok(duree < 1000, `${Math.round(duree)} ms`)
})

test('CO₂, les tirets et les blancs se trouvent comme on les tape', () => {
  const doc = lecture([
    page(1, [{ kind: 'paragraph', spans: [{ text: 'Les émissions de CO₂ entre Paris–Nairobi  et Bakou.' }] }]),
  ])
  assert.equal(chercherDansLeDocument(doc, 'co2').length, 1)
  assert.equal(chercherDansLeDocument(doc, 'paris-nairobi et').length, 1)
  assert.equal(chercherDansLeDocument(doc, 'paris-nairobi et')[0]?.extrait.trouve, 'Paris–Nairobi  et')
})

test('une lettre seule ne cherche rien : elle trouverait tout le document', () => {
  const doc = lecture([page(1, [{ kind: 'paragraph', spans: [{ text: 'adaptation' }] }])])
  assert.deepEqual(chercherDansLeDocument(doc, 'a'), [])
  assert.equal(chercherDansLeDocument(doc, 'ad').length, 1)
})

test('apostrophes, guillemets, ligatures et traits de PDFium se replient comme pour le repérage', () => {
  const doc = lecture([
    page(1, [{ kind: 'paragraph', spans: [{ text: 'Avant que l‘ordre du jour, aﬁn de “Responding” au\u0002delà.' }] }]),
  ])
  assert.equal(chercherDansLeDocument(doc, "l'ordre").length, 1)
  assert.equal(chercherDansLeDocument(doc, 'afin de "responding"').length, 1)
  assert.equal(chercherDansLeDocument(doc, 'au-delà')[0]?.extrait.trouve, 'au\u0002delà')
})

test('l’extrait d’un passage se retrouve sur la couche de texte de pdf.js, découpée autrement', () => {
  const texte = 'Les Parties devront s’accorder sur la mobilisation de capitaux supplémentaires au-delà des montants de 758 millions.'
  const doc = lecture([page(1, [{ kind: 'paragraph', spans: [{ text: texte }] }])])
  const couche = ['Les Parties devront s’accorder sur la mobilisation de capitaux supplémentaires au-', 'delà des montants', ' ', 'de 758 millions.']
  for (const expression of ['accorder', 'capitaux supplémentaires au-delà', 'montants de 758']) {
    const [p] = chercherDansLeDocument(doc, expression)
    assert.ok(p)
    const r = repererPassage([{ page: 1, chaines: couche }], passageAReperer(doc, p))
    assert.equal(r.issue, 'trouve', expression)
  }
})

test('le passage à repérer porte son rang parmi les occurrences de sa page', () => {
  const doc = lecture([
    page(1, [
      { kind: 'heading', level: 3, spans: [{ text: 'Pertes et Préjudices' }] },
      { kind: 'paragraph', spans: [{ text: 'Des progrès sur les pertes et préjudices (P&P).' }] },
    ]),
  ])
  const [titre, texte] = chercherDansLeDocument(doc, 'pertes et prejudices')
  assert.ok(titre && texte)
  assert.deepEqual(passageAReperer(doc, titre).rang, { occurrence: 0, total: 2 })
  assert.deepEqual(passageAReperer(doc, texte).rang, { occurrence: 1, total: 2 })
})
