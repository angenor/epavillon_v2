import messages from './.generated/en'

// English entry point — mirrors `locales/fr/` file for file. A missing key here
// falls back to French, which is the platform's pivot language.
export default defineI18nLocale(() => messages)
