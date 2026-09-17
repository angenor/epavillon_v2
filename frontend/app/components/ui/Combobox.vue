<script setup lang="ts">
import type { FieldProps, SelectOption, Size } from '~/types/ui'

/**
 * Liste déroulante avec saisie — on tape, la liste se resserre.
 *
 * Le pendant de `UiSelect` quand les options se comptent par centaines (les
 * fuseaux horaires) : le `<select>` natif oblige alors à faire défiler une liste
 * qu'on ne sait pas lire. La recherche ignore accents, casse et ponctuation, et
 * porte sur le libellé, la précision et la valeur : « sao paulo » trouve
 * `America/Sao_Paulo`.
 *
 * La saisie n'est qu'un filtre : seule une option retenue devient la valeur.
 * Quitter le champ sans choisir rend le libellé de l'option en place.
 */

interface Props extends FieldProps {
  modelValue?: string | null
  options: SelectOption[]
  placeholder?: string
  size?: Size
  hideOptional?: boolean
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const { t } = useI18n()
const listId = useId()
const input = ref<HTMLInputElement | null>(null)
const list = ref<HTMLUListElement | null>(null)

const open = ref(false)
const query = ref('')
const highlighted = ref(-1)

const selected = computed(() => props.options.find((option) => option.value === props.modelValue))
const isLocked = computed(() => Boolean(props.disabled || props.readonly))

const indexed = computed(() =>
  props.options.map((option) => ({
    option,
    label: normalizeLabelLike(option.label),
    haystack: normalizeLabelLike(`${option.label} ${option.description ?? ''} ${option.value}`),
  })),
)

/** Les libellés qui COMMENCENT par la saisie passent devant : c'est la ville qu'on cherche. */
const filtered = computed<SelectOption[]>(() => {
  const needle = normalizeLabelLike(query.value)
  if (!needle) return props.options
  const words = needle.split(' ')
  const matches = indexed.value.filter((entry) => words.every((word) => entry.haystack.includes(word)))
  return [
    ...matches.filter((entry) => entry.label.startsWith(needle)),
    ...matches.filter((entry) => !entry.label.startsWith(needle)),
  ].map((entry) => entry.option)
})

const displayValue = computed(() => (open.value ? query.value : (selected.value?.label ?? '')))

function optionId(index: number): string {
  return `${listId}-option-${index}`
}

function scrollToHighlighted(): void {
  nextTick(() => {
    list.value?.querySelector(`#${CSS.escape(optionId(highlighted.value))}`)?.scrollIntoView({ block: 'nearest' })
  })
}

function openList(): void {
  if (isLocked.value || open.value) return
  query.value = ''
  open.value = true
  highlighted.value = props.options.findIndex((option) => option.value === props.modelValue)
  scrollToHighlighted()
}

function closeList(): void {
  open.value = false
  query.value = ''
  highlighted.value = -1
}

function choose(option: SelectOption): void {
  if (option.disabled) return
  emit('update:modelValue', option.value)
  closeList()
}

function onInput(event: Event): void {
  if (!open.value) openList()
  query.value = (event.target as HTMLInputElement).value
  highlighted.value = filtered.value.length > 0 ? 0 : -1
  if (list.value) list.value.scrollTop = 0
}

function move(step: number): void {
  if (!open.value) {
    openList()
    return
  }
  const total = filtered.value.length
  if (total === 0) return
  highlighted.value = (highlighted.value + step + total) % total
  scrollToHighlighted()
}

function onKeydown(event: KeyboardEvent): void {
  if (isLocked.value) return
  switch (event.key) {
    case 'ArrowDown':
      event.preventDefault()
      move(1)
      break
    case 'ArrowUp':
      event.preventDefault()
      move(-1)
      break
    case 'Enter': {
      if (!open.value) return
      // Sans ce blocage, Entrée soumettrait le formulaire au lieu de retenir l'option.
      event.preventDefault()
      const option = filtered.value[highlighted.value]
      if (option) choose(option)
      break
    }
    case 'Escape':
      if (open.value) {
        event.preventDefault()
        closeList()
      }
      break
  }
}

function toggle(): void {
  if (open.value) {
    closeList()
    return
  }
  input.value?.focus()
  openList()
}

const classes = computed(() =>
  fieldControlClasses({
    hasError: Boolean(props.error),
    disabled: props.disabled,
    readonly: props.readonly,
    size: props.size,
    trailingIcon: true,
  }),
)
</script>

<template>
  <UiFormField
    :id="props.id"
    :label="props.label"
    :hint="props.hint"
    :error="props.error"
    :required="props.required"
    :disabled="props.disabled"
    :readonly="props.readonly"
    :hide-optional="props.hideOptional"
    :hide-label="props.hideLabel"
  >
    <template #default="{ control }">
      <div class="relative">
        <input
          :id="control.id"
          ref="input"
          type="text"
          role="combobox"
          aria-autocomplete="list"
          :aria-expanded="open"
          :aria-controls="listId"
          :aria-activedescendant="open && highlighted >= 0 ? optionId(highlighted) : undefined"
          :aria-describedby="control['aria-describedby']"
          :aria-invalid="control['aria-invalid']"
          :aria-required="control['aria-required']"
          :value="displayValue"
          :placeholder="open ? (selected?.label ?? t('common.search.placeholder')) : props.placeholder"
          :disabled="props.disabled"
          :readonly="props.readonly"
          autocomplete="off"
          spellcheck="false"
          :class="classes"
          @focus="openList"
          @click="openList"
          @input="onInput"
          @keydown="onKeydown"
          @blur="closeList"
        >

        <button
          v-if="!isLocked"
          type="button"
          tabindex="-1"
          class="absolute top-1/2 right-1 flex size-9 -translate-y-1/2 cursor-pointer items-center justify-center rounded-sm text-text-subtle hover:text-text"
          :aria-label="t('form.combobox.toggle')"
          @mousedown.prevent="toggle"
        >
          <UiIcon
            name="chevron-down"
            size="1.05em"
            class="transition-transform duration-(--duration-fast)"
            :class="open ? 'rotate-180' : ''"
          />
        </button>
        <UiIcon v-else name="chevron-down" :class="[FIELD_ICON_CLASSES, 'right-3']" size="1.05em" />

        <div
          v-show="open"
          class="absolute z-20 mt-1 w-full overflow-hidden rounded-md border border-border-strong bg-surface-raised shadow-lg"
        >
          <ul :id="listId" ref="list" role="listbox" class="max-h-72 overflow-y-auto py-1">
            <li
              v-for="(option, index) in filtered"
              :id="optionId(index)"
              :key="option.value"
              role="option"
              :aria-selected="option.value === props.modelValue"
              :aria-disabled="option.disabled || undefined"
              class="flex min-h-(--target-min) cursor-pointer items-center gap-3 px-3 py-2 text-sm"
              :class="[
                index === highlighted ? 'bg-accent-surface' : '',
                option.disabled ? 'cursor-not-allowed text-text-subtle' : 'text-text',
              ]"
              @mousedown.prevent="choose(option)"
              @mouseenter="highlighted = index"
            >
              <span class="flex min-w-0 flex-1 flex-col">
                <span class="truncate" :class="option.value === props.modelValue ? 'font-bold' : ''">
                  {{ option.label }}
                </span>
                <span v-if="option.description" class="truncate text-xs text-text-muted">
                  {{ option.description }}
                </span>
              </span>
              <UiIcon
                v-if="option.value === props.modelValue"
                name="check"
                size="1rem"
                class="shrink-0 text-accent"
              />
            </li>
          </ul>
          <p v-if="filtered.length === 0" class="px-3 py-3 text-sm text-text-muted">
            {{ t('common.search.noResults', { query }) }}
          </p>
        </div>
      </div>

      <p aria-live="polite" class="sr-only">
        {{ open && query ? t('common.pagination.results', filtered.length) : '' }}
      </p>
    </template>
  </UiFormField>
</template>
