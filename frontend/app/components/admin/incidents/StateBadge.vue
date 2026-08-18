<script setup lang="ts">
import type { IncidentState } from '~/types/admin-incidents'
import type { Intent } from '~/types/ui'

/**
 * L'ÉTAT D'UN MESSAGE — publié ou non, en train de parler ou non.
 *
 * À NE PAS CONFONDRE AVEC LA GRAVITÉ, qui est dans la même ligne. La gravité dit
 * ce qui se passe (information, avertissement, incident) ; l'état dit si le
 * public le lit. Un incident majeur jamais publié ne se voit nulle part, et
 * c'est exactement ce que cette pastille existe pour dire.
 *
 * LES COULEURS SUIVENT LES RÔLES DU GUIDE DE STYLE, pas l'intuition :
 * · `active`      — vert : c'est un fait établi, le bandeau est en ligne ;
 * · `scheduled`   — cyan : une information, rien n'est encore arrivé ;
 * · `draft`       — jaune : ce qui demande attention, une décision est en attente ;
 * · `expired` et `unpublished` — gris : ce qui est clos.
 *
 * LES DEUX FINS SE DISTINGUENT PAR LA FORME, PAS PAR LA COULEUR : l'une est
 * venue seule à l'heure prévue (horloge), l'autre est une décision (œil barré).
 * Toutes deux sont closes, donc grises — les opposer par la couleur donnerait à
 * une dépublication un poids d'alerte qu'elle n'a plus.
 */

interface Props {
  state: IncidentState
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })

const { t } = useI18n()

const INTENTS: Record<IncidentState, Intent> = {
  active: 'success',
  scheduled: 'info',
  draft: 'warning',
  expired: 'neutral',
  unpublished: 'neutral',
}

const ICONS: Record<IncidentState, string> = {
  active: 'broadcast',
  scheduled: 'calendar',
  draft: 'edit',
  expired: 'clock',
  unpublished: 'eye-off',
}
</script>

<template>
  <UiBadge
    :intent="INTENTS[props.state]"
    :icon="ICONS[props.state]"
    :size="props.size"
    solid
    :label="t(`admin.incident.list.state.${props.state}`)"
    :title="t(`admin.incident.list.stateHint.${props.state}`)"
  />
</template>
