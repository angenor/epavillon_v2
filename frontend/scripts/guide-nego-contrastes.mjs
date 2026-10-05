#!/usr/bin/env node
/**
 * Contrastes de Guide Négo, thème unique « Nuit » (ADR-023).
 *
 * 7:1 pour le texte courant, 4,5:1 pour le texte secondaire, 3:1 pour ce qui n'est
 * pas du texte mais porte un sens. Une mesure à l'œil ne survit pas au premier jeton
 * ajouté.
 *
 * Les filets de la maquette (`--gn-filet`, `--gn-filet-doux`) ne sont pas mesurés :
 * ils séparent sans porter de sens, et un champ se reconnaît à son aplat de bloc et à
 * son libellé. Le créneau neutre du fil du jour (2,1:1 sur sa piste) non plus : la
 * maquette le veut ainsi, et chaque créneau se lit en entier dans l'agenda qu'il
 * résume. Ce qui porte un sens sans texte (focus, pictogramme, point d'état) l'est.
 */
import { readFileSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const FEUILLE = join(dirname(fileURLToPath(import.meta.url)), '../app/assets/guide-nego/theme.css')
const css = readFileSync(FEUILLE, 'utf8').replace(/\/\*[\s\S]*?\*\//g, '')

function declarations(selecteur) {
  const jetons = {}
  const motif = /([^{}]+)\{([^{}]*)\}/g
  for (const [, entete, corps] of css.matchAll(motif)) {
    if (entete.trim() !== selecteur) continue
    for (const [, nom, valeur] of corps.matchAll(/(--gn-[\w-]+)\s*:\s*([^;]+);/g)) {
      jetons[nom] = valeur.trim()
    }
  }
  return jetons
}

const nuit = declarations('[data-app="guide-nego"]')

function resoudre(jetons, nom, vus = new Set()) {
  if (vus.has(nom)) throw new Error(`boucle de var() sur ${nom}`)
  vus.add(nom)
  const valeur = jetons[nom]
  if (!valeur) throw new Error(`jeton inconnu : ${nom}`)
  const renvoi = valeur.match(/^var\((--gn-[\w-]+)\)$/)
  return renvoi ? resoudre(jetons, renvoi[1], vus) : valeur
}

function luminance(hex) {
  const [r, g, b] = [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
  const lineaire = (c) => (c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4)
  return 0.2126 * lineaire(r) + 0.7152 * lineaire(g) + 0.0722 * lineaire(b)
}

function contraste(a, b) {
  const [haut, bas] = [luminance(a), luminance(b)].sort((x, y) => y - x)
  return (haut + 0.05) / (bas + 0.05)
}

const COURANT = 7
const SECONDAIRE = 4.5
const NON_TEXTE = 3

// [avant-plan, fond, seuil, ce que c'est]
const PAIRES = [
  ['--gn-texte', '--gn-fond', COURANT, 'texte courant'],
  ['--gn-texte', '--gn-fond-2', COURANT, 'texte courant sur bloc'],
  ['--gn-texte', '--gn-presse', COURANT, 'texte sur ligne pressée'],
  ['--gn-texte', '--gn-bloc-releve', COURANT, 'initiales de l’avatar'],
  ['--gn-texte', '--gn-barre-fond', COURANT, 'barre d’onglets'],
  ['--gn-texte-lecture', '--gn-fond', COURANT, 'texte de lecture'],
  ['--gn-texte-lecture', '--gn-fond-2', COURANT, 'texte de lecture sur bloc'],
  ['--gn-titre', '--gn-fond', COURANT, 'titre'],
  ['--gn-accent', '--gn-fond', COURANT, 'compte à rebours, terme, lien'],
  ['--gn-accent', '--gn-barre-fond', COURANT, 'onglet actif'],
  ['--gn-accent', '--gn-fond-2', COURANT, 'accent sur bloc'],
  ['--gn-accent-inv', '--gn-accent', COURANT, 'texte sur action principale'],
  ['--gn-attention-aplat-texte', '--gn-attention-aplat', COURANT, 'bandeau de connexion'],
  ['--gn-texte', '--gn-attention-fond', COURANT, 'bandeau « Remplacé par… »'],
  ['--gn-bulle-envoyee-texte', '--gn-bulle-envoyee', COURANT, 'bulle envoyée'],
  ['--gn-sur-titre', '--gn-titre', COURANT, 'texte sur aplat de titre'],
  ['--gn-action-sur-titre', '--gn-titre', COURANT, 'action sur aplat de titre'],
  ['--gn-danger-texte', '--gn-danger-aplat', COURANT, 'texte sur bouton dangereux'],
  ['--gn-texte-2', '--gn-fond', SECONDAIRE, 'texte secondaire'],
  ['--gn-texte-2', '--gn-fond-2', SECONDAIRE, 'texte secondaire sur bloc'],
  ['--gn-texte-2', '--gn-bloc-releve', SECONDAIRE, 'marque « EN »'],
  ['--gn-texte-2', '--gn-barre-fond', SECONDAIRE, 'onglet inactif'],
  ['--gn-attention', '--gn-fond-2', SECONDAIRE, 'marque jaune sur bloc'],
  ['--gn-danger', '--gn-fond-2', SECONDAIRE, 'marque rouge sur bloc'],
  ['--gn-desactive-texte', '--gn-desactive-fond', SECONDAIRE, 'commande désactivée'],
  ['--gn-attention', '--gn-fond', SECONDAIRE, 'marque d’état jaune'],
  ['--gn-succes', '--gn-fond', SECONDAIRE, 'marque d’état verte'],
  ['--gn-information', '--gn-fond', SECONDAIRE, 'marque d’état cyan'],
  ['--gn-danger', '--gn-fond', SECONDAIRE, 'marque d’état rouge'],
  ['--gn-reseau', '--gn-fond', SECONDAIRE, 'marque d’état violette'],
  ['--gn-terminee', '--gn-fond', SECONDAIRE, 'marque d’état grise'],
  ['--gn-accent', '--gn-bloc-releve', NON_TEXTE, 'ma session sur sa piste'],
  ['--gn-focus', '--gn-fond', NON_TEXTE, 'anneau de focus'],
  ['--gn-focus', '--gn-fond-2', NON_TEXTE, 'anneau de focus sur bloc'],
  ['--gn-picto', '--gn-fond', NON_TEXTE, 'pictogramme'],
  ['--gn-picto-secondaire', '--gn-fond', NON_TEXTE, 'pictogramme secondaire'],
  ['--gn-picto-document', '--gn-fond', NON_TEXTE, 'pictogramme de document'],
]

let echecs = 0
const detail = process.argv.includes('--detail')
for (const [nom, jetons] of [['nuit', nuit]]) {
  for (const [avant, fond, seuil, quoi] of PAIRES) {
    const [a, b] = [resoudre(jetons, avant), resoudre(jetons, fond)]
    const mesure = contraste(a, b)
    const tenu = mesure >= seuil
    if (!tenu) echecs++
    if (!tenu || detail) {
      console.log(
        `  ${tenu ? '✓' : '✗'} ${nom.padEnd(6)} ${quoi.padEnd(34)} ${a} sur ${b}  ${mesure.toFixed(1)}:1 (seuil ${seuil})`,
      )
    }
  }
}

if (echecs > 0) {
  console.error(`\nGuide Négo — ${echecs} paire(s) sous le seuil.`)
  process.exit(1)
}
console.log(`Guide Négo — ${PAIRES.length} paires mesurées, toutes aux seuils.`)
