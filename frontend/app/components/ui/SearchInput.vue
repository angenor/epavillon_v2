<script setup lang="ts">
import type { FieldProps, Size } from '~/types/ui'

/**
 * Champ de recherche — loupe, effacement, temporisation, annonce du nombre de
 * résultats.
 *
 * C'EST LE CONTRÔLE DU RATTACHEMENT À UNE ORGANISATION (écran A2), l'écran le
 * plus important du jalon pour la qualité des données. D'où trois exigences qui
 * ne sont pas des détails :
 *
 * · TEMPORISATION — la recherche part après une pause de frappe, jamais à chaque
 *   touche. Le référentiel des organisations se cherche sur toutes les
 *   dénominations à la fois ; une requête par caractère le mettrait à genoux.
 *   `@search` porte le terme temporisé, `@update:modelValue` la frappe brute.
 *
 * · SEUIL — en deçà de `minLength` (deux caractères, comme le prévoit
 *   `useApi().organizations.search()`), rien n'est envoyé.
 *
 * · ANNONCE — le nombre de résultats est annoncé en région vivante. Une liste
 *   qui se remplit sous un champ ne se voit pas quand on ne regarde pas l'écran.
 *
 * Le bouton d'effacement n'apparaît qu'une fois le champ rempli, et redonne le
 * focus au champ : effacer pour retaper est le geste le plus fréquent.
 */

interface Props extends FieldProps {
  modelValue?: string
  placeholder?: string
  size?: Size
  /** Millisecondes de silence avant l'émission de `search`. */
  debounce?: number
  /** Longueur minimale avant toute recherche. */
  minLength?: number
  /** Une requête est en cours : le tourniquet remplace la loupe. */
  loading?: boolean
  /** Nombre de résultats, annoncé en région vivante. `null` = rien à annoncer. */
  resultCount?: number | null
}

const props = withDefaults(defineProps<Props>(), {
  size: 'md',
  debounce: 300,
  minLength: 2,
  resultCount: null,
})

const emit = defineEmits<{
  'update:modelValue': [value: string]
  /** Terme temporisé, seuil franchi. Une chaîne vide signale un effacement. */
  search: [value: string]
  clear: []
}>()

const { t } = useI18n()
const input = ref<HTMLInputElement | null>(null)
let timer: ReturnType<typeof setTimeout> | undefined

function schedule(value: string): void {
  if (timer) clearTimeout(timer)
  timer = setTimeout(() => {
    emit('search', value.trim().length >= props.minLength ? value.trim() : '')
  }, props.debounce)
}

function onInput(event: Event): void {
  const value = (event.target as HTMLInputElement).value
  emit('update:modelValue', value)
  schedule(value)
}

function clear(): void {
  if (timer) clearTimeout(timer)
  emit('update:modelValue', '')
  emit('search', '')
  emit('clear')
  input.value?.focus()
}

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})

const hasValue = computed(() => (props.modelValue ?? '').length > 0)

const classes = computed(() =>
  fieldControlClasses({
    hasError: Boolean(props.error),
    disabled: props.disabled,
    readonly: props.readonly,
    size: props.size,
    leadingIcon: true,
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
    :hide-label="props.hideLabel"
  >
    <template #default="{ control }">
      <div class="relative">
        <UiSpinner v-if="props.loading" :class="[FIELD_ICON_CLASSES, 'left-3']" size="1.05em" />
        <UiIcon v-else name="search" :class="[FIELD_ICON_CLASSES, 'left-3']" size="1.05em" />

        <!-- `type="search"` sans la croix native : celle du navigateur ne suit ni
             le thème ni le clavier, et n'émet aucun événement exploitable. -->
        <input
          :id="control.id"
          ref="input"
          type="search"
          :value="props.modelValue ?? ''"
          :placeholder="props.placeholder ?? t('common.search.placeholder')"
          :disabled="control.disabled"
          :readonly="control.readonly"
          :aria-describedby="control['aria-describedby']"
          :aria-invalid="control['aria-invalid']"
          autocomplete="off"
          spellcheck="false"
          :class="[classes, 'ui-search']"
          @input="onInput"
          @keydown.esc="hasValue && clear()"
        >

        <button
          v-if="hasValue && !props.disabled && !props.readonly"
          type="button"
          class="absolute top-1/2 right-2 -translate-y-1/2 rounded-sm p-1 text-text-subtle transition-colors hover:text-text"
          @click="clear"
        >
          <span class="sr-only">{{ t('common.search.clear') }}</span>
          <UiIcon name="close" size="1em" />
        </button>
      </div>

      <!-- Région vivante : le nombre de résultats est annoncé sans voler le focus. -->
      <p aria-live="polite" class="sr-only">
        {{ props.resultCount === null ? '' : t('common.pagination.results', props.resultCount) }}
      </p>
    </template>
  </UiFormField>
</template>

<style scoped>
/* La croix native de `type="search"` ferait doublon avec la nôtre. */
.ui-search::-webkit-search-cancel-button,
.ui-search::-webkit-search-decoration {
  -webkit-appearance: none;
  appearance: none;
}
</style>
