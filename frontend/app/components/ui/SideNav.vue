<script setup lang="ts">
import type { NavSection } from '~/types/navigation'

/**
 * Navigation latérale du back-office — sections, entrées, périmètre.
 *
 * SECTIONS TITRÉES plutôt qu'une liste plate : « Programmation », « Référentiels »,
 * « Exploitation ». Douze entrées d'affilée ne se parcourent pas ; trois groupes
 * de quatre, si. Chaque titre est un vrai `<h2>` — c'est ce qui permet de
 * naviguer par en-têtes avec un lecteur d'écran.
 *
 * LE CRÉNEAU `scope` MATÉRIALISE LE PÉRIMÈTRE D'ADMINISTRATION (règle métier
 * n° 8). Ce n'est pas un confort de navigation : c'est le filtre qui s'applique à
 * toutes les listes de la section, et il doit rester visible en permanence. Le
 * filtrage lui-même appartient à l'API — une URL forgée à la main ne doit rien
 * laisser filtrer de plus.
 *
 * COLONNE FIXE au-delà de 1024 px, repliée en dessous : le back-office est un
 * outil de bureau, mais un arbitrage se fait parfois depuis un téléphone, au
 * fond d'une salle de négociation.
 */

interface Props {
  sections: NavSection[]
  /** Nom de la navigation, annoncé par les lecteurs d'écran. */
  label: string
  /** Panneau déployé sur écran étroit — contrôlé par le layout. */
  open?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const localePath = useLocalePath()
const route = useRoute()

const isCurrent = (to: string): boolean => route.path === localePath(to)
</script>

<template>
  <aside
    id="ui-sidenav"
    class="border-b border-border bg-surface-raised lg:sticky lg:top-0 lg:h-screen lg:w-72 lg:shrink-0 lg:overflow-y-auto lg:border-r lg:border-b-0"
    :class="props.open ? 'block' : 'hidden lg:block'"
  >
    <div v-if="$slots.brand" class="flex items-center gap-3 border-b border-border-subtle px-4 py-4">
      <slot name="brand" />
    </div>

    <!-- Périmètre d'administration : toujours visible, jamais replié. -->
    <div v-if="$slots.scope" class="border-b border-border-subtle px-4 py-4">
      <slot name="scope" />
    </div>

    <nav class="px-2 py-4" :aria-label="props.label">
      <div v-for="section in props.sections" :key="section.labelKey" class="mb-5 last:mb-0">
        <h2 class="px-3 pb-2 text-xs font-semibold tracking-wide text-text-subtle uppercase">
          {{ t(section.labelKey) }}
        </h2>
        <ul>
          <li v-for="item in section.items" :key="item.to">
            <NuxtLink
              :to="localePath(item.to)"
              class="block rounded-md px-3 py-2 text-sm no-underline transition-colors"
              :class="
                isCurrent(item.to)
                  ? 'bg-surface-selected font-semibold text-accent'
                  : 'text-text-muted hover:bg-surface-hover hover:text-text'
              "
              :aria-current="isCurrent(item.to) ? 'page' : undefined"
            >
              {{ t(item.labelKey) }}
            </NuxtLink>
          </li>
        </ul>
      </div>
    </nav>

    <div v-if="$slots.footer" class="border-t border-border-subtle px-4 py-4">
      <slot name="footer" />
    </div>
  </aside>
</template>
