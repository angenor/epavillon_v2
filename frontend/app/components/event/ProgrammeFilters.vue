<script setup lang="ts">
import type { ProgrammeFilterState } from '~/types/event-programme'
import type { SelectOption } from '~/types/ui'

/**
 * FILTRES DE LA PROGRAMMATION — jour, thématique, format, salle.
 *
 * ILS SONT PARTAGÉS PAR LES DEUX VUES. C'est l'exigence du prompt, et ce n'est
 * pas un détail d'implémentation : quelqu'un qui a filtré sur « Adaptation » en
 * vue grille et bascule en calendrier veut voir SES activités placées dans le
 * temps, pas tout recommencer. L'état vit donc au-dessus, dans la section, et ce
 * composant ne fait que l'afficher et le modifier.
 *
 * LES OPTIONS VIENNENT DES DONNÉES AFFICHÉES, jamais d'une liste écrite en dur :
 * les thématiques sont celles que porte ce programme (avec le libellé et la
 * couleur de `reference.taxonomy_terms`), les salles celles de cette édition,
 * les jours ceux qui portent au moins une activité. Un filtre qui propose une
 * valeur ne ramenant rien fait perdre un clic à chaque fois.
 *
 * LES FILTRES ACTIFS SONT RAPPELÉS EN JETONS sous les listes : sur mobile, les
 * listes déroulantes se replient et on oublie ce qu'on a filtré — un écran vide
 * se lit alors comme une panne.
 */

interface Props {
  modelValue: ProgrammeFilterState
  days: SelectOption[]
  themes: SelectOption[]
  formats: SelectOption[]
  rooms: SelectOption[]
  /** Nombre d'activités après filtrage — affiché à côté des filtres. */
  resultCount: number
  /** Nombre total, avant filtrage. */
  totalCount: number
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: ProgrammeFilterState] }>()

const { t } = useI18n()

type FilterKey = keyof ProgrammeFilterState

function set(key: FilterKey, value: string | null): void {
  emit('update:modelValue', { ...props.modelValue, [key]: value === '' ? null : value })
}

function reset(): void {
  emit('update:modelValue', { day: null, theme: null, format: null, room: null })
}

const optionsByKey = computed<Record<FilterKey, SelectOption[]>>(() => ({
  day: props.days,
  theme: props.themes,
  format: props.formats,
  room: props.rooms,
}))

/** Jetons des filtres posés, avec le critère nommé : « Thématique : Adaptation ». */
const activeChips = computed(() =>
  (Object.keys(optionsByKey.value) as FilterKey[])
    .map((key) => {
      const value = props.modelValue[key]
      if (!value) return null
      const option = optionsByKey.value[key].find((entry) => entry.value === value)
      return option ? { key, label: option.label } : null
    })
    .filter((chip): chip is { key: FilterKey; label: string } => chip !== null),
)

const hasFilters = computed(() => activeChips.value.length > 0)
</script>

<template>
  <div class="rounded-lg border border-border bg-surface-raised px-4 py-4">
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
      <UiSelect
        :model-value="props.modelValue.day ?? ''"
        :options="props.days"
        :label="t('programme.filters.day')"
        :disabled="!props.days.length"
        hide-optional
        @update:model-value="set('day', $event)"
      />
      <UiSelect
        :model-value="props.modelValue.theme ?? ''"
        :options="props.themes"
        :label="t('programme.filters.theme')"
        :disabled="!props.themes.length"
        hide-optional
        @update:model-value="set('theme', $event)"
      />
      <UiSelect
        :model-value="props.modelValue.format ?? ''"
        :options="props.formats"
        :label="t('programme.filters.format')"
        :disabled="!props.formats.length"
        hide-optional
        @update:model-value="set('format', $event)"
      />
      <UiSelect
        :model-value="props.modelValue.room ?? ''"
        :options="props.rooms"
        :label="t('programme.filters.room')"
        :disabled="!props.rooms.length"
        hide-optional
        @update:model-value="set('room', $event)"
      />
    </div>

    <div class="mt-3 flex flex-wrap items-center gap-2 border-t border-separator pt-3">
      <p class="text-sm text-text-muted" aria-live="polite">
        {{
          hasFilters
            ? t('programme.filters.filtered', {
                count: props.resultCount,
                total: props.totalCount,
              })
            : t('programme.filters.total', { count: props.totalCount })
        }}
      </p>

      <UiChip
        v-for="chip in activeChips"
        :key="chip.key"
        :facet="t(`programme.filters.${chip.key}`)"
        :label="chip.label"
        @remove="set(chip.key, null)"
      />

      <UiButton
        v-if="hasFilters"
        class="ml-auto"
        variant="ghost"
        size="sm"
        :label="t('common.actions.reset')"
        @click="reset()"
      />
    </div>
  </div>
</template>
