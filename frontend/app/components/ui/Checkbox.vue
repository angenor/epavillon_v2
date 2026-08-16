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
 * LA CIBLE FAIT 44 px DE HAUT, pas la taille du carré. Le contrôle visible en
 * mesure 20 avec un trait de 2 px — assez pour se voir sans écraser le libellé —
 * mais c'est toute la ligne, libellé compris, qui reçoit le clic. Viser un carré
 * de 20 px au doigt, dans un train ou debout dans un couloir de conférence, n'est
 * pas raisonnable.
 *
 * LE TRAIT DE CONTRÔLE EST À 2 px là où celui d'un champ de saisie est à 1 : un
 * carré de 20 px bordé d'un cheveu disparaît, alors qu'un champ de 300 px de
 * large se voit par sa seule étendue.
 *
 * LARGEUR PLAFONNÉE À `--measure` : un libellé de case porte souvent une phrase
 * entière (« Désertification — indisponible pour cette conférence »), qui suit la
 * même règle de lisibilité que les paragraphes.
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
  <div class="max-w-(--measure)">
    <div class="flex min-h-(--target-min) items-start gap-3 py-2">
      <!-- L'opacité du désactivé est portée par l'ENVELOPPE, pas par la case :
           le carré et sa coche s'éteignent alors ensemble. Posée sur la seule
           case, elle laisserait une coche à pleine intensité sur un carré pâle. -->
      <span
        class="relative mt-0.5 flex size-5 shrink-0 items-center"
        :class="props.disabled ? 'opacity-[.45]' : ''"
      >
        <input
          :id="fieldId"
          type="checkbox"
          class="peer size-5 shrink-0 cursor-pointer appearance-none rounded-sm border-(length:--border-medium) border-solid bg-surface-raised transition-colors duration-(--duration-fast)
                 checked:border-accent-solid checked:bg-accent-solid
                 indeterminate:border-accent-solid indeterminate:bg-accent-solid
                 hover:border-accent
                 disabled:cursor-not-allowed"
          :class="props.error ? 'border-danger' : 'border-border-strong'"
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
             couleurs — l'état doit rester lisible en niveaux de gris. La marque
             se superpose à la case en position absolue plutôt que d'être son
             `::after` : une case `appearance-none` n'accepte pas d'enfant, et le
             pseudo-élément échapperait au jeu d'icônes commun. -->
        <span
          v-if="props.indeterminate"
          class="pointer-events-none absolute inset-0 grid place-items-center text-accent-contrast"
        >
          <UiIcon name="minus" size="0.95rem" :stroke-width="2.6" />
        </span>
        <span
          v-else
          class="pointer-events-none absolute inset-0 grid place-items-center text-accent-contrast opacity-0 peer-checked:opacity-100"
        >
          <UiIcon name="check" size="0.95rem" :stroke-width="2.6" />
        </span>
      </span>

      <label
        v-if="props.label || $slots.default"
        :for="fieldId"
        class="text-sm leading-snug"
        :class="[
          isLocked ? 'cursor-default text-text-muted' : 'cursor-pointer text-text',
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

    <!-- Aligné sous le LIBELLÉ, pas sous la case : 20 px de contrôle et 12 px
         d'écart. Un message d'erreur qui commence à gauche de son libellé se lit
         comme celui de la case précédente. -->
    <p v-if="props.error" :id="errorId" role="alert" class="mt-1.5 pl-8 text-sm font-bold text-danger">
      <span class="sr-only">{{ t('form.errorPrefix') }} </span>{{ props.error }}
    </p>
  </div>
</template>
