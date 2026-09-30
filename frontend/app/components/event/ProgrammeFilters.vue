<script setup lang="ts">
import type { ProgrammeFilterState, ProgrammeThemeOption } from '~/types/event-programme'
import type { TaxonomyTermCode } from '~/types/shared'

/**
 * La barre du programme, sur une ligne : recherche, thématiques repliées dans
 * une fenêtre, choix de la vue (fente `view`). Les filtres posés se rappellent
 * en étiquettes qu'on retire d'un clic. Ils ne passent jamais devant le tableau.
 */

interface Props {
  modelValue: ProgrammeFilterState
  themes: ProgrammeThemeOption[]
  resultCount: number
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: ProgrammeFilterState] }>()

const { t } = useI18n()
const { fill } = useProgrammeSession()

const open = ref(false)
const root = useTemplateRef<HTMLElement>('root')

function patch(value: Partial<ProgrammeFilterState>): void {
  emit('update:modelValue', { ...props.modelValue, ...value })
}

function toggleTheme(code: TaxonomyTermCode): void {
  const themes = props.modelValue.themes
  patch({ themes: themes.includes(code) ? themes.filter((entry) => entry !== code) : [...themes, code] })
}

const search = computed(() => props.modelValue.search.trim())

const chips = computed(() => [
  ...(search.value ? [{ key: 'search', label: `« ${search.value} »`, background: undefined, remove: () => patch({ search: '' }) }] : []),
  ...props.modelValue.themes.map((code) => {
    const theme = props.themes.find((entry) => entry.code === code)
    return { key: code, label: theme?.label ?? code, background: fill(theme?.color ?? null), remove: () => toggleTheme(code) }
  }),
])

function onDocument(event: Event): void {
  if (open.value && root.value && !root.value.contains(event.target as Node)) open.value = false
}
function onKey(event: KeyboardEvent): void {
  if (event.key === 'Escape') open.value = false
}
onMounted(() => {
  document.addEventListener('pointerdown', onDocument)
  document.addEventListener('keydown', onKey)
})
onBeforeUnmount(() => {
  document.removeEventListener('pointerdown', onDocument)
  document.removeEventListener('keydown', onKey)
})
</script>

<template>
  <div class="flex flex-col gap-3">
    <div ref="root" class="relative flex flex-wrap items-center gap-2.5">
      <slot name="title" />

      <label
        class="flex h-11 min-w-0 basis-full items-center sm:flex-[1_1_14rem] sm:basis-auto gap-2.5 rounded-md border-2 border-poster-ink bg-poster-paper-raised px-3 focus-within:outline-2 focus-within:outline-offset-2 focus-within:outline-focus sm:max-w-72"
      >
        <UiIcon name="search" size="1.0625rem" class="shrink-0" />
        <span class="sr-only">{{ t('programme.filters.search') }}</span>
        <input
          type="search"
          class="min-w-0 flex-1 bg-transparent text-[0.9375rem] text-poster-ink outline-none placeholder:text-poster-ink-muted"
          :value="props.modelValue.search"
          :placeholder="t('programme.filters.searchPlaceholder')"
          @input="patch({ search: ($event.target as HTMLInputElement).value })"
        >
      </label>

      <button
        type="button"
        class="inline-flex h-11 cursor-pointer items-center gap-2 rounded-md border-2 border-poster-ink px-3.5 text-sm font-bold"
        :class="open ? 'bg-poster-ink text-poster-on-ink-accent' : 'bg-poster-paper-raised text-poster-ink'"
        :aria-expanded="open"
        aria-controls="programmation-thematiques"
        @click="open = !open"
      >
        {{ t('programme.filters.themes') }}
        <span
          v-if="props.modelValue.themes.length"
          class="inline-flex h-5 min-w-5 items-center justify-center rounded-full bg-poster-today-strong px-1.5 font-poster-mono text-xs text-poster-on-today"
        >
          {{ props.modelValue.themes.length }}
        </span>
        <UiIcon :name="open ? 'chevron-up' : 'chevron-down'" size="0.875rem" />
      </button>

      <slot name="view" />

      <div
        v-if="open"
        id="programmation-thematiques"
        class="absolute top-full right-0 z-40 mt-2 flex w-full max-w-[36rem] flex-wrap gap-2 rounded-lg border-2 border-poster-ink bg-poster-paper-raised p-4 shadow-[6px_6px_0_var(--color-poster-ink)]"
      >
        <button
          v-for="theme in props.themes"
          :key="theme.code"
          type="button"
          class="inline-flex h-11 cursor-pointer items-center gap-2 rounded-full border-2 border-poster-ink pr-3.5 pl-2.5 text-sm font-semibold text-poster-ink"
          :style="props.modelValue.themes.includes(theme.code) ? { background: fill(theme.color) } : undefined"
          :aria-pressed="props.modelValue.themes.includes(theme.code)"
          @click="toggleTheme(theme.code)"
        >
          <span
            class="size-3.5 rounded-full border-2 border-poster-ink"
            :style="{ background: theme.color ?? 'var(--color-poster-paper-sunken)' }"
            aria-hidden="true"
          />
          {{ theme.label }}
          <span class="font-poster-mono text-xs text-poster-ink-muted">{{ theme.count }}</span>
        </button>
        <div class="mt-1 flex w-full items-center justify-between gap-3">
          <button
            type="button"
            class="h-11 cursor-pointer text-sm font-semibold text-poster-ink underline underline-offset-4"
            @click="patch({ themes: [] })"
          >
            {{ t('programme.filters.clear') }}
          </button>
          <button
            type="button"
            class="h-11 cursor-pointer rounded-md border-2 border-poster-ink bg-poster-ink px-4 text-sm font-bold text-poster-on-ink-accent"
            @click="open = false"
          >
            {{ t('programme.filters.show', props.resultCount) }}
          </button>
        </div>
      </div>
    </div>

    <div v-if="chips.length" class="flex flex-wrap items-center gap-1.5">
      <button
        v-for="chip in chips"
        :key="chip.key"
        type="button"
        class="inline-flex h-8 cursor-pointer items-center gap-1.5 rounded-full border-2 border-poster-ink bg-poster-paper-raised pr-2 pl-3 text-[0.8125rem] font-semibold text-poster-ink"
        :style="chip.background ? { background: chip.background } : undefined"
        :aria-label="t('programme.filters.remove', { filter: chip.label })"
        @click="chip.remove()"
      >
        {{ chip.label }}
        <UiIcon name="close" size="0.75rem" />
      </button>
      <button
        type="button"
        class="h-8 cursor-pointer px-1.5 text-[0.8125rem] font-semibold text-poster-ink underline underline-offset-4"
        @click="emit('update:modelValue', { themes: [], search: '' })"
      >
        {{ t('programme.filters.clear') }}
      </button>
    </div>
  </div>
</template>
