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
 *
 * LE RADIO COCHÉ EST UN APLAT ACCENT AVEC PASTILLE CLAIRE, pas un anneau épais.
 * L'anneau — un trait de 6 px qui mange le centre du cercle — donne un contrôle
 * dont l'état ne se lit qu'à la couleur, et qui se confond de loin avec un
 * bouton désactivé. L'aplat plus pastille reprend la forme du bouton radio que
 * tout le monde connaît, et reste lisible en niveaux de gris.
 *
 * CIBLE DE 44 px, CONTRÔLE DE 20, TRAIT DE 2 : mêmes valeurs que `UiCheckbox`.
 * Deux contrôles voisins dans un même formulaire qui ne s'alignent pas au pixel
 * près donnent l'impression d'un formulaire monté à la va-vite.
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
    <!-- Même graisse que le libellé d'un champ : le titre du groupe EST le
         libellé de la question posée, il ne pèse pas moins. -->
    <legend
      v-if="props.label"
      class="mb-1 max-w-(--measure) text-sm font-bold"
      :class="props.disabled ? 'text-text-subtle' : 'text-text'"
    >
      {{ props.label }}
      <span v-if="props.required" class="ml-0.5 text-danger" aria-hidden="true">*</span>
      <span v-if="props.required" class="sr-only"> — {{ t('form.required') }}</span>
    </legend>

    <div :class="props.inline ? 'flex flex-wrap gap-x-6' : ''">
      <div
        v-for="option in props.options"
        :key="option.value"
        class="flex min-h-(--target-min) max-w-(--measure) items-start gap-3 py-2"
      >
        <!-- Comme pour la case à cocher, l'opacité du désactivé est portée par
             l'enveloppe : le cercle et sa pastille s'éteignent d'un seul geste. -->
        <span
          class="relative mt-0.5 flex size-5 shrink-0 items-center"
          :class="props.disabled || option.disabled ? 'opacity-[.45]' : ''"
        >
          <input
            :id="`${groupName}-${option.value}`"
            type="radio"
            :name="groupName"
            :value="option.value"
            :checked="props.modelValue === option.value"
            :disabled="props.disabled || option.disabled"
            class="peer size-5 shrink-0 cursor-pointer appearance-none rounded-full border-(length:--border-medium) border-solid bg-surface-raised transition-colors duration-(--duration-fast)
                   checked:border-accent-solid checked:bg-accent-solid
                   hover:border-accent
                   disabled:cursor-not-allowed"
            :class="props.error ? 'border-danger' : 'border-border-strong'"
            :aria-readonly="props.readonly ? 'true' : undefined"
            @change="select(option)"
          >
          <!-- La pastille intérieure : 8 px de clair au centre de l'aplat. -->
          <span
            class="pointer-events-none absolute inset-0 grid place-items-center opacity-0 peer-checked:opacity-100"
          >
            <span class="size-2 rounded-full bg-accent-contrast" />
          </span>
        </span>

        <label
          :for="`${groupName}-${option.value}`"
          class="text-sm leading-snug"
          :class="[
            props.disabled || option.disabled || props.readonly
              ? 'cursor-default text-text-muted'
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

    <p v-if="props.hint" :id="hintId" class="mt-2 max-w-(--measure) text-sm text-text-muted">
      {{ props.hint }}
    </p>
    <p
      v-if="props.error"
      :id="errorId"
      role="alert"
      class="mt-2 max-w-(--measure) text-sm font-bold text-danger"
    >
      <span class="sr-only">{{ t('form.errorPrefix') }} </span>{{ props.error }}
    </p>
  </fieldset>
</template>
