<script setup lang="ts">
import type { SelectOption } from '~/types/ui'

/**
 * Groupe de boutons radio — un choix parmi peu, tous visibles.
 *
 * GROUPE ET NON BOUTON ISOLÉ : un radio seul n'existe pas, et c'est le groupe
 * qui porte la sémantique (`role="radiogroup"` et son libellé). Les composer un
 * par un laisserait chaque écran réinventer le `<fieldset>` — et l'oublier.
 *
 * QUAND LE PRÉFÉRER À UN `Select` : trois à cinq options qu'il est utile de
 * comparer d'un coup d'œil (mode de participation, visibilité d'un commentaire).
 * Au-delà, la liste déroulante fatigue moins.
 *
 * Le clavier est celui du natif : flèches pour parcourir le groupe, tabulation
 * pour en sortir. Rien n'est réimplémenté.
 */

interface Props {
  modelValue?: string | null
  options: SelectOption[]
  /** Libellé du groupe — rendu en `<legend>`. */
  label?: string
  hint?: string
  error?: string
  required?: boolean
  disabled?: boolean
  readonly?: boolean
  /** Nom du groupe ; généré si absent. */
  name?: string
  /** Options côte à côte plutôt qu'empilées. À réserver aux libellés courts. */
  inline?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const { t } = useI18n()
const generatedId = useId()
const groupName = computed(() => props.name ?? `radio-${generatedId}`)
const hintId = computed(() => `${groupName.value}-hint`)
const errorId = computed(() => `${groupName.value}-error`)

const describedBy = computed(() => {
  const ids = [props.hint ? hintId.value : null, props.error ? errorId.value : null].filter(Boolean)
  return ids.length ? ids.join(' ') : undefined
})

function select(option: SelectOption): void {
  if (props.disabled || props.readonly || option.disabled) return
  emit('update:modelValue', option.value)
}
</script>

<template>
  <fieldset
    :aria-describedby="describedBy"
    :aria-invalid="props.error ? true : undefined"
    :aria-required="props.required ? true : undefined"
    :disabled="props.disabled"
  >
    <legend v-if="props.label" class="mb-2 text-sm font-medium" :class="props.disabled ? 'text-text-subtle' : 'text-text'">
      {{ props.label }}
      <span v-if="props.required" class="ml-0.5 text-danger" aria-hidden="true">*</span>
      <span v-if="props.required" class="sr-only"> — {{ t('form.required') }}</span>
    </legend>

    <div :class="props.inline ? 'flex flex-wrap gap-x-6 gap-y-2.5' : 'space-y-2.5'">
      <div v-for="option in props.options" :key="option.value" class="flex items-start gap-2.5">
        <input
          :id="`${groupName}-${option.value}`"
          type="radio"
          :name="groupName"
          :value="option.value"
          :checked="props.modelValue === option.value"
          :disabled="props.disabled || option.disabled"
          class="peer mt-0.5 size-4.5 shrink-0 cursor-pointer appearance-none rounded-full border border-border-strong bg-surface-raised transition-colors
                 checked:border-6 checked:border-accent-solid
                 hover:border-text-subtle
                 disabled:cursor-not-allowed disabled:border-border disabled:bg-surface-sunken
                 disabled:checked:border-border"
          :class="props.error ? 'border-danger' : ''"
          :aria-readonly="props.readonly ? 'true' : undefined"
          @change="select(option)"
        >

        <label
          :for="`${groupName}-${option.value}`"
          class="text-sm leading-snug"
          :class="[
            props.disabled || option.disabled || props.readonly
              ? 'cursor-default text-text-subtle'
              : 'cursor-pointer text-text',
          ]"
        >
          {{ option.label }}
          <span v-if="option.description" class="mt-0.5 block text-sm text-text-subtle">
            {{ option.description }}
          </span>
        </label>
      </div>
    </div>

    <p v-if="props.hint" :id="hintId" class="mt-2 text-sm text-text-subtle">{{ props.hint }}</p>
    <p v-if="props.error" :id="errorId" role="alert" class="mt-2 text-sm font-medium text-danger">
      <span class="sr-only">{{ t('form.errorPrefix') }} </span>{{ props.error }}
    </p>
  </fieldset>
</template>
