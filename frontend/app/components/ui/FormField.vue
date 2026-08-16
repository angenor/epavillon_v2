<script setup lang="ts">
/**
 * Enveloppe commune à TOUS les champs : libellé, marque d'obligation, aide
 * contextuelle, message d'erreur.
 *
 * POURQUOI UNE ENVELOPPE PLUTÔT QUE SEPT FOIS LE MÊME BALISAGE. Le lien entre un
 * champ et ses textes se fait par `aria-describedby`, et il se casse en silence :
 * rien ne prévient qu'une aide contextuelle n'est plus annoncée. Le composer une
 * fois ici, c'est la seule façon de le tenir sur tous les champs.
 *
 * ORDRE DE LECTURE : l'aide vient AVANT l'erreur dans `aria-describedby`. Un
 * lecteur d'écran annonce donc « format attendu… » puis « la date est
 * antérieure à l'ouverture de l'appel » : la consigne d'abord, le reproche
 * ensuite.
 *
 * ÉTAT D'ERREUR : c'est la seule présence de `error` qui le déclenche. Pas de
 * booléen `invalid` en parallèle — deux sources pour un même état finissent
 * toujours par diverger.
 *
 * `readonly` n'est PAS `disabled` : un champ en lecture seule reste focalisable,
 * copiable et soumis avec le formulaire ; un champ désactivé sort du parcours de
 * tabulation et sa valeur ne part pas. Les confondre, c'est empêcher de copier
 * un numéro de dossier.
 */

interface Props {
  /** Identifiant du contrôle ; généré si absent. */
  id?: string
  label?: string
  hint?: string
  error?: string
  required?: boolean
  disabled?: boolean
  readonly?: boolean
  /** Compteur de caractères ou toute mention alignée à droite du libellé. */
  counter?: string
  /** Le compteur a-t-il dépassé la limite ? Il passe alors en rouge. */
  counterExceeded?: boolean
  /** Champ sans libellé visible (barre de recherche d'un en-tête, cellule de tableau). */
  hideLabel?: boolean
}

const props = defineProps<Props>()
const { t } = useI18n()

const generatedId = useId()
const fieldId = computed(() => props.id ?? `field-${generatedId}`)
const hintId = computed(() => `${fieldId.value}-hint`)
const errorId = computed(() => `${fieldId.value}-error`)

/** Aide d'abord, erreur ensuite. `undefined` quand il n'y a rien à décrire. */
const describedBy = computed(() => {
  const ids = [props.hint ? hintId.value : null, props.error ? errorId.value : null].filter(Boolean)
  return ids.length ? ids.join(' ') : undefined
})

/**
 * Ce que l'enveloppe transmet au contrôle. Le champ n'a plus qu'à l'étaler sur
 * son `<input>` : aucun identifiant à recomposer, aucun lien à refaire.
 */
const control = computed(() => ({
  id: fieldId.value,
  'aria-describedby': describedBy.value,
  'aria-invalid': props.error ? true : undefined,
  'aria-required': props.required ? true : undefined,
  disabled: props.disabled,
  readonly: props.readonly,
  hasError: Boolean(props.error),
}))

defineSlots<{
  /** Le contrôle lui-même. Reçoit les attributs à étaler. */
  default: (props: { control: typeof control.value }) => unknown
  /** Contenu additionnel sous le champ (compteur détaillé, aperçu). */
  footer?: () => unknown
}>()
</script>

<template>
  <div class="w-full">
    <div v-if="props.label" class="mb-1.5 flex items-baseline justify-between gap-3">
      <label
        :for="fieldId"
        class="text-sm font-medium text-text"
        :class="[props.hideLabel ? 'sr-only' : '', props.disabled ? 'text-text-subtle' : '']"
      >
        {{ props.label }}
        <!-- On marque l'obligation, pas la facultativité : dans un formulaire de
             dépôt, presque tout est obligatoire, et vingt astérisques ne
             renseignent personne. Le mot complet plutôt qu'un astérisque nu. -->
        <span v-if="props.required" class="ml-0.5 text-danger" aria-hidden="true">*</span>
        <span v-if="props.required" class="sr-only"> — {{ t('form.required') }}</span>
        <span v-else class="ml-1.5 text-xs font-normal text-text-subtle">
          {{ t('form.optional') }}
        </span>
      </label>

      <span
        v-if="props.counter"
        class="shrink-0 font-mono text-xs tabular-nums"
        :class="props.counterExceeded ? 'font-semibold text-danger' : 'text-text-subtle'"
      >
        {{ props.counter }}
      </span>
    </div>

    <slot :control="control" />

    <p v-if="props.hint" :id="hintId" class="mt-1.5 text-sm text-text-subtle">
      {{ props.hint }}
    </p>

    <!-- `role="alert"` : le message est annoncé dès qu'il apparaît, sans attendre
         que le champ reprenne le focus. -->
    <p
      v-if="props.error"
      :id="errorId"
      role="alert"
      class="mt-1.5 flex items-start gap-1.5 text-sm font-medium text-danger"
    >
      <UiIcon name="warning" size="1.05em" class="mt-0.5" />
      <span><span class="sr-only">{{ t('form.errorPrefix') }} </span>{{ props.error }}</span>
    </p>

    <slot name="footer" />
  </div>
</template>
