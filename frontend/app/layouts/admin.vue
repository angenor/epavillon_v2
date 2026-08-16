<script setup lang="ts">
import type { NavSection } from '~/types/navigation'
import type { LocaleCode } from '~/types/shared'
import type { ThemeChoice } from '~/stores/preferences'

/**
 * Layout du back-office — navigation latérale, fil d'Ariane, sélecteur
 * d'événement.
 *
 * RÈGLE MÉTIER N° 8 : un administrateur peut n'avoir accès qu'à un seul
 * événement. Le sélecteur ci-dessous n'est pas un confort : il matérialise le
 * périmètre d'administration qui filtre toutes les listes de cette section.
 * Le filtrage lui-même appartient à l'API — une URL forgée à la main ne doit
 * rien laisser filtrer de plus.
 */

const { t, locale, locales } = useI18n()
const localePath = useLocalePath()
const switchLocalePath = useSwitchLocalePath()
const route = useRoute()
const preferences = usePreferencesStore()
const adminScope = useAdminScopeStore()
const { tr } = useI18nText()

const sections: NavSection[] = [
  {
    labelKey: 'nav.admin.sections.programme',
    items: [
      { labelKey: 'nav.admin.dashboard', to: '/admin' },
      { labelKey: 'nav.admin.proposals', to: '/admin/propositions' },
      { labelKey: 'nav.admin.reviews', to: '/admin/evaluations' },
      { labelKey: 'nav.admin.schedule', to: '/admin/programmation' },
    ],
  },
  {
    labelKey: 'nav.admin.sections.referential',
    items: [
      { labelKey: 'nav.admin.events', to: '/admin/evenements' },
      { labelKey: 'nav.admin.organizations', to: '/admin/organisations' },
      { labelKey: 'nav.admin.users', to: '/admin/utilisateurs' },
    ],
  },
  {
    labelKey: 'nav.admin.sections.operations',
    items: [
      { labelKey: 'nav.admin.incidents', to: '/admin/incidents' },
      { labelKey: 'nav.admin.settings', to: '/admin/parametres' },
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

const isSidebarOpen = ref(false)
watch(() => route.fullPath, () => (isSidebarOpen.value = false))

// Le fil d'Ariane est déclaré par chaque page (`definePageMeta({ breadcrumb })`).
// Le layout n'en invente aucun : mieux vaut pas de fil qu'un fil faux.
const breadcrumb = computed(() => route.meta.breadcrumb ?? [])

const isCurrent = (to: string) => route.path === localePath(to)

const selectedEventId = computed({
  get: () => adminScope.currentEventId ?? '',
  set: (value: string) => adminScope.selectEvent(value === '' ? null : value),
})
</script>

<template>
  <div class="min-h-screen bg-surface text-text lg:flex">
    <a class="skip-link" href="#contenu-admin">{{ t('common.a11y.skipToContent') }}</a>

    <!-- Navigation latérale -->
    <aside
      id="navigation-admin"
      class="border-b border-border bg-surface-raised lg:sticky lg:top-0 lg:h-screen lg:w-72 lg:shrink-0 lg:overflow-y-auto lg:border-r lg:border-b-0"
      :class="isSidebarOpen ? 'block' : 'hidden lg:block'"
    >
      <div class="flex items-center gap-3 border-b border-border-subtle px-4 py-4">
        <NuxtLink :to="localePath('/')" class="flex items-center gap-2 no-underline">
          <img
            src="/logos/ifdd-horizontal-gris.svg"
            :alt="t('nav.site.owner')"
            class="h-7 w-auto dark:hidden"
            width="140"
            height="28"
          >
          <img
            src="/logos/ifdd-horizontal-blanc.svg"
            :alt="t('nav.site.owner')"
            class="hidden h-7 w-auto dark:block"
            width="140"
            height="28"
          >
        </NuxtLink>
        <span class="font-display text-sm uppercase tracking-wide text-text-subtle">
          {{ t('nav.admin.title') }}
        </span>
      </div>

      <!-- Périmètre d'administration -->
      <div class="border-b border-border-subtle px-4 py-4">
        <label
          for="admin-event-scope"
          class="block text-xs font-semibold uppercase tracking-wide text-text-subtle"
        >
          {{ t('nav.admin.eventScope.label') }}
        </label>

        <p v-if="adminScope.isEmpty" class="mt-2 text-sm text-text-muted">
          {{ t('nav.admin.eventScope.empty') }}
        </p>

        <p
          v-else-if="adminScope.isRestricted"
          class="mt-2 rounded-md border border-border-subtle bg-surface-sunken px-3 py-2 text-sm text-text"
        >
          {{ tr(adminScope.currentEvent?.title) }}
          <span class="mt-1 block text-xs text-text-subtle">
            {{ t('nav.admin.eventScope.restricted') }}
          </span>
        </p>

        <select
          v-else
          id="admin-event-scope"
          v-model="selectedEventId"
          class="mt-2 w-full rounded-md border border-border-strong bg-surface-raised px-3 py-2 text-sm text-text"
        >
          <option value="">{{ t('nav.admin.eventScope.placeholder') }}</option>
          <option v-for="event in adminScope.events" :key="event.id" :value="event.id">
            {{ tr(event.title) }}
          </option>
        </select>

        <p v-if="!adminScope.isEmpty" class="mt-2 text-xs text-text-subtle">
          {{ t('nav.admin.eventScope.hint') }}
        </p>
      </div>

      <nav class="px-2 py-4" :aria-label="t('nav.admin.sidebar.label')">
        <div v-for="section in sections" :key="section.labelKey" class="mb-5 last:mb-0">
          <h2 class="px-3 pb-2 text-xs font-semibold uppercase tracking-wide text-text-subtle">
            {{ t(section.labelKey) }}
          </h2>
          <ul>
            <li v-for="item in section.items" :key="item.to">
              <NuxtLink
                :to="localePath(item.to)"
                class="block rounded-md px-3 py-2 text-sm no-underline text-text-muted hover:bg-surface-hover hover:text-text"
                :class="isCurrent(item.to) ? 'bg-surface-selected font-semibold text-accent' : ''"
                :aria-current="isCurrent(item.to) ? 'page' : undefined"
              >
                {{ t(item.labelKey) }}
              </NuxtLink>
            </li>
          </ul>
        </div>
      </nav>

      <div class="border-t border-border-subtle px-4 py-4">
        <NuxtLink :to="localePath('/')" class="text-sm no-underline text-text-muted hover:text-text">
          {{ t('nav.admin.backToSite') }}
        </NuxtLink>
      </div>
    </aside>

    <div class="flex min-w-0 flex-1 flex-col">
      <header
        class="sticky top-0 z-20 flex items-center gap-3 border-b border-border bg-surface-raised px-4 py-3 sm:px-6"
      >
        <button
          type="button"
          class="rounded-md border border-border p-2 text-text lg:hidden"
          :aria-expanded="isSidebarOpen"
          aria-controls="navigation-admin"
          @click="isSidebarOpen = !isSidebarOpen"
        >
          <span class="sr-only">
            {{ isSidebarOpen ? t('nav.admin.sidebar.collapse') : t('nav.admin.sidebar.expand') }}
          </span>
          <svg
            class="size-5"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            aria-hidden="true"
          >
            <path v-if="!isSidebarOpen" d="M4 7h16M4 12h16M4 17h16" stroke-linecap="round" />
            <path v-else d="M6 6l12 12M18 6 6 18" stroke-linecap="round" />
          </svg>
        </button>

        <!-- Fil d'Ariane -->
        <nav v-if="breadcrumb.length" :aria-label="t('nav.breadcrumb.label')" class="min-w-0 flex-1">
          <ol class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm">
            <li class="flex items-center gap-2">
              <NuxtLink :to="localePath('/admin')" class="no-underline text-text-subtle hover:text-text">
                {{ t('nav.admin.title') }}
              </NuxtLink>
            </li>
            <li
              v-for="(crumb, index) in breadcrumb"
              :key="`${crumb.to ?? crumb.labelKey ?? index}`"
              class="flex items-center gap-2"
            >
              <span aria-hidden="true" class="text-text-subtle">/</span>
              <NuxtLink
                v-if="crumb.to && index < breadcrumb.length - 1"
                :to="localePath(crumb.to)"
                class="no-underline text-text-subtle hover:text-text"
              >
                {{ crumb.labelKey ? t(crumb.labelKey) : crumb.label }}
              </NuxtLink>
              <span v-else class="font-semibold text-text" aria-current="page">
                {{ crumb.labelKey ? t(crumb.labelKey) : crumb.label }}
              </span>
            </li>
          </ol>
        </nav>
        <div v-else class="min-w-0 flex-1" />

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
            :lang="entry.code"
          >
            {{ entry.code }}
          </NuxtLink>
        </nav>

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
            <!-- Les trois pictogrammes sont dupliqués depuis le layout public :
                 le prompt A0.4 les factorisera en composant d'interface. -->
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
      </header>

      <main id="contenu-admin" class="min-w-0 flex-1 px-4 py-6 sm:px-6 sm:py-8">
        <slot />
      </main>
    </div>
  </div>
</template>
