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
 * sous 640 px — `hideBelow` fait de même à 1 024, 1 280 et 1 536 px, pour les
 * tableaux de plus de huit colonnes —, et ce qui reste défile DANS le tableau, jamais
 * dans la page.
 * C'est pourquoi le cadre extérieur est en `overflow-hidden` (il porte le
 * rayon, la barre d'outils et le filet) et que SEULE la zone du tableau défile :
 * mettre le défilement sur le cadre emporterait la barre d'outils avec lui.
 *
 * TRI CONTRÔLÉ PAR L'APPELANT. Le composant n'ordonne rien : il émet `sort` et
 * affiche l'état qu'on lui donne. Trier ici produirait un classement faux dès
 * que les données sont paginées côté serveur — ce qui est le cas de la liste des
 * propositions.
 *
 * SÉLECTION PORTÉE PAR L'APPELANT, elle aussi (`v-model:selected`) : ce sont des
 * CLÉS de lignes, pas des indices ni des objets. Conséquence voulue — une clé
 * retenue sur la page 2 reste retenue quand on revient page 1, et « tout
 * sélectionner » ne touche QUE les lignes visibles. Une case à cocher qui
 * retiendrait silencieusement cent trente-sept dossiers dont douze seulement
 * sont sous les yeux est un piège à action de masse.
 *
 * ÉTATS : chargement (lignes squelettes à la bonne forme, pas un tourniquet),
 * vide (créneau `empty`, ou `UiEmptyState` fourni par l'appelant). L'erreur et
 * l'accès refusé se traitent AU-DESSUS du tableau : ce n'est pas au tableau de
 * dire qu'il n'a pas le droit d'exister.
 *
 * ACCESSIBILITÉ : `<caption>` est obligatoire — c'est le nom du tableau pour les
 * lecteurs d'écran. `visuallyHiddenCaption` la masque à l'œil sans la retirer.
 * Elle est en chasse fixe : elle porte un périmètre chiffré (« COP30, Belém, 10
 * au 21 novembre »), pas une phrase, et l'alignement des chiffres se lit mieux.
 */

interface Props {
  columns: TableColumn[]
  rows: T[]
  /** Nom du champ qui identifie une ligne. Jamais l'index : il change au tri. */
  rowKey: keyof T & string
  /**
   * Champ qui NOMME une ligne pour les lecteurs d'écran — le numéro de dossier,
   * pas son identifiant technique. Sans lui, la colonne de cases à cocher
   * s'annonce « Sélectionner la ligne 0198c1a0-0000-7040-… », ce qui n'est
   * lisible par personne. À défaut, `rowKey` est utilisé.
   */
  rowLabelKey?: keyof T & string
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
  /** Colonne de cases à cocher en tête de rangée, plus la case « tout sélectionner ». */
  selectable?: boolean
  /** Clés des lignes retenues — `v-model:selected`. Voir l'en-tête sur la portée. */
  selected?: string[]
}

const props = withDefaults(defineProps<Props>(), {
  loadingRows: 5,
  hoverable: true,
  selected: () => [],
})
const emit = defineEmits<{
  /** Demande de tri : au parent d'ordonner et de renvoyer `sortKey`/`sortDirection`. */
  sort: [key: string, direction: Exclude<SortDirection, null>]
  rowClick: [row: T]
  'update:selected': [keys: string[]]
}>()

/**
 * Créneaux typés. `cell-<clé>` reçoit la ligne COMPLÈTE et typée : c'est ce qui
 * permet d'écrire `row.title` dans un écran sans conversion, et au compilateur
 * de signaler une colonne renommée dans la vue SQL.
 *
 * `toolbar` est rendu DANS le cadre, au-dessus de l'en-tête : recherche, jetons
 * de filtre actifs, export. Les poser au-dessus du cadre les détacherait
 * visuellement du tableau qu'ils pilotent.
 */
defineSlots<
  { empty?: () => unknown; toolbar?: () => unknown } & {
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

/**
 * Seuil d'apparition d'une colonne. `hideOnMobile` reste le raccourci du cas le
 * plus courant ; `hideBelow` ouvre les deux autres paliers, pour les tableaux
 * qui portent plus de huit colonnes.
 */
const RESPONSIVE: Record<'sm' | 'lg' | 'xl' | '2xl', string> = {
  sm: 'hidden sm:table-cell',
  lg: 'hidden lg:table-cell',
  xl: 'hidden xl:table-cell',
  '2xl': 'hidden 2xl:table-cell',
}

function responsiveClass(column: TableColumn): string {
  if (column.hideBelow) return RESPONSIVE[column.hideBelow]
  return column.hideOnMobile ? RESPONSIVE.sm : ''
}

// --- Sélection ---------------------------------------------------------------

/** Clés des lignes AFFICHÉES ; la sélection, elle, peut en contenir d'autres. */
const pageKeys = computed(() => props.rows.map((row) => String(row[props.rowKey])))
const selectedKeys = computed(() => new Set(props.selected))

const allPageSelected = computed(
  () => pageKeys.value.length > 0 && pageKeys.value.every((key) => selectedKeys.value.has(key)),
)
/** Sélection partielle → case indéterminée, que `UiCheckbox` sait déjà rendre. */
const somePageSelected = computed(
  () => !allPageSelected.value && pageKeys.value.some((key) => selectedKeys.value.has(key)),
)

/** Ce que le lecteur d'écran énonce pour cette ligne. */
function rowLabel(row: T): string {
  return String(row[props.rowLabelKey ?? props.rowKey])
}

function isSelected(row: T): boolean {
  return selectedKeys.value.has(String(row[props.rowKey]))
}

function toggleRow(row: T, checked: boolean): void {
  const next = new Set(selectedKeys.value)
  const key = String(row[props.rowKey])
  if (checked) next.add(key)
  else next.delete(key)
  emit('update:selected', [...next])
}

/** N'agit QUE sur les lignes visibles — voir l'en-tête. */
function toggleAll(checked: boolean): void {
  const next = new Set(selectedKeys.value)
  for (const key of pageKeys.value) {
    if (checked) next.add(key)
    else next.delete(key)
  }
  emit('update:selected', [...next])
}

/**
 * « Sélectionner toutes les lignes AFFICHÉES » — le libellé dit ce que la case
 * fait vraiment. Sur une liste paginée, cocher n'atteint jamais les 28 dossiers
 * des pages suivantes, et un « tout sélectionner » qui laisserait croire le
 * contraire ferait manquer une action de masse sans que personne s'en aperçoive.
 */
const selectAllLabel = computed(() => t('data.table.selectAll'))

/** Colonnes réelles + la colonne de cases, pour les `colspan` des états. */
const spanCount = computed(() => props.columns.length + (props.selectable ? 1 : 0))
</script>

<template>
  <div class="overflow-hidden rounded-lg border border-border bg-surface-raised">
    <!-- Barre d'outils : recherche, jetons de filtre, export. Dans le cadre,
         au-dessus de l'en-tête, séparée par un filet — elle appartient au
         tableau, pas à la page. -->
    <div
      v-if="$slots.toolbar"
      class="flex flex-wrap items-center gap-3 border-b border-separator px-4 py-3"
    >
      <slot name="toolbar" />
    </div>

    <div class="overflow-x-auto">
      <table class="w-full border-collapse text-sm" :aria-busy="props.loading ? 'true' : undefined">
        <caption
          class="caption-top px-4 pt-3 text-left font-mono text-xs text-text-muted"
          :class="props.visuallyHiddenCaption ? 'sr-only' : ''"
        >
          {{ props.caption }}
        </caption>

        <!-- Filet de 2 px en `border-strong` : c'est LUI qui sépare l'en-tête du
             corps, pas un aplat. Un trait franc tient la lecture sur douze lignes
             denses là où un fond gris se confond avec les lignes survolées. -->
        <thead
          class="bg-surface-sunken text-xs text-text-muted uppercase"
          :class="props.stickyHeader ? 'sticky top-0 z-10' : ''"
        >
          <tr>
            <th
              v-if="props.selectable"
              scope="col"
              class="ui-table-check w-8.5 border-b-(length:--border-medium) border-b-border-strong px-3 py-2"
            >
              <UiCheckbox
                :model-value="allPageSelected"
                :indeterminate="somePageSelected"
                :label="selectAllLabel"
                @update:model-value="toggleAll"
              />
            </th>

            <th
              v-for="column in props.columns"
              :key="column.key"
              scope="col"
              :style="column.width ? { width: column.width } : undefined"
              :aria-sort="ariaSort(column)"
              class="border-b-(length:--border-medium) border-b-border-strong px-3 py-2 font-semibold tracking-caps whitespace-nowrap"
              :class="[alignClass(column), responsiveClass(column)]"
            >
              <button
                v-if="column.sortable"
                type="button"
                class="inline-flex min-h-(--target-compact) cursor-pointer items-center gap-2 rounded-sm text-inherit uppercase transition-colors duration-(--duration-fast) hover:text-accent"
                :class="props.sortKey === column.key ? 'text-accent' : ''"
                @click="toggleSort(column)"
              >
                {{ column.label }}
                <!-- Atténuée au repos : douze chevrons à pleine encre feraient
                     une colonne de bruit. Seule la colonne triée s'affirme. -->
                <UiIcon
                  :name="props.sortKey === column.key && props.sortDirection === 'desc' ? 'sort-desc' : 'sort-asc'"
                  size="0.75rem"
                  :class="props.sortKey === column.key ? 'opacity-100' : 'opacity-40'"
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
            <td v-if="props.selectable" :class="cellPadding">
              <UiSkeletonLoader width="1.125rem" height="1.125rem" rounded="var(--radius-sm)" />
            </td>
            <td
              v-for="column in props.columns"
              :key="column.key"
              :class="[cellPadding, responsiveClass(column)]"
            >
              <UiSkeletonLoader :width="column.numeric ? '3rem' : '80%'" height="0.9rem" />
            </td>
          </tr>
        </tbody>

        <tbody v-else-if="props.rows.length === 0">
          <tr>
            <td :colspan="spanCount" class="px-3 py-10">
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
            :aria-selected="props.selectable ? isSelected(row) : undefined"
            :class="[
              props.hoverable ? 'transition-colors duration-(--duration-fast) hover:bg-surface-hover' : '',
              // Après le survol dans l'ordre des classes : une ligne retenue le
              // reste sous le pointeur, sinon la sélection clignote au passage.
              isSelected(row) ? 'bg-accent-surface hover:bg-accent-surface' : '',
            ]"
            @click="emit('rowClick', row)"
          >
            <!-- `click.stop` : cocher une ligne n'est pas l'ouvrir. Sans lui,
                 chaque case cochée déclencherait aussi la navigation. -->
            <td v-if="props.selectable" class="ui-table-check" :class="cellPadding" @click.stop>
              <UiCheckbox
                :model-value="isSelected(row)"
                :label="t('data.table.selectRow', { label: rowLabel(row) })"
                @update:model-value="(checked: boolean) => toggleRow(row, checked)"
              />
            </td>

            <td
              v-for="column in props.columns"
              :key="column.key"
              :class="[
                cellPadding,
                alignClass(column),
                column.numeric ? 'font-mono tabular-nums' : '',
                responsiveClass(column),
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
  </div>
</template>

<style scoped>
/* La case d'une colonne de sélection n'a pas de libellé VISIBLE, mais elle en a
   un : sans nom accessible, une colonne de cases est une colonne d'inconnues au
   lecteur d'écran. On masque donc le libellé de `UiCheckbox` à l'œil sans le
   retirer de l'arbre d'accessibilité. Le `position: absolute` est le point
   important : il sort le libellé du flux, sinon la gouttière que le composant
   réserve entre la case et son texte élargirait la colonne d'une dizaine de
   pixels pour un libellé de largeur nulle. */
.ui-table-check :deep(label) {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip-path: inset(50%);
  white-space: nowrap;
}
</style>
