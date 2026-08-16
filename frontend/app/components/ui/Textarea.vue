<script setup lang="ts">
import type { FieldProps } from '~/types/ui'

/**
 * Zone de texte multiligne — objectifs d'une activité, présentation détaillée,
 * motif d'une décision.
 *
 * COMPTEUR DE CARACTÈRES : affiché dès qu'une limite existe, et pas seulement
 * quand elle est dépassée. Rédiger 900 signes pour en voir 400 refusés à
 * l'envoi est le meilleur moyen de perdre un dossier.
 *
 * La saisie n'est jamais BLOQUÉE à la limite : le texte au-delà reste visible et
 * le compteur passe en rouge. Couper la frappe à mi-mot fait croire à un clavier
 * cassé — c'est le formulaire qui refuse à l'envoi, avec un message.
 *
 * `autoGrow` fait suivre la hauteur au contenu, dans la limite de `maxRows` :
 * au-delà, la zone défile, sinon un texte long pousse le bouton d'envoi hors de
 * l'écran.
 */

interface Props extends FieldProps {
  modelValue?: string | null
  placeholder?: string
  rows?: number
  maxlength?: number
  /** Compteur de caractères. Affiché d'office quand `maxlength` est posé. */
  showCounter?: boolean
  autoGrow?: boolean
  maxRows?: number
}

const props = withDefaults(defineProps<Props>(), { rows: 4, maxRows: 16 })
const emit = defineEmits<{
  'update:modelValue': [value: string]
  blur: [event: FocusEvent]
}>()

const { t } = useI18n()
const textarea = ref<HTMLTextAreaElement | null>(null)

const length = computed(() => String(props.modelValue ?? '').length)
const exceeded = computed(() => Boolean(props.maxlength && length.value > props.maxlength))

const counter = computed(() => {
  // Le compteur suit la limite : présent dès qu'elle existe, sauf refus explicite.
  if (!props.maxlength || props.showCounter === false) return undefined
  return exceeded.value
    ? t('form.counterExceeded', {
        count: length.value,
        max: props.maxlength,
        over: length.value - props.maxlength,
      })
    : t('form.counter', { count: length.value, max: props.maxlength })
})

const classes = computed(() =>
  fieldControlClasses({
    hasError: Boolean(props.error) || exceeded.value,
    disabled: props.disabled,
    readonly: props.readonly,
  }).map((entry) =>
    // La hauteur d'un `textarea` vient de `rows`, pas d'une classe de hauteur.
    entry.startsWith('h-') ? '' : entry,
  ),
)

/** Ajuste la hauteur au contenu, plafonnée à `maxRows` lignes. */
function grow(): void {
  const element = textarea.value
  if (!element || !props.autoGrow) return
  element.style.height = 'auto'
  const lineHeight = Number.parseFloat(getComputedStyle(element).lineHeight) || 24
  const maxHeight = lineHeight * props.maxRows
  element.style.height = `${Math.min(element.scrollHeight, maxHeight)}px`
  element.style.overflowY = element.scrollHeight > maxHeight ? 'auto' : 'hidden'
}

function onInput(event: Event): void {
  emit('update:modelValue', (event.target as HTMLTextAreaElement).value)
  nextTick(grow)
}

onMounted(grow)
watch(() => props.modelValue, () => nextTick(grow))
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
    :counter="counter"
    :counter-exceeded="exceeded"
  >
    <template #default="{ control }">
      <textarea
        :id="control.id"
        ref="textarea"
        :value="props.modelValue ?? ''"
        :placeholder="props.placeholder"
        :rows="props.rows"
        :disabled="control.disabled"
        :readonly="control.readonly"
        :required="props.required"
        :aria-describedby="control['aria-describedby']"
        :aria-invalid="control['aria-invalid']"
        :class="[classes, 'py-2 leading-normal', props.autoGrow ? 'resize-none' : 'resize-y']"
        @input="onInput"
        @blur="emit('blur', $event)"
      />
    </template>
  </UiFormField>
</template>
