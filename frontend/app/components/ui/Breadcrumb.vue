<script setup lang="ts">
import type { BreadcrumbItem } from '~/types/navigation'

/**
 * Fil d'Ariane — où je suis, et comment remonter.
 *
 * IL N'INVENTE RIEN. Chaque page déclare son fil dans `definePageMeta({
 * breadcrumb })` ; le composant se contente de le rendre. Mieux vaut aucun fil
 * qu'un fil déduit de l'URL : `/admin/propositions/0198c1a0-…` donnerait un
 * maillon « 0198c1a0-… », ce qui n'aide personne.
 *
 * DEUX SORTES DE LIBELLÉS, à ne pas confondre — c'est le piège de la v1 :
 * `labelKey` est une clé i18n (libellé d'interface), `label` un texte DÉJÀ
 * RÉSOLU venu de la base (titre d'activité, nom d'organisation). Les deux
 * cohabitent dans un même fil : « Back-office / Propositions / Financer
 * l'adaptation côtière ».
 *
 * LE DERNIER MAILLON N'EST PAS UN LIEN : c'est la page courante. Il porte
 * `aria-current="page"`.
 *
 * REPLI SUR ÉCRAN ÉTROIT : seuls le parent immédiat et la page courante restent
 * visibles. Le fil complet reste dans le DOM pour les lecteurs d'écran, qui
 * n'ont pas de problème de largeur.
 */

interface Props {
  items: BreadcrumbItem[]
  /** Nom de la navigation, annoncé par les lecteurs d'écran. */
  label?: string
  /** Racine du fil — « Accueil », « Back-office ». */
  root?: { label: string; to: string }
}

const props = defineProps<Props>()
const { t } = useI18n()
const localePath = useLocalePath()

const labelOf = (item: BreadcrumbItem): string => (item.labelKey ? t(item.labelKey) : (item.label ?? ''))
</script>

<template>
  <nav :aria-label="props.label ?? t('nav.breadcrumb.label')">
    <ol class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm">
      <li v-if="props.root" class="hidden items-center gap-2 sm:flex">
        <NuxtLink :to="localePath(props.root.to)" class="no-underline text-text-subtle hover:text-text">
          {{ props.root.label }}
        </NuxtLink>
      </li>

      <li
        v-for="(item, index) in props.items"
        :key="`${item.to ?? item.labelKey ?? item.label ?? index}`"
        class="items-center gap-2"
        :class="
          index >= props.items.length - 2 ? 'flex' : 'hidden sm:flex'
        "
      >
        <span aria-hidden="true" class="text-text-subtle">/</span>
        <NuxtLink
          v-if="item.to && index < props.items.length - 1"
          :to="localePath(item.to)"
          class="max-w-[16rem] truncate no-underline text-text-subtle hover:text-text"
        >
          {{ labelOf(item) }}
        </NuxtLink>
        <span
          v-else
          class="max-w-[20rem] truncate font-semibold text-text"
          aria-current="page"
        >
          {{ labelOf(item) }}
        </span>
      </li>
    </ol>
  </nav>
</template>
