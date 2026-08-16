<script setup lang="ts" generic="T extends object">
import type { SortDirection, TableColumn } from '~/types/ui'

/**
 * Tableau de données — le composant le plus dense de la plateforme.
 *
 * DENSITÉ ASSUMÉE : ces gens lisent des documents de négociation, un tableau
 * bien composé ne les rebute pas. Douze lignes lisibles valent mieux que six
 * aérées, et la pagination reste une navigation, pas un pis-aller.
 *
 * PAS DE DÉFILEMENT HORIZONTAL DU CORPS DE PAGE — règle du projet. Deux
 * dispositifs, complémentaires : `hideOnMobile` retire les colonnes secondaires
 * sous 640 px, et ce qui reste défile DANS le tableau, jamais dans la page.
 *
 * TRI CONTRÔLÉ PAR L'APPELANT. Le composant n'ordonne rien : il émet `sort` et
 * affiche l'état qu'on lui donne. Trier ici produirait un classement faux dès
 * que les données sont paginées côté serveur — ce qui est le cas de la liste des
 * propositions.
 *
 * ÉTATS : chargement (lignes squelettes à la bonne forme, pas un tourniquet),
 * vide (créneau `empty`, ou `UiEmptyState` fourni par l'appelant). L'erreur et
 * l'accès refusé se traitent AU-DESSUS du tableau : ce n'est pas au tableau de
 * dire qu'il n'a pas le droit d'exister.
 *
 * ACCESSIBILITÉ : `<caption>` est obligatoire — c'est le nom du tableau pour les
 * lecteurs d'écran. `visuallyHiddenCaption` la masque à l'œil sans la retirer.
 */

interface Props {
  columns: TableColumn[]
  rows: T[]
  /** Nom du champ qui identifie une ligne. Jamais l'index : il change au tri. */
  rowKey: keyof T & string
  /** Nom du tableau, annoncé par les lecteurs d'écran. Obligatoire. */
  caption: string
  /** La légende est-elle masquée à l'œil ? Vrai quand un titre de section la porte déjà. */
  visuallyHiddenCaption?: boolean
  /** Colonne triée et sens du tri — état, pas comportement. */
  sortKey?: string | null
  sortDirection?: SortDirection
  loading?: boolean
  /** Nombre de lignes squelettes pendant le chargement. */
  loadingRows?: number
  /** Interlignage réduit — listes de contrôle, exports à l'écran. */
  dense?: boolean
  /** En-tête collant lors du défilement vertical d'une longue liste. */
  stickyHeader?: boolean
  /** Les lignes réagissent-elles au survol ? Faux pour un tableau non cliquable. */
  hoverable?: boolean
}

const props = withDefaults(defineProps<Props>(), { loadingRows: 5, hoverable: true })
const emit = defineEmits<{
  /** Demande de tri : au parent d'ordonner et de renvoyer `sortKey`/`sortDirection`. */
  sort: [key: string, direction: Exclude<SortDirection, null>]
  rowClick: [row: T]
}>()

/**
 * Créneaux typés. `cell-<clé>` reçoit la ligne COMPLÈTE et typée : c'est ce qui
 * permet d'écrire `row.title` dans un écran sans conversion, et au compilateur
 * de signaler une colonne renommée dans la vue SQL.
 */
defineSlots<
  { empty?: () => unknown } & {
    [K in `cell-${string}`]?: (props: { row: T; value: unknown }) => unknown
  }
>()

const { t } = useI18n()

/**
 * Valeur brute d'une cellule. La clé de colonne est une chaîne libre — elle peut
 * désigner une colonne calculée (`actions`) qui n'existe dans aucune ligne —,
 * d'où la lecture indexée plutôt qu'un accès typé.
 */
function cellValue(row: T, key: string): unknown {
  return (row as Record<string, unknown>)[key]
}

/** Trois positions : croissant, décroissant, retour au tri par défaut. */
function toggleSort(column: TableColumn): void {
  if (!column.sortable) return
  const isCurrent = props.sortKey === column.key
  emit('sort', column.key, isCurrent && props.sortDirection === 'asc' ? 'desc' : 'asc')
}

function ariaSort(column: TableColumn): 'ascending' | 'descending' | 'none' | undefined {
  if (!column.sortable) return undefined
  if (props.sortKey !== column.key || !props.sortDirection) return 'none'
  return props.sortDirection === 'asc' ? 'ascending' : 'descending'
}

const alignClass = (column: TableColumn): string => {
  if (column.align === 'end' || column.numeric) return 'text-right'
  if (column.align === 'center') return 'text-center'
  return 'text-left'
}

const cellPadding = computed(() => (props.dense ? 'px-3 py-2' : 'px-3 py-3'))
</script>

<template>
  <div class="overflow-x-auto rounded-lg border border-border bg-surface-raised">
    <table class="w-full border-collapse text-sm" :aria-busy="props.loading ? 'true' : undefined">
      <caption
        class="px-3 py-2 text-left text-sm text-text-subtle"
        :class="props.visuallyHiddenCaption ? 'sr-only' : ''"
      >
        {{ props.caption }}
      </caption>

      <thead
        class="bg-surface-sunken text-xs tracking-wide text-text-muted uppercase"
        :class="props.stickyHeader ? 'sticky top-0 z-10' : ''"
      >
        <tr>
          <th
            v-for="column in props.columns"
            :key="column.key"
            scope="col"
            :style="column.width ? { width: column.width } : undefined"
            :aria-sort="ariaSort(column)"
            class="border-b border-border font-semibold"
            :class="[cellPadding, alignClass(column), column.hideOnMobile ? 'hidden sm:table-cell' : '']"
          >
            <button
              v-if="column.sortable"
              type="button"
              class="inline-flex items-center gap-1 rounded-sm text-inherit uppercase transition-colors hover:text-text"
              @click="toggleSort(column)"
            >
              {{ column.label }}
              <UiIcon
                :name="props.sortKey === column.key && props.sortDirection === 'desc' ? 'sort-desc' : 'sort-asc'"
                size="0.9rem"
                :class="props.sortKey === column.key ? 'text-accent' : 'text-text-subtle'"
              />
              <span class="sr-only">
                {{ props.sortKey === column.key && props.sortDirection === 'asc'
                  ? t('data.table.sortDesc')
                  : t('data.table.sortAsc') }}
              </span>
            </button>
            <template v-else>{{ column.label }}</template>
          </th>
        </tr>
      </thead>

      <!-- Chargement : des lignes squelettes à la forme du tableau. Un tourniquet
           centré n'annonce pas ce qui arrive et fait sauter la mise en page. -->
      <tbody v-if="props.loading">
        <tr v-for="index in props.loadingRows" :key="`skeleton-${index}`" class="border-b border-border-subtle last:border-0">
          <td
            v-for="column in props.columns"
            :key="column.key"
            :class="[cellPadding, column.hideOnMobile ? 'hidden sm:table-cell' : '']"
          >
            <UiSkeletonLoader :width="column.numeric ? '3rem' : '80%'" height="0.9rem" />
          </td>
        </tr>
      </tbody>

      <tbody v-else-if="props.rows.length === 0">
        <tr>
          <td :colspan="props.columns.length" class="px-3 py-10">
            <slot name="empty">
              <p class="text-center text-text-muted">{{ t('common.states.empty.title') }}</p>
            </slot>
          </td>
        </tr>
      </tbody>

      <tbody v-else>
        <tr
          v-for="row in props.rows"
          :key="String(row[props.rowKey])"
          class="border-b border-border-subtle last:border-0"
          :class="props.hoverable ? 'transition-colors hover:bg-surface-hover' : ''"
          @click="emit('rowClick', row)"
        >
          <td
            v-for="column in props.columns"
            :key="column.key"
            :class="[
              cellPadding,
              alignClass(column),
              column.numeric ? 'font-mono tabular-nums' : '',
              column.hideOnMobile ? 'hidden sm:table-cell' : '',
              'align-top text-text',
            ]"
          >
            <!-- Un créneau par colonne : `#cell-status`, `#cell-title`… Sans
                 créneau, la valeur brute est affichée. -->
            <slot :name="`cell-${column.key}`" :row="row" :value="cellValue(row, column.key)">
              {{ cellValue(row, column.key) ?? '—' }}
            </slot>
          </td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
