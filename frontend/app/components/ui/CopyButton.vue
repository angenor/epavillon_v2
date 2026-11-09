<script setup lang="ts">
import type { ButtonVariant, Size } from '~/types/ui'

/**
 * Copier un texte (un lien, le plus souvent) d'un clic. Sans presse-papiers
 * (contexte non sécurisé, refus du navigateur), le champ `selectTarget` est
 * sélectionné pour une copie au clavier, et on le dit.
 */

interface Props {
  text: string
  label: string
  labelCopied?: string
  /** Identifiant du champ à sélectionner si la copie échoue. */
  selectTarget?: string
  variant?: ButtonVariant
  size?: Size
  block?: boolean
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), { variant: 'secondary', size: 'md' })

const { t } = useI18n()

const state = ref<'idle' | 'copied' | 'failed'>('idle')
let timer: ReturnType<typeof setTimeout> | null = null

async function copy(): Promise<void> {
  if (timer) clearTimeout(timer)
  try {
    await navigator.clipboard.writeText(props.text)
    state.value = 'copied'
  } catch {
    const field = props.selectTarget ? document.getElementById(props.selectTarget) : null
    if (field instanceof HTMLInputElement || field instanceof HTMLTextAreaElement) {
      field.focus()
      field.select()
    }
    state.value = 'failed'
  }
  timer = setTimeout(() => (state.value = 'idle'), state.value === 'copied' ? 2500 : 6000)
}

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})
</script>

<template>
  <span :class="props.block ? 'flex flex-col' : 'inline-flex'">
    <UiButton
      :variant="props.variant"
      :size="props.size"
      :block="props.block"
      :disabled="props.disabled || !props.text"
      :icon="state === 'copied' ? 'check' : 'copy'"
      @click="copy"
    >
      {{ state === 'copied' ? (props.labelCopied ?? t('copy-button.copied')) : props.label }}
    </UiButton>
    <span class="sr-only" aria-live="polite">
      {{ state === 'copied' ? (props.labelCopied ?? t('copy-button.copied')) : state === 'failed' ? t('copy-button.failed') : '' }}
    </span>
  </span>
</template>
