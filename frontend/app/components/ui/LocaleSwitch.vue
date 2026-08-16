<script setup lang="ts">
import type { LocaleCode } from '~/types/shared'

/**
 * Sélecteur de langue.
 *
 * DES LIENS, PAS DES BOUTONS. Chaque langue a sa propre URL
 * (`/programme` et `/en/programme`, stratégie `prefix_except_default`) :
 * `switchLocalePath()` donne l'adresse équivalente dans l'autre langue, en
 * conservant la page courante. Un bouton qui changerait la langue sans changer
 * l'URL rendrait la page impossible à partager dans la langue qu'on lit.
 *
 * `lang` et `hreflang` sur chaque lien : c'est ce qui permet à un lecteur
 * d'écran de prononcer « English » avec l'accent anglais, et aux moteurs de
 * comprendre l'équivalence des deux pages.
 *
 * Le code de langue en majuscules (FR, EN) plutôt qu'un drapeau : une langue
 * n'est pas un pays, et le français n'est le drapeau de personne en particulier
 * dans la Francophonie.
 */

const { t, locale, locales } = useI18n()
const switchLocalePath = useSwitchLocalePath()

const available = computed(() =>
  (unref(locales) as Array<{ code: LocaleCode; name?: string; language?: string }>).map((entry) => ({
    code: entry.code,
    name: entry.name ?? entry.code,
    language: entry.language ?? entry.code,
  })),
)
</script>

<template>
  <nav class="flex items-center rounded-md border border-border" :aria-label="t('nav.language.label')">
    <NuxtLink
      v-for="entry in available"
      :key="entry.code"
      :to="switchLocalePath(entry.code)"
      class="px-2.5 py-1.5 text-xs font-semibold uppercase no-underline transition-colors first:rounded-l-md last:rounded-r-md"
      :class="
        entry.code === locale
          ? 'bg-accent-surface text-accent'
          : 'text-text-subtle hover:bg-surface-hover hover:text-text'
      "
      :aria-current="entry.code === locale ? 'true' : undefined"
      :lang="entry.language"
      :hreflang="entry.language"
    >
      <span class="sr-only">{{ entry.name }}</span>
      <span aria-hidden="true">{{ entry.code }}</span>
    </NuxtLink>
  </nav>
</template>
