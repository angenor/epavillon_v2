/**
 * Agrégation de l'arborescence des traductions.
 *
 * Le découpage est PAR ÉCRAN : `pages/proposal.form.step-speakers.json` porte
 * les libellés de cette étape et de nulle autre. Pour éviter d'énumérer des
 * dizaines de fichiers dans `nuxt.config.ts`, `modules/i18n-messages.ts` balaie
 * l'arborescence à la construction et confie le résultat à cette fonction.
 *
 * RÈGLE DE NOMMAGE : la clé racine est le nom du fichier, sans son préfixe `_`
 * ni son dossier, et ses points deviennent des niveaux d'imbrication.
 *
 *   _common.json                        → common.*
 *   _nav.json                           → nav.*
 *   pages/auth.login.json               → auth.login.*
 *   pages/proposal.form.step-speakers.json → proposal.form['step-speakers'].*
 *   components/session-card.json        → session-card.*
 *
 * L'imbrication est indispensable : vue-i18n résout `auth.login.title` en
 * descendant l'arbre. Une clé littérale « auth.login » ne serait jamais trouvée.
 */

export type TranslationNode = string | { [key: string]: TranslationNode }
export type LocaleMessages = Record<string, TranslationNode>

const isRecord = (value: unknown): value is Record<string, TranslationNode> =>
  typeof value === 'object' && value !== null && !Array.isArray(value)

/** Fusionne `source` dans `target` en profondeur, sans écraser une branche entière. */
function deepMerge(target: Record<string, TranslationNode>, source: Record<string, TranslationNode>): void {
  for (const [key, value] of Object.entries(source)) {
    const existing = target[key]
    if (isRecord(existing) && isRecord(value)) {
      deepMerge(existing, value)
    } else {
      target[key] = value
    }
  }
}

/** Dérive le chemin de clés depuis le nom du fichier : `pages/a.b.json` → ['a', 'b']. */
export function keyPathFromFile(filePath: string): string[] {
  const fileName = filePath.split('/').pop() ?? ''
  return fileName
    .replace(/\.json$/, '')
    .replace(/^_/, '')
    .split('.')
    .filter(Boolean)
}

/**
 * @param modules dictionnaire `{ './fr/_common.json': { … } }` produit par
 *                `modules/i18n-messages.ts`.
 */
export function mergeLocaleMessages(modules: Record<string, unknown>): LocaleMessages {
  const messages: LocaleMessages = {}

  // Tri explicite : l'ordre de `import.meta.glob` ne fait pas partie de son
  // contrat, et deux fichiers peuvent alimenter la même branche.
  for (const filePath of Object.keys(modules).sort()) {
    const content = modules[filePath]
    if (!isRecord(content)) {
      throw new Error(`Traductions : ${filePath} ne contient pas un objet JSON.`)
    }

    const path = keyPathFromFile(filePath)
    if (path.length === 0) {
      throw new Error(`Traductions : nom de fichier inexploitable — ${filePath}`)
    }

    let branch: Record<string, TranslationNode> = messages
    for (const segment of path) {
      const next = branch[segment]
      if (!isRecord(next)) {
        const created: Record<string, TranslationNode> = {}
        branch[segment] = created
        branch = created
      } else {
        branch = next
      }
    }
    deepMerge(branch, content)
  }

  return messages
}
