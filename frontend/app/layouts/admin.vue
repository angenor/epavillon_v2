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
 *
 * CHAQUE ENTRÉE PORTE SON ICÔNE — le guide en dessine une par ligne, et la
 * colonne n'en avait aucune : douze libellés de même graisse, de même longueur,
 * ne se distinguent qu'à la lecture. Les pictogrammes reprennent ceux du guide,
 * dont le calendrier pour les événements et l'horloge pour la programmation.
 *
 * LE COMPTE CONNECTÉ VIT AU PIED DE LA COLONNE, avec la déconnexion. L'en-tête du
 * back-office n'en portait aucune trace : pour se déconnecter, il fallait repasser
 * par le site public. Et savoir SOUS QUEL COMPTE on arbitre n'est pas un confort
 * quand le périmètre d'administration décide de ce que la page montre (règle
 * métier n° 8).
 */

const { t } = useI18n()
const localePath = useLocalePath()
const route = useRoute()
const adminScope = useAdminScopeStore()
const auth = useAuthStore()

// SANS `await` : la garde d'accès du back-office a déjà résolu la session, cette
// lecture ne coûte donc aucun appel de plus — mais le layout ne doit pas retarder
// le rendu pour un pied de colonne.
void auth.ensureLoaded()

async function signOut(): Promise<void> {
  await auth.signOut()
  await navigateTo(localePath('/'))
}

const sections: NavSection[] = [
  {
    labelKey: 'nav.admin.sections.programme',
    items: [
      { labelKey: 'nav.admin.dashboard', to: '/admin', icon: 'grid' },
      { labelKey: 'nav.admin.proposals', to: '/admin/propositions', icon: 'inbox' },
      { labelKey: 'nav.admin.reviews', to: '/admin/evaluations', icon: 'check-circle' },
      { labelKey: 'nav.admin.schedule', to: '/admin/programmation', icon: 'clock' },
    ],
  },
  {
    labelKey: 'nav.admin.sections.referential',
    items: [
      { labelKey: 'nav.admin.events', to: '/admin/evenements', icon: 'calendar' },
      { labelKey: 'nav.admin.organizations', to: '/admin/organisations', icon: 'building' },
      { labelKey: 'nav.admin.users', to: '/admin/utilisateurs', icon: 'users' },
      { labelKey: 'nav.admin.showcase', to: '/admin/vitrine', icon: 'monitor' },
    ],
  },
  {
    labelKey: 'nav.admin.sections.operations',
    items: [
      { labelKey: 'nav.admin.incidents', to: '/admin/incidents', icon: 'broadcast' },
      { labelKey: 'nav.admin.settings', to: '/admin/parametres', icon: 'sliders' },
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

    <UiSideNav
      :sections="sections"
      :label="t('nav.admin.sidebar.label')"
      :open="isSidebarOpen"
      @close="isSidebarOpen = false"
    >
      <template #brand>
        <!-- LOGO ET MENTION EMPILÉS, ET NON CÔTE À CÔTE : la colonne fait 260 px,
             le logo 140, et « Back-office » ne tenait à côté qu'en rognant l'un
             des deux. Le logo mène au tableau de bord — le retour au site public
             est au pied, où on le cherche. -->
        <NuxtLink :to="localePath('/admin')" class="block no-underline">
          <img
            :src="assetUrl('/logos/ifdd-horizontal-gris.svg')"
            :alt="t('nav.site.owner')"
            class="h-7 w-auto dark:hidden"
            width="140"
            height="28"
          >
          <img
            :src="assetUrl('/logos/ifdd-horizontal-blanc.svg')"
            :alt="t('nav.site.owner')"
            class="hidden h-7 w-auto dark:block"
            width="140"
            height="28"
          >
          <span
            class="mt-2 block font-display text-xs font-semibold tracking-caps text-text-subtle uppercase"
          >
            {{ t('nav.admin.title') }}
          </span>
        </NuxtLink>
      </template>

      <template #footer>
        <div v-if="auth.person" class="mb-1 flex items-center gap-3 px-3 py-2">
          <span
            class="flex size-9 shrink-0 items-center justify-center rounded-full bg-accent-surface font-mono text-xs font-bold text-accent"
            aria-hidden="true"
          >
            {{ initialsOf(auth.person.display_name) }}
          </span>
          <span class="min-w-0 flex-1">
            <span class="block truncate text-sm font-semibold text-text">
              {{ auth.person.display_name }}
            </span>
            <span class="block truncate text-xs text-text-subtle">
              {{ auth.person.primary_email }}
            </span>
          </span>
        </div>

        <NuxtLink
          :to="localePath('/')"
          class="group flex min-h-(--target-min) items-center gap-3 rounded-md px-3 text-sm text-text-secondary no-underline transition-colors duration-(--duration-fast) hover:bg-surface-hover hover:text-text"
        >
          <UiIcon
            name="arrow-left"
            size="1.05rem"
            class="shrink-0 text-text-subtle transition-colors duration-(--duration-fast) group-hover:text-text"
          />
          {{ t('nav.admin.backToSite') }}
        </NuxtLink>

        <button
          v-if="auth.isAuthenticated"
          type="button"
          class="group flex min-h-(--target-min) w-full cursor-pointer items-center gap-3 rounded-md px-3 text-sm text-text-secondary transition-colors duration-(--duration-fast) hover:bg-danger-surface hover:text-danger"
          @click="signOut"
        >
          <UiIcon
            name="log-out"
            size="1.05rem"
            class="shrink-0 text-text-subtle transition-colors duration-(--duration-fast) group-hover:text-danger"
          />
          {{ t('nav.account.logout') }}
        </button>
      </template>
    </UiSideNav>

    <div class="flex min-w-0 flex-1 flex-col">
      <UiApiOfflineBanner />
      <UiMockDataBanner />

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

        <!-- LA PLACE DU FIL EST RÉSERVÉE, QU'IL S'AFFICHE OU NON. `UiBreadcrumb`
             se tait en dessous de trois maillons — un fil à deux n'apprend rien —,
             et le `v-else` qui poussait la langue et le thème à droite ne voyait
             pas ce silence : sur toutes les listes du back-office, dont le fil
             n'a qu'un maillon, les deux commandes se retrouvaient collées au
             bouton de menu, à gauche. -->
        <div class="min-w-0 flex-1">
          <UiBreadcrumb
            v-if="breadcrumb.length"
            :items="breadcrumb"
            :root="{ label: t('nav.admin.title'), to: '/admin' }"
          />
        </div>

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
