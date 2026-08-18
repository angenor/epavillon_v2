<script setup lang="ts">
import type { NavItem } from '~/types/navigation'

/**
 * Layout public — barre de navigation, sélecteur de langue, bascule de thème,
 * pied de page.
 *
 * Les liens pointent vers les écrans du jalon en cours ; ils sont créés par les
 * prompts A1 à A5. Deux espaces — Communauté et Négociations — sont annoncés
 * dans la barre alors que leur module reste fermé : leur page existe et affiche
 * l'état de maintenance. Un lien de barre qui répond 404 serait pire que pas de
 * lien du tout (leçon du prompt A3) ; une entrée qui dit « bientôt » informe.
 * Les autres modules fermés (Publications, Formations, Outils) n'ont toujours
 * aucune entrée ici tant que `platform.feature_flags` ne les ouvre pas.
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

/**
 * OÙ MÈNE « MON ORGANISATION », ET POURQUOI CELA DÉPEND DE LA PERSONNE.
 *
 * Rattachée, elle va à son espace — ses dossiers, ce qui l'attend, ses membres.
 * Pas encore rattachée, elle va à l'écran de rattachement : lui ouvrir un espace
 * qui n'a ni dossier ni membre serait lui montrer une pièce vide et la laisser
 * chercher la porte. Une demande EN ATTENTE ne suffit pas — aucun référent n'a
 * accepté, et l'espace la refuserait.
 *
 * Le store est déjà chargé pour la garde `requires-organization` : cette lecture
 * ne coûte aucun appel de plus.
 */
const memberships = useMembershipStore()
void memberships.ensureLoaded()

const myOrganizationTo = computed(() =>
  memberships.hasActiveOrganization ? '/mon-organisation' : '/rattachement-organisation',
)

async function signOut(): Promise<void> {
  await auth.signOut()
  await navigateTo(localePath('/'))
}

/**
 * A3 — les entrées mènent à des pages qui EXISTENT.
 *
 * `/evenements`, `/programme` et `/appel-a-propositions` étaient trois adresses
 * sans page : la barre les proposait, et elles répondaient 404.
 *
 * PAS D'ENTRÉE « ACCUEIL » : le logo, à gauche, EST le lien d'accueil sur tous
 * les sites, et l'accueil ne fait ici que rediriger vers l'édition en cours. La
 * doubler d'une entrée de menu occupe la place d'une destination réelle.
 *
 * « PROGRAMMATIONS » AU PLURIEL, et c'est une page à part entière : elle porte
 * les programmes de TOUTES les éditions, celui de la COP31 comme le cycle de
 * webinaires PACO. L'appel à propositions, lui, reste une section de la page de
 * l'édition en cours — il n'y en a qu'un par édition (règle métier n° 5), il n'a
 * donc pas de liste à lui.
 *
 * La leçon vient du prompt A2 : un écran qui répond à toutes ses exigences peut
 * rester inatteignable, et cela ne se voit qu'en refaisant le chemin.
 */
const mainNav: NavItem[] = [
  { labelKey: 'nav.main.programme', to: '/programmations' },
  { labelKey: 'nav.main.community', to: '/communaute' },
  { labelKey: 'nav.main.negotiations', to: '/negociations' },
]

/**
 * LE MENU DU COMPTE NE CONTIENT QUE DES DESTINATIONS PERSONNELLES. La barre
 * porte les espaces du site — ce qu'on vient consulter ; la bulle porte ce qui
 * n'appartient qu'à la personne connectée. « Mon organisation » y a donc sa
 * place, alors qu'elle encombrait la barre pour tous les autres.
 *
 * La déconnexion n'est pas une entrée de cette liste : ce n'est pas un lien, et
 * `UiUserMenu` la rend à part, sous un trait.
 */
const accountNav = computed<NavItem[]>(() => [
  { labelKey: 'nav.account.myOrganization', to: myOrganizationTo.value, icon: 'building' },
])

const footerSections: { labelKey: string; items: NavItem[] }[] = [
  {
    labelKey: 'nav.footer.sections.platform',
    items: [
      { labelKey: 'nav.main.programme', to: '/programmations' },
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

    <UiNavBar v-model:open="isMobileNavOpen" :items="mainNav" :label="t('nav.main.label')">
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
        <UiUserMenu
          v-if="auth.isAuthenticated && auth.person"
          :name="auth.person.display_name"
          :email="auth.person.primary_email"
          :items="accountNav"
          :label="t('nav.account.menuLabel')"
          :sign-out-label="t('nav.account.logout')"
          @sign-out="signOut()"
        />
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
          :to="localePath(myOrganizationTo)"
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

    <!-- LE PIED DE PAGE EST UN APLAT INSTITUTIONNEL. C'est le seul grand bloc
         que porte CHAQUE page, donc le seul endroit où l'on peut poser le bleu
         riche de la charte sans dépendre d'un écran particulier. Il ferme la
         page sur la marque plutôt que sur un gris de plus, et il ne s'inverse
         pas au thème sombre — un aplat est un bloc de mise en page.

         Le logo n'a donc plus de variante : sur cet aplat, c'est le tracé blanc
         qui vaut dans les deux thèmes. -->
    <footer class="bg-surface-inverse text-text-on-inverse">
      <div class="mx-auto grid w-full max-w-[1280px] gap-8 px-4 py-10 sm:px-6 md:grid-cols-4">
        <div class="md:col-span-1">
          <img
            src="/logos/oif-ifdd-blanc.svg"
            :alt="`${t('nav.site.parent')} — ${t('nav.site.owner')}`"
            class="h-14 w-auto"
            width="224"
            height="56"
          >
          <p class="mt-4 text-sm text-text-on-inverse-muted">{{ t('nav.site.tagline') }}</p>
        </div>

        <div v-for="section in footerSections" :key="section.labelKey">
          <h2 class="font-display text-sm tracking-wide text-text-on-inverse uppercase">
            {{ t(section.labelKey) }}
          </h2>
          <ul class="mt-3 space-y-2">
            <li v-for="item in section.items" :key="item.to">
              <NuxtLink
                :to="localePath(item.to)"
                class="text-sm text-text-on-inverse-muted no-underline hover:text-text-on-inverse"
              >
                {{ t(item.labelKey) }}
              </NuxtLink>
            </li>
          </ul>
        </div>
      </div>

      <div class="border-t border-border-on-inverse bg-surface-inverse-raised">
        <p class="mx-auto w-full max-w-[1280px] px-4 py-4 text-xs text-text-on-inverse-muted sm:px-6">
          {{ t('nav.footer.copyright', { year: currentYear }) }}
        </p>
      </div>
    </footer>
  </div>
</template>
