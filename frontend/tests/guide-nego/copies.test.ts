import { test } from 'node:test'
import assert from 'node:assert/strict'
import {
  CACHE_PUBLICS,
  CACHE_RESERVES,
  appliquerLaReconciliation,
  cleDeCopie,
  copieIntacte,
  demanderLaPersistance,
  relireLaPersistance,
  garderUneCopie,
  retirerUneCopie,
  toutRetirer,
  verifierLesCopies,
} from '../../app/utils/guide-nego/copies.ts'
import { lireProgression, noterProgression } from '../../app/utils/guide-nego/appareil-lecture.ts'
import { copieDe, fauxDepots, fauxStockage } from './faux-depots.ts'

test('une copie se garde entière : ses entrées dans le bon cache, puis sa fiche', async () => {
  const { depots, clesDe } = fauxDepots()
  const publique = copieDe('guide', { images: [18, 59] })
  const reservee = copieDe('resume', { reserve: true })
  assert.equal(await garderUneCopie(depots, publique.copie, publique.entrees), true)
  assert.equal(await garderUneCopie(depots, reservee.copie, reservee.entrees), true)
  assert.equal(clesDe(CACHE_PUBLICS).length, 3)
  assert.equal(clesDe(CACHE_RESERVES).length, 1)
  assert.deepEqual((await depots.copies.lire()).map((c) => c.id).sort(), ['guide', 'resume'])
})

test('un téléchargement interrompu ne laisse rien : la place manque au troisième put', async () => {
  const { depots, clesDe } = fauxDepots({ refuserAuPut: 3 })
  const { copie, entrees } = copieDe('guide', { images: [18, 59] })
  assert.equal(await garderUneCopie(depots, copie, entrees), false)
  assert.deepEqual(clesDe(CACHE_PUBLICS), [], 'les deux entrées posées sont défaites')
  assert.equal(await depots.copies.lireUne('guide'), null, 'aucune fiche')
})

test('« Retirer du téléphone » ne touche qu’à ce document', async () => {
  const { depots, clesDe } = fauxDepots()
  for (const id of ['guide', 'note']) {
    const { copie, entrees } = copieDe(id, { images: [2] })
    await garderUneCopie(depots, copie, entrees)
  }
  await retirerUneCopie(depots, 'guide')
  assert.deepEqual(clesDe(CACHE_PUBLICS), copieDe('note', { images: [2] }).copie.cles.sort())
  assert.deepEqual((await depots.copies.lire()).map((c) => c.id), ['note'])
})

test('« Tout retirer » laisse les lectures, la progression et les autres caches', async () => {
  const { depots, noms } = fauxDepots()
  noms.set('gn-coquille-v7', new Map())
  const { copie, entrees } = copieDe('guide')
  await garderUneCopie(depots, copie, entrees)
  await depots.aTelecharger.poser({ id: 'note', reserve: false, octets: null, demande_a: '2026-11-12T09:00:00Z' })
  const stockage = fauxStockage()
  noterProgression(stockage, 'guide', '2025', 59, '2026-11-12T10:00:00Z')

  await toutRetirer(depots)

  assert.deepEqual([...noms.keys()], ['gn-coquille-v7'])
  assert.deepEqual(await depots.copies.lire(), [])
  assert.deepEqual(await depots.aTelecharger.lire(), [])
  assert.equal(lireProgression(stockage, 'guide', '2025')?.page, 59)
})

test('à la relecture : une copie dépubliée s’efface, un réservé sans accès aussi', async () => {
  const { depots } = fauxDepots()
  for (const id of ['guide', 'depublie', 'devenu-reserve']) {
    const { copie, entrees } = copieDe(id)
    await garderUneCopie(depots, copie, entrees)
  }
  const r = await appliquerLaReconciliation(depots, [
    { id: 'guide', restricted: false, accessible: true, reading_etag: '"guide-1"' },
    { id: 'devenu-reserve', restricted: true, accessible: false, reading_etag: null },
  ])
  assert.deepEqual(r.aEffacer.sort(), ['depublie', 'devenu-reserve'])
  assert.deepEqual((await depots.copies.lire()).map((c) => c.id), ['guide'])
})

test('devenu réservé avec l’accès : la copie reste, et passe dans le cache qui s’efface à la déconnexion', async () => {
  const { depots, clesDe } = fauxDepots()
  const { copie, entrees } = copieDe('note', { images: [3] })
  await garderUneCopie(depots, copie, entrees)
  const r = await appliquerLaReconciliation(depots, [
    { id: 'note', restricted: true, accessible: true, reading_etag: '"note-1"' },
  ])
  assert.deepEqual(r.aDeplacer, ['note'])
  assert.deepEqual(clesDe(CACHE_PUBLICS), [])
  assert.deepEqual(clesDe(CACHE_RESERVES), copie.cles.sort())
  assert.equal((await depots.copies.lireUne('note'))?.reserve, true)
})

test('une autre empreinte se signale, sans rien retélécharger ni effacer', async () => {
  const { depots } = fauxDepots()
  const { copie, entrees } = copieDe('guide')
  await garderUneCopie(depots, copie, entrees)
  const r = await appliquerLaReconciliation(depots, [
    { id: 'guide', restricted: false, accessible: true, reading_etag: '"guide-2"' },
  ])
  assert.deepEqual(r, { aEffacer: [], aDeplacer: [], autreVersion: ['guide'] })
  assert.ok(await depots.copies.lireUne('guide'))
})

test('le navigateur a vidé une image : la copie redevient « non téléchargée » à l’ouverture', async () => {
  const { depots, noms, clesDe } = fauxDepots()
  const entiere = copieDe('guide', { images: [18] })
  const videe = copieDe('note', { images: [4, 5] })
  await garderUneCopie(depots, entiere.copie, entiere.entrees)
  await garderUneCopie(depots, videe.copie, videe.entrees)
  noms.get(CACHE_PUBLICS)!.delete(videe.copie.cles[2]!)
  assert.equal(await copieIntacte(depots, videe.copie), false)

  assert.deepEqual(await verifierLesCopies(depots), ['note'])
  assert.deepEqual((await depots.copies.lire()).map((c) => c.id), ['guide'])
  assert.deepEqual(clesDe(CACHE_PUBLICS), entiere.copie.cles.sort(), 'ni fiche, ni entrée restante')
})

test('un cache entier vidé par le navigateur : ses copies disparaissent, les autres restent', async () => {
  const { depots, noms } = fauxDepots()
  const publique = copieDe('guide')
  const reservee = copieDe('resume', { reserve: true })
  await garderUneCopie(depots, publique.copie, publique.entrees)
  await garderUneCopie(depots, reservee.copie, reservee.entrees)
  noms.delete(CACHE_RESERVES)
  assert.deepEqual(await verifierLesCopies(depots), ['resume'])
  assert.deepEqual((await depots.copies.lire()).map((c) => c.id), ['guide'])
})

test('une entrée que plus aucune fiche ne désigne s’efface', async () => {
  const { depots, noms, clesDe } = fauxDepots()
  const { copie, entrees } = copieDe('guide')
  await garderUneCopie(depots, copie, entrees)
  noms.get(CACHE_PUBLICS)!.set('https://api.test/negotiation/documents/perdu/reading', new Response('{}'))
  assert.deepEqual(await verifierLesCopies(depots), [])
  assert.deepEqual(clesDe(CACHE_PUBLICS), copie.cles)
})

test('la persistance se demande tout de suite : accordée, refusée, ou impossible à demander', async () => {
  let lue = false
  const navigateur = (accorde: boolean) => ({
    persisted: async () => {
      lue = true
      return accorde
    },
    persist: async () => accorde,
  })
  assert.equal(await demanderLaPersistance(undefined), 'indisponible')
  assert.equal(await demanderLaPersistance(navigateur(true)), 'accordee')
  assert.equal(await demanderLaPersistance(navigateur(false)), 'refusee')
  assert.equal(lue, false, 'aucune attente avant persist() : le geste de la personne doit tenir')
  assert.equal(await demanderLaPersistance({ persist: async () => Promise.reject(new Error('refus du navigateur')) }), 'indisponible')
})

test('un refus se relit à l’ouverture : le navigateur peut accorder plus tard', async () => {
  assert.equal(await relireLaPersistance({ persisted: async () => true }), 'accordee')
  assert.equal(await relireLaPersistance({ persisted: async () => false }), 'refusee')
  assert.equal(await relireLaPersistance(undefined), 'indisponible')
})

test('la clé d’une copie est une adresse absolue, même quand la base de l’API est relative', () => {
  const page = 'https://epavillon.example/v2/guide-nego/ressources/documents'
  assert.equal(
    cleDeCopie('/negotiation/documents/g/reading', '/v2/api', page),
    'https://epavillon.example/v2/api/negotiation/documents/g/reading',
  )
  assert.equal(
    cleDeCopie('/negotiation/documents/g/reading', 'http://localhost:8080/api/', page),
    'http://localhost:8080/api/negotiation/documents/g/reading',
  )
  assert.equal(cleDeCopie('/negotiation/documents/g/reading', '', page), 'https://epavillon.example/gn-exemples/negotiation/documents/g/reading')
})

test('un magasin illisible n’est pas un magasin vide : la vérification n’efface rien', async () => {
  const { depots, clesDe } = fauxDepots()
  const { copie, entrees } = copieDe('guide', { images: [18] })
  await garderUneCopie(depots, copie, entrees)
  const illisible = { ...depots, copies: { ...depots.copies, lire: async () => null } }
  assert.deepEqual(await verifierLesCopies(illisible), [])
  assert.deepEqual(clesDe(CACHE_PUBLICS), copie.cles.sort())
})

test('une fiche qui ne s’écrit pas défait la copie : complet ou rien', async () => {
  const { depots, clesDe } = fauxDepots()
  const { copie, entrees } = copieDe('guide', { images: [18] })
  const refuse = {
    ...depots,
    copies: {
      ...depots.copies,
      poser: async () => {
        throw new Error('Écriture refusée dans « copies »')
      },
    },
  }
  assert.equal(await garderUneCopie(refuse, copie, entrees), false)
  assert.deepEqual(clesDe(CACHE_PUBLICS), [])
})

test('un nouvel essai qui échoue n’abandonne pas des morceaux de l’ancienne copie', async () => {
  const { depots, clesDe } = fauxDepots({ refuserAuPut: 3 })
  const ancienne = copieDe('guide', { images: [18] })
  await garderUneCopie(depots, ancienne.copie, ancienne.entrees)
  const nouvelle = copieDe('guide', { images: [18, 59] })
  assert.equal(await garderUneCopie(depots, nouvelle.copie, nouvelle.entrees), false)
  assert.deepEqual(clesDe(CACHE_PUBLICS), [], 'une entrée écrasée rend l’ancienne incomplète : tout part')
  assert.equal(await depots.copies.lireUne('guide'), null)
})
