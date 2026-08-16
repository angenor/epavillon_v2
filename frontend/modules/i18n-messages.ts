import { mkdirSync, readdirSync, statSync, writeFileSync } from 'node:fs'
import { join, relative } from 'node:path'
import { createResolver, defineNuxtModule } from '@nuxt/kit'

/**
 * Agrégation de l'arborescence des traductions, par écran.
 *
 * POURQUOI CE MODULE
 * ------------------
 * La première version balayait le dossier directement depuis
 * `i18n/locales/fr.ts` avec `import.meta.glob`. Le rendu client fonctionnait, le
 * rendu serveur non : Nitro compile ces fichiers hors du pipeline Vite et
 * remplace `import.meta` par `globalThis._importMeta_`, qui n'a pas de `glob`.
 * La page se rendait alors avec ses clés brutes (« nav.site.name »), et rien ne
 * le signalait à la construction — l'erreur n'apparaissait qu'au premier rendu.
 *
 * La deuxième version passait par un template `.nuxt/` importé via `#build` :
 * refusée elle aussi, « Vue app aliases are not allowed in server runtime ».
 *
 * Ce module écrit donc un module d'agrégation ordinaire, dans le dossier des
 * locales et à côté de ce qu'il agrège. Des imports statiques et des chemins
 * relatifs : rien que Vite et Nitro ne sachent résoudre l'un comme l'autre.
 *
 * L'objectif d'origine est conservé : AJOUTER UN ÉCRAN, C'EST AJOUTER UN
 * FICHIER JSON. Ni `nuxt.config.ts` ni les points d'entrée de locale ne changent.
 *
 * Le dossier `i18n/locales/.generated/` n'est pas versionné : il se reconstruit
 * à chaque démarrage.
 */
export default defineNuxtModule({
  meta: {
    name: 'i18n-messages',
    configKey: 'i18nMessages',
  },
  async setup(_options, nuxt) {
    const resolver = createResolver(import.meta.url)
    const localesDir = resolver.resolve('../i18n/locales')
    const generatedDir = join(localesDir, '.generated')

    /** Un dossier de `i18n/locales/` = une locale. Le dossier généré est écarté. */
    const localeCodes = (): string[] =>
      readdirSync(localesDir)
        .filter((entry) => !entry.startsWith('.'))
        .filter((entry) => statSync(join(localesDir, entry)).isDirectory())
        .sort()

    /** Tous les `.json` d'une locale, chemins relatifs à `i18n/locales/`. */
    const jsonFilesOf = (code: string): string[] => {
      const walk = (dir: string): string[] =>
        readdirSync(dir).flatMap((entry) => {
          const full = join(dir, entry)
          if (statSync(full).isDirectory()) return walk(full)
          return entry.endsWith('.json') ? [full] : []
        })
      return walk(join(localesDir, code))
        .map((file) => relative(localesDir, file).split('\\').join('/'))
        .sort()
    }

    const contentsFor = (code: string): string => {
      const files = jsonFilesOf(code)
      const imports = files
        .map((file, index) => `import m${index} from '../${file}'`)
        .join('\n')
      const entries = files
        .map((file, index) => `  ${JSON.stringify(`./${file}`)}: m${index},`)
        .join('\n')

      return [
        '// Fichier généré par modules/i18n-messages.ts — ne pas modifier à la main.',
        `// ${files.length} fichier(s) de traduction pour la locale « ${code} ».`,
        "import { mergeLocaleMessages } from '../../merge-messages'",
        imports,
        '',
        'export default mergeLocaleMessages({',
        entries,
        '})',
        '',
      ].join('\n')
    }

    const generate = (): void => {
      mkdirSync(generatedDir, { recursive: true })
      for (const code of localeCodes()) {
        writeFileSync(join(generatedDir, `${code}.ts`), contentsFor(code), 'utf8')
      }
    }

    generate()

    // Ajouter ou retirer un fichier de traduction régénère l'agrégation sans
    // redémarrage. Modifier son CONTENU ne passe pas par ici : Vite recharge le
    // JSON tout seul, l'agrégation n'ayant pas changé.
    //
    // Le hook `builder:watch` de Nuxt ne convient pas : il ne surveille que
    // `app/` et quelques fichiers de la racine, jamais `i18n/`. D'où un
    // observateur dédié, actif en développement seulement.
    if (nuxt.options.dev) {
      const { watch } = await import('chokidar')
      const watcher = watch(localesDir, { ignoreInitial: true })
      const onChange = (path: string) => {
        if (path.endsWith('.json')) generate()
      }
      watcher.on('add', onChange)
      watcher.on('unlink', onChange)
      nuxt.hook('close', () => watcher.close())
    }
  },
})
