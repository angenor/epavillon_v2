<script setup lang="ts">
import type { PasswordRequirement } from '~/utils/password-strength'
import { evaluatePassword, MIN_PASSWORD_LENGTH } from '~/utils/password-strength'

/**
 * Exigences et robustesse du mot de passe.
 *
 * DEUX CHOSES DISTINCTES, DANS CET ORDRE. La liste des EXIGENCES d'abord — huit
 * caractères, une majuscule, une minuscule — parce qu'elle est opposable et
 * qu'elle dit précisément ce qui manque ; la ROBUSTESSE ensuite, qui conseille
 * sans jamais bloquer. Un mot de passe conforme mais faible s'enregistre : c'est
 * ce qui permet de signaler `Azerty12` sans le refuser.
 *
 * LES EXIGENCES SONT AFFICHÉES AVANT LA SAISIE, pas au moment de l'échec. Une
 * règle découverte en cliquant sur « Créer mon compte » est une règle subie ;
 * la même, lue d'avance, ne coûte rien. Le calcul vit dans
 * `utils/password-strength.ts`, hors de Vue.
 *
 * QUATRE SEGMENTS, PAS UNE BARRE CONTINUE. Une barre qui glisse de 61 % à 64 %
 * suggère une précision que l'estimation n'a pas. Quatre paliers disent ce qu'ils
 * savent : faible, moyen, bon, solide.
 *
 * LA COULEUR NE PORTE JAMAIS SEULE. Le palier est ÉCRIT à côté des segments, et
 * `role="meter"` avec ses valeurs le rend au lecteur d'écran. Sans ce texte,
 * l'indicateur ne dirait rien à qui ne distingue pas le rouge de l'orange —
 * exactement la faute que le guide de style interdit.
 *
 * ANNONCE POLIE ET DIFFÉRÉE. `aria-live="polite"` sur le seul libellé : annoncer
 * chaque frappe couvrirait le retour du clavier. La zone est mise à jour à la
 * frappe, le lecteur d'écran l'énonce quand il a fini ce qu'il disait.
 */

interface Props {
  password: string
  /** Masque la barre de robustesse tant que rien n'est saisi. */
  hideWhenEmpty?: boolean
}

const props = withDefaults(defineProps<Props>(), { hideWhenEmpty: true })
const { t } = useI18n()

const strength = computed(() => evaluatePassword(props.password))

/** Les trois conditions opposables, plus le caractère spécial, marqué facultatif. */
const requirements = computed(() => {
  const list: { key: PasswordRequirement | 'special'; met: boolean; optional: boolean }[] = [
    { key: 'length', met: !strength.value.missing.includes('length'), optional: false },
    { key: 'uppercase', met: !strength.value.missing.includes('uppercase'), optional: false },
    { key: 'lowercase', met: !strength.value.missing.includes('lowercase'), optional: false },
    { key: 'special', met: strength.value.hasSpecial, optional: true },
  ]
  return list
})

/**
 * Les couleurs sont celles des états de la plateforme, et elles disent la même
 * chose qu'ailleurs : rouge ce qui échoue, jaune ce qui demande attention, vert
 * ce qui est acquis. « Moyen » et « bon » partagent le jaune : ils appellent le
 * même geste — allonger —, seul le nombre de segments les distingue.
 */
const TONES: Record<number, { fill: string; text: string }> = {
  0: { fill: 'bg-danger-solid', text: 'text-danger' },
  1: { fill: 'bg-danger-solid', text: 'text-danger' },
  2: { fill: 'bg-warning-solid', text: 'text-warning' },
  3: { fill: 'bg-warning-solid', text: 'text-warning' },
  4: { fill: 'bg-success-solid', text: 'text-success' },
}

const tone = computed(() => TONES[strength.value.score] ?? TONES[0]!)
const isVisible = computed(() => !props.hideWhenEmpty || props.password.length > 0)

const levelLabel = computed(() => t(`auth-form.strength.level.${strength.value.level}`))
const adviceLabel = computed(() =>
  strength.value.advice === 'none' ? null : t(`auth-form.strength.advice.${strength.value.advice}`),
)
</script>

<template>
  <div class="mt-2">
    <!-- Les exigences, toujours visibles. Chacune porte un GLYPHE en plus de sa
         couleur — coche ou point — : la couleur seule ne signale jamais un état,
         et une liste de quatre lignes vertes et rouges serait illisible sans lui. -->
    <ul class="grid gap-1 text-xs sm:grid-cols-2">
      <li
        v-for="requirement in requirements"
        :key="requirement.key"
        class="flex items-center gap-1.5"
        :class="requirement.met ? 'text-success' : 'text-text-muted'"
      >
        <UiIcon :name="requirement.met ? 'check-circle' : 'minus'" size="0.9rem" />
        <span>
          {{ t(`auth-form.strength.requirements.${requirement.key}`, { min: MIN_PASSWORD_LENGTH }) }}
          <span v-if="requirement.optional" class="text-text-subtle">
            {{ t('auth-form.strength.requirements.optionalSuffix') }}
          </span>
        </span>
      </li>
    </ul>

    <div v-if="isVisible" class="mt-2.5">
      <div
        class="flex gap-1.5"
        role="meter"
        :aria-valuenow="strength.score"
        aria-valuemin="0"
        aria-valuemax="4"
        :aria-valuetext="levelLabel"
        :aria-label="t('auth-form.strength.label')"
      >
        <span
          v-for="segment in 4"
          :key="segment"
          class="h-1.5 flex-1 rounded-full transition-colors duration-(--duration-fast)"
          :class="segment <= strength.score ? tone.fill : 'bg-surface-sunken'"
        />
      </div>

      <p class="mt-1.5 flex flex-wrap items-baseline gap-x-2 text-sm" aria-live="polite">
        <span class="font-bold" :class="tone.text">{{ levelLabel }}</span>
        <span v-if="adviceLabel" class="text-text-muted">{{ adviceLabel }}</span>
      </p>
    </div>
  </div>
</template>
