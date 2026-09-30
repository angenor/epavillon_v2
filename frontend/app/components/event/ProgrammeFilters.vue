<script setup lang="ts">
import type { ProgrammeFilterState, ProgrammeThemeOption } from '~/types/event-programme'
import type { TaxonomyTermCode } from '~/types/shared'

/**
 * Filtres de la programmation, partagés par la semaine et la liste du jour.
 * Une commande allumée s'enfonce : pas de case à cocher, le geste se voit.
 * Les thématiques viennent du programme affiché, libellé et couleur de la base.
 */

interface Props {
  modelValue: ProgrammeFilterState
  themes: ProgrammeThemeOption[]
  resultLabel: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: ProgrammeFilterState] }>()

const { t } = useI18n()

const themesOpen = ref(false)

function patch(value: Partial<ProgrammeFilterState>): void {
  emit('update:modelValue', { ...props.modelValue, ...value })
}

function toggleTheme(code: TaxonomyTermCode): void {
  const themes = props.modelValue.themes
  patch({ themes: themes.includes(code) ? themes.filter((entry) => entry !== code) : [...themes, code] })
}

const hasFilters = computed(
  () =>
    props.modelValue.themes.length > 0 ||
    props.modelValue.search.trim().length > 0 ||
    props.modelValue.streamedOnly ||
    props.modelValue.hidePast,
)

const pressable = (on: boolean) =>
  on
    ? 'translate-x-[3px] translate-y-[3px] bg-poster-ink text-poster-on-ink-accent'
    : 'bg-poster-paper-raised text-poster-ink shadow-poster-sm hover:-translate-y-0.5'
</script>

<template>
  <div class="flex flex-col gap-4">
    <div class="flex flex-wrap items-center gap-3">
      <label
        class="flex h-14 min-w-0 flex-[1_1_20rem] items-center gap-3 rounded-md border-2 border-poster-ink bg-poster-paper-raised px-4 shadow-poster focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-focus sm:max-w-[32rem]"
      >
        <UiIcon name="search" size="1.25rem" class="shrink-0" />
        <span class="sr-only">{{ t('programme.filters.search') }}</span>
        <input
          type="search"
          class="min-w-0 flex-1 bg-transparent text-base text-poster-ink outline-none placeholder:text-poster-ink-muted"
          :value="props.modelValue.search"
          :placeholder="t('programme.filters.searchPlaceholder')"
          @input="patch({ search: ($event.target as HTMLInputElement).value })"
        >
      </label>

      <button
        type="button"
        class="inline-flex h-14 cursor-pointer items-center gap-2.5 rounded-md border-2 border-poster-ink px-4 font-semibold transition-transform"
        :class="pressable(props.modelValue.streamedOnly)"
        :aria-pressed="props.modelValue.streamedOnly"
        @click="patch({ streamedOnly: !props.modelValue.streamedOnly })"
      >
        <UiIcon name="broadcast" size="1.125rem" />
        {{ t('programme.filters.streamed') }}
      </button>

      <button
        type="button"
        class="inline-flex h-14 cursor-pointer items-center gap-2.5 rounded-md border-2 border-poster-ink px-4 font-semibold transition-transform"
        :class="pressable(props.modelValue.hidePast)"
        :aria-pressed="props.modelValue.hidePast"
        @click="patch({ hidePast: !props.modelValue.hidePast })"
      >
        <UiIcon name="eye-off" size="1.125rem" />
        {{ t('programme.filters.hidePast') }}
      </button>

      <div class="basis-full sm:ml-auto sm:basis-auto">
        <slot name="view" />
      </div>
    </div>

    <div class="flex flex-wrap items-center gap-2.5">
      <span class="hidden font-poster-mono text-xs font-semibold tracking-[0.08em] text-poster-ink-muted uppercase sm:inline">
        {{ t('programme.filters.themes') }}
      </span>
      <button
        type="button"
        class="inline-flex h-11 cursor-pointer items-center gap-2 font-poster-mono text-xs font-semibold tracking-[0.08em] text-poster-ink-muted uppercase sm:hidden"
        :aria-expanded="themesOpen"
        aria-controls="programmation-thematiques"
        @click="themesOpen = !themesOpen"
      >
        {{ t('programme.filters.themes') }}
        <span v-if="props.modelValue.themes.length" class="text-poster-ink">· {{ props.modelValue.themes.length }}</span>
        <UiIcon :name="themesOpen ? 'chevron-up' : 'chevron-down'" size="1rem" />
      </button>

      <div
        id="programmation-thematiques"
        class="flex-wrap items-center gap-2.5"
        :class="themesOpen ? 'flex basis-full sm:basis-auto' : 'hidden sm:flex'"
      >
        <button
          v-for="theme in props.themes"
          :key="theme.code"
          type="button"
          class="inline-flex h-11 cursor-pointer items-center gap-2.5 rounded-full border-2 border-poster-ink pr-4 pl-2.5 text-sm font-semibold text-poster-ink transition-transform"
          :class="props.modelValue.themes.includes(theme.code) ? 'translate-x-[3px] translate-y-[3px]' : 'bg-poster-paper-raised shadow-poster-sm hover:-translate-y-0.5'"
          :style="
            props.modelValue.themes.includes(theme.code)
              ? { background: theme.color ? `color-mix(in oklab, ${theme.color} var(--poster-theme-strength), var(--color-poster-paper-raised))` : 'var(--color-poster-paper-sunken)' }
              : undefined
          "
          :aria-pressed="props.modelValue.themes.includes(theme.code)"
          @click="toggleTheme(theme.code)"
        >
          <span
            class="size-4.5 rounded-full border-2 border-poster-ink"
            :style="{ background: theme.color ?? 'var(--color-poster-paper-sunken)' }"
            aria-hidden="true"
          />
          {{ theme.label }}
          <span class="font-poster-mono text-xs text-poster-ink-muted">{{ theme.count }}</span>
        </button>
      </div>

      <button
        v-if="hasFilters"
        type="button"
        class="h-11 cursor-pointer px-2 text-sm font-semibold text-poster-ink underline underline-offset-4"
        @click="emit('update:modelValue', { themes: [], search: '', streamedOnly: false, hidePast: false })"
      >
        {{ t('programme.filters.clear') }}
      </button>

      <p class="w-full font-poster-mono text-sm text-poster-ink sm:ml-auto sm:w-auto" aria-live="polite">
        {{ props.resultLabel }}
      </p>
    </div>
  </div>
</template>
