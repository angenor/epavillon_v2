<script setup lang="ts">
/**
 * Interrupteur — bascule un réglage qui prend effet IMMÉDIATEMENT.
 *
 * NE PAS CONFONDRE AVEC UNE CASE À COCHER. Une case exprime un choix qui sera
 * envoyé avec le formulaire (« j'accepte les conditions ») ; un interrupteur
 * change un état sur-le-champ (« diffuser cette activité en direct »,
 * « recevoir les rappels »). Si un bouton « Enregistrer » suit, c'est une case
 * qu'il faut.
 *
 * L'état est porté par une case native masquée, comme pour `UiCheckbox` : focus,
 * barre d'espace et annonce vocale viennent du navigateur.
 *
 * `loading` couvre le cas réel de l'interrupteur qui déclenche un appel réseau :
 * l'interrupteur reste dans son ancienne position, non basculable, jusqu'à la
 * réponse. Le faire glisser tout de suite puis revenir en arrière sur erreur est
 * la pire des deux solutions.
 */

interface Props {
  modelValue?: boolean
  label?: string
  hint?: string
  disabled?: boolean
  loading?: boolean
  id?: string
  /** Libellé à gauche de l'interrupteur plutôt qu'à droite — lignes de réglages. */
  labelPosition?: 'start' | 'end'
}

const props = withDefaults(defineProps<Props>(), { labelPosition: 'end' })
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()

const { t } = useI18n()
const generatedId = useId()
const fieldId = computed(() => props.id ?? `switch-${generatedId}`)
const hintId = computed(() => `${fieldId.value}-hint`)
const isLocked = computed(() => props.disabled || props.loading)

function onChange(event: Event): void {
  if (isLocked.value) {
    ;(event.target as HTMLInputElement).checked = Boolean(props.modelValue)
    return
  }
  emit('update:modelValue', (event.target as HTMLInputElement).checked)
}
</script>

<template>
  <div
    class="flex items-start gap-3"
    :class="props.labelPosition === 'start' ? 'flex-row-reverse justify-end' : ''"
  >
    <span class="relative inline-flex shrink-0 items-center">
      <input
        :id="fieldId"
        type="checkbox"
        role="switch"
        class="peer h-6 w-11 cursor-pointer appearance-none rounded-full border border-border-strong bg-surface-sunken transition-colors
               checked:border-accent-solid checked:bg-accent-solid
               hover:border-text-subtle checked:hover:bg-accent-solid-hover
               disabled:cursor-not-allowed disabled:border-border disabled:bg-surface-sunken
               disabled:checked:bg-border"
        :checked="props.modelValue"
        :disabled="isLocked"
        :aria-describedby="props.hint ? hintId : undefined"
        :aria-busy="props.loading ? 'true' : undefined"
        @change="onChange"
      >
      <!-- Le curseur. `pointer-events-none` : c'est la case dessous qui reçoit
           le clic, sans quoi le glissement du curseur avalerait l'événement. -->
      <span
        class="pointer-events-none absolute left-0.5 flex size-5 items-center justify-center rounded-full bg-surface-raised shadow-xs transition-transform duration-150 peer-checked:translate-x-5"
        aria-hidden="true"
      >
        <UiSpinner v-if="props.loading" size="0.8rem" class="text-text-muted" />
      </span>
    </span>

    <label
      v-if="props.label || $slots.default"
      :for="fieldId"
      class="text-sm leading-snug"
      :class="isLocked ? 'cursor-not-allowed text-text-subtle' : 'cursor-pointer text-text'"
    >
      <slot>{{ props.label }}</slot>
      <span :id="hintId" v-if="props.hint" class="mt-0.5 block text-sm text-text-subtle">
        {{ props.hint }}
      </span>
      <!-- État annoncé aux lecteurs d'écran : `role="switch"` l'expose déjà,
           mais le doubler par un texte sert les infobulles de survol prolongé. -->
      <span class="sr-only">{{ props.modelValue ? t('form.switch.on') : t('form.switch.off') }}</span>
    </label>
  </div>
</template>
