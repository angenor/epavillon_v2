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

const TONES: Record<Intent, string> = {
  neutral: 'border-border bg-surface-sunken',
  info: 'border-info-border bg-info-surface',
  success: 'border-success-border bg-success-surface',
  warning: 'border-warning-border bg-warning-surface',
  danger: 'border-danger-border bg-danger-surface',
}

const ICON_TONES: Record<Intent, string> = {
  neutral: 'text-text-muted',
  info: 'text-info',
  success: 'text-success',
  warning: 'text-warning',
  danger: 'text-danger',
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
    class="flex gap-3 rounded-lg border"
    :class="[TONES[props.intent], props.compact ? 'px-3 py-2' : 'px-4 py-3.5']"
  >
    <UiIcon
      v-if="!props.compact"
      :name="ICONS[props.intent]"
      :class="['mt-0.5 shrink-0', ICON_TONES[props.intent]]"
      size="1.15rem"
    />

    <div class="min-w-0 flex-1">
      <p v-if="props.title" class="font-display text-base leading-snug text-text">
        {{ props.title }}
      </p>
      <div
        class="text-sm text-text-muted"
        :class="props.title ? 'mt-1' : ''"
      >
        <slot>{{ props.message }}</slot>
      </div>
      <div v-if="$slots.actions" class="mt-3 flex flex-wrap items-center gap-2">
        <slot name="actions" />
      </div>
    </div>

    <button
      v-if="props.dismissible"
      type="button"
      class="-mt-1 -mr-1 shrink-0 self-start rounded-md p-1.5 text-text-subtle transition-colors hover:bg-surface-hover hover:text-text"
      @click="dismiss"
    >
      <span class="sr-only">{{ t('common.actions.close') }}</span>
      <UiIcon name="close" size="1rem" />
    </button>
  </div>
</template>
