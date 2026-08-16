<script setup lang="ts">
import type { FieldProps, Size } from '~/types/ui'

/**
 * Sélecteur de date, et de date-heure.
 *
 * LE CONTRÔLE NATIF, ASSUMÉ. `<input type="date">` ouvre le calendrier du
 * système : localisé, navigable au clavier, utilisable au doigt, et sans une
 * ligne de JavaScript. Un calendrier reconstruit demanderait de réimplémenter la
 * navigation par flèches, la gestion des semaines, les fuseaux et le passage à
 * l'heure d'été — pour un résultat toujours en retard sur le natif.
 *
 * LE FUSEAU N'EST PAS DANS LE CONTRÔLE. `<input type="datetime-local">` ne
 * connaît pas les fuseaux : il rend « 2027-11-14T14:30 », une heure de mur, sans
 * décalage. Or la règle du projet veut que TOUTE date affichée porte son fuseau.
 * D'où `timezoneLabel`, affiché en suffixe du champ et rappelé dans l'aide
 * contextuelle : la personne qui saisit 14:30 doit savoir que c'est 14:30 à
 * Belém, pas chez elle. La conversion en instant `timestamptz` est l'affaire de
 * l'appelant, qui seul connaît le fuseau de l'édition.
 *
 * BORNES : `min` et `max` sont posées sur le contrôle natif, qui refuse alors la
 * saisie hors plage — utile pour contenir un créneau dans les jours de
 * l'édition. Ce n'est pas un contrôle de validité : celui-ci reste du ressort de
 * l'appelant et de la base.
 */

interface Props extends FieldProps {
  /** `AAAA-MM-JJ`, ou `AAAA-MM-JJTHH:MM` quand `withTime` est vrai. */
  modelValue?: string | null
  withTime?: boolean
  size?: Size
  /** Borne basse, au même format que la valeur. */
  min?: string
  /** Borne haute, au même format que la valeur. */
  max?: string
  /**
   * Libellé du fuseau de référence — « heure de Belém ». Sans lui, le champ
   * laisse croire à une heure locale. Le passer dès que la date porte une heure.
   */
  timezoneLabel?: string
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const { t } = useI18n()

/** L'aide de l'appelant l'emporte ; à défaut, le fuseau tient lieu de consigne. */
const effectiveHint = computed(() => {
  if (props.hint) return props.hint
  if (props.withTime && props.timezoneLabel) {
    return t('form.date.timezoneHint', { zone: props.timezoneLabel })
  }
  return undefined
})

const classes = computed(() =>
  fieldControlClasses({
    hasError: Boolean(props.error),
    disabled: props.disabled,
    readonly: props.readonly,
    size: props.size,
    leadingIcon: true,
  }),
)
</script>

<template>
  <UiFormField
    :id="props.id"
    :label="props.label"
    :hint="effectiveHint"
    :error="props.error"
    :required="props.required"
    :disabled="props.disabled"
    :readonly="props.readonly"
  >
    <template #default="{ control }">
      <div class="relative">
        <UiIcon
          :name="props.withTime ? 'clock' : 'calendar'"
          :class="[FIELD_ICON_CLASSES, 'left-3']"
          size="1.05em"
        />

        <input
          :id="control.id"
          :type="props.withTime ? 'datetime-local' : 'date'"
          :value="props.modelValue ?? ''"
          :min="props.min"
          :max="props.max"
          :disabled="control.disabled"
          :readonly="control.readonly"
          :required="props.required"
          :aria-describedby="control['aria-describedby']"
          :aria-invalid="control['aria-invalid']"
          :class="[classes, 'tabular-nums', props.timezoneLabel ? 'pr-28' : '']"
          @input="emit('update:modelValue', ($event.target as HTMLInputElement).value)"
        >

        <!-- Le fuseau EST une part de la valeur saisie : il reste visible dans le
             cadre du champ, pas seulement dans l'aide contextuelle. -->
        <span
          v-if="props.timezoneLabel"
          class="pointer-events-none absolute top-1/2 right-3 -translate-y-1/2 text-xs text-text-subtle"
        >
          {{ props.timezoneLabel }}
        </span>
      </div>
    </template>
  </UiFormField>
</template>
