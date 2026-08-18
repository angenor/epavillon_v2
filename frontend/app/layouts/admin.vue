<script setup lang="ts">
import type { NavSection } from '~/types/navigation'

/**
 * Layout du back-office — navigation latérale, fil d'Ariane, sélecteur
 * d'événement.
 *
 * RÈGLE MÉTIER N° 8 : un administrateur peut n'avoir accès qu'à un seul
 * événement. Le sélecteur d'édition n'est pas un confort : il matérialise le
 * périmètre d'administration qui filtre toutes les listes de cette section.
 * Le filtrage lui-même appartient à l'API — une URL forgée à la main ne doit
 * rien laisser filtrer de plus.
 *
 * A0.4 — la navigation latérale, le fil d'Ariane, le sélecteur de langue et la
 * bascule de thème sont devenus des composants d'interface (`UiSideNav`,
 * `UiBreadcrumb`, `UiLocaleSwitch`, `UiThemeToggle`). Les trois pictogrammes de
 * thème étaient dupliqués depuis le layout public ; c'est cette duplication que
 * la note du prompt annonçait de résorber.
 *
 * A6 — LE SÉLECTEUR D'ÉVÉNEMENT A QUITTÉ LA NAVIGATION LATÉRALE pour la tête de
 * page (`AdminEventScope`, posé juste au-dessus du contenu). Sur écran étroit, la
 * navigation latérale est un tiroir qu'il faut ouvrir : le périmètre d'un écran
 * ne peut pas être caché derrière un bouton, c'est le sujet de tout ce qu'on lit
 * en dessous. Le layout le pose une fois pour tous les écrans du back-office,
 * plutôt que chaque page à sa manière.
 */

const { t } = useI18n()
const localePath = useLocalePath()
const route = useRoute()
const adminScope = useAdminScopeStore()

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

const isSidebarOpen = ref(false)
watch(() => route.fullPath, () => (isSidebarOpen.value = false))

// Le fil d'Ariane est déclaré par chaque page (`definePageMeta({ breadcrumb })`).
// Le layout n'en invente aucun : mieux vaut pas de fil qu'un fil faux.
const breadcrumb = computed(() => route.meta.breadcrumb ?? [])

// Le périmètre est chargé ICI, une fois pour tous les écrans du back-office :
// le sélecteur en a besoin avant que la page ait fini de se rendre, et chaque
// écran le rechargerait sinon pour son propre compte.
await adminScope.ensureLoaded()
</script>

<template>
  <div class="min-h-screen bg-surface text-text lg:flex">
    <a class="skip-link" href="#contenu-admin">{{ t('common.a11y.skipToContent') }}</a>

    <UiSideNav :sections="sections" :label="t('nav.admin.sidebar.label')" :open="isSidebarOpen">
      <template #brand>
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
        <span class="font-display text-sm tracking-wide text-text-subtle uppercase">
          {{ t('nav.admin.title') }}
        </span>
      </template>

      <template #footer>
        <NuxtLink :to="localePath('/')" class="text-sm text-text-muted no-underline hover:text-text">
          {{ t('nav.admin.backToSite') }}
        </NuxtLink>
      </template>
    </UiSideNav>

    <div class="flex min-w-0 flex-1 flex-col">
      <header
        class="sticky top-0 z-20 flex items-center gap-3 border-b border-border bg-surface-raised px-4 py-3 sm:px-6"
      >
        <button
          type="button"
          class="rounded-md border border-border p-2 text-text transition-colors hover:bg-surface-hover lg:hidden"
          :aria-expanded="isSidebarOpen"
          aria-controls="ui-sidenav"
          @click="isSidebarOpen = !isSidebarOpen"
        >
          <span class="sr-only">
            {{ isSidebarOpen ? t('nav.admin.sidebar.collapse') : t('nav.admin.sidebar.expand') }}
          </span>
          <UiIcon :name="isSidebarOpen ? 'close' : 'menu'" size="1.25rem" :stroke-width="1.8" />
        </button>

        <UiBreadcrumb
          v-if="breadcrumb.length"
          :items="breadcrumb"
          :root="{ label: t('nav.admin.title'), to: '/admin' }"
          class="min-w-0 flex-1"
        />
        <div v-else class="min-w-0 flex-1" />

        <UiLocaleSwitch class="hidden sm:flex" />
        <UiThemeToggle />
      </header>

      <!-- LE PÉRIMÈTRE, EN TÊTE DE PAGE — règle métier n° 8. Un compte détaché
           sur une seule édition n'y voit qu'un nom : pas de liste déroulante à
           une entrée, rien qui laisse deviner qu'il en existe d'autres. -->
      <AdminEventScope />

      <main id="contenu-admin" class="min-w-0 flex-1 px-4 py-6 sm:px-6 sm:py-8">
        <slot />
      </main>
    </div>
  </div>
</template>
