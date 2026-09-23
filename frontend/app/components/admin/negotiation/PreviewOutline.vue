<script setup lang="ts">
import type { OutlineEntry } from '~/types/negotiation-documents'

interface Props {
  outline: OutlineEntry[]
  /** `index` → `label` : l'entrée renvoie à un index, on affiche le numéro imprimé. */
  labels: Map<number, string>
  currentIndex: number | null
}

const props = defineProps<Props>()
defineEmits<{ go: [index: number] }>()

const { t } = useI18n()

interface Row {
  key: string
  entry: OutlineEntry
}

const rows = computed<Row[]>(() => {
  const out: Row[] = []
  const walk = (entries: OutlineEntry[], path: string): void => {
    entries.forEach((entry, i) => {
      const key = `${path}${i}`
      out.push({ key, entry })
      walk(entry.children, `${key}.`)
    })
  }
  walk(props.outline, '')
  return out
})

/** La partie qui contient la page affichée : la dernière entrée ouverte avant elle. */
const currentKey = computed(() => {
  if (props.currentIndex === null) return null
  let found: string | null = null
  for (const row of rows.value) {
    if (row.entry.page_index <= props.currentIndex) found = row.key
  }
  return found
})

const INDENT = { 1: '', 2: 'ps-4', 3: 'ps-8' } as const
</script>

<template>
  <nav :aria-label="t('admin.negociations.documents.preview.outline.title')">
    <h2 class="text-sm font-semibold tracking-caps text-text-muted uppercase">
      {{ t('admin.negociations.documents.preview.outline.title') }}
    </h2>
    <p v-if="rows.length === 0" class="mt-2 text-sm text-text-muted">
      {{ t('admin.negociations.documents.preview.outline.empty') }}
    </p>
    <ol v-else class="mt-2 space-y-0.5 text-sm">
      <li v-for="row in rows" :key="row.key" :class="INDENT[row.entry.level]">
        <button
          type="button"
          class="flex min-h-(--target-min) w-full cursor-pointer items-baseline gap-3 rounded-md px-2 py-2 text-start hover:bg-surface-hover"
          :class="[
            row.key === currentKey ? 'bg-surface-selected text-text' : 'text-text-secondary',
            row.entry.level === 1 && 'font-semibold',
          ]"
          :aria-current="row.key === currentKey ? 'location' : undefined"
          @click="$emit('go', row.entry.page_index)"
        >
          <span class="min-w-0 grow">{{ row.entry.title }}</span>
          <span class="shrink-0 text-xs text-text-muted tabular-nums">
            {{
              t('admin.negociations.documents.preview.outline.page', {
                label: labels.get(row.entry.page_index) ?? row.entry.page_index,
              })
            }}
          </span>
        </button>
      </li>
    </ol>
  </nav>
</template>
