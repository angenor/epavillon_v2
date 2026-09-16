<script setup lang="ts">
/**
 * Layout des écrans d'authentification — sobre, centré, sans image de fond.
 *
 * POURQUOI UN TROISIÈME LAYOUT. Le layout public porte une barre de navigation à
 * quatre entrées, un pied de page à trois colonnes et un bouton « Se connecter »
 * — tout cela autour d'un formulaire de sept champs. Ce n'est pas une question
 * d'esthétique : chaque lien affiché est une occasion de quitter un parcours
 * qu'on venait d'entamer. Ici, une seule chose est à faire, et la page ne
 * propose rien d'autre.
 *
 * CE QUI RESTE, ET POURQUOI :
 *  · le logo, qui ramène à l'accueil — s'être trompé de page ne doit pas être un
 *    cul-de-sac ;
 *  · la langue et le thème, parce qu'ils se règlent AVANT la connexion et que
 *    les préférences du compte ne s'appliquent qu'après ;
 *  · les mentions légales et la politique de confidentialité, exigibles depuis
 *    l'écran où l'on crée un compte.
 *
 * PAS D'IMAGE DE FOND — demande explicite du prompt A1, et elle se défend : une
 * photographie derrière un formulaire dégrade le contraste de façon imprévisible
 * en thème clair comme en sombre, et la charte ne prévoit ni dégradé, ni voile.
 * Le seul relief est celui de la carte sur le fond en retrait.
 */

const { t } = useI18n()
const localePath = useLocalePath()

const legalLinks = [
  { labelKey: 'nav.footer.legal', to: '/mentions-legales' },
  { labelKey: 'nav.footer.privacy', to: '/confidentialite' },
  { labelKey: 'nav.footer.help', to: '/aide' },
]
</script>

<template>
  <div class="flex min-h-screen flex-col bg-surface-sunken text-text">
    <a class="skip-link" href="#contenu-principal">{{ t('common.a11y.skipToContent') }}</a>

    <UiApiOfflineBanner />
    <UiMockDataBanner />

    <header class="mx-auto flex w-full max-w-[64rem] items-center justify-between gap-4 px-4 py-5 sm:px-6">
      <NuxtLink :to="localePath('/')" class="flex shrink-0 items-center gap-3 no-underline">
        <img
          :src="assetUrl('/logos/svg/epavillon-symbole.svg')"
          :alt="t('nav.site.name')"
          class="h-10 w-auto dark:hidden"
          width="42"
          height="40"
        >
        <img
          :src="assetUrl('/logos/svg/epavillon-symbole-inverse.svg')"
          :alt="t('nav.site.name')"
          class="hidden h-10 w-auto dark:block"
          width="42"
          height="40"
        >
      </NuxtLink>

      <div class="flex items-center gap-2">
        <UiLocaleSwitch />
        <UiThemeToggle />
      </div>
    </header>

    <main
      id="contenu-principal"
      class="mx-auto flex w-full max-w-[30rem] flex-1 flex-col justify-center px-4 py-8 sm:px-6 sm:py-12"
    >
      <slot />
    </main>

    <footer class="border-t border-border-subtle">
      <div
        class="mx-auto flex w-full max-w-[64rem] flex-col gap-3 px-4 py-5 text-xs text-text-subtle sm:flex-row sm:items-center sm:justify-between sm:px-6"
      >
        <p>{{ t('nav.footer.copyright', { year: new Date().getFullYear() }) }}</p>
        <ul class="flex flex-wrap gap-x-5 gap-y-2">
          <li v-for="link in legalLinks" :key="link.to">
            <NuxtLink :to="localePath(link.to)" class="text-text-subtle no-underline hover:text-text">
              {{ t(link.labelKey) }}
            </NuxtLink>
          </li>
        </ul>
      </div>
    </footer>
  </div>
</template>
