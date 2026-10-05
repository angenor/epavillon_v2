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
const siteName = computed(() => t('nav.site.name'))

const IFDD_URL = 'https://www.ifdd.francophonie.org'
const OIF_URL = 'https://www.francophonie.org'
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
 * chercher la porte. Une demande en attente suffit : l'espace lui montre ses
 * propres dossiers.
 *
 * Le store est déjà chargé pour la garde `requires-organization` : cette lecture
 * ne coûte aucun appel de plus.
 */
const memberships = useMembershipStore()
void memberships.ensureLoaded()

const myOrganizationTo = computed(() =>
  memberships.hasSubmittableOrganization ? '/mon-organisation' : '/rattachement-organisation',
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

const { date } = useDateTime()
const space = useAccountSpace()
const pendingActions = computed(() => space.overview.value?.actions ?? [])
const organizationLabel = computed(() => {
  const organization = space.organization.value
  return organization ? (organization.acronym ?? organization.legal_name) : undefined
})

/**
 * LE MENU DU COMPTE NE CONTIENT QUE DES DESTINATIONS PERSONNELLES. La barre
 * porte les espaces du site — ce qu'on vient consulter ; la bulle porte ce qui
 * n'appartient qu'à la personne connectée. « Mon organisation » y a donc sa
 * place, alors qu'elle encombrait la barre pour tous les autres.
 *
 * La déconnexion n'est pas une entrée de cette liste : ce n'est pas un lien, et
 * `UiUserMenu` la rend à part, sous un trait.
 *
 * « Mes propositions » d'abord (05/10) : qui a déposé un dossier doit trouver
 * où le suivre sans deviner qu'il vit sous « Mon organisation ».
 */
const accountNav = computed<NavItem[]>(() => {
  if (!memberships.hasSubmittableOrganization) {
    return [{ labelKey: 'nav.account.joinOrganization', to: myOrganizationTo.value, icon: 'building' }]
  }
  const overview = space.overview.value
  const items: NavItem[] = [
    { labelKey: 'nav.account.myProposals', to: '/mon-organisation#mes-dossiers', icon: 'document', count: overview?.proposals.length },
    { labelKey: 'nav.account.myOrganization', to: '/mon-organisation', icon: 'building', meta: organizationLabel.value },
    { labelKey: 'nav.account.members', to: '/mon-organisation#membres', icon: 'users', count: overview?.members.length },
  ]
  const call = overview?.open_call
  if (call && callPhase(call) === 'open') {
    items.push({
      labelKey: 'nav.account.submit',
      to: '/deposer-une-proposition',
      icon: 'plus',
      meta: t('nav.account.until', { date: date(effectiveDeadline(call), overview.call_edition?.timezone ?? 'UTC') }),
    })
  }
  return items
})

const footerSections: { labelKey: string; items: NavItem[] }[] = [
  {
    labelKey: 'nav.footer.sections.platform',
    items: [
      { labelKey: 'nav.main.programme', to: '/programmations' },
      { labelKey: 'nav.main.call', to: '/#appel-a-propositions' },
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

    <UiApiOfflineBanner />
    <UiMockDataBanner />

    <!-- Le bandeau institutionnel (04/10) : même aplat que la barre, séparé d'un filet.
         Il défile ; la barre, elle, reste collée. -->
    <div class="h-(--topbar-height) border-b border-border-on-inverse bg-surface-inverse text-text-on-inverse-muted [--color-focus:var(--color-accent-on-inverse)]">
      <div class="mx-auto flex h-full w-full max-w-[1280px] items-center gap-4 px-4 text-xs sm:px-6">
        <!-- Le nom de l'IFDD ne se traduit pas : c'est son nom officiel, en français
             dans toutes les langues du site. -->
        <p class="min-w-0 truncate">
          <a
            :href="IFDD_URL"
            target="_blank"
            rel="noopener noreferrer"
            lang="fr"
            class="text-text-on-inverse-muted no-underline hover:text-text-on-inverse hover:underline"
          >
            <span class="hidden md:inline">{{ t('nav.site.owner') }}</span>
            <span class="md:hidden">{{ t('nav.site.ownerShort') }}</span>
            <span class="sr-only">— {{ t('common.a11y.externalLink') }}</span>
          </a>
          <span aria-hidden="true"> · </span>
          <a
            :href="OIF_URL"
            target="_blank"
            rel="noopener noreferrer"
            :title="t('nav.site.parent')"
            class="text-text-on-inverse-muted no-underline hover:text-text-on-inverse hover:underline"
          >
            {{ t('nav.site.parentShort') }}<span class="sr-only"> — {{ t('nav.site.parent') }}, {{ t('common.a11y.externalLink') }}</span>
          </a>
        </p>
        <div class="ml-auto flex shrink-0 items-center gap-3">
          <UiLocaleSwitch tone="inverse" />
          <UiThemeToggle />
        </div>
      </div>
    </div>

    <UiNavBar v-model:open="isMobileNavOpen" :items="mainNav" :label="t('nav.main.label')">
      <template #brand>
        <!-- La barre est un aplat bleu nuit dans les deux thèmes : le symbole inversé
             toujours, et le nom écrit, dont le texte nomme le lien. -->
        <NuxtLink :to="localePath('/')" class="flex shrink-0 items-center gap-3 no-underline">
          <img
            :src="assetUrl('/logos/svg/epavillon-symbole-inverse.svg')"
            alt=""
            class="h-10 w-auto"
            width="42"
            height="40"
          >
          <span class="flex flex-col leading-none text-text-on-inverse">
            <span class="font-display text-xl leading-none tracking-tight">
              <span class="font-light">{{ siteName.slice(0, 1) }}</span><span class="font-bold">{{ siteName.slice(1) }}</span>
            </span>
            <span
              class="mt-1.5 text-[0.6875rem] leading-none uppercase text-text-on-inverse-muted"
              :style="{ letterSpacing: '0.16em' }"
            >
              {{ t('nav.site.nameSuffix') }}
            </span>
          </span>
        </NuxtLink>
      </template>

      <template #actions>
        <UiUserMenu
          v-if="auth.isAuthenticated && auth.person"
          :name="auth.person.display_name"
          :email="auth.person.primary_email"
          :items="accountNav"
          :title="t('nav.account.mySpace')"
          :subtitle="organizationLabel"
          :badge="pendingActions.length"
          :badge-label="t('nav.account.pendingCount', pendingActions.length)"
          :items-label="memberships.hasSubmittableOrganization ? t('nav.account.mySpace') : undefined"
          :label="t('nav.account.menuLabel')"
          :sign-out-label="t('nav.account.logout')"
          tone="inverse"
          @sign-out="signOut()"
          @open="space.refresh()"
        >
          <template #lead>
            <WorkspaceActionPreview
              :actions="pendingActions"
              :timezone="space.overview.value?.call_edition?.timezone ?? 'UTC'"
            />
          </template>
        </UiUserMenu>
        <!-- L'enveloppe porte le masquage : posé sur le bouton, il perd contre son propre `inline-flex`. -->
        <span v-else class="hidden sm:inline-flex">
          <UiButton variant="inverse" :to="localePath('/connexion')" :label="t('nav.account.login')" />
        </span>
      </template>

      <template #mobile-footer>
        <NuxtLink
          v-if="auth.isAuthenticated"
          :to="localePath(myOrganizationTo)"
          class="flex items-center gap-2 text-sm text-text-on-inverse-muted no-underline hover:text-text-on-inverse"
        >
          {{ t('nav.account.mySpace') }}
          <span
            v-if="pendingActions.length"
            class="grid h-4.5 min-w-4.5 place-items-center rounded-full bg-warning-on-inverse px-1 text-[11px] font-bold text-text-on-warning-inverse tabular-nums"
          >
            {{ pendingActions.length }}
            <span class="sr-only">— {{ t('nav.account.pendingCount', pendingActions.length) }}</span>
          </span>
        </NuxtLink>
        <UiButton
          v-if="auth.isAuthenticated"
          class="ml-auto"
          variant="inverse"
          size="sm"
          :label="t('nav.account.logout')"
          @click="signOut()"
        />
        <UiButton
          v-else
          class="ml-auto"
          variant="inverse"
          size="sm"
          :to="localePath('/connexion')"
          :label="t('nav.account.login')"
        />
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

         Le logo n'a donc plus de variante : sur cet aplat, c'est la version
         inverse qui vaut dans les deux thèmes. -->
    <footer class="bg-surface-inverse text-text-on-inverse">
      <div class="mx-auto grid w-full max-w-[1280px] gap-8 px-4 py-10 sm:px-6 md:grid-cols-4">
        <div class="md:col-span-1">
          <img
            :src="assetUrl('/logos/svg/epavillon-inverse.svg')"
            :alt="t('nav.site.name')"
            class="h-24 w-auto"
            width="75"
            height="96"
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
