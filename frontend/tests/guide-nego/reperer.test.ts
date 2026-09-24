import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  pagesAInterroger,
  repererLaNote,
  repererPassage,
  type Intervalle,
  type PageDeTexte,
} from '../../app/utils/guide-nego/pdf/reperer.ts'

const sansContexte = { avant: '', apres: '' }

/** Le texte que couvre un intervalle, lu dans les éléments d'origine. */
function lu(chaines: readonly string[], { debut, fin }: Intervalle): string {
  if (debut.element === fin.element) return (chaines[debut.element] ?? '').slice(debut.caractere, fin.caractere)
  const morceaux = [(chaines[debut.element] ?? '').slice(debut.caractere)]
  for (let e = debut.element + 1; e < fin.element; e += 1) morceaux.push(chaines[e] ?? '')
  morceaux.push((chaines[fin.element] ?? '').slice(0, fin.caractere))
  return morceaux.join('')
}

function trouve(pages: PageDeTexte[], page: number, expression: string, contexte = sansContexte) {
  const r = repererPassage(pages, { page, contexte, expression })
  assert.equal(r.issue, 'trouve', `« ${expression} » retrouvé`)
  if (r.issue !== 'trouve') throw new Error('inaccessible')
  const chaines = pages.find((p) => p.page === r.page)?.chaines ?? []
  return { ...r, texte: lu(chaines, r.courant) }
}

const PAGE_56 = [
  'CdP29 à Bakou a marqué la conclusion de la phase de négociation,',
  'permettant à la CdP30 de Belém de passer à une phase post-',
  'négociation axée sur l’action et',
  'la mise en',
  ' ',
  'œuvre. Les Parties se concentrent sur la transparence du mécanisme centralisé (Article 6.4).',
]

test('le contexte désigne le passage, même quand l’expression est répétée sur la page', () => {
  const pages = [{ page: 56, chaines: ['la mise en œuvre et la mise en', ' ', 'œuvre du mécanisme de la mise en œuvre'] }]
  const r = trouve(pages, 56, 'la mise en œuvre', { avant: 'la mise en œuvre et ', apres: ' du mécanisme' })
  assert.deepEqual(r.courant.debut, { element: 0, caractere: 20 })
  assert.equal(r.texte, 'la mise en œuvre', 'la seconde occurrence, à cheval sur trois éléments')
  assert.equal(r.autres.length, 2, 'les deux autres occurrences de la page, en clair')
})

test('sans contexte retrouvé, une expression unique sur la page suffit', () => {
  const r = trouve([{ page: 56, chaines: PAGE_56 }], 56, 'centralisé (Article 6.4)', { avant: 'Rien de tel ', apres: ' ici' })
  assert.equal(r.page, 56)
  assert.equal(r.texte, 'centralisé (Article 6.4)')
  assert.deepEqual(r.autres, [])
})

test('une expression répétée sans contexte est ambiguë : toutes en clair, aucune pleine', () => {
  const r = repererPassage([{ page: 56, chaines: ['la mise en œuvre', ' ', 'et la mise en œuvre'] }], {
    page: 56,
    contexte: sansContexte,
    expression: 'mise en œuvre',
  })
  assert.equal(r.issue, 'ambigu')
  if (r.issue !== 'ambigu') return
  assert.equal(r.page, 56)
  assert.equal(r.occurrences.length, 2)
})

test('le rang départage une expression répétée, en dernier recours', () => {
  const pages = [{ page: 62, chaines: ['Pertes et Préjudices', 'Des progrès sur les', ' ', 'pertes et préjudices', ' (P&P).'] }]
  const r = trouve(pages, 62, 'pertes et préjudices', { avant: 'Des progrès sur les ', apres: '' })
  assert.equal(r.page, 62)
  const titre = repererPassage(pages, { page: 62, contexte: sansContexte, expression: 'Pertes et Préjudices', rang: { occurrence: 0, total: 2 } })
  assert.equal(titre.issue, 'trouve')
  if (titre.issue !== 'trouve') return
  assert.deepEqual(titre.courant.debut, { element: 0, caractere: 0 })
  assert.equal(titre.autres.length, 1, 'l’autre occurrence, en clair')
})

test('le rang se tait si les deux textes ne comptent pas autant d’occurrences, ou sur une autre page', () => {
  const pages = [
    { page: 62, chaines: ['mise en œuvre', ' ', 'et mise en œuvre', ' ', 'puis mise en œuvre'] },
    { page: 63, chaines: ['Rien ici.'] },
  ]
  const autantMoins = repererPassage(pages, { page: 62, contexte: sansContexte, expression: 'mise en œuvre', rang: { occurrence: 0, total: 2 } })
  assert.equal(autantMoins.issue, 'ambigu')
  const voisine = repererPassage(pages, { page: 63, contexte: sansContexte, expression: 'mise en œuvre', rang: { occurrence: 0, total: 3 } })
  assert.equal(voisine.issue, 'ambigu', 'le rang compte les occurrences de la page de l’index, pas de sa voisine')
})

test('un passage à cheval sur deux éléments, coupé par une césure', () => {
  const r = trouve([{ page: 56, chaines: PAGE_56 }], 56, 'phase post-négociation axée', {
    avant: 'de passer à une ',
    apres: ' sur l’action',
  })
  assert.deepEqual(r.courant.debut, { element: 1, caractere: 47 })
  assert.deepEqual(r.courant.fin, { element: 2, caractere: 16 })
  assert.equal(r.texte, 'phase post-négociation axée')
})

test('un passage absent de la page et de ses voisines est introuvable : rien n’est marqué', () => {
  const pages = [55, 56, 57].map((page) => ({ page, chaines: PAGE_56 }))
  assert.deepEqual(repererPassage(pages, { page: 56, contexte: sansContexte, expression: 'finance climatique' }), { issue: 'introuvable' })
})

test('les pages interrogées : la page, la suivante, la précédente, dans le document', () => {
  assert.deepEqual(pagesAInterroger(9, 90), [9, 10, 8])
  assert.deepEqual(pagesAInterroger(1, 90), [1, 2])
  assert.deepEqual(pagesAInterroger(90, 90), [90, 89])
})

// --- Les cas que l'essai a manqués au premier temps, en éléments tels que pdf.js les rend

test('p. 16 : la puce ▪ entre deux points de liste', () => {
  const pages = [
    {
      page: 16,
      chaines: [
        'Le niveau du financement (jugé insuffisant par certains) et les modalités de sa',
        'progression (linéaire ou par paliers).',
        '▪',
        ' ',
        'L’élargissement de la base de contributeurs, contesté par les pays en développement',
      ],
    },
  ]
  const r = trouve(pages, 16, 'par paliers). L’élargissement de')
  assert.equal(r.texte, 'par paliers).▪ L’élargissement de')
})

test('p. 48 : le numéro de liste « 5. »', () => {
  const pages = [
    {
      page: 48,
      chaines: [
        '”',
        '4.',
        ' ',
        '“Special needs and special circumstances of Africa',
        '”',
        '5.',
        ' ',
        '“Responding to the NDC Synthesis Report and Addressing the 1.5°C Ambition',
      ],
    },
  ]
  const r = trouve(pages, 48, 'Africa ” “Responding to')
  assert.deepEqual(r.courant.debut, { element: 3, caractere: 44 })
  assert.deepEqual(r.courant.fin, { element: 7, caractere: 14 })
})

test('p. 56 : l’appel de note « 15 » seul sur son élément', () => {
  const pages = [
    {
      page: 56,
      chaines: [
        ' ',
        'l’intégrité',
        'environnementale et la transparence des mécanismes.',
        '15',
        ' ',
        'ITMO signifie « «Résultat d\'atténuation transféré à l\'échelle internationale» » (Internationally Transferred Mitigation Outcomes).',
      ],
    },
  ]
  const r = trouve(pages, 56, 'mécanismes. ITMO signifie')
  assert.deepEqual(r.courant.debut, { element: 2, caractere: 40 })
  assert.deepEqual(r.courant.fin, { element: 5, caractere: 13 })
})

test('p. 68 : l’appel « 18 » après une adresse coupée sur deux lignes', () => {
  const pages = [
    {
      page: 68,
      chaines: [
        'En date du 30 octobre 2024. Voir [en ligne] https://unfccc.int/fr/process-and-meetings/the-convention/status-of-',
        'ratification/etat-des-ratifications-de-la-convention',
        '18',
        ' ',
        'L’Union européenne (UE) a signé la Convention alors qu’elle était encore la Communauté économique européenne (CEE).',
      ],
    },
  ]
  const r = trouve(
    pages,
    68,
    '[en ligne] https://unfccc.int/fr/process-and-meetings/the-convention/status-of-ratification/etat-des-ratifications-de-la-convention L’Union',
  )
  assert.deepEqual(r.courant.debut, { element: 0, caractere: 33 })
  assert.deepEqual(r.courant.fin, { element: 4, caractere: 7 })
})

test('p. 79 : « Autres³⁷ » se retrouve au premier temps, que le second perdrait', () => {
  const pages = [
    {
      page: 79,
      chaines: [
        'Caraïbes ; l’Asie et la région du Paciﬁque ; l’Europe de l’Est ; et l’Europe de l’Ouest et les',
        '« Autres',
        '37',
        ' ',
        '».',
      ],
    },
  ]
  const r = trouve(pages, 79, 'et les « Autres³⁷', { avant: '; l’Europe de l’Est ; et l’Europe de l’Ouest ', apres: ' ».' })
  assert.equal(r.texte, 'et les« Autres37')
  // Le second temps seul, qui écarte l'appel, ne le retrouverait pas.
  const sansAppel = repererPassage([{ page: 79, chaines: pages[0]!.chaines.filter((c) => c !== '37') }], {
    page: 79,
    contexte: sansContexte,
    expression: 'et les « Autres³⁷',
  })
  assert.equal(sansAppel.issue, 'introuvable')
})

test('p. 39 : la puce « o » de second niveau, seule sur son élément', () => {
  const pages = [
    {
      page: 39,
      chaines: [' ', "de l'Accord de Paris.", 'o', ' ', 'Le', ' ', 'renforcement des capacités', '.', 'o', ' ', 'Une combinaison de ces éléments.'],
    },
  ]
  const r = trouve(pages, 39, 'renforcement des capacités', {
    avant: "Un soutien aux questions liées à l'article 6 de l'Accord de Paris. Le ",
    apres: '. Une combinaison',
  })
  assert.deepEqual(r.courant.debut, { element: 6, caractere: 0 })
})

test('9 → 10 : le passage est sur la page suivante de celle de l’index', () => {
  const pages = [
    { page: 9, chaines: ['Les priorités de la CdP30 pour le financement.'] },
    { page: 10, chaines: ['mobilisation de capitaux supplémentaires au-delà des montants', ' ', 'de 758 millions de'] },
    { page: 8, chaines: ['Rien ici.'] },
  ]
  const r = trouve(pages, 9, 'supplémentaires au-delà de', {
    avant: 'la mobilisation de capitaux ',
    apres: 's montants de 758 millions de dollars américains',
  })
  assert.equal(r.page, 10)
  assert.equal(r.texte, 'supplémentaires au-delà de')
})

test('13 → 12 : le passage est sur la page précédente, pas sur la frise de la page 13', () => {
  const pages = [
    { page: 13, chaines: ['Copenhague', ' ', 'Katowice', ' ', 'Charm el-', 'Cheikh', ' ', 'Bakou'] },
    { page: 14, chaines: ['Rien ici.'] },
    {
      page: 12,
      chaines: [
        'climatique. La CdP26 a également lancé le Programme de travail de Glasgow',
        '–',
        'Charm',
        'el-',
        'Cheikh sur l’Objectif mondial',
      ],
    },
  ]
  const r = trouve(pages, 13, 'Glasgow–Charm el-Cheikh', {
    avant: 'a également lancé le Programme de travail de ',
    apres: ' sur l’Objectif mondial',
  })
  assert.equal(r.page, 12)
  assert.deepEqual(r.courant.debut, { element: 0, caractere: 66 })
  assert.deepEqual(r.courant.fin, { element: 4, caractere: 6 })
})

// --- Le passage cité d'une note (R10, FR-032) : ses morceaux, tels que pdf.js rend le guide ---

const PAGE_78 = [
  'auquel le lecteur est invité à se référer.',
  'Sur la base de l’ensemble de ces rapports, tous accessibles en ligne',
  '33',
  ', le Rapport de',
  'synthèse afférent au sixième Rapport d’évaluation (',
  'disponible en anglais uniquement',
  ') a',
  'été',
  ' ',
  'publié',
  ' ',
  'en',
  ' ',
  'mars',
  ' ',
  '2023',
  '34',
  '.',
  ' ',
  'Ce',
]

const PAGE_73 = [
  'dans la pratique.',
  'Ces principes ne s’appliquent pas uniquement aux',
  ' ',
  'personnes qui président',
  ', mais à l’ensemble',
  'des participants et participantes. En veillant à ce que chaque personne soit consciente des',
]

function noteLue(page: PageDeTexte, passage: string | null): string | null {
  const r = repererLaNote(page, passage)
  return r ? lu(page.chaines, r) : null
}

test('note p. 78 : le passage entier, appels de note compris', () => {
  const passage =
    'Sur la base de l’ensemble de ces rapports, tous accessibles en ligne³³, le Rapport de synthèse afférent au sixième Rapport d’évaluation ( disponible en anglais uniquement ) a été publié en mars 202334.'
  assert.equal(
    noteLue({ page: 78, chaines: PAGE_78 }, passage),
    PAGE_78.slice(1, 18).join(''),
    'du premier mot au point final, sur dix-sept éléments',
  )
})

test('note p. 5 : la ligature « ﬁ » et le titre en italique sur son élément', () => {
  const chaines = [
    'Compte tenu de l’environnement essentiellement anglophone des négociations, ici retranscrites',
    'en français, un index des sigles et acronymes utilisés, indiquant leur équivalent en anglais,',
    'ﬁgurent',
    ' ',
    'en annexe du',
    ' ',
    'Guide',
    '. Lorsqu’il est fait référence aux documents issus des négociations,',
  ]
  const passage =
    'Compte tenu de l’environnement essentiellement anglophone des négociations, ici retranscrites en français, un index des sigles et acronymes utilisés, indiquant leur équivalent en anglais, figurent en annexe du  Guide .'
  assert.equal(noteLue({ page: 5, chaines }, passage), `${chaines.slice(0, 7).join('')}.`)
})

test('note p. 73 : un passage cité que la page ne porte pas en entier se place par ses huit premiers mots', () => {
  const passage = 'Ces principes ne s’appliquent pas uniquement aux personnes qui président, mais à tous les délégués.'
  assert.equal(noteLue({ page: 73, chaines: PAGE_73 }, passage), 'Ces principes ne s’appliquent pas uniquement aux personnes')
})

test('note : un passage répété sur la page se place à sa première occurrence', () => {
  const chaines = ['Les Parties se réunissent.', ' ', 'Plus bas, les Parties se réunissent', ' ', 'encore.']
  const r = repererLaNote({ page: 12, chaines }, 'les Parties se réunissent')
  assert.deepEqual(r?.debut, { element: 0, caractere: 0 })
})

test('note : sans passage, ou introuvable, pas de hauteur — la note va en tête de page', () => {
  const page = { page: 73, chaines: PAGE_73 }
  assert.equal(repererLaNote(page, null), null)
  assert.equal(repererLaNote(page, '   '), null)
  assert.equal(repererLaNote(page, 'Un passage que la page ne porte nulle part, pas même ses premiers mots.'), null)
})
