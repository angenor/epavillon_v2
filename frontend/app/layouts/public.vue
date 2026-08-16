<script setup lang="ts">
import type { NavItem } from '~/types/navigation'
import type { LocaleCode } from '~/types/shared'
import type { ThemeChoice } from '~/stores/preferences'

/**
 * Layout public — barre de navigation, sélecteur de langue, bascule de thème,
 * pied de page.
 *
 * Les liens pointent vers les écrans du jalon en cours ; ils sont créés par les
 * prompts A1 à A5. Les modules affichant « En cours de maintenance »
 * (Publications, Négociations, Formations, Outils) n'ont volontairement aucune
 * entrée ici tant que `platform.feature_flags` ne les ouvre pas.
 */

const { t, locale, locales } = useI18n()
const localePath = useLocalePath()
const switchLocalePath = useSwitchLocalePath()
const route = useRoute()
const preferences = usePreferencesStore()

const mainNav: NavItem[] = [
  { labelKey: 'nav.main.home', to: '/' },
  { labelKey: 'nav.main.events', to: '/evenements' },
  { labelKey: 'nav.main.programme', to: '/programme' },
  { labelKey: 'nav.main.call', to: '/appel-a-propositions' },
]

const footerSections: { labelKey: string; items: NavItem[] }[] = [
  {
    labelKey: 'nav.footer.sections.platform',
    items: [
      { labelKey: 'nav.main.events', to: '/evenements' },
      { labelKey: 'nav.main.programme', to: '/programme' },
      { labelKey: 'nav.main.call', to: '/appel-a-propositions' },
    ],
  },
  {
    labelKey: 'nav.footer.sections.resources',
    items: [
      { labelKey: 'nav.footer.help', to: '/aide' },
      { labelKey: 'nav.footer.accessibility', to: '/accessibilite' },
      { labelKey: 'nav.footer.contact', to: '/contact' },
    ],
  },
  {
    labelKey: 'nav.footer.sections.institution',
    items: [
      { labelKey: 'nav.main.about', to: '/a-propos' },
      { labelKey: 'nav.footer.legal', to: '/mentions-legales' },
      { labelKey: 'nav.footer.privacy', to: '/confidentialite' },
      { labelKey: 'nav.footer.terms', to: '/conditions-utilisation' },
    ],
  },
]

const themeChoices: { value: ThemeChoice; labelKey: string }[] = [
  { value: 'system', labelKey: 'nav.theme.system' },
  { value: 'light', labelKey: 'nav.theme.light' },
  { value: 'dark', labelKey: 'nav.theme.dark' },
]

const availableLocales = computed(() =>
  (unref(locales) as Array<{ code: LocaleCode; name?: string }>).map((entry) => ({
    code: entry.code,
    name: entry.name ?? entry.code,
  })),
)

const isMobileNavOpen = ref(false)
watch(() => route.fullPath, () => (isMobileNavOpen.value = false))

const isCurrent = (to: string) => route.path === localePath(to)
const currentYear = new Date().getFullYear()
</script>

<template>
  <div class="min-h-screen flex flex-col bg-surface text-text">
    <a class="skip-link" href="#contenu-principal">{{ t('common.a11y.skipToContent') }}</a>

    <header class="sticky top-0 z-30 border-b border-border bg-surface-raised">
      <div class="mx-auto flex w-full max-w-[1280px] items-center gap-4 px-4 py-3 sm:px-6">
        <NuxtLink :to="localePath('/')" class="flex shrink-0 items-center gap-3 no-underline">
          <img
            src="/logos/ifdd-horizontal-gris.svg"
            :alt="t('nav.site.owner')"
            class="h-9 w-auto dark:hidden"
            width="176"
            height="36"
          >
          <img
            src="/logos/ifdd-horizontal-blanc.svg"
            :alt="t('nav.site.owner')"
            class="hidden h-9 w-auto dark:block"
            width="176"
            height="36"
          >
          <span class="sr-only sm:not-sr-only sm:inline-block sm:border-l sm:border-border sm:pl-3">
            <span class="block font-display text-lg leading-tight text-text">{{ t('nav.site.name') }}</span>
          </span>
        </NuxtLink>

        <nav
          class="ml-auto hidden items-center gap-1 lg:flex"
          :aria-label="t('nav.main.home')"
        >
          <NuxtLink
            v-for="item in mainNav"
            :key="item.to"
            :to="localePath(item.to)"
            class="rounded-md px-3 py-2 text-sm no-underline text-text-muted hover:bg-surface-hover hover:text-text"
            :class="isCurrent(item.to) ? 'text-accent font-semibold' : ''"
            :aria-current="isCurrent(item.to) ? 'page' : undefined"
          >
            {{ t(item.labelKey) }}
          </NuxtLink>
        </nav>

        <div class="ml-auto flex items-center gap-2 lg:ml-0">
          <!-- Sélecteur de langue -->
          <nav class="hidden items-center rounded-md border border-border sm:flex" :aria-label="t('nav.language.label')">
            <NuxtLink
              v-for="entry in availableLocales"
              :key="entry.code"
              :to="switchLocalePath(entry.code)"
              class="px-2.5 py-1.5 text-xs font-semibold uppercase no-underline first:rounded-l-md last:rounded-r-md"
              :class="
                entry.code === locale
                  ? 'bg-accent-surface text-accent'
                  : 'text-text-subtle hover:bg-surface-hover hover:text-text'
              "
              :aria-current="entry.code === locale ? 'true' : undefined"
              :lang="entry.code"
            >
              {{ entry.code }}
            </NuxtLink>
          </nav>

          <!-- Bascule de thème -->
          <div
            class="flex items-center rounded-md border border-border"
            role="group"
            :aria-label="t('nav.theme.label')"
          >
            <button
              v-for="choice in themeChoices"
              :key="choice.value"
              type="button"
              class="px-2.5 py-1.5 text-xs first:rounded-l-md last:rounded-r-md"
              :class="
                preferences.theme === choice.value
                  ? 'bg-accent-surface text-accent'
                  : 'text-text-subtle hover:bg-surface-hover hover:text-text'
              "
              :aria-pressed="preferences.theme === choice.value"
              :title="t(choice.labelKey)"
              @click="preferences.setTheme(choice.value)"
            >
              <span class="sr-only">{{ t(choice.labelKey) }}</span>
              <svg
                v-if="choice.value === 'light'"
                class="size-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.7"
                aria-hidden="true"
              >
                <circle cx="12" cy="12" r="4.2" />
                <path
                  d="M12 2.6v2.2M12 19.2v2.2M4.2 12H2M22 12h-2.2M6.3 6.3 4.8 4.8M19.2 19.2l-1.5-1.5M17.7 6.3l1.5-1.5M4.8 19.2l1.5-1.5"
                  stroke-linecap="round"
                />
              </svg>
              <svg
                v-else-if="choice.value === 'dark'"
                class="size-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.7"
                aria-hidden="true"
              >
                <path d="M20 14.2A8.4 8.4 0 0 1 9.8 4a8.4 8.4 0 1 0 10.2 10.2Z" stroke-linejoin="round" />
              </svg>
              <svg
                v-else
                class="size-4"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.7"
                aria-hidden="true"
              >
                <rect x="3" y="4.5" width="18" height="12" rx="1.5" />
                <path d="M8.5 20h7M12 16.5V20" stroke-linecap="round" />
              </svg>
            </button>
          </div>

          <NuxtLink
            :to="localePath('/connexion')"
            class="hidden rounded-md border border-border px-3 py-2 text-sm no-underline text-text hover:bg-surface-hover sm:inline-block"
          >
            {{ t('nav.account.login') }}
          </NuxtLink>

          <button
            type="button"
            class="rounded-md border border-border p-2 text-text lg:hidden"
            :aria-expanded="isMobileNavOpen"
            aria-controls="navigation-mobile"
            @click="isMobileNavOpen = !isMobileNavOpen"
          >
            <span class="sr-only">
              {{ isMobileNavOpen ? t('common.a11y.closeMenu') : t('common.a11y.openMenu') }}
            </span>
            <svg
              class="size-5"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              stroke-width="1.8"
              aria-hidden="true"
            >
              <path v-if="!isMobileNavOpen" d="M4 7h16M4 12h16M4 17h16" stroke-linecap="round" />
              <path v-else d="M6 6l12 12M18 6 6 18" stroke-linecap="round" />
            </svg>
          </button>
        </div>
      </div>

      <nav
        v-if="isMobileNavOpen"
        id="navigation-mobile"
        class="border-t border-border-subtle bg-surface-raised px-4 py-2 lg:hidden"
        :aria-label="t('nav.main.home')"
      >
        <NuxtLink
          v-for="item in mainNav"
          :key="item.to"
          :to="localePath(item.to)"
          class="block rounded-md px-3 py-2.5 text-sm no-underline text-text-muted hover:bg-surface-hover hover:text-text"
          :class="isCurrent(item.to) ? 'text-accent font-semibold' : ''"
          :aria-current="isCurrent(item.to) ? 'page' : undefined"
        >
          {{ t(item.labelKey) }}
        </NuxtLink>
        <div class="mt-2 flex items-center gap-2 border-t border-border-subtle pt-2 sm:hidden">
          <NuxtLink
            v-for="entry in availableLocales"
            :key="entry.code"
            :to="switchLocalePath(entry.code)"
            class="rounded-md border border-border px-3 py-2 text-xs font-semibold uppercase no-underline text-text-subtle"
            :lang="entry.code"
          >
            {{ entry.code }}
          </NuxtLink>
          <NuxtLink
            :to="localePath('/connexion')"
            class="ml-auto rounded-md border border-border px-3 py-2 text-sm no-underline text-text"
          >
            {{ t('nav.account.login') }}
          </NuxtLink>
        </div>
      </nav>
    </header>

    <main id="contenu-principal" class="mx-auto w-full max-w-[1280px] flex-1 px-4 py-8 sm:px-6 sm:py-10">
      <slot />
    </main>

    <footer class="border-t border-border bg-surface-sunken">
      <div class="mx-auto grid w-full max-w-[1280px] gap-8 px-4 py-10 sm:px-6 md:grid-cols-4">
        <div class="md:col-span-1">
          <img
            src="/logos/oif-ifdd-gris.svg"
            :alt="`${t('nav.site.parent')} — ${t('nav.site.owner')}`"
            class="h-14 w-auto dark:hidden"
            width="224"
            height="56"
          >
          <img
            src="/logos/oif-ifdd-blanc.svg"
            :alt="`${t('nav.site.parent')} — ${t('nav.site.owner')}`"
            class="hidden h-14 w-auto dark:block"
            width="224"
            height="56"
          >
          <p class="mt-4 text-sm text-text-muted">{{ t('nav.site.tagline') }}</p>
        </div>

        <div v-for="section in footerSections" :key="section.labelKey">
          <h2 class="font-display text-sm uppercase tracking-wide text-text-subtle">
            {{ t(section.labelKey) }}
          </h2>
          <ul class="mt-3 space-y-2">
            <li v-for="item in section.items" :key="item.to">
              <NuxtLink
                :to="localePath(item.to)"
                class="text-sm no-underline text-text-muted hover:text-text"
              >
                {{ t(item.labelKey) }}
              </NuxtLink>
            </li>
          </ul>
        </div>
      </div>

      <div class="border-t border-border-subtle">
        <p class="mx-auto w-full max-w-[1280px] px-4 py-4 text-xs text-text-subtle sm:px-6">
          {{ t('nav.footer.copyright', { year: currentYear }) }}
        </p>
      </div>
    </footer>
  </div>
</template>
