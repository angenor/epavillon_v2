#!/usr/bin/env node
/**
 * Icônes d'installation de Guide Négo.
 *
 * Une application installée garde ses icônes telles qu'elles ont été déposées : on les
 * engendre ici, on les commite, et le navigateur n'a rien à rastériser au vol.
 *
 * La source est `symbole-vectorise.svg` et non le symbole du site : celui-ci exprime sa
 * lettre « e » par un <text> en Helvetica Neue, que librsvg remplace en silence par la
 * police de la machine — l'icône changerait de forme d'un poste de travail à l'autre.
 */
import { mkdirSync, statSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import sharp from 'sharp'

const FRONT = join(dirname(fileURLToPath(import.meta.url)), '..')
const DOSSIER = join(FRONT, 'public/guide-nego/icones')
const SOURCE = join(DOSSIER, 'symbole-vectorise.svg')
const FOND = '#233400'

/** Part du carré occupée par le symbole : pleine pour une icône affichée telle quelle,
 *  zone de sécurité d'Android pour une icône `maskable`, que le masque rogne aux bords. */
const PLEINE = 0.86
const ZONE_SURE = 0.8

const ICONES = [
  ['192.png', 192, PLEINE],
  ['512.png', 512, PLEINE],
  ['192-masque.png', 192, ZONE_SURE],
  ['512-masque.png', 512, ZONE_SURE],
  ['180.png', 180, PLEINE],
]

const { width: largeurSource } = await sharp(SOURCE).metadata()

/** Le symbole seul, ajusté et centré dans un carré transparent de `cote` pixels. */
function symbole(cote) {
  // Rastérisé au double avant réduction : librsvg crénèle les obliques au ras du besoin.
  const densite = Math.ceil((96 * 2 * cote) / largeurSource)
  return sharp(SOURCE, { density: densite })
    .resize(cote, cote, { fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } })
    .png()
    .toBuffer()
}

mkdirSync(DOSSIER, { recursive: true })

for (const [fichier, taille, occupation] of ICONES) {
  const chemin = join(DOSSIER, fichier)
  await sharp({ create: { width: taille, height: taille, channels: 4, background: FOND } })
    .composite([{ input: await symbole(Math.round(taille * occupation)), gravity: 'center' }])
    .png({ compressionLevel: 9 })
    .toFile(chemin)

  const poids = (statSync(chemin).size / 1024).toFixed(1)
  const zone = occupation === ZONE_SURE ? 'masquable' : 'telle quelle'
  console.log(`  ✓ ${relative(FRONT, chemin).padEnd(38)} ${taille}×${taille}  ${poids.padStart(6)} kio  ${zone}`)
}

console.log(`Guide Négo — ${ICONES.length} icônes écrites sur fond ${FOND}.`)
