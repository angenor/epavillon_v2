import tailwindcss from '@tailwindcss/vite'

// ePavillon v2 — configuration Nuxt.
//
// Rappel d'emplacement : en Nuxt 4, `srcDir` vaut `app/`. Tout le code
// applicatif vit donc sous `frontend/app/` et y est auto-importé. Seul
// `frontend/i18n/` reste hors de `app/` : c'est la convention du module
// @nuxtjs/i18n. Un fichier posé ailleurs n'est pas auto-importé et l'erreur
// n'apparaît qu'à l'exécution.
export default defineNuxtConfig({
  compatibilityDate: '2026-08-16',

  devtools: { enabled: true },

  modules: ['@nuxtjs/i18n', '@pinia/nuxt'],

  css: ['~/assets/css/main.css'],

  // TailwindCSS v4 s'installe comme greffon Vite : il n'y a ni fichier de
  // configuration JavaScript, ni directive `@tailwind`. Le thème est décrit en
  // CSS dans app/assets/css/design-tokens.css.
  vite: {
    plugins: [tailwindcss()],
  },

  typescript: {
    strict: true,
    // La vérification passe par `npm run typecheck` (appelé par `make check`),
    // pas à chaque rechargement à chaud.
    typeCheck: false,
  },

  // Connexion à l'API. Déclarée dès maintenant, inutilisée jusqu'au prompt B7 :
  // les écrans du jalon travaillent sur des données simulées.
  // Surchargeable par la variable d'environnement NUXT_PUBLIC_API_BASE, lue
  // dans le `.env` de la racine du dépôt (voir le drapeau --dotenv des scripts npm).
  runtimeConfig: {
    public: {
      apiBase: 'http://localhost:8080/api',
    },
  },

  i18n: {
    // Les fichiers de traduction vivent dans frontend/i18n/locales/.
    // Chaque locale a un point d'entrée unique qui agrège l'arborescence
    // (voir modules/i18n-messages.ts) : ajouter un écran ne demande aucune
    // modification de ce fichier.
    langDir: 'locales',
    // Adresse publique du site : elle sert aux liens alternatifs de langue
    // (`hreflang`) et aux URL canoniques. Sans elle, le module avertit à chaque
    // rendu que les balises SEO produites sont incomplètes.
    baseUrl: process.env.NUXT_PUBLIC_SITE_URL || 'http://localhost:3000',
    defaultLocale: 'fr',
    strategy: 'prefix_except_default',
    // Pas d'option `lazy` : elle a disparu en @nuxtjs/i18n v10, où le chargement
    // à la demande est le comportement par défaut dès qu'une locale a un `file`.
    // Chaque `locales/<code>.ts` est donc importé dynamiquement, et l'agrégation
    // produite par modules/i18n-messages.ts part dans ce paquet-là.
    locales: [
      { code: 'fr', language: 'fr-FR', name: 'Français', file: 'fr.ts', dir: 'ltr' },
      { code: 'en', language: 'en-GB', name: 'English', file: 'en.ts', dir: 'ltr' },
    ],
    detectBrowserLanguage: {
      useCookie: true,
      cookieKey: 'epavillon_locale',
      cookieCrossOrigin: false,
      redirectOn: 'root',
      alwaysRedirect: false,
      fallbackLocale: 'fr',
    },
  },

  app: {
    head: {
      htmlAttrs: { lang: 'fr' },
      meta: [
        { charset: 'utf-8' },
        { name: 'viewport', content: 'width=device-width, initial-scale=1' },
        // La couleur de la barre d'adresse suit le thème actif.
        { name: 'theme-color', content: '#ffffff', media: '(prefers-color-scheme: light)' },
        { name: 'theme-color', content: '#231f20', media: '(prefers-color-scheme: dark)' },
      ],
      link: [{ rel: 'icon', type: 'image/svg+xml', href: '/favicon.svg' }],
    },
  },
})
