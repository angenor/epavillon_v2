<script setup lang="ts">
import type { NavItem } from '~/types/navigation'

/**
 * « EN COURS DE MAINTENANCE » — une seule page pour les six modules fermés.
 *
 * ELLE N'EST PAS ATTEINTE PAR UN LIEN. C'est le middleware global `feature-flag`
 * qui l'amène, quand le drapeau `<module>.enabled` d'un espace est éteint. Le
 * paramètre d'adresse porte la clé technique du module (`/maintenance/training`)
 * et rien d'autre : le registre `utils/feature-modules.ts` en déduit le drapeau,
 * les routes couvertes et le chemin de retour.
 *
 * SOBRE ET HONNÊTE, comme le demande le prompt. Trois choses, pas une de plus :
 * ce qu'est l'espace, ce qu'il portera, et où aller en attendant. AUCUNE DATE —
 * ni « bientôt », ni un trimestre, ni un compte à rebours. Le jalon en cours ne
 * contient pas ces modules et personne ne sait quand il les contiendra ;
 * annoncer une échéance qu'on ne tiendra pas coûte plus cher que de se taire.
 * Aucune illustration non plus : ni chantier, ni robot, ni forme flottante — la
 * direction artistique les écarte, et un panneau de travaux ferait passer une
 * décision de périmètre pour une panne.
 *
 * UNE CLÉ INCONNUE REND 404, et c'est voulu. `/maintenance/nimportequoi` doit
 * répondre « cette page n'existe pas » plutôt que d'annoncer la maintenance d'un
 * module imaginaire — sans quoi l'adresse deviendrait un générateur d'écrans
 * plausibles pour n'importe quel mot.
 */

definePageMeta({
  layout: 'public',
  validate: (route) => closedModuleByKey(route.params.module) !== null,
})

const route = useRoute()
const { t } = useI18n()
const localePath = useLocalePath()

/**
 * `validate` a déjà écarté les clés inconnues : le repli sur le premier module
 * ne sert qu'à typer sans `!`. Il n'est jamais rendu.
 */
const module = computed(() => closedModuleByKey(route.params.module) ?? CLOSED_MODULES[0]!)

const title = computed(() => t(`maintenance.modules.${module.value.key}.title`))
const description = computed(() => t(`maintenance.modules.${module.value.key}.description`))

/**
 * OÙ ALLER EN ATTENDANT. Les trois destinations du jalon en cours, et rien
 * qu'elles : la programmation de toutes les éditions, l'édition en cours — c'est
 * ce que rend l'accueil — et l'appel à propositions, qui est la raison d'être de
 * ce jalon. Les libellés viennent de `nav.main.*` : ce sont les mêmes
 * destinations que la barre de navigation, et deux formulations pour un même
 * lien finiraient par diverger.
 */
const availableLinks: NavItem[] = [
  { labelKey: 'nav.main.programme', to: '/programmations', icon: 'calendar' },
  { labelKey: 'nav.main.call', to: '/#appel-a-propositions', icon: 'document' },
  { labelKey: 'nav.main.home', to: '/', icon: 'home' },
]

useHead({
  title,
  // Un espace fermé n'a rien à indexer, et une page de maintenance remontée dans
  // les résultats de recherche survivrait à la fermeture qu'elle annonce.
  meta: [{ name: 'robots', content: 'noindex' }],
})
</script>

<template>
  <UiMaintenanceState :title="title" :description="description">
    <template #actions>
      <div class="rounded-lg border border-border bg-surface-sunken px-5 py-6 text-left">
        <p class="font-display text-base text-text">{{ t('maintenance.available.title') }}</p>
        <p class="mt-1 text-sm text-text-secondary">
          {{ t('maintenance.available.description') }}
        </p>

        <ul class="mt-4 flex flex-col gap-2 sm:flex-row sm:flex-wrap">
          <li v-for="link in availableLinks" :key="link.to">
            <UiButton variant="secondary" :to="localePath(link.to)" :icon="link.icon">
              {{ t(link.labelKey) }}
            </UiButton>
          </li>
        </ul>
      </div>
    </template>
  </UiMaintenanceState>
</template>
