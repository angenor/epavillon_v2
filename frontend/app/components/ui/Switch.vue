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
 * L'état est porté par une case native, comme pour `UiCheckbox` : focus, barre
 * d'espace et annonce vocale viennent du navigateur. Elle n'est pas masquée
 * mais rendue transparente et étalée sur toute la commande — c'est elle qui
 * reçoit le clic, le dessin mécanique n'est qu'un décor (`aria-hidden`).
 *
 * DESSIN MÉCANIQUE, arbitré le 19/08. La commande imite un basculeur moulé :
 * piste creusée, curseur bombé, voyant au centre, rainures qui s'allument. Le
 * relief vient de DEUX couleurs — `--color-relief-light` et `-shade` — et non
 * d'une ombre noire : une pièce moulée reçoit sa lumière d'un côté et son ombre
 * de l'autre. Ces deux jetons existent dans les deux thèmes ; en sombre, la
 * lumière est un gris sourd et non un blanc, sans quoi le relief vire au néon.
 *
 * LES COULEURS DISENT L'ÉTAT, pas la marque : vert (`--color-success`) pour le
 * voyant allumé, parce que l'interrupteur allumé est un réglage CONFIRMÉ ; cyan
 * (`--color-accent-solid`) pour les rainures, qui signalent le courant qui
 * passe ; gris sourd pour le voyant éteint. Aucune valeur n'est écrite ici.
 *
 * PISTE 80 × 40, CURSEUR 32. Les trois nombres tiennent ensemble : 4 px de
 * piste tout autour du curseur, et une course de 40 px assez longue pour que le
 * geste se voie. Ils sont pilotés par `--sw-track-w`, `--sw-track-h` et
 * `--sw-thumb` — une barre d'outils dense peut les redéfinir sur le composant
 * sans toucher au dessin. La cible tactile reste `--target-min` grâce au
 * conteneur.
 *
 * DURÉE `--duration-base × 1.6` : la bascule est le seul mouvement de
 * l'interface qu'il faut SUIVRE DE L'ŒIL — c'est lui qui dit dans quel sens le
 * réglage vient de partir. La courbe dépasse légèrement en fin de course, comme
 * un ressort qui claque. `prefers-reduced-motion` coupe tout, globalement.
 *
 * `loading` couvre le cas réel de l'interrupteur qui déclenche un appel réseau :
 * l'interrupteur reste dans son ancienne position, non basculable, jusqu'à la
 * réponse. Le faire glisser tout de suite puis revenir en arrière sur erreur est
 * la pire des deux solutions. Le voyant cède alors la place au sablier.
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
    class="flex min-h-(--target-min) max-w-(--measure) items-center gap-3"
    :class="props.labelPosition === 'start' ? 'flex-row-reverse justify-end' : ''"
  >
    <!-- L'opacité du désactivé enveloppe la commande entière : un curseur resté
         opaque sur une piste éteinte se lirait comme une troisième position. -->
    <span class="switch" :class="props.disabled ? 'switch--disabled' : ''">
      <input
        :id="fieldId"
        type="checkbox"
        role="switch"
        class="switch__input"
        :checked="props.modelValue"
        :disabled="isLocked"
        :aria-describedby="props.hint ? hintId : undefined"
        :aria-busy="props.loading ? 'true' : undefined"
        @change="onChange"
      >
      <span class="switch__track" aria-hidden="true">
        <span class="switch__glow" />
        <span class="switch__thumb">
          <span class="switch__engraving" />
          <UiSpinner v-if="props.loading" size="0.9rem" class="switch__spinner" />
          <span v-else class="switch__led" />
        </span>
      </span>
    </span>

    <label
      v-if="props.label || $slots.default"
      :for="fieldId"
      class="text-sm leading-snug"
      :class="isLocked ? 'cursor-not-allowed text-text-muted' : 'cursor-pointer text-text'"
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

<style scoped>
.switch {
  /* Les trois nombres du dessin, redéfinissables sur l'élément appelant. */
  --sw-track-w: 80px;
  --sw-track-h: 40px;
  --sw-thumb: 32px;
  --sw-gap: calc((var(--sw-track-h) - var(--sw-thumb)) / 2);

  --sw-duration: calc(var(--duration-base) * 1.6);
  --sw-ease: cubic-bezier(0.175, 0.885, 0.32, 1.275);
  --sw-motion: all var(--sw-duration) var(--sw-ease);

  /* Le creux de la piste et la bosse du curseur, dessinés par les deux
     couleurs de relief plutôt que par une ombre noire. */
  --sw-hollow: inset 2px 2px 5px var(--color-relief-shade),
    inset -2px -2px 5px var(--color-relief-light);
  --sw-raised: 5px 5px 10px
      color-mix(in srgb, var(--color-relief-shade) 70%, transparent),
    -5px -5px 10px color-mix(in srgb, var(--color-relief-light) 70%, transparent);

  position: relative;
  display: inline-block;
  flex-shrink: 0;
  width: var(--sw-track-w);
  height: var(--sw-track-h);
}

.switch--disabled {
  opacity: 0.45;
}

/* La case native couvre toute la commande : elle reçoit le clic et le focus,
   le décor n'intercepte rien. Transparente et non `display: none`, sans quoi
   Safari la retire de l'ordre de tabulation. */
.switch__input {
  position: absolute;
  inset: 0;
  z-index: 1;
  margin: 0;
  width: 100%;
  height: 100%;
  cursor: pointer;
  opacity: 0;
  appearance: none;
}

.switch__input:disabled {
  cursor: not-allowed;
}

.switch__track {
  position: absolute;
  inset: 0;
  overflow: hidden;
  border: var(--border-thin) solid var(--color-border-subtle);
  border-radius: var(--radius-full);
  background-color: var(--color-surface-sunken);
  box-shadow: var(--sw-hollow);
  transition: var(--sw-motion);
}

/* Les trois rainures du fond. Éteintes, elles ne sont qu'un creux ; allumées,
   elles portent le cyan et disent que le courant passe. */
.switch__track::before {
  content: "";
  position: absolute;
  top: 50%;
  left: calc(var(--sw-track-w) * 0.1875);
  z-index: 0;
  width: calc(var(--sw-track-w) * 0.625);
  height: 1px;
  transform: translateY(-50%);
  background: var(--color-relief-shade);
  box-shadow:
    0 4px 0 var(--color-relief-shade),
    0 -4px 0 var(--color-relief-shade);
  transition: var(--sw-motion);
}

/* La lueur du voyant, qui suit le curseur. Elle est le SEUL flou de la charte :
   c'est ce qui distingue un voyant allumé d'une pastille verte. */
.switch__glow {
  position: absolute;
  top: 0;
  left: 0;
  width: var(--sw-track-h);
  height: var(--sw-track-h);
  background: var(--color-success);
  filter: blur(15px);
  opacity: 0;
  transition: var(--sw-motion);
}

.switch__thumb {
  position: absolute;
  top: 50%;
  left: var(--sw-gap);
  z-index: 2;
  display: flex;
  width: var(--sw-thumb);
  height: var(--sw-thumb);
  align-items: center;
  justify-content: center;
  transform: translateY(-50%);
  border: var(--border-thin) solid var(--color-relief-light);
  border-radius: var(--radius-full);
  background-color: var(--color-surface-raised);
  box-shadow: var(--sw-raised);
  transition: var(--sw-motion);
}

/* Le cercle gravé du curseur — le détail qui fait la pièce usinée. */
.switch__engraving {
  position: absolute;
  top: 50%;
  left: 50%;
  width: calc(var(--sw-thumb) * 0.625);
  height: calc(var(--sw-thumb) * 0.625);
  transform: translate(-50%, -50%);
  /* `--color-border` et non `-subtle` : en thème sombre, le trait subtil se
     confond avec le curseur, et la gravure disparaît au lieu d'être discrète. */
  border: 1px dashed var(--color-border);
  border-radius: var(--radius-full);
}

.switch__led {
  position: relative;
  width: 6px;
  height: 6px;
  border-radius: var(--radius-full);
  background: var(--color-text-subtle);
  box-shadow: inset 1px 1px 2px var(--color-relief-shade);
  transition: var(--sw-motion);
}

.switch__spinner {
  position: relative;
  color: var(--color-text-muted);
}

/* -- Position allumée -- */
.switch__input:checked + .switch__track {
  border-color: color-mix(in srgb, var(--color-success) 35%, transparent);
}

.switch__input:checked + .switch__track::before {
  background: var(--color-accent-solid);
  box-shadow:
    0 4px 0 var(--color-accent-solid),
    0 -4px 0 var(--color-accent-solid);
}

.switch__input:checked + .switch__track .switch__thumb {
  left: calc(var(--sw-track-w) - var(--sw-thumb) - var(--sw-gap));
  border-color: color-mix(in srgb, var(--color-success) 40%, transparent);
}

.switch__input:checked + .switch__track .switch__led {
  background: var(--color-success);
  box-shadow: 0 0 8px var(--color-success);
}

.switch__input:checked + .switch__track .switch__glow {
  left: calc(var(--sw-track-w) - var(--sw-track-h));
  opacity: 0.18;
}

/* -- Survol, appui, focus -- */
.switch__input:not(:disabled):hover + .switch__track .switch__thumb {
  box-shadow:
    3px 3px 7px color-mix(in srgb, var(--color-relief-shade) 60%, transparent),
    -3px -3px 7px color-mix(in srgb, var(--color-relief-light) 60%, transparent);
}

.switch__input:not(:disabled):active + .switch__track .switch__thumb {
  transform: translateY(-50%) scale(0.95);
  box-shadow: var(--sw-hollow);
}

.switch__input:focus-visible + .switch__track {
  outline: 2px solid var(--color-focus);
  outline-offset: 2px;
}
</style>
