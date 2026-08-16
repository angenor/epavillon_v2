<script setup lang="ts">
/**
 * Pagination — quarante dossiers ne tiennent pas sur un écran, et ne doivent pas
 * y tenir.
 *
 * CE QU'ELLE ANNONCE, avant les numéros : « Résultats 1 à 20 sur 40 ». Le
 * décompte total est l'information la plus utile de la barre — c'est lui qui dit
 * si le filtre a bien mordu.
 *
 * FENÊTRE GLISSANTE : première page, dernière page, et les voisines de la page
 * courante. Les sauts sont marqués par des points de suspension non
 * focalisables. Vingt boutons de page seraient vingt cibles de tabulation entre
 * la liste et le pied de page.
 *
 * DES BOUTONS BORDÉS DE 40 px, ET UN APLAT PLEIN POUR LA PAGE COURANTE. Le fond
 * pâle qui servait ici auparavant se lisait mal au milieu d'une rangée de
 * chiffres : la page où l'on se trouve doit se repérer sans lecture. Les
 * flèches partagent exactement le même habillage — trois formes différentes
 * dans une barre de dix boutons donnent une barre qui bégaie.
 *
 * SUR ÉCRAN ÉTROIT, seuls « précédent » et « suivant » restent, avec le
 * décompte. Les numéros de page sur 375 px produisent des cibles trop petites.
 *
 * NOMS ACCESSIBLES EXPLICITES : « Aller à la page 3 », jamais « 3 » seul. Une
 * liste de chiffres nus est incompréhensible à l'oreille.
 */

interface Props {
  /** Page courante, à partir de 1. */
  page: number
  perPage: number
  /** Nombre total d'éléments, toutes pages confondues. */
  total: number
  /** Choix de taille de page proposés. Absent, le sélecteur n'est pas rendu. */
  perPageOptions?: number[]
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:page': [page: number]
  'update:perPage': [perPage: number]
}>()

const { t } = useI18n()

/**
 * Habillage commun à toutes les cibles de la barre — flèches et numéros. Le
 * garder en une seule constante est ce qui empêche les deux de diverger à la
 * première retouche.
 */
const CONTROL
  = 'inline-grid min-h-(--target-compact) min-w-(--target-compact) place-items-center'
    + ' rounded-md border px-2 text-xs font-semibold tabular-nums transition-colors'
    + ' duration-(--duration-fast) disabled:cursor-not-allowed disabled:opacity-40'

const CONTROL_IDLE = 'border-border bg-surface-raised text-text-secondary hover:border-accent hover:text-accent'
const CONTROL_CURRENT = 'border-accent-solid bg-accent-solid text-accent-contrast'

const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.perPage)))
const from = computed(() => (props.total === 0 ? 0 : (props.page - 1) * props.perPage + 1))
const to = computed(() => Math.min(props.page * props.perPage, props.total))

/** Fenêtre glissante : 1 … 4 [5] 6 … 12. `null` marque un saut. */
const pages = computed<(number | null)[]>(() => {
  const last = totalPages.value
  const current = props.page
  if (last <= 7) return Array.from({ length: last }, (_, index) => index + 1)

  const window = new Set<number>([1, last, current, current - 1, current + 1])
  if (current <= 3) [2, 3, 4].forEach((page) => window.add(page))
  if (current >= last - 2) [last - 3, last - 2, last - 1].forEach((page) => window.add(page))

  const sorted = [...window].filter((page) => page >= 1 && page <= last).sort((a, b) => a - b)
  const result: (number | null)[] = []
  let previous = 0
  for (const page of sorted) {
    if (previous && page - previous > 1) result.push(null)
    result.push(page)
    previous = page
  }
  return result
})

function go(page: number): void {
  if (props.disabled) return
  const target = Math.min(Math.max(1, page), totalPages.value)
  if (target !== props.page) emit('update:page', target)
}
</script>

<template>
  <nav
    :aria-label="t('data.pagination.label')"
    class="flex flex-wrap items-center justify-between gap-3"
  >
    <p class="text-sm text-text-muted" aria-live="polite">
      {{ t('common.pagination.showing', { from, to, total: props.total }) }}
    </p>

    <div class="flex items-center gap-3">
      <label v-if="props.perPageOptions?.length" class="flex items-center gap-2 text-sm text-text-muted">
        {{ t('common.pagination.perPage') }}
        <select
          :value="props.perPage"
          :disabled="props.disabled"
          class="rounded-md border border-border-strong bg-surface-raised px-2 py-1 text-sm text-text disabled:cursor-not-allowed disabled:bg-surface-sunken"
          @change="emit('update:perPage', Number(($event.target as HTMLSelectElement).value))"
        >
          <option v-for="option in props.perPageOptions" :key="option" :value="option">{{ option }}</option>
        </select>
      </label>

      <ul class="flex items-center gap-1">
        <li>
          <button
            type="button"
            :class="[CONTROL, CONTROL_IDLE]"
            :aria-label="t('common.pagination.previous')"
            :disabled="props.disabled || props.page <= 1"
            @click="go(props.page - 1)"
          >
            <UiIcon name="chevron-left" size="0.875rem" :stroke-width="2.2" />
          </button>
        </li>

        <li v-for="(entry, index) in pages" :key="entry ?? `gap-${index}`" class="hidden sm:block">
          <span v-if="entry === null" class="px-1.5 text-text-subtle" aria-hidden="true">…</span>
          <button
            v-else
            type="button"
            :class="[CONTROL, entry === props.page ? CONTROL_CURRENT : CONTROL_IDLE]"
            :aria-current="entry === props.page ? 'page' : undefined"
            :aria-label="t('data.pagination.goTo', { page: entry })"
            :disabled="props.disabled"
            @click="go(entry)"
          >
            {{ entry }}
          </button>
        </li>

        <!-- Sur écran étroit, le repère de position remplace les numéros. -->
        <li class="px-2 text-sm text-text-muted sm:hidden">
          {{ t('common.pagination.page', { current: props.page, total: totalPages }) }}
        </li>

        <li>
          <button
            type="button"
            :class="[CONTROL, CONTROL_IDLE]"
            :aria-label="t('common.pagination.next')"
            :disabled="props.disabled || props.page >= totalPages"
            @click="go(props.page + 1)"
          >
            <UiIcon name="chevron-right" size="0.875rem" :stroke-width="2.2" />
          </button>
        </li>
      </ul>
    </div>
  </nav>
</template>
