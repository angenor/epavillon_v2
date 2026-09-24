import { cpSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { defineNuxtModule } from '@nuxt/kit'
import { engendrerLaFeuille } from '../guide-nego/feuille-pdfjs'
import { DOSSIER_PDFJS, fichiersPdfjs, RACINE_PDFJS } from '../guide-nego/liste-de-garde'

/**
 * Ce que pdf.js lit par adresse, copié sous `public/guide-nego/pdfjs/<version>/` (ADR-022, R4),
 * et sa feuille bornée, écrite dans `app/assets/guide-nego/`. En développement comme en
 * construction. La liste copiée est celle que garde la coquille : `fichiersPdfjs`.
 */
export default defineNuxtModule({
  meta: { name: 'guide-nego-pdfjs' },
  setup(_options, nuxt) {
    const front = nuxt.options.rootDir
    const destination = join(front, 'public/guide-nego', DOSSIER_PDFJS)

    // Repartir de zéro : un fichier retiré de la liste, ou une ancienne version, ne doit pas rester servi.
    rmSync(join(front, 'public/guide-nego/pdfjs'), { recursive: true, force: true })
    for (const fichier of fichiersPdfjs()) {
      cpSync(join(RACINE_PDFJS, fichier), join(destination, fichier))
    }

    engendrerLaFeuille(front)
  },
})
