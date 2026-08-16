import messages from './.generated/fr'

// Point d'entrée de la locale française — langue par défaut, langue pivot de la
// Francophonie.
//
// Le contenu vient de `i18n/locales/fr/`, agrégé à la construction par
// `modules/i18n-messages.ts` : créer un écran, c'est ajouter un fichier dans
// `fr/pages/`, sans jamais toucher à ce fichier ni à `nuxt.config.ts`.
// Voir l'en-tête du module pour la raison — un `import.meta.glob` posé ici ne
// survivrait pas au rendu serveur.
export default defineI18nLocale(() => messages)
