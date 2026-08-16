<script setup lang="ts">
/**
 * Case à cocher — y compris l'état INDÉTERMINÉ, celui d'une case « tout
 * sélectionner » quand une partie seulement des lignes est retenue.
 *
 * La case native est masquée mais toujours présente : c'est elle qui reçoit le
 * focus, répond à la barre d'espace et porte l'état pour les technologies
 * d'assistance. Le carré visible n'est qu'un décor piloté par `peer-*`. Un
 * `<div role="checkbox">` aurait exigé de réécrire tout cela à la main, et l'un
 * des trois aurait fini par manquer.
 *
 * La zone cliquable englobe le libellé et l'aide : viser un carré de 16 px au
 * doigt n'est pas raisonnable.
 */

interface Props {
  modelValue?: boolean
  /** Coché en partie — ni vrai, ni faux. Prime sur `modelValue` à l'affichage. */
  indeterminate?: boolean
  label?: string
  /** Précision d'une ligne sous le libellé. */
  hint?: string
  error?: string
  disabled?: boolean
  readonly?: boolean
  required?: boolean
  id?: string
  /** Valeur transmise quand la case appartient à un groupe. */
  value?: string
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>()

const { t } = useI18n()
const generatedId = useId()
const fieldId = computed(() => props.id ?? `check-${generatedId}`)
const hintId = computed(() => `${fieldId.value}-hint`)
const errorId = computed(() => `${fieldId.value}-error`)

const describedBy = computed(() => {
  const ids = [props.hint ? hintId.value : null, props.error ? errorId.value : null].filter(Boolean)
  return ids.length ? ids.join(' ') : undefined
})

const isLocked = computed(() => props.disabled || props.readonly)

function onChange(event: Event): void {
  if (props.readonly) {
    // Un champ en lecture seule reste focalisable et soumis : on annule le
    // basculement sans retirer la case du parcours de tabulation.
    ;(event.target as HTMLInputElement).checked = Boolean(props.modelValue)
    return
  }
  emit('update:modelValue', (event.target as HTMLInputElement).checked)
}
</script>

<template>
  <div>
    <div class="flex items-start gap-2.5">
      <span class="relative flex items-center">
        <input
          :id="fieldId"
          type="checkbox"
          class="peer size-4.5 shrink-0 cursor-pointer appearance-none rounded-sm border border-border-strong bg-surface-raised transition-colors
                 checked:border-accent-solid checked:bg-accent-solid
                 indeterminate:border-accent-solid indeterminate:bg-accent-solid
                 hover:border-text-subtle checked:hover:border-accent-solid-hover
                 disabled:cursor-not-allowed disabled:border-border disabled:bg-surface-sunken
                 disabled:checked:bg-border"
          :class="props.error ? 'border-danger' : ''"
          :checked="props.modelValue"
          :indeterminate="props.indeterminate"
          :disabled="props.disabled"
          :required="props.required"
          :value="props.value"
          :aria-describedby="describedBy"
          :aria-invalid="props.error ? true : undefined"
          :aria-checked="props.indeterminate ? 'mixed' : undefined"
          :aria-readonly="props.readonly ? 'true' : undefined"
          @change="onChange"
        >
        <!-- Coche et trait d'indétermination : deux FORMES distinctes, pas deux
             couleurs — l'état doit rester lisible en niveaux de gris. -->
        <UiIcon
          v-if="props.indeterminate"
          name="minus"
          class="pointer-events-none absolute left-0.5 text-accent-contrast peer-disabled:text-text-subtle"
          size="0.95rem"
          :stroke-width="2.6"
        />
        <UiIcon
          v-else
          name="check"
          class="pointer-events-none absolute left-0.5 hidden text-accent-contrast peer-checked:block peer-disabled:text-text-subtle"
          size="0.95rem"
          :stroke-width="2.6"
        />
      </span>

      <label
        v-if="props.label || $slots.default"
        :for="fieldId"
        class="text-sm leading-snug"
        :class="[
          isLocked ? 'cursor-default text-text-subtle' : 'cursor-pointer text-text',
          props.readonly && !props.disabled ? 'text-text-muted' : '',
        ]"
      >
        <slot>{{ props.label }}</slot>
        <span v-if="props.required" class="ml-0.5 text-danger" aria-hidden="true">*</span>
        <span v-if="props.required" class="sr-only"> — {{ t('form.required') }}</span>
        <span v-if="props.hint" :id="hintId" class="mt-0.5 block text-sm text-text-subtle">
          {{ props.hint }}
        </span>
      </label>
    </div>

    <p v-if="props.error" :id="errorId" role="alert" class="mt-1.5 pl-7 text-sm font-medium text-danger">
      <span class="sr-only">{{ t('form.errorPrefix') }} </span>{{ props.error }}
    </p>
  </div>
</template>
