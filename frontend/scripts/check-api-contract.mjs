// Le site et l'API disent-ils la même chose ?
//
// Deux vérifications, toutes deux portant sur des choses qui ne se voient
// autrement qu'à l'exécution, sur l'écran de la personne qui s'en sert :
//
//   1. CHEMINS — tout chemin appelé par `composables/useApi.ts` et les fabriques
//      de `composables/api/` figure au contrat. Une faute de frappe dans une
//      adresse n'est visible nulle part ailleurs.
//
//   2. FORMES — tout nom TypeScript annoncé par l'API existe dans `app/types/`.
//      L'API ne décrit pas ses corps : elle les NOMME, et confie leur définition
//      au site. Ce lien est déclaratif — sans contrôle, il se défait en silence.
//
// Les deux lisent `app/types/api.ts`, qui est engendré et versionné : la
// vérification ne demande donc ni base, ni API démarrée, ni build Rust.
//
//   3. DETTE — les appels passés par `pending()` déclarent une route que l'API
//      ne sert pas encore ; l'écran lit des exemples et le dit. Ils ne sont pas
//      comptés comme des fautes, mais une route `pending` qui EXISTE au contrat
//      en est une : l'écran s'est privé de la vraie donnée.
//
// `--verbose` ajoute l'inventaire des routes du contrat qu'aucun écran n'appelle
// et celui de la dette. Informatif : une route livrée avant son écran n'est pas
// un défaut.

import { readFileSync, readdirSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const RACINE = join(dirname(fileURLToPath(import.meta.url)), '..')
const CONTRAT = join(RACINE, 'app/types/api.ts')
const VERBEUX = process.argv.includes('--verbose')

if (!existsSync(CONTRAT)) {
  console.error("ÉCHEC : app/types/api.ts est absent — lancer `make openapi`.")
  process.exit(1)
}
const contrat = readFileSync(CONTRAT, 'utf8')

/** `/proposals/{id}/reviews` et `/proposals/${id}/reviews` deviennent la même chose. */
const normaliser = (p) => p.replace(/\{[^}]*\}|\$\{[^}]*\}/g, '{}').replace(/\/+$/, '')

// --- 1. Les chemins du contrat -----------------------------------------------
const cheminsContrat = new Map()
for (const m of contrat.matchAll(/^ {4}"(\/[^"]*)": \{$/gm)) {
  cheminsContrat.set(normaliser(m[1]), m[1])
}
if (cheminsContrat.size === 0) {
  console.error('ÉCHEC : aucun chemin lu dans app/types/api.ts — le format a changé.')
  process.exit(1)
}

// --- 2. Les chemins appelés par le site --------------------------------------
const sources = [
  join(RACINE, 'app/composables/useApi.ts'),
  ...readdirSync(join(RACINE, 'app/composables/api'))
    .filter((f) => f.endsWith('.ts'))
    .map((f) => join(RACINE, 'app/composables/api', f)),
]

const appels = []
const dette = []
for (const fichier of sources) {
  const src = readFileSync(fichier, 'utf8')
  // Premier argument de call(…), send(…), sendForm(…) et pending(…). Il n'est pas toujours un
  // littéral collé à la parenthèse : deux appels choisissent entre création et
  // modification par un ternaire, et ne pas lire les deux branches laissait
  // passer deux chemins sans qu'aucune des listes ne les nomme.
  for (const m of src.matchAll(/\b(call|sendForm|send|pending)\s*(?:<[^(]*>)?\s*\(\s*([\s\S]{0,200}?)(?:,\s*(?:\(|async)|\)\s*$)/gm)) {
    const ligne = src.slice(0, m.index).split('\n').length
    const chemins = [...m[2].matchAll(/[`'"](\/[^`'"]*)[`'"]/g)].map((c) => c[1])
    for (const chemin of chemins) {
      const entree = { chemin, fichier: fichier.slice(RACINE.length + 1), ligne }
      // `pending` déclare une route que l'API ne sert PAS encore : c'est une
      // dette nommée, pas une faute. La compter comme un appel inconnu ferait
      // échouer la vérification sur trois écrans qu'on sait non branchés.
      if (m[1] === 'pending') dette.push(entree)
      else appels.push(entree)
    }
  }
}

const inconnus = appels.filter((a) => !cheminsContrat.has(normaliser(a.chemin)))
// Une route passée en `pending` alors qu'elle EXISTE au contrat est un oubli de
// bascule : l'écran lit des exemples quand la plateforme sait répondre.
const detteObsolete = dette.filter((a) => cheminsContrat.has(normaliser(a.chemin)))

// --- 3. Les formes annoncées par l'API ---------------------------------------
// L'API nomme ses corps dans la description de chaque opération, sous la forme
// « `Entrée` → `Sortie` », et redit le nom de la sortie en description du 200.
const nomsAnnonces = new Map()
const retenir = (nom, ou) => {
  const propre = nom.replace(/\[\]$/, '')
  if (!/^[A-Z][A-Za-z0-9]+$/.test(propre)) return
  if (!nomsAnnonces.has(propre)) nomsAnnonces.set(propre, ou)
}
for (const m of contrat.matchAll(/@description `([A-Za-z0-9]+(?:\[\])?)`\s*(?:→|->)\s*`?([A-Za-z0-9]+(?:\[\])?)`?/g)) {
  retenir(m[1], 'corps de requête')
  retenir(m[2], 'corps de réponse')
}
for (const m of contrat.matchAll(/@description ([A-Z][A-Za-z0-9]*(?:\[\])?) \*\/\n\s*200:/g)) {
  retenir(m[1], 'corps de réponse')
}

// Ce que `app/types/` exporte. Le contrat lui-même est exclu : il ne définit
// aucune de ces formes, et s'y référer ferait passer la vérification à vide.
const exportes = new Set()
const parcourir = (dossier) => {
  for (const entree of readdirSync(dossier, { withFileTypes: true })) {
    const chemin = join(dossier, entree.name)
    if (entree.isDirectory()) { parcourir(chemin); continue }
    if (!entree.name.endsWith('.ts') || entree.name === 'api.ts') continue
    const src = readFileSync(chemin, 'utf8')
    for (const m of src.matchAll(/^export (?:interface|type|class|enum|const enum) ([A-Za-z0-9_]+)/gm)) {
      exportes.add(m[1])
    }
  }
}
parcourir(join(RACINE, 'app/types'))

// Les formes que l'API porte elle-même — elles sont dans `components.schemas`,
// donc réellement décrites, et n'ont rien à faire dans `app/types/`.
const portesParLApi = new Set(
  [...contrat.matchAll(/^ {8}([A-Za-z0-9_]+): \{$/gm)].map((m) => m[1]),
)
const orphelines = [...nomsAnnonces]
  .filter(([nom]) => !exportes.has(nom) && !portesParLApi.has(nom))

// --- Rapport -----------------------------------------------------------------
let echec = false

if (inconnus.length > 0) {
  echec = true
  console.error(`\nÉCHEC — ${inconnus.length} appel(s) vers un chemin absent du contrat :\n`)
  for (const a of inconnus) console.error(`  ${a.fichier}:${a.ligne}  ${a.chemin}`)
  console.error("\n  Soit l'adresse est fautive côté site, soit la route manque à l'API.")
  console.error('  Aucune des deux ne se corrige par une conversion.')
}

if (detteObsolete.length > 0) {
  echec = true
  console.error(`\nÉCHEC — ${detteObsolete.length} route(s) laissée(s) en données d'exemple alors que l'API les sert :\n`)
  for (const a of detteObsolete) console.error(`  ${a.fichier}:${a.ligne}  ${a.chemin}`)
  console.error('\n  Remplacer `pending(` par `call(` ou `send(` : la plateforme sait répondre.')
}

if (orphelines.length > 0) {
  echec = true
  console.error(`\nÉCHEC — ${orphelines.length} forme(s) annoncée(s) par l'API et absente(s) de app/types/ :\n`)
  for (const [nom, ou] of orphelines) console.error(`  ${nom}  (${ou})`)
  console.error("\n  L'API désigne ses corps par leur nom TypeScript : un nom sans définition")
  console.error('  ne documente rien. Soit le type manque au site, soit le nom est fautif.')
}

if (VERBEUX) {
  const appeles = new Set(appels.map((a) => normaliser(a.chemin)))
  const jamais = [...cheminsContrat].filter(([n]) => !appeles.has(n)).map(([, p]) => p)
  console.log(`\n${jamais.length} route(s) du contrat qu'aucun écran n'appelle :\n`)
  for (const p of jamais.sort()) console.log(`  ${p}`)
}

if (echec) process.exit(1)
console.log(
  `Contrat d'API : ${appels.length} appels sur ${cheminsContrat.size} chemins, ` +
    `${nomsAnnonces.size} formes annoncées — toutes définies.` +
    (dette.length > 0 ? ` ${dette.length} route(s) en attente d'API.` : ''),
)
if (VERBEUX && dette.length > 0) {
  console.log(`\n${dette.length} route(s) que le site attend et que l'API ne sert pas :\n`)
  for (const a of dette) console.log(`  ${a.chemin}  (${a.fichier}:${a.ligne})`)
}
