// Options passées à vue-i18n. La configuration du module (locales, stratégie
// d'URL, détection) reste dans nuxt.config.ts ; ce fichier ne porte que le
// comportement d'exécution de la bibliothèque.
export default defineI18nConfig(() => ({
  legacy: false,
  // Le français est la langue pivot : une clé absente de l'anglais s'affiche en
  // français plutôt qu'en clé brute.
  fallbackLocale: 'fr',
  // Les replis sont normaux et attendus pendant la construction des écrans :
  // inutile d'en faire du bruit en console.
  fallbackWarn: false,
  missingWarn: import.meta.dev,
}))
