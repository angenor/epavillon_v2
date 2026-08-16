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
 * COLONNE FIXE DE 260 px au-delà de 1024 px, repliée en dessous : le back-office
 * est un outil de bureau, mais un arbitrage se fait parfois depuis un téléphone,
 * au fond d'une salle de négociation. La largeur vient du guide — elle tient
 * « Espace négociateurs » et son compteur sur une seule ligne, ce qui est le
 * libellé le plus long de la section, sans voler de place au tableau qui suit.
 *
 * ICÔNE ET COMPTEUR FACULTATIFS. L'icône aide au repérage d'une liste de douze
 * entrées parcourue chaque jour ; le compteur poussé à droite dit ce qui attend
 * (« Dossiers déposés · 24 ») sans obliger à ouvrir l'écran. Ni l'un ni l'autre
 * n'est requis : une entrée sans icône ne décale pas son libellé, une section
 * entière peut s'en passer.
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
    class="border-b border-border bg-surface-raised lg:sticky lg:top-0 lg:h-screen lg:w-65 lg:shrink-0 lg:overflow-y-auto lg:border-r lg:border-b-0"
    :class="props.open ? 'block' : 'hidden lg:block'"
  >
    <div v-if="$slots.brand" class="flex items-center gap-3 border-b border-border-subtle px-4 py-4">
      <slot name="brand" />
    </div>

    <!-- Périmètre d'administration : toujours visible, jamais replié. -->
    <div v-if="$slots.scope" class="border-b border-border-subtle px-4 py-4">
      <slot name="scope" />
    </div>

    <nav class="p-3" :aria-label="props.label">
      <div v-for="section in props.sections" :key="section.labelKey" class="mb-4 last:mb-0">
        <h2 class="px-3 pt-4 pb-2 text-xs font-semibold tracking-caps text-text-muted uppercase">
          {{ t(section.labelKey) }}
        </h2>
        <ul>
          <li v-for="item in section.items" :key="item.to">
            <NuxtLink
              :to="localePath(item.to)"
              class="flex min-h-(--target-min) items-center gap-3 rounded-md px-3 text-sm no-underline transition-colors duration-(--duration-fast)"
              :class="
                isCurrent(item.to)
                  ? 'bg-accent-surface font-semibold text-accent'
                  : 'text-text-secondary hover:bg-surface-hover hover:text-text'
              "
              :aria-current="isCurrent(item.to) ? 'page' : undefined"
            >
              <UiIcon v-if="item.icon" :name="item.icon" size="1rem" class="shrink-0" />
              {{ t(item.labelKey) }}
              <!-- Compteur poussé à la marge intérieure droite, jamais collé au
                   libellé : c'est ce qui le rend comparable d'une entrée à
                   l'autre en descendant la colonne. Même pastille que les
                   onglets — 22 px, aplat accent. -->
              <UiCounter
                v-if="item.count !== undefined"
                :value="item.count"
                tone="accent"
                class="ms-auto"
              />
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
