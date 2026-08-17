<script setup lang="ts">
/**
 * Champ de mot de passe, avec bouton de révélation.
 *
 * POURQUOI RÉVÉLER PLUTÔT QUE CONFIRMER. Le second champ « confirmez votre mot
 * de passe » existe pour rattraper une faute de frappe qu'on ne voit pas ; le
 * bouton qui affiche le texte règle le même problème sans doubler la saisie, et
 * il rend service aussi sur un téléphone, où l'on se trompe le plus. La création
 * de compte d'A1 ne demande donc PAS de confirmation — c'est le seul écran où la
 * question se posait, le mot de passe n'y étant tapé qu'une fois.
 *
 * LE BOUTON N'EST PAS UNE BASCULE D'ÉTAT VISUEL : `aria-pressed` porte l'état,
 * et son libellé change (« Afficher » / « Masquer »). Une icône seule laisserait
 * un lecteur d'écran annoncer « bouton œil », ce qui ne dit ni ce qui se passe,
 * ni ce qui se passera.
 *
 * TAILLE COMPACTE ASSUMÉE À L'INTÉRIEUR DU CADRE. La règle des 44 px vaut pour
 * les actions d'un écran ; ce bouton-ci est un accessoire de champ, logé dans sa
 * bordure. Le champ lui-même, lui, fait bien 44 px de haut.
 */

interface Props {
  modelValue: string
  label: string
  id?: string
  hint?: string
  error?: string
  required?: boolean
  disabled?: boolean
  /** `current-password` à la connexion, `new-password` partout ailleurs. */
  autocomplete?: 'current-password' | 'new-password'
  placeholder?: string
}

const props = withDefaults(defineProps<Props>(), { autocomplete: 'current-password' })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const { t } = useI18n()
const isRevealed = ref(false)
const root = ref<HTMLElement | null>(null)

/**
 * Un mot de passe laissé en clair à l'écran finit par l'être devant quelqu'un :
 * il se remasque dès que le champ perd le focus.
 *
 * SAUF SI LE FOCUS VA SUR LE BOUTON DE RÉVÉLATION — sinon le clic qui doit
 * masquer produit l'inverse : le `blur` remasque d'abord, puis le `click`
 * rebascule, et le mot de passe reste affiché. Le cas est invisible à la
 * relecture et se voit tout de suite à l'usage.
 */
function hideOnBlur(event: FocusEvent): void {
  const next = event.relatedTarget
  if (next instanceof Node && root.value?.contains(next)) return
  isRevealed.value = false
}
</script>

<template>
  <div ref="root">
    <UiInput
      :id="props.id"
      :model-value="props.modelValue"
      :type="isRevealed ? 'text' : 'password'"
      :label="props.label"
      :hint="props.hint"
      :error="props.error"
      :required="props.required"
      :disabled="props.disabled"
      :autocomplete="props.autocomplete"
      :placeholder="props.placeholder"
      icon="lock"
      @update:model-value="emit('update:modelValue', $event)"
      @blur="hideOnBlur"
    >
      <template #suffix>
        <UiButton
          variant="ghost"
          size="sm"
          icon-only
          :icon="isRevealed ? 'eye-off' : 'eye'"
          :label="isRevealed ? t('auth-form.password.hide') : t('auth-form.password.show')"
          :pressed="isRevealed"
          :disabled="props.disabled"
          @click="isRevealed = !isRevealed"
        />
      </template>
    </UiInput>
  </div>
</template>
