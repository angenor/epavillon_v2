<script setup lang="ts">
import type { Intent } from '~/types/ui'

/**
 * Message d'alerte dans le flux d'une page — information, succès, avertissement,
 * erreur.
 *
 * TROIS SIGNAUX POUR UNE MÊME INFORMATION : une icône DE FORME distincte, une
 * couleur, un titre. Une alerte reconnaissable à sa seule couleur est perdue
 * pour une personne daltonienne, et invisible à l'impression.
 *
 * LE TON EST PORTÉ PAR LE CONTENEUR, pas par l'icône seule. Le bloc entier prend
 * `color: var(--color-info|success|warning|danger)`, et tout ce qui n'a pas de
 * couleur propre — icône, croix de fermeture, liens du créneau — en hérite. Deux
 * exceptions assumées, et deux seulement : le TITRE revient en `--color-text`
 * (une phrase de titre en jaune 700 se lit mal), et le corps en
 * `--color-text-secondary`. Ce sont exactement les deux règles du guide.
 *
 * ANNONCE VOCALE. Une alerte de résultat (erreur de formulaire, échec
 * d'enregistrement) doit être annoncée dès qu'elle apparaît : `role="alert"`,
 * qui interrompt la lecture en cours. Une alerte de contexte, présente au
 * chargement (« l'appel ferme dans trois jours »), ne doit RIEN interrompre :
 * `role="note"`. D'où `live`, faux par défaut — c'est le cas le plus fréquent, et
 * une page qui interrompt la lecture à chaque bloc est inutilisable.
 *
 * À NE PAS CONFONDRE AVEC `UiIncidentBanner` : celui-ci est un message
 * d'exploitation publié depuis le back-office (`live.incidents`) et s'affiche en
 * pleine largeur, au-dessus du contenu. `UiAlert` est un élément de page.
 */

interface Props {
  intent?: Intent
  /** Titre de l'alerte. Facultatif : un message court se suffit. */
  title?: string
  /** Corps du message, si l'on ne passe pas par le créneau par défaut. */
  message?: string
  /** L'alerte peut-elle être refermée par l'utilisateur ? */
  dismissible?: boolean
  /**
   * Alerte de RÉSULTAT, annoncée dès son apparition (`role="alert"`).
   * Faux pour une alerte de contexte présente au chargement.
   */
  live?: boolean
  /** Version compacte, sans icône : notes de bas de formulaire. */
  compact?: boolean
}

const props = withDefaults(defineProps<Props>(), { intent: 'info' })
const emit = defineEmits<{ dismiss: [] }>()

const { t } = useI18n()
const isVisible = ref(true)

/** Une forme par intention — la couleur ne porte jamais seule l'information. */
const ICONS: Record<Intent, string> = {
  neutral: 'info',
  info: 'info',
  success: 'check-circle',
  warning: 'warning',
  danger: 'error',
}

/** Fond, bordure ET couleur de texte du bloc : les trois vont ensemble. */
const TONES: Record<Intent, string> = {
  neutral: 'border-border bg-surface-sunken text-text-muted',
  info: 'border-info-border bg-info-surface text-info',
  success: 'border-success-border bg-success-surface text-success',
  warning: 'border-warning-border bg-warning-surface text-warning',
  danger: 'border-danger-border bg-danger-surface text-danger',
}

function dismiss(): void {
  isVisible.value = false
  emit('dismiss')
}
</script>

<template>
  <div
    v-if="isVisible"
    :role="props.live ? 'alert' : 'note'"
    class="flex items-start gap-3 rounded-lg border"
    :class="[TONES[props.intent], props.compact ? 'px-3 py-2' : 'p-4']"
  >
    <!-- Sans classe de couleur : l'icône hérite du ton du conteneur. -->
    <UiIcon v-if="!props.compact" :name="ICONS[props.intent]" class="mt-0.5 shrink-0" size="1.25rem" />

    <div class="min-w-0 flex-1">
      <p v-if="props.title" class="font-display text-base leading-snug text-text">
        {{ props.title }}
      </p>
      <!-- Plafonné à `--measure` : au-delà de 68 signes, l'œil perd la ligne en
           revenant à gauche, et une alerte se lit d'un seul coup ou pas du tout. -->
      <div
        class="max-w-(--measure) text-sm text-text-secondary"
        :class="props.title ? 'mt-1' : ''"
      >
        <slot>{{ props.message }}</slot>
      </div>
      <div v-if="$slots.actions" class="mt-3 flex flex-wrap items-center gap-2">
        <slot name="actions" />
      </div>
    </div>

    <!-- 32 px : cible d'une croix posée en coin de bloc, jamais l'action
         principale. Le survol se teinte du ton courant plutôt que d'un gris
         fixe — sur un fond déjà coloré, un gris de survol fait une tache. -->
    <button
      v-if="props.dismissible"
      type="button"
      class="ui-alert-close -mt-1 -mr-1 grid size-8 shrink-0 cursor-pointer place-items-center self-start rounded-sm text-inherit transition-colors duration-(--duration-fast)"
      @click="dismiss"
    >
      <span class="sr-only">{{ t('common.actions.close') }}</span>
      <UiIcon name="close" size="0.875rem" />
    </button>
  </div>
</template>

<style scoped>
/* `color-mix` sur `currentColor` : la teinte du survol suit l'intention sans
   qu'aucune couleur ne soit écrite ici. */
.ui-alert-close:hover {
  background: color-mix(in srgb, currentColor 12%, transparent);
}
</style>
