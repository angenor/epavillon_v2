<script setup lang="ts">
import type { FieldProps, Size } from '~/types/ui'

/**
 * Champ de saisie sur une ligne.
 *
 * SEPT ÉTATS, tous rendus : vide, rempli, focus, erreur avec message, aide
 * contextuelle, désactivé, lecture seule. L'enveloppe `UiFormField` porte le
 * libellé et les textes ; ce composant ne s'occupe que du contrôle.
 *
 * PRÉFIXE ET SUFFIXE : le créneau `prefix` sert aux unités et aux symboles qui
 * font partie de la saisie (« https:// », « € »), le créneau `suffix` aux
 * actions (effacer, révéler un mot de passe). Ils vivent DANS le cadre du champ,
 * pas à côté : un cadre par champ, c'est ce qui rend un formulaire lisible en
 * diagonale.
 */

interface Props extends FieldProps {
  modelValue?: string | number | null
  type?: 'text' | 'email' | 'url' | 'tel' | 'password' | 'number' | 'search'
  placeholder?: string
  size?: Size
  autocomplete?: string
  inputmode?: 'text' | 'email' | 'tel' | 'url' | 'numeric' | 'decimal' | 'search'
  maxlength?: number
  min?: number | string
  max?: number | string
  step?: number | string
  /** Icône affichée à gauche, à l'intérieur du cadre. */
  icon?: string
  /** Compte les caractères saisis face à `maxlength`. */
  showCounter?: boolean
}

const props = withDefaults(defineProps<Props>(), { type: 'text', size: 'md' })
const emit = defineEmits<{
  'update:modelValue': [value: string]
  blur: [event: FocusEvent]
  focus: [event: FocusEvent]
}>()

const { t } = useI18n()
const slots = useSlots()

const length = computed(() => String(props.modelValue ?? '').length)
const counter = computed(() => {
  if (!props.showCounter || !props.maxlength) return undefined
  return t('form.counter', { count: length.value, max: props.maxlength })
})

const classes = computed(() =>
  fieldControlClasses({
    hasError: Boolean(props.error),
    disabled: props.disabled,
    readonly: props.readonly,
    size: props.size,
    leadingIcon: Boolean(props.icon),
    trailingIcon: Boolean(slots.suffix),
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
    :hide-label="props.hideLabel"
    :counter="counter"
    :counter-exceeded="Boolean(props.maxlength && length > props.maxlength)"
  >
    <template #default="{ control }">
      <div class="relative">
        <UiIcon
          v-if="props.icon"
          :name="props.icon"
          :class="[FIELD_ICON_CLASSES, 'left-3']"
          size="1.05em"
        />

        <input
          :id="control.id"
          :value="props.modelValue ?? ''"
          :type="props.type"
          :placeholder="props.placeholder"
          :autocomplete="props.autocomplete"
          :inputmode="props.inputmode"
          :maxlength="props.maxlength"
          :min="props.min"
          :max="props.max"
          :step="props.step"
          :disabled="control.disabled"
          :readonly="control.readonly"
          :required="props.required"
          :aria-describedby="control['aria-describedby']"
          :aria-invalid="control['aria-invalid']"
          :class="[classes, props.type === 'number' ? 'tabular-nums' : '']"
          @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
          @blur="emit('blur', $event)"
          @focus="emit('focus', $event)"
        >

        <div v-if="slots.suffix" class="absolute top-1/2 right-2 -translate-y-1/2">
          <slot name="suffix" />
        </div>
      </div>
    </template>
  </UiFormField>
</template>
