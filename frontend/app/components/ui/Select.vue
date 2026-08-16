<script setup lang="ts">
import type { FieldProps, SelectOption, Size } from '~/types/ui'

/**
 * Liste déroulante — le `<select>` natif, habillé.
 *
 * POURQUOI LE NATIF plutôt qu'une liste reconstruite en JavaScript : sur mobile
 * il ouvre le sélecteur du système, il fonctionne sans script, il est navigable
 * au clavier sans une ligne de code, et il ne se retrouve jamais coupé par le
 * débordement d'un conteneur. Une liste reconstruite ne se justifie que là où
 * l'on cherche dans des centaines d'entrées — c'est le rôle de `UiSearchInput`,
 * pas celui-ci.
 *
 * `placeholder` produit une option vide et désactivée, sélectionnée tant que
 * rien n'est choisi : c'est le seul moyen qu'un `<select>` obligatoire montre
 * qu'il attend une réponse plutôt que d'afficher sa première option comme si
 * elle avait été choisie.
 */

interface Props extends FieldProps {
  modelValue?: string | null
  options: SelectOption[]
  /** Option vide affichée en tête tant que rien n'est choisi. */
  placeholder?: string
  size?: Size
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const classes = computed(() => [
  ...fieldControlClasses({
    hasError: Boolean(props.error),
    disabled: props.disabled,
    readonly: props.readonly,
    size: props.size,
    trailingIcon: true,
  }),
  // Le chevron natif est remplacé par celui du jeu d'icônes, pour que tous les
  // champs de la plateforme se ressemblent d'un navigateur à l'autre.
  'appearance-none',
])

/**
 * `readonly` n'existe pas sur un `<select>` : le navigateur l'ignore. On le rend
 * donc par un champ désactivé DOUBLÉ d'un champ caché qui porte la valeur — sans
 * quoi une liste en lecture seule ne serait pas soumise avec le formulaire.
 */
const isLocked = computed(() => props.disabled || props.readonly)
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
  >
    <template #default="{ control }">
      <div class="relative">
        <select
          :id="control.id"
          :value="props.modelValue ?? ''"
          :disabled="isLocked"
          :required="props.required"
          :aria-describedby="control['aria-describedby']"
          :aria-invalid="control['aria-invalid']"
          :class="classes"
          @change="emit('update:modelValue', ($event.target as HTMLSelectElement).value)"
        >
          <option v-if="props.placeholder" value="" disabled>{{ props.placeholder }}</option>
          <option
            v-for="option in props.options"
            :key="option.value"
            :value="option.value"
            :disabled="option.disabled"
          >
            {{ option.label }}{{ option.description ? ` — ${option.description}` : '' }}
          </option>
        </select>

        <input
          v-if="props.readonly && !props.disabled"
          type="hidden"
          :name="control.id"
          :value="props.modelValue ?? ''"
        >

        <UiIcon
          name="chevron-down"
          :class="[FIELD_ICON_CLASSES, 'right-3']"
          size="1.05em"
        />
      </div>
    </template>
  </UiFormField>
</template>
