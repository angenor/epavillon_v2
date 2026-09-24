import { test } from 'node:test'
import assert from 'node:assert/strict'
import { existsSync, readFileSync } from 'node:fs'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import {
  BORNE,
  bornerLaFeuille,
  EN_TETE,
  engendrerLaFeuille,
  FEUILLE_ENGENDREE,
  REGLE_DE_DIMENSIONNEMENT,
  SOURCE_DE_LA_FEUILLE,
} from '../../guide-nego/feuille-pdfjs.ts'

const FRONT = join(dirname(fileURLToPath(import.meta.url)), '../..')
const source = readFileSync(join(FRONT, SOURCE_DE_LA_FEUILLE), 'utf8')
const feuille = bornerLaFeuille(source)

/** Les sélecteurs de premier niveau : hors des règles, `@media` et `@supports` ouverts. */
function selecteursDePremierNiveau(css: string): string[] {
  const propre = css.replace(/\/\*[\s\S]*?\*\//g, '').replace(/"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/g, (chaine) => chaine.replace(/[{};]/g, ' '))
  const trouves: string[] = []
  const pile: string[] = []
  let tampon = ''
  for (const caractere of propre) {
    if (caractere === '{') {
      const entete = tampon.trim()
      const groupe = /^@(media|supports)\b/.test(entete)
      if (!pile.includes('regle') && !pile.includes('brut') && !groupe) trouves.push(entete)
      pile.push(groupe ? 'groupe' : entete.startsWith('@') ? 'brut' : 'regle')
      tampon = ''
    } else if (caractere === '}' || caractere === ';') {
      if (caractere === '}') pile.pop()
      tampon = ''
    } else tampon += caractere
  }
  return trouves
}

const decouper = (entete: string) => {
  const morceaux = ['']
  let profondeur = 0
  for (const c of entete) {
    if (c === '(' || c === '[') profondeur++
    if (c === ')' || c === ']') profondeur--
    if (c === ',' && profondeur === 0) morceaux.push('')
    else morceaux[morceaux.length - 1] += c
  }
  return morceaux.map((s) => s.trim())
}

test('la feuille du contrôle est celle de la construction, octet pour octet', () => {
  const engendree = engendrerLaFeuille(FRONT)
  assert.equal(engendree, feuille)
  assert.equal(readFileSync(join(FRONT, FEUILLE_ENGENDREE), 'utf8'), feuille)
  assert.equal(bornerLaFeuille(source), feuille, 'la fonction est déterministe')
  // Les deux appellent la même fonction, sur le même fichier.
  for (const appelant of ['modules/guide-nego-pdfjs.ts', 'scripts/check-guide-nego.mjs']) {
    assert.match(readFileSync(join(FRONT, appelant), 'utf8'), /engendrerLaFeuille\((front|FRONT)\)/, appelant)
  }
})

test('aucun sélecteur hors de la borne, et plus de :root', () => {
  const selecteurs = selecteursDePremierNiveau(feuille).flatMap(decouper)
  assert.ok(selecteurs.length > 100, `${selecteurs.length} sélecteurs lus : la feuille a-t-elle changé de forme ?`)
  const hors = selecteurs.filter((s) => !s.startsWith(BORNE))
  assert.deepEqual(hors, [])
  assert.ok(!selecteurs.some((s) => s.includes(':root')))
})

test('la feuille se termine par la règle content-box, et commence par son en-tête', () => {
  assert.ok(feuille.trimEnd().endsWith(REGLE_DE_DIMENSIONNEMENT))
  assert.ok(feuille.startsWith(`${EN_TETE}\n`))
  assert.equal(
    REGLE_DE_DIMENSIONNEMENT,
    '[data-app="guide-nego"] .pdfViewer, [data-app="guide-nego"] .pdfViewer * { box-sizing: content-box; }',
  )
})

test('les jetons du site cèdent à leur valeur de repli', () => {
  assert.doesNotMatch(feuille, /var\(\s*--(color|ifdd)-/)
  assert.match(feuille, /--outline-color:#0060df;/)
})

test('les images du visionneur restent atteignables depuis la feuille engendrée', () => {
  const images = [...feuille.matchAll(/url\((?!["']?(?:data:|#))["']?([^)"']+)/g)].map((m) => m[1]!.trim())
  assert.ok(images.length > 0)
  for (const image of images) {
    assert.ok(existsSync(resolve(FRONT, dirname(FEUILLE_ENGENDREE), image)), image)
  }
})

test('le schéma de couleurs de la borne reste celui du thème de Guide Négo', () => {
  const fin = feuille.indexOf('--viewer-container-height')
  const racine = feuille.slice(feuille.lastIndexOf(`${BORNE}{`, fin), fin)
  assert.match(source, /:root\{\s*color-scheme:light dark;\s*--viewer-container-height/, 'la source le déclare bien là')
  assert.doesNotMatch(racine, /color-scheme/)
  assert.match(feuille, /color-scheme:only light;/, 'ailleurs, il reste')
})

test('règles imbriquées, @keyframes, @font-face et @media : chacun à sa place', () => {
  const css = `:root{--a:1;}
@media (forced-colors){ :root{--a:2;} .x, .y:is(.a, .b){ color:red; } }
@keyframes tourner{ from{ rotate:0 } to{ rotate:1turn } }
@font-face{ font-family:"P"; src:url(p.woff2); }
.page{ & .texte{ color:red; } > div{ margin:0; } }
/* commentaire */ .avec{ content:"{ ; }"; }`
  const borne = bornerLaFeuille(css)
  assert.match(borne, /\[data-app="guide-nego"\]\{--a:1;\}/)
  assert.match(borne, /@media \(forced-colors\)\{ \[data-app="guide-nego"\]\{--a:2;\}/)
  assert.match(borne, / \[data-app="guide-nego"\] \.x, \[data-app="guide-nego"\] \.y:is\(\.a, \.b\)\{/)
  assert.match(borne, /@keyframes tourner\{ from\{ rotate:0 \} to\{ rotate:1turn \} \}/)
  assert.match(borne, /@font-face\{ font-family:"P"; src:url\(p\.woff2\); \}/)
  assert.match(borne, /\[data-app="guide-nego"\] \.page\{ & \.texte\{ color:red; \} > div\{ margin:0; \} \}/)
  assert.match(borne, /\/\* commentaire \*\/ \[data-app="guide-nego"\] \.avec\{ content:"\{ ; \}"; \}/)
  assert.equal(bornerLaFeuille(borne).split(BORNE).length, borne.split(BORNE).length + 2, 'la borne ne se double pas')
})
