<script setup lang="ts">
import type { CommentPerson } from '~/types/programme/proposal'
import type { MentionQuery } from '~/utils/comment-mentions'

/**
 * La zone de saisie d'un message, avec mentions : « @ » suivi d'un nom propose
 * les membres du comité, et la personne retenue rejoint les destinataires.
 * Au clavier : flèches pour parcourir, Entrée ou Tab pour retenir, Échap pour
 * continuer d'écrire sans mentionner.
 */

interface Props {
  modelValue: string
  people: CommentPerson[]
  /** Les mentions n'existent que pour un message du comité qui ouvre un fil. */
  enabled: boolean
  label: string
  placeholder?: string
  hint?: string
  error?: string
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:modelValue': [value: string]
  mention: [person: CommentPerson]
}>()

const { t } = useI18n()
const listId = useId()
const field = ref<{ element: HTMLTextAreaElement | null } | null>(null)

const mention = ref<MentionQuery | null>(null)
const highlighted = ref(0)
const dismissedAt = ref<number | null>(null)

const candidates = computed(() =>
  props.enabled && mention.value ? matchMentionCandidates(props.people, mention.value.query) : [],
)
const open = computed(
  () => candidates.value.length > 0 && mention.value !== null && dismissedAt.value !== mention.value.start,
)

const optionId = (index: number): string => `${listId}-option-${index}`

function onCaret(value: string, position: number): void {
  const next = props.enabled ? activeMention(value, position) : null
  if (next?.start !== mention.value?.start) {
    highlighted.value = 0
    dismissedAt.value = null
  }
  mention.value = next
}

function choose(person: CommentPerson): void {
  const current = mention.value
  if (!current) return
  const result = insertMention(props.modelValue, current, person.display_name)
  emit('update:modelValue', result.text)
  emit('mention', person)
  mention.value = null
  nextTick(() => {
    const element = field.value?.element
    element?.focus()
    element?.setSelectionRange(result.caret, result.caret)
  })
}

function onKeydown(event: KeyboardEvent): void {
  if (!open.value) return
  const total = candidates.value.length
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      highlighted.value = (highlighted.value + 1) % total
      break
    case 'ArrowUp':
      event.preventDefault()
      highlighted.value = (highlighted.value - 1 + total) % total
      break
    case 'Enter':
    case 'Tab': {
      const person = candidates.value[highlighted.value]
      if (!person) return
      event.preventDefault()
      choose(person)
      break
    }
    case 'Escape':
      // Sans cet arrêt, Échap fermerait aussi une fenêtre qui contiendrait le champ.
      event.preventDefault()
      event.stopPropagation()
      dismissedAt.value = mention.value?.start ?? null
      break
  }
}

function close(): void {
  mention.value = null
}

const controlAttrs = computed(() =>
  props.enabled
    ? {
        role: 'combobox',
        'aria-autocomplete': 'list',
        'aria-haspopup': 'listbox',
        'aria-expanded': open.value,
        'aria-controls': listId,
        'aria-activedescendant': open.value ? optionId(highlighted.value) : undefined,
      }
    : {},
)

watch(
  () => props.enabled,
  (enabled) => {
    if (!enabled) close()
  },
)
</script>

<template>
  <div class="relative">
    <UiTextarea
      ref="field"
      :model-value="props.modelValue"
      :label="props.label"
      :placeholder="props.placeholder"
      :hint="props.hint"
      :error="props.error"
      :rows="3"
      auto-grow
      block
      required
      :disabled="props.disabled"
      :control-attrs="controlAttrs"
      @update:model-value="(value: string) => emit('update:modelValue', value)"
      @caret="onCaret"
      @keydown="onKeydown"
      @blur="close"
    />

    <ul
      v-show="open"
      :id="listId"
      role="listbox"
      :aria-label="t('admin.proposal.review.comments.mention.listLabel')"
      class="absolute z-20 mt-1 max-h-72 w-full max-w-sm overflow-y-auto rounded-md border border-border-strong bg-surface-raised py-1 shadow-lg"
    >
      <li
        v-for="(person, index) in candidates"
        :id="optionId(index)"
        :key="person.person_id"
        role="option"
        :aria-selected="index === highlighted"
        class="flex min-h-(--target-min) cursor-pointer items-center px-3 py-2 text-sm text-text"
        :class="index === highlighted ? 'bg-accent-surface' : ''"
        @mousedown.prevent="choose(person)"
        @mouseenter="highlighted = index"
      >
        {{ person.display_name }}
      </li>
    </ul>

    <p aria-live="polite" class="sr-only">
      {{ open ? t('admin.proposal.review.comments.mention.suggestions', candidates.length) : '' }}
    </p>
  </div>
</template>
