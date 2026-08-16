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
 * IL N'APPARAÎT QU'À PARTIR DU TROISIÈME NIVEAU DE PROFONDEUR — règle du guide,
 * portée par `minDepth`. Sous ce seuil, le fil ne fait que répéter ce que la
 * barre de navigation dit déjà : « Accueil › Événements » au-dessus de la page
 * Événements est du bruit, et le bruit finit par rendre le fil invisible là où
 * il sert vraiment. La profondeur compte TOUS les maillons, racine comprise.
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
 * SÉPARATEUR DESSINÉ, pas typographié : le chevron de 12 px reste un signe de
 * direction à toute taille, là où la barre oblique se confond avec le texte des
 * libellés — et se lit à voix haute par certains lecteurs d'écran.
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
  /**
   * Profondeur minimale — racine comprise — en deçà de laquelle RIEN n'est
   * rendu. Trois par défaut, conformément au guide. La descendre à 1 force
   * l'affichage d'un fil court, ce qui ne se justifie que sur un écran isolé du
   * reste de la navigation.
   */
  minDepth?: number
}

const props = withDefaults(defineProps<Props>(), { minDepth: 3 })
const { t } = useI18n()
const localePath = useLocalePath()

const depth = computed(() => props.items.length + (props.root ? 1 : 0))
const isDeepEnough = computed(() => depth.value >= props.minDepth)

const labelOf = (item: BreadcrumbItem): string => (item.labelKey ? t(item.labelKey) : (item.label ?? ''))
</script>

<template>
  <nav v-if="isDeepEnough" :aria-label="props.label ?? t('nav.breadcrumb.label')">
    <ol class="flex flex-wrap items-center gap-x-2 gap-y-1 text-sm">
      <li v-if="props.root" class="hidden items-center gap-2 sm:flex">
        <NuxtLink
          :to="localePath(props.root.to)"
          class="no-underline text-text-secondary transition-colors duration-(--duration-fast) hover:text-accent hover:underline"
        >
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
        <UiIcon name="chevron-right" size="0.75rem" :stroke-width="2.2" class="text-text-muted" />
        <NuxtLink
          v-if="item.to && index < props.items.length - 1"
          :to="localePath(item.to)"
          class="max-w-[16rem] truncate no-underline text-text-secondary transition-colors duration-(--duration-fast) hover:text-accent hover:underline"
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
