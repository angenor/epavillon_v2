<script setup lang="ts">
import type { Block, NoteBlock } from '~/types/negotiation-documents'

const props = defineProps<{ blocks: Block[] }>()
defineEmits<{ 'show-origin': [] }>()

const { t } = useI18n()

/** Le contrat rend les notes en fin de page, où qu'elles soient dans le flux. */
const flow = computed(() => props.blocks.filter((block) => block.kind !== 'note'))
const notes = computed(() => props.blocks.filter((block): block is NoteBlock => block.kind === 'note'))

const HEADING_TAG = { 1: 'h3', 2: 'h4', 3: 'h5' } as const
const HEADING_CLASS = {
  1: 'text-2xl font-semibold',
  2: 'text-xl font-semibold',
  3: 'text-lg font-semibold text-text-secondary',
} as const
const INDENT = { 0: '', 1: 'ms-6', 2: 'ms-12' } as const
</script>

<template>
  <div class="space-y-3 leading-relaxed">
    <template v-for="(block, index) in flow" :key="index">
      <component
        :is="HEADING_TAG[block.level]"
        v-if="block.kind === 'heading'"
        class="pt-2 leading-snug text-heading"
        :class="HEADING_CLASS[block.level]"
      >
        <AdminNegotiationPreviewSpans :spans="block.spans" />
      </component>

      <p v-else-if="block.kind === 'paragraph'">
        <AdminNegotiationPreviewSpans :spans="block.spans" />
      </p>

      <p v-else-if="block.kind === 'list_item'" class="flex gap-2" :class="INDENT[block.depth]">
        <span class="shrink-0 text-text-muted tabular-nums">{{ block.marker }}</span>
        <span><AdminNegotiationPreviewSpans :spans="block.spans" /></span>
      </p>

      <div
        v-else-if="block.kind === 'origin'"
        class="rounded-md border border-dashed border-info-border bg-info-surface p-3 text-sm"
      >
        <div class="flex flex-wrap items-center justify-between gap-2">
          <p class="flex items-center gap-2 font-semibold text-info">
            <UiIcon :name="block.reason === 'table' ? 'grid' : 'image'" />
            {{ t(`admin.negociations.documents.preview.origin.${block.reason}`) }}
            <span class="font-normal text-text-muted">
              {{ t('admin.negociations.documents.preview.origin.hint') }}
            </span>
          </p>
          <UiButton variant="secondary" icon="eye" @click="$emit('show-origin')">
            {{ t('admin.negociations.documents.preview.origin.see') }}
          </UiButton>
        </div>
        <p v-if="block.caption?.length" class="mt-2 italic">
          <AdminNegotiationPreviewSpans :spans="block.caption" />
        </p>
        <details v-if="block.text?.length" class="mt-2">
          <summary class="min-h-(--target-min) cursor-pointer content-center text-text-secondary">
            {{ t('admin.negociations.documents.preview.origin.text') }}
          </summary>
          <p class="mt-1 text-text-secondary">
            <AdminNegotiationPreviewSpans :spans="block.text" />
          </p>
        </details>
      </div>
    </template>

    <footer v-if="notes.length" class="mt-6 border-t border-border pt-3 text-sm text-text-secondary">
      <h3 class="sr-only">{{ t('admin.negociations.documents.preview.reading.notes') }}</h3>
      <ol class="space-y-1.5">
        <li v-for="(note, index) in notes" :key="index" class="flex gap-2">
          <span class="shrink-0 font-semibold text-text tabular-nums">{{ note.mark }}</span>
          <span><AdminNegotiationPreviewSpans :spans="note.spans" /></span>
        </li>
      </ol>
    </footer>
  </div>
</template>
