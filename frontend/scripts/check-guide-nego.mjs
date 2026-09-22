#!/usr/bin/env node
/**
 * Guide Négo reste borné (constitution, XIII) — et ce contrôle le tient.
 *
 * Deux designs vivent sous un même toit. Rien de Guide Négo ne doit s'appliquer au
 * site, rien du site ne doit entrer dans Guide Négo. Une règle écrite ne survit pas
 * au trentième composant : celle-ci échoue la construction.
 */
import { readdirSync, readFileSync, statSync, existsSync } from 'node:fs'
import { join, relative, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'

const FRONT = join(dirname(fileURLToPath(import.meta.url)), '..')
const BORNE = '[data-app="guide-nego"]'

const DOSSIERS = [
  'app/pages/guide-nego',
  'app/components/guide-nego',
  'app/composables/guide-nego',
  'app/utils/guide-nego',
  'app/assets/guide-nego',
  'guide-nego',
]
const FICHIERS = ['app/layouts/guide-nego.vue', 'modules/guide-nego-garde.ts']

function parcourir(dossier) {
  if (!existsSync(dossier)) return []
  return readdirSync(dossier).flatMap((nom) => {
    const chemin = join(dossier, nom)
    return statSync(chemin).isDirectory() ? parcourir(chemin) : [chemin]
  })
}

const fichiers = [
  ...DOSSIERS.flatMap((d) => parcourir(join(FRONT, d))),
  ...FICHIERS.map((f) => join(FRONT, f)).filter(existsSync),
].filter((f) => /\.(vue|ts|js|mjs|css)$/.test(f))

const ecarts = []
const signaler = (fichier, message) => ecarts.push(`${relative(FRONT, fichier)} — ${message}`)

/** Rend les sélecteurs de premier niveau d'une feuille, blocs `@media` ouverts, autres `@` ignorés. */
function selecteurs(css) {
  const propre = css.replace(/\/\*[\s\S]*?\*\//g, '')
  const trouves = []
  const pile = []
  let tampon = ''
  for (const caractere of propre) {
    if (caractere === '{') {
      const entete = tampon.trim()
      tampon = ''
      const estMedia = /^@(media|supports)\b/.test(entete)
      const dansRegle = pile.includes('regle')
      if (entete.startsWith('@') && !estMedia) {
        if (!/^@(font-face|keyframes gn-[\w-]+)$/.test(entete)) trouves.push(entete)
      } else if (!estMedia && !dansRegle) {
        trouves.push(entete)
      }
      pile.push(estMedia ? 'media' : 'regle')
    } else if (caractere === '}') {
      tampon = ''
      pile.pop()
    } else if (caractere === ';') {
      tampon = ''
    } else {
      tampon += caractere
    }
  }
  return trouves
}

/** Les virgules de `:where(a, b)` ne séparent pas des sélecteurs. */
function decouperHorsParentheses(entete) {
  const morceaux = ['']
  let profondeur = 0
  for (const caractere of entete) {
    if (caractere === '(') profondeur++
    if (caractere === ')') profondeur--
    if (caractere === ',' && profondeur === 0) morceaux.push('')
    else morceaux[morceaux.length - 1] += caractere
  }
  return morceaux
}

function verifierBornage(fichier, css) {
  for (const entete of selecteurs(css)) {
    if (entete.startsWith('@')) {
      signaler(fichier, `règle « ${entete} » : seuls @font-face, @keyframes gn-*, @media et @supports sont admis`)
      continue
    }
    for (const selecteur of decouperHorsParentheses(entete)) {
      if (!selecteur.trim().startsWith(BORNE)) {
        signaler(fichier, `sélecteur hors de ${BORNE} : « ${selecteur.trim()} »`)
      }
    }
  }
}

for (const fichier of fichiers) {
  const contenu = readFileSync(fichier, 'utf8')
  const lignes = contenu.split('\n').length

  if (lignes > 1000) signaler(fichier, `${lignes} lignes — la limite est de 1000`)

  if (/from\s+['"]~\/components\/(?!guide-nego\/)/.test(contenu)) {
    signaler(fichier, 'importe un composant du site')
  }
  const composantDuSite = contenu.match(/<(Ui|Admin|Media|Proposal|Organization|Event|Home|Workspace|Auth)[A-Z]\w*/)
  if (composantDuSite) signaler(fichier, `emploie un composant du site : ${composantDuSite[0]}>`)

  if (/--(ifdd|color)-[\w-]+\s*:/.test(contenu)) signaler(fichier, 'déclare un jeton du site')
  if (/var\(--(ifdd|color)-/.test(contenu)) signaler(fichier, 'lit un jeton du site')

  if (/\bsetLocale\b/.test(contenu)) signaler(fichier, 'appelle setLocale, qui écrit le cookie de langue du site')
  if (/epavillon_/.test(contenu)) signaler(fichier, 'nomme un cookie du site')

  if (/<Teleport\s+[^>]*to=["']body["']/.test(contenu)) {
    signaler(fichier, 'téléporte dans <body> : viser #gn-portail, seul à porter les jetons et le thème')
  }

  if (fichier.endsWith('.css')) verifierBornage(fichier, contenu)
  if (fichier.endsWith('.vue')) {
    for (const bloc of contenu.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)) {
      verifierBornage(fichier, bloc[1])
    }
  }
}

const PROGRAMME_SEUL = /\bprogrammes?\b(?!\s+(officiel|de la CCNUCC))/i
for (const locale of ['fr', 'en']) {
  const racine = join(FRONT, 'i18n/locales', locale)
  const traductions = parcourir(racine).filter((f) =>
    /\/(pages\/guide-nego\.|components\/gn-)[^/]*\.json$/.test(f),
  )
  for (const fichier of traductions) {
    const contenu = readFileSync(fichier, 'utf8')
    if (locale === 'fr' && PROGRAMME_SEUL.test(contenu)) {
      signaler(fichier, 'le mot « Programme » seul est banni : nommer l’agenda en entier')
    }
  }
}

// SC-007 de l'étape 0c : un libellé de thématique est une donnée. Les libellés sont lus
// dans le semis SQL, pour que ce contrôle n'en recopie aucun. Les données d'exemple
// miroitent la base, et la planche des composants s'en sert comme spécimens.
const semis = readFileSync(join(FRONT, '../docs/database/020_reference.sql'), 'utf8')
const LIBELLES_DE_THEMATIQUES = new Set(
  [...semis.matchAll(/\('negotiation_theme',\s*'\w+',\s*'(\{[^']*\})'/g)].flatMap((m) =>
    Object.values(JSON.parse(m[1])),
  ),
)
if (LIBELLES_DE_THEMATIQUES.size === 0) {
  signaler(join(FRONT, '../docs/database/020_reference.sql'), 'aucun terme negotiation_theme lu : le semis a changé de forme')
}
const libellesTrouves = (valeurs) => valeurs.filter((v) => LIBELLES_DE_THEMATIQUES.has(v.trim()))
const valeursDe = (objet) =>
  typeof objet === 'string' ? [objet] : Object.values(objet).flatMap(valeursDe)
for (const fichier of parcourir(join(FRONT, 'i18n/locales'))) {
  if (!fichier.endsWith('.json') || /\/components\/gn-planche-/.test(fichier)) continue
  const trouves = libellesTrouves(valeursDe(JSON.parse(readFileSync(fichier, 'utf8'))))
  if (trouves.length) signaler(fichier, `libellé de thématique recopié : ${trouves.join(', ')} — il vit en base`)
}
for (const fichier of parcourir(join(FRONT, 'app'))) {
  if (!/\.(vue|ts)$/.test(fichier) || /\/app\/mocks\//.test(fichier)) continue
  const litteraux = [...readFileSync(fichier, 'utf8').matchAll(/(['"`])([^'"`\n]{3,40})\1/g)].map((m) => m[2])
  const trouves = libellesTrouves(litteraux)
  if (trouves.length) signaler(fichier, `libellé de thématique en dur : ${trouves.join(', ')} — il vit en base`)
}

// Un test que la porte ne ramasse pas est pire qu'un test absent : il donne le vert
// sans avoir tourné. La porte est le glob `tests/guide-nego/*.test.ts`, et rien d'autre.
const HORS_PORTEE = new Set(['node_modules', '.nuxt', '.output', 'dist', 'tests', '.git'])
function testsEgares(dossier) {
  return readdirSync(dossier).flatMap((nom) => {
    if (HORS_PORTEE.has(nom)) return []
    const chemin = join(dossier, nom)
    if (statSync(chemin).isDirectory()) return testsEgares(chemin)
    return /\.test\.[cm]?[jt]s$/.test(nom) ? [chemin] : []
  })
}
for (const fichier of testsEgares(FRONT)) {
  signaler(fichier, 'un test hors de tests/ : la porte ne le voit pas, il ne tourne jamais')
}

if (ecarts.length > 0) {
  console.error(`Guide Négo — ${ecarts.length} écart(s) au bornage :\n`)
  for (const ecart of ecarts) console.error(`  ✗ ${ecart}`)
  process.exit(1)
}
console.log(`Guide Négo — ${fichiers.length} fichier(s) vérifié(s), bornage tenu.`)
