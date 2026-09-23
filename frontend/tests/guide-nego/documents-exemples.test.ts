import { beforeEach, test } from 'node:test'
import assert from 'node:assert/strict'
import { existsSync } from 'node:fs'
import { registerHooks } from 'node:module'
import { dirname, resolve } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'

// Le jeu d'exemple s'écrit pour Nuxt : `~/` et les imports sans extension se résolvent ici.
const APP = fileURLToPath(new URL('../../app/', import.meta.url))
registerHooks({
  resolve(specifier, context, nextResolve) {
    const parent = context.parentURL?.startsWith('file:') ? dirname(fileURLToPath(context.parentURL)) : null
    let chemin = specifier.startsWith('~/')
      ? resolve(APP, specifier.slice(2))
      : specifier.startsWith('.') && parent
        ? resolve(parent, specifier)
        : null
    if (chemin && !/\.[cm]?[jt]s$/.test(chemin)) {
      chemin = [`${chemin}.ts`, `${chemin}/index.ts`].find((c) => existsSync(c)) ?? chemin
    }
    return nextResolve(chemin ? pathToFileURL(chemin).href : specifier, context)
  },
})

const m = await import('../../app/mocks/negotiation-documents.ts')
const acces = await import('../../app/mocks/negotiation-access.ts')
const { events } = await import('../../app/mocks/event.ts')
const { EVENT } = await import('../../app/mocks/ids.ts')

const D = m.DOCUMENT_NEGO

interface Refus {
  code: string
  status: number
  message: string
  field: string | null
}

function refus(geste: () => unknown): Refus {
  try {
    geste()
  } catch (erreur) {
    return erreur as Refus
  }
  return assert.fail("le jeu d'exemple devait refuser")
}

const dansLaListe = (id: string, langue = 'fr') => {
  const d = m.bibliothequeDeDocuments(langue).valeur.documents.find((x) => x.id === id)
  assert.ok(d, `${id} absent de la bibliothèque`)
  return d
}

const pagesTrouvees = (q: string, id: string = D.guideCop31): number[] =>
  (m.rechercherDansLesDocuments(q).hits.find((h) => h.document_id === id)?.pages ?? []).map((p) => p.index)

/** Un brouillon à lien, publié : le plus court chemin vers un document publié. */
function lienPublie(titre: string): string {
  const d = m.creerUnDocument({ title: { fr: titre }, type: 'report', external_url: 'https://example.org/rapport' })
  m.publierLeDocument(d.id)
  return d.id
}

beforeEach(() => {
  m.reinitialiserLesDocuments()
  acces.reinitialiserLAcces()
})

test('réservé sans accès : thématiques toujours masquées, adresse du lien tue, hôte gardé', () => {
  const resume = dansLaListe(D.resumeCop31)
  assert.deepEqual([resume.themes, resume.themes_hidden, resume.summary, resume.accessible], [[], true, null, false])

  m.modifierLeDocument(D.bulletin11Novembre, { restricted: true })
  const bulletin = dansLaListe(D.bulletin11Novembre)
  assert.deepEqual(bulletin.themes, [], 'le bulletin ne porte aucune thématique')
  assert.equal(bulletin.themes_hidden, true, 'masqué même sans thématique : sinon « rien à cacher » se lit')
  assert.equal(bulletin.external_url, null, "l'adresse d'un lien est son contenu (SC-007)")
  assert.equal(bulletin.link_host, 'enb.iisd.org')

  acces.saisirUnCode('NEGO002')
  const ouvert = dansLaListe(D.bulletin11Novembre)
  assert.deepEqual([ouvert.themes_hidden, ouvert.external_url], [false, 'https://enb.iisd.org/fr/cop31-bulletin-11-novembre'])
  const resumeOuvert = dansLaListe(D.resumeCop31)
  assert.deepEqual([resumeOuvert.themes, resumeOuvert.themes_hidden], [['adaptation', 'finance', 'loss_and_damage'], false])
  assert.match(resumeOuvert.summary ?? '', /lignes rouges/)
})

test("un document ouvert à tous n'a rien de masqué", () => {
  const bulletin = dansLaListe(D.bulletin11Novembre)
  assert.deepEqual([bulletin.themes_hidden, bulletin.accessible], [false, true])
  assert.equal(bulletin.external_url, 'https://enb.iisd.org/fr/cop31-bulletin-11-novembre')
})

test("le téléchargement d'un lien est compté ; seuls le non-publié et le réservé refusent", () => {
  assert.equal(m.compterUnTelechargement(D.bulletin11Novembre), undefined)
  assert.equal(m.compterUnTelechargement(D.guideCop31), undefined)
  assert.equal(refus(() => m.lectureDuDocument(D.bulletin11Novembre)).code, 'NEGOTIATION_DOCUMENT_NOT_READABLE', 'la lecture, elle, refuse un lien')

  const brouillon = m.creerUnDocument({ title: { fr: 'Brouillon' }, type: 'report' })
  for (const id of [brouillon.id, '0199b010-0000-7000-8000-00000000dead']) {
    const r = refus(() => m.compterUnTelechargement(id))
    assert.deepEqual([r.code, r.status], ['NEGOTIATION_DOCUMENT_NOT_FOUND', 404])
  }

  const r = refus(() => m.compterUnTelechargement(D.resumeCop31))
  assert.deepEqual([r.code, r.status], ['NEGOTIATION_DOCUMENT_RESTRICTED', 403])
  assert.equal(
    r.message,
    "Ce document est réservé aux négociatrices et négociateurs. Saisissez votre code d'invitation pour l'ouvrir.",
  )
  acces.saisirUnCode('NEGO002')
  assert.equal(m.compterUnTelechargement(D.resumeCop31), undefined)
})

test('dépublier un brouillon ne fait rien : il reste supprimable', () => {
  const brouillon = m.creerUnDocument({ title: { fr: 'Brouillon à dépublier' }, type: 'report' })
  const apres = m.depublierLeDocument(brouillon.id)
  assert.deepEqual([apres.state, apres.unpublished_at, apres.file_locked], ['draft', null, false])
  m.supprimerLeDocument(brouillon.id)
  assert.equal(refus(() => m.ficheDuDocument(brouillon.id)).code, 'NEGOTIATION_DOCUMENT_NOT_FOUND')

  const publie = lienPublie('Rapport publié')
  const depublie = m.depublierLeDocument(publie)
  assert.deepEqual([depublie.state, depublie.file_locked], ['unpublished', true])
  assert.ok(depublie.unpublished_at)
  assert.equal(refus(() => m.supprimerLeDocument(publie)).code, 'NEGOTIATION_DOCUMENT_PUBLISHED_UNDELETABLE')
})

test("le lien d'un document déjà publié est figé dès que le champ est présent, même inchangé", () => {
  const url = 'https://enb.iisd.org/fr/cop31-bulletin-11-novembre'
  for (const external_url of [url, null, 'https://enb.iisd.org/autre']) {
    const r = refus(() => m.modifierLeDocument(D.bulletin11Novembre, { external_url }))
    assert.deepEqual([r.code, r.status, r.field], ['NEGOTIATION_DOCUMENT_FILE_LOCKED', 409, 'external_url'])
  }
  assert.equal(m.modifierLeDocument(D.bulletin11Novembre, { publisher: 'IISD ENB' }).publisher, 'IISD ENB')

  m.depublierLeDocument(D.bulletin11Novembre)
  assert.equal(refus(() => m.modifierLeDocument(D.bulletin11Novembre, { external_url: url })).code, 'NEGOTIATION_DOCUMENT_FILE_LOCKED')

  const brouillon = m.creerUnDocument({ title: { fr: 'Brouillon à lien' }, type: 'report', external_url: url })
  assert.equal(m.modifierLeDocument(brouillon.id, { external_url: url }).external_url, url)
  assert.equal(m.modifierLeDocument(brouillon.id, { external_url: null }).external_url, null)
})

test('la recherche exige tous les mots, dans n’importe quel ordre, sans les vouloir voisins', () => {
  assert.deepEqual(pagesTrouvees('progres collectifs').sort((a, b) => a - b), [18, 26, 59, 68])
  assert.deepEqual(pagesTrouvees('collectifs progrès'), pagesTrouvees('progres collectifs'))
  assert.deepEqual(pagesTrouvees('adaptation indicateurs'), [59], 'les deux mots de la page 59 sont loin l’un de l’autre')
  assert.deepEqual(m.rechercherDansLesDocuments('progres Belem').hits, [], 'aucune page ne porte les deux')
  assert.deepEqual(m.rechercherDansLesDocuments('de la').hits, [], 'des mots vides ne trouvent rien')

  const hit = m.rechercherDansLesDocuments('progres collectifs').hits.find((h) => h.document_id === D.guideCop31)
  assert.equal(hit?.pages[0]?.index, 59, 'la page qui porte le plus de passages vient en tête')
  for (const p of hit?.pages ?? []) {
    assert.match(p.excerpt, /progrès collectifs/)
    assert.ok(p.excerpt.split(/\s+/).length <= 24, `extrait trop long page ${p.index}`)
  }
  assert.equal(refus(() => m.rechercherDansLesDocuments('   ')).field, 'q')
})

test('un document ouvert tel quel reste trouvable : le mode change la lecture, pas le texte cherché', () => {
  assert.deepEqual(pagesTrouvees('Belem financement', D.noteBilanCop30), [1])
  m.ouvrirTelQuel(D.noteBilanCop30, true)
  const lecture = m.lectureDuDocument(D.noteBilanCop30).valeur
  assert.equal(lecture.mode, 'as_is')
  assert.deepEqual(lecture.pages[0]?.blocks, [])
  assert.deepEqual(pagesTrouvees('Belem financement', D.noteBilanCop30), [1])
})

test('un réservé se dit trouvé, sans page ni extrait, tant que l’accès manque', () => {
  const sans = m.rechercherDansLesDocuments('positions').hits.find((h) => h.document_id === D.resumeCop31)
  assert.deepEqual(sans?.pages, [])
  acces.saisirUnCode('NEGO002')
  const avec = m.rechercherDansLesDocuments('positions').hits.find((h) => h.document_id === D.resumeCop31)
  assert.deepEqual(avec?.pages.map((p) => p.index), [1])
  assert.match(avec?.pages[0]?.excerpt ?? '', /positions/)
})

test('un brouillon naît hors de l’assistant : rag_eligible faux, sauf demande', () => {
  assert.equal(m.creerUnDocument({ title: { fr: 'Sans marqueur' }, type: 'report' }).rag_eligible, false)
  assert.equal(m.creerUnDocument({ title: { fr: 'Avec marqueur' }, type: 'report', rag_eligible: true }).rag_eligible, true)
})

test('les documents vont du plus récemment publié au plus ancien', () => {
  const ordre = () => m.bibliothequeDeDocuments().valeur.documents.map((d) => d.id)
  assert.deepEqual(ordre(), [D.bulletin11Novembre, D.resumeCop31, D.guideCop31, D.noteBilanCop30, D.auNomDeMaDelegation])
  const nouveau = lienPublie('Rapport du jour')
  assert.equal(ordre()[0], nouveau)
})

test('le vocabulaire suit sort_order, pas l’ordre des documents', () => {
  m.modifierLeDocument(D.bulletin11Novembre, { themes: ['technology', 'transparency'] })
  const { types, themes } = m.bibliothequeDeDocuments().valeur.vocabulary
  assert.deepEqual(types.map((t) => t.code), ['negotiation_guide', 'summary', 'technical_note', 'bulletin'])
  assert.deepEqual(
    themes.map((t) => t.code),
    ['adaptation', 'mitigation', 'finance', 'loss_and_damage', 'article_6', 'transparency', 'gender', 'just_transition', 'technology'],
  )
  assert.deepEqual(themes.map((t) => t.sort_order), [10, 20, 30, 40, 50, 60, 70, 80, 100])
})

test('les favoris vont du plus récent au plus ancien ; poser et retirer sont idempotents', () => {
  const ids = () => m.mesFavorisDeDocuments().valeur.bookmarks.map((b) => b.document_id)
  assert.deepEqual(ids(), [D.guideCop31, D.auNomDeMaDelegation])
  m.poserUnFavori(D.bulletin11Novembre)
  const pose = m.mesFavorisDeDocuments().valeur.bookmarks[0]
  m.poserUnFavori(D.bulletin11Novembre)
  assert.deepEqual(ids(), [D.bulletin11Novembre, D.guideCop31, D.auNomDeMaDelegation])
  assert.equal(m.mesFavorisDeDocuments().valeur.bookmarks[0]?.created_at, pose?.created_at, 'poser deux fois ne rajeunit rien')
  m.retirerUnFavori(D.bulletin11Novembre)
  m.retirerUnFavori(D.bulletin11Novembre)
  assert.deepEqual(ids(), [D.guideCop31, D.auNomDeMaDelegation])
  assert.equal(refus(() => m.poserUnFavori('0199b010-0000-7000-8000-00000000dead')).code, 'NEGOTIATION_DOCUMENT_NOT_FOUND')
})

test('la ville de la COP31 est celle de l’édition, pas une valeur à part', () => {
  const cops = m.bibliothequeDeDocuments().valeur.vocabulary.cops
  const edition = events.find((e) => e.id === EVENT.cop31)
  assert.ok(edition)
  assert.deepEqual(cops.map((c) => c.id), [EVENT.cop31, EVENT.cop30], 'la plus récente d’abord')
  assert.deepEqual(cops[0], { id: EVENT.cop31, label: edition.edition_label, city: edition.city })
})

test('titres et libellés suivent la langue demandée, repli sur le français', () => {
  assert.equal(dansLaListe(D.guideCop31, 'en').title, 'Negotiations guide — COP31')
  assert.equal(dansLaListe(D.guideCop31, 'fr').title, 'Guide des négociations — CdP31')
  assert.equal(dansLaListe(D.resumeCop31, 'en').title, 'Résumé pour les décideurs — CdP31', 'sans anglais, le français')
  assert.equal(dansLaListe(D.noteBilanCop30, 'en').superseded_by?.title, 'Negotiations guide — COP31')

  const libelle = (langue: string, table: 'types' | 'themes', code: string) =>
    m.bibliothequeDeDocuments(langue).valeur.vocabulary[table].find((t) => t.code === code)?.label
  assert.deepEqual([libelle('fr', 'types', 'summary'), libelle('en', 'types', 'summary')], ['Résumé', 'Summary'])
  assert.deepEqual([libelle('fr', 'themes', 'mitigation'), libelle('en', 'themes', 'mitigation')], ['Atténuation', 'Mitigation'])

  const ligne = m.listeDesDocuments('en').documents.find((d) => d.id === D.guideCop31)
  assert.equal(ligne?.title, 'Negotiations guide — COP31')
  assert.equal(m.ficheDuDocument(D.noteBilanCop30, 'en').superseded_by?.title, 'Negotiations guide — COP31')
  assert.equal(m.ficheDuDocument(D.noteBilanCop30, 'fr').superseded_by?.title, 'Guide des négociations — CdP31')
})

test('le corps des notes suit la langue, repli sur le français', () => {
  const posee = m.poserUneNote(D.guideCop31, { page_index: 60, body: { fr: 'Seulement en français.' } })
  const corps = (langue: string, id: string) => m.notesDeCorrection(langue).valeur.notes.find((n) => n.id === id)?.body
  assert.match(corps('en', '0199b011-0000-7000-8000-000000000001') ?? '', /^The 100 GGA indicators/)
  assert.match(corps('fr', '0199b011-0000-7000-8000-000000000001') ?? '', /^Les 100 indicateurs du GGA/)
  assert.equal(corps('en', posee.id), 'Seulement en français.')
})

test('les notes se rangent par page, et le passage est coupé, vide devenant nul', () => {
  assert.deepEqual(m.notesDuDocument(D.guideCop31).notes.map((n) => n.page_index), [19, 59])
  const avant = m.poserUneNote(D.guideCop31, { page_index: 18, passage: '  progrès collectifs  ', body: { fr: 'Note.' } })
  assert.equal(avant.passage, 'progrès collectifs')
  assert.equal(m.poserUneNote(D.guideCop31, { page_index: 18, passage: '   ', body: { fr: 'Note.' } }).passage, null)
  assert.equal(m.poserUneNote(D.guideCop31, { page_index: 18, body: { fr: 'Note.' } }).passage, null)
  assert.deepEqual(m.notesDuDocument(D.guideCop31).notes.map((n) => n.page_index), [18, 18, 18, 19, 59])
  assert.deepEqual(
    m.notesDeCorrection().valeur.notes.map((n) => n.page_index),
    [18, 18, 18, 59],
    'les notes publiques : vivantes seulement, rangées de même',
  )
})

test('une seconde nouvelle version est refusée en nommant le premier remplaçant', () => {
  const premiere = m.nouvelleVersion(D.auNomDeMaDelegation)
  assert.equal(premiere.supersedes?.id, D.auNomDeMaDelegation)
  assert.equal(premiere.version, 'Deuxième édition (nouvelle version)')
  m.modifierLeDocument(premiere.id, { title: { fr: 'Au nom de ma délégation, troisième édition', en: 'On behalf of my delegation, third edition' } })

  const fr = refus(() => m.nouvelleVersion(D.auNomDeMaDelegation))
  assert.deepEqual([fr.code, fr.status, fr.field], ['NEGOTIATION_DOCUMENT_ALREADY_SUPERSEDED', 409, 'supersedes_id'])
  assert.equal(fr.message, 'Ce document est déjà remplacé par « Au nom de ma délégation, troisième édition ».')
  const en = refus(() => m.nouvelleVersion(D.auNomDeMaDelegation, 'en'))
  assert.equal(en.message, 'Ce document est déjà remplacé par « On behalf of my delegation, third edition ».')

  const autre = m.creerUnDocument({ title: { fr: 'Concurrent' }, type: 'report' })
  const parPatch = refus(() => m.modifierLeDocument(autre.id, { supersedes_id: D.auNomDeMaDelegation }))
  assert.equal(parPatch.message, fr.message)
  assert.equal(m.modifierLeDocument(autre.id, { supersedes_id: D.resumeCop31 }).supersedes?.id, D.resumeCop31)
})

test('la chaîne de remplacement s’arrête au premier brouillon, et mène au bout publié', () => {
  const b = m.nouvelleVersion(D.auNomDeMaDelegation)
  assert.equal(dansLaListe(D.auNomDeMaDelegation).superseded_by, null, 'un brouillon ne remplace encore rien')

  const c = m.creerUnDocument({ title: { fr: 'Troisième maillon' }, type: 'report', supersedes_id: b.id, external_url: 'https://example.org/c' })
  m.publierLeDocument(c.id)
  assert.equal(dansLaListe(D.auNomDeMaDelegation).superseded_by, null, 'un maillon en brouillon interrompt la chaîne')

  m.modifierLeDocument(b.id, { external_url: 'https://example.org/b' })
  m.publierLeDocument(b.id)
  assert.equal(dansLaListe(D.auNomDeMaDelegation).superseded_by?.id, c.id, 'une chaîne de trois mène au bout')
  assert.equal(dansLaListe(b.id).superseded_by?.id, c.id)
})

test('les refus portent le message et le champ de l’API', () => {
  const theme = refus(() => m.creerUnDocument({ title: { fr: 'X' }, type: 'report', themes: ['mitigation', 'climat'] }))
  assert.deepEqual([theme.code, theme.status, theme.message, theme.field], [
    'NEGOTIATION_DOCUMENT_UNKNOWN_THEME', 400, "Cette thématique n'existe pas : climat.", 'themes',
  ])
  assert.deepEqual(m.creerUnDocument({ title: { fr: 'X' }, type: 'report', themes: ['mitigation'] }).themes, ['mitigation'])

  const sansTitre = refus(() => m.creerUnDocument({ type: 'report' }))
  assert.deepEqual([sansTitre.message, sansTitre.field], ['Le titre est obligatoire.', 'title'])
  const titreVide = refus(() => m.creerUnDocument({ title: { fr: '  ', en: 'Only English' }, type: 'report' }))
  assert.deepEqual([titreVide.message, titreVide.field], ['Le texte en français est obligatoire.', 'title'])

  const note = refus(() => m.poserUneNote(D.guideCop31, { page_index: 59, body: { fr: ' ', en: 'English only' } }))
  assert.deepEqual([note.code, note.message, note.field], ['VALIDATION_FAILED', 'Le texte de la note en français est obligatoire.', 'body'])
  const page = refus(() => m.poserUneNote(D.guideCop31, { page_index: 93, body: { fr: 'Hors du document.' } }))
  assert.deepEqual([page.code, page.field], ['NEGOTIATION_CORRECTION_PAGE_UNKNOWN', 'page_index'])
  assert.equal(m.poserUneNote(D.guideCop31, { page_index: 92, body: { fr: 'Dernière page.' } }).page_index, 92)

  const inconnue = refus(() => m.retirerUneNote('0199b011-0000-7000-8000-00000000dead'))
  assert.deepEqual([inconnue.code, inconnue.status, inconnue.message], ['NOT_FOUND', 404, 'La ressource demandée est introuvable.'])

  const lien = m.creerUnDocument({ title: { fr: 'Lien' }, type: 'report', external_url: 'https://example.org/l' })
  const lesDeux = refus(() => m.attacherLeFichier(lien.id, '0199b012-0000-7000-8000-000000000099'))
  assert.deepEqual([lesDeux.code, lesDeux.field], ['NEGOTIATION_DOCUMENT_SOURCE_BOTH', 'external_url'])
  const vierge = m.creerUnDocument({ title: { fr: 'Vierge' }, type: 'report' })
  assert.equal(m.attacherLeFichier(vierge.id, '0199b012-0000-7000-8000-000000000099').source, 'file')
  const figee = refus(() => m.attacherLeFichier(D.guideCop31, '0199b012-0000-7000-8000-000000000099'))
  assert.deepEqual([figee.code, figee.field], ['NEGOTIATION_DOCUMENT_FILE_LOCKED', 'asset_id'])
})
