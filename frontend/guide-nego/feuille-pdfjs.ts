/**
 * La feuille du visionneur de pdf.js, bornée à Guide Négo (R4, ADR-022).
 *
 * `pdf_viewer.css` déclare ses variables sur `:root` et ses sélecteurs au niveau du
 * document : importée telle quelle, elle toucherait le site. `bornerLaFeuille` en
 * écrit une copie où chaque sélecteur de premier niveau part de la borne. Les règles
 * imbriquées restent relatives à leur parent, `@keyframes` et `@font-face` ne bougent pas.
 *
 * La construction (`modules/guide-nego-pdfjs.ts`) et le contrôle
 * (`scripts/check-guide-nego.mjs`) appellent tous deux `engendrerLaFeuille`.
 */
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

export const BORNE = '[data-app="guide-nego"]'
export const SOURCE_DE_LA_FEUILLE = 'node_modules/pdfjs-dist/legacy/web/pdf_viewer.css'
export const FEUILLE_ENGENDREE = 'app/assets/guide-nego/pdfjs-viewer.css'

export const EN_TETE =
  '/* Engendré par guide-nego/feuille-pdfjs.ts depuis pdfjs-dist/legacy/web/pdf_viewer.css. Ne pas modifier. */'

/** pdf.js compte en `content-box` ; sans cette règle, la couche de texte dérive d'une ligne ou plus. */
export const REGLE_DE_DIMENSIONNEMENT = `${BORNE} .pdfViewer, ${BORNE} .pdfViewer * { box-sizing: content-box; }`

// Depuis `app/assets/guide-nego/`, vers les images du paquet.
const IMAGES_DU_PAQUET = '../../../node_modules/pdfjs-dist/legacy/web/images/'

/** Les blocs dont le contenu n'est pas fait de règles à borner. */
const AT_REGLES_DE_GROUPE = /^@(media|supports|layer|container|starting-style|document)\b/

type Contexte = 'groupe' | 'regle' | 'racine' | 'brut'

/** Coupe aux virgules qui ne sont ni entre parenthèses, ni entre crochets, ni dans une chaîne. */
function decouperLaListe(prelude: string): string[] {
  const morceaux = ['']
  let profondeur = 0
  let guillemet: string | null = null
  for (const caractere of prelude) {
    if (guillemet) {
      if (caractere === guillemet) guillemet = null
    } else if (caractere === '"' || caractere === "'") guillemet = caractere
    else if (caractere === '(' || caractere === '[') profondeur++
    else if (caractere === ')' || caractere === ']') profondeur--
    else if (caractere === ',' && profondeur === 0) {
      morceaux.push('')
      continue
    }
    morceaux[morceaux.length - 1] += caractere
  }
  return morceaux
}

function bornerLeSelecteur(selecteur: string): string {
  const blanc = selecteur.match(/^\s*/)![0]
  const coeur = selecteur.slice(blanc.length).trimEnd()
  if (coeur.startsWith(BORNE)) return blanc + coeur
  if (coeur.startsWith(':root')) return blanc + BORNE + coeur.slice(':root'.length)
  return `${blanc}${BORNE} ${coeur}`
}

/** Un appel de jeton du site (préfixes `color` et `ifdd`) cède la place à sa valeur de repli : ces jetons n’entrent pas dans Guide Négo. */
function retirerLesJetonsDuSite(css: string): string {
  const motif = /var\(\s*--(?:color|ifdd)-/g
  let sortie = ''
  let depuis = 0
  for (let trouve = motif.exec(css); trouve; trouve = motif.exec(css)) {
    let profondeur = 0
    let virgule = -1
    let fin = trouve.index
    for (; fin < css.length; fin++) {
      const caractere = css[fin]
      if (caractere === '(') profondeur++
      else if (caractere === ')' && --profondeur === 0) break
      else if (caractere === ',' && profondeur === 1 && virgule < 0) virgule = fin
    }
    if (virgule < 0) continue
    sortie += css.slice(depuis, trouve.index) + css.slice(virgule + 1, fin).trim()
    depuis = fin + 1
    motif.lastIndex = depuis
  }
  return sortie + css.slice(depuis)
}

export function bornerLaFeuille(source: string): string {
  const css = retirerLesJetonsDuSite(source).replace(/url\((['"]?)images\//g, `url($1${IMAGES_DU_PAQUET}`)
  const pile: Contexte[] = []
  let sortie = ''
  let tampon = ''

  for (let i = 0; i < css.length; i++) {
    const caractere = css[i]!

    if (caractere === '/' && css[i + 1] === '*') {
      const fin = css.indexOf('*/', i + 2)
      const bout = fin < 0 ? css.length : fin + 2
      tampon += css.slice(i, bout)
      i = bout - 1
      continue
    }
    if (caractere === '"' || caractere === "'") {
      let fin = i + 1
      while (fin < css.length && css[fin] !== caractere) fin += css[fin] === '\\' ? 2 : 1
      tampon += css.slice(i, fin + 1)
      i = fin
      continue
    }

    if (caractere === '{') {
      const parent = pile.at(-1)
      const prelude = tampon.replace(/\/\*[\s\S]*?\*\//g, '').trim()
      if (parent === 'brut' || parent === 'regle' || parent === 'racine') {
        sortie += tampon + caractere
        pile.push(parent === 'brut' ? 'brut' : 'regle')
      } else if (prelude.startsWith('@')) {
        sortie += tampon + caractere
        pile.push(AT_REGLES_DE_GROUPE.test(prelude) ? 'groupe' : 'brut')
      } else {
        const avant = tampon.match(/^(?:\s|\/\*[\s\S]*?\*\/)*/)![0]
        const selecteurs = decouperLaListe(tampon.slice(avant.length).replace(/\/\*[\s\S]*?\*\//g, '').trimEnd())
        sortie += avant + selecteurs.map(bornerLeSelecteur).join(',') + caractere
        pile.push(prelude === ':root' ? 'racine' : 'regle')
      }
      tampon = ''
    } else if (caractere === ';') {
      // Le schéma de couleurs de la borne est celui du thème de Guide Négo, pas celui de pdf.js.
      if (!(pile.at(-1) === 'racine' && /^\s*color-scheme\s*:/.test(tampon))) sortie += tampon + caractere
      tampon = ''
    } else if (caractere === '}') {
      sortie += tampon + caractere
      tampon = ''
      pile.pop()
    } else {
      tampon += caractere
    }
  }

  return `${EN_TETE}\n${sortie}${tampon}\n\n${REGLE_DE_DIMENSIONNEMENT}\n`
}

/** Lit la feuille du paquet, la borne, et ne réécrit le fichier que s'il change. */
export function engendrerLaFeuille(front: string): string {
  const feuille = bornerLaFeuille(readFileSync(join(front, SOURCE_DE_LA_FEUILLE), 'utf8'))
  const destination = join(front, FEUILLE_ENGENDREE)
  if (!existsSync(destination) || readFileSync(destination, 'utf8') !== feuille) {
    mkdirSync(dirname(destination), { recursive: true })
    writeFileSync(destination, feuille, 'utf8')
  }
  return feuille
}
