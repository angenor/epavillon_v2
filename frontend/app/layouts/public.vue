<script setup lang="ts">
import type { NavItem } from '~/types/navigation'

/**
 * Layout public — barre de navigation, sélecteur de langue, bascule de thème,
 * pied de page.
 *
 * Les liens pointent vers les écrans du jalon en cours ; ils sont créés par les
 * prompts A1 à A5. Les modules affichant « En cours de maintenance »
 * (Publications, Négociations, Formations, Outils) n'ont volontairement aucune
 * entrée ici tant que `platform.feature_flags` ne les ouvre pas.
 *
 * A0.4 — la barre, le sélecteur de langue et la bascule de thème sont désormais
 * des composants d'interface (`UiNavBar`, `UiLocaleSwitch`, `UiThemeToggle`).
 * Le layout ne garde que ce qui lui appartient : la LISTE des entrées, l'état du
 * menu mobile, et le pied de page.
 */

const { t } = useI18n()
const localePath = useLocalePath()
const route = useRoute()
const auth = useAuthStore()

/**
 * La barre publique connaît la session depuis le prompt A2 : elle affichait
 * « Se connecter » à quelqu'un qui venait de se connecter, sur le premier écran
 * du parcours qui exige d'être connecté.
 *
 * SANS `await` : le layout enveloppe aussi des pages entièrement publiques, que
 * rien ne doit retarder. La session se résout pendant l'affichage ; jusque-là,
 * `isAuthenticated` est faux et la barre montre l'entrée de connexion — ce qui
 * est vrai tant qu'on ne sait pas.
 *
 * Le nom n'est pas encore un lien : la page de profil n'existe pas (elle viendra
 * avec l'espace organisation, prompt A5). Un lien vers une page absente serait
 * pire que pas de lien.
 */
void auth.ensureLoaded()

async function signOut(): Promise<void> {
  await auth.signOut()
  await navigateTo(localePath('/'))
}

/**
 * A3 — les entrées mènent désormais à des pages qui EXISTENT.
 *
 * `/evenements`, `/programme` et `/appel-a-propositions` étaient trois adresses
 * sans page : la barre les proposait, et elles répondaient 404. La programmation
 * et l'appel ne sont pas des écrans à part — ce sont deux sections de la page
 * publique de l'édition en cours, atteignables par leur ancre. L'accueil, lui,
 * redirige vers cette édition en conservant le fragment (voir `pages/index.vue`).
 *
 * La leçon vient du prompt A2 : un écran qui répond à toutes ses exigences peut
 * rester inatteignable, et cela ne se voit qu'en refaisant le chemin.
 */
const mainNav: NavItem[] = [
  { labelKey: 'nav.main.home', to: '/' },
  { labelKey: 'nav.main.programme', to: '/#programmation' },
  { labelKey: 'nav.main.call', to: '/#appel-a-propositions' },
]

const footerSections: { labelKey: string; items: NavItem[] }[] = [
  {
    labelKey: 'nav.footer.sections.platform',
    items: [
      { labelKey: 'nav.main.programme', to: '/#programmation' },
      { labelKey: 'nav.main.call', to: '/#appel-a-propositions' },
      { labelKey: 'nav.main.criteria', to: '/#criteres' },
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

// Le menu mobile appartient au layout : c'est lui qui connaît la route et peut
// donc le refermer à chaque navigation.
const isMobileNavOpen = ref(false)
watch(() => route.fullPath, () => (isMobileNavOpen.value = false))

const currentYear = new Date().getFullYear()
</script>

<template>
  <div class="flex min-h-screen flex-col bg-surface text-text">
    <a class="skip-link" href="#contenu-principal">{{ t('common.a11y.skipToContent') }}</a>

    <UiNavBar v-model:open="isMobileNavOpen" :items="mainNav" :label="t('nav.main.home')">
      <template #brand>
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
      </template>

      <template #actions>
        <UiLocaleSwitch class="hidden sm:flex" />
        <UiThemeToggle />
        <span v-if="auth.isAuthenticated && auth.person" class="hidden items-center gap-2 sm:flex">
          <!-- Le rattachement à une organisation est ATTEINT une fois dans le
               parcours d'inscription, et doit rester atteignable ensuite : on
               change d'employeur, on rejoint une seconde structure, on suit une
               demande en attente. Le prompt dit « accessible depuis le profil » ;
               la page de profil n'existe pas encore (prompt A5), cette entrée
               tient le rôle en attendant et disparaîtra avec elle. -->
          <NuxtLink
            :to="localePath('organization-join')"
            class="text-sm text-text-secondary no-underline hover:text-text"
          >
            {{ t('nav.account.myOrganization') }}
          </NuxtLink>
          <span class="max-w-[16ch] truncate text-sm text-text-secondary">
            {{ auth.person.display_name }}
          </span>
          <UiButton variant="secondary" size="sm" :label="t('nav.account.logout')" @click="signOut()" />
        </span>
        <NuxtLink
          v-else
          :to="localePath('/connexion')"
          class="hidden rounded-md border border-border px-3 py-2 text-sm text-text no-underline transition-colors hover:bg-surface-hover sm:inline-block"
        >
          {{ t('nav.account.login') }}
        </NuxtLink>
      </template>

      <template #mobile-footer>
        <UiLocaleSwitch class="sm:hidden" />
        <NuxtLink
          v-if="auth.isAuthenticated"
          :to="localePath('organization-join')"
          class="text-sm text-text-secondary no-underline"
        >
          {{ t('nav.account.myOrganization') }}
        </NuxtLink>
        <UiButton
          v-if="auth.isAuthenticated"
          class="ml-auto"
          variant="secondary"
          size="sm"
          :label="t('nav.account.logout')"
          @click="signOut()"
        />
        <NuxtLink
          v-else
          :to="localePath('/connexion')"
          class="ml-auto rounded-md border border-border px-3 py-2 text-sm text-text no-underline"
        >
          {{ t('nav.account.login') }}
        </NuxtLink>
      </template>
    </UiNavBar>

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
          <h2 class="font-display text-sm tracking-wide text-text-subtle uppercase">
            {{ t(section.labelKey) }}
          </h2>
          <ul class="mt-3 space-y-2">
            <li v-for="item in section.items" :key="item.to">
              <NuxtLink
                :to="localePath(item.to)"
                class="text-sm text-text-muted no-underline hover:text-text"
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
