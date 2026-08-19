<script setup lang="ts">
import type { ShowcaseBroadcastState } from '~/types/admin-showcase'

/**
 * L'ÉTAT RÉEL DE DIFFUSION D'UNE DIAPOSITIVE — statut ET fenêtre combinés.
 *
 * POURQUOI CE N'EST PAS `UiStatusBadge`. Celui-ci rend les cinq états d'une
 * SÉANCE (`upcoming`, `ongoing`, `past`, `postponed`, `cancelled`, `live`) ;
 * ceux d'une diapositive n'en sont pas une déclinaison, et les faire entrer de
 * force dans son vocabulaire ferait annoncer « en cours » pour « publiée ».
 *
 * LE STATUT SEUL NE DIT PAS CE QUI EST À L'ÉCRAN. « Publiée » plus une fenêtre
 * passée, c'est un contenu invisible que le back-office doit montrer comme tel —
 * sans quoi l'éditeur cherche pendant une heure pourquoi son témoignage
 * « publié » n'apparaît pas sur l'accueil.
 *
 * LES COULEURS SUIVENT LA RÈGLE DU GUIDE, PAS L'INTUITION :
 *   `live`      vert — c'est confirmé, la diapositive est bien à l'écran ;
 *   `scheduled` cyan — une information, rien à faire, elle s'allumera seule ;
 *   `draft`     jaune — elle demande une action pour exister ;
 *   `expired`   gris — clos, sa fenêtre est passée ;
 *   `archived`  gris — retirée, conservée, réutilisable.
 *
 * `expired` et `archived` partagent le gris et se distinguent par la FORME :
 * une horloge pour ce que le temps a éteint, une interdiction pour ce qu'on a
 * retiré. La couleur ne porte jamais seule l'information.
 *
 * Le LIBELLÉ est fourni par l'appelant, déjà traduit : chaque écran porte son
 * fichier de traduction, et un composant partagé qui appellerait `t()` sur les
 * clés d'un écran les rendrait indéplaçables.
 */

interface Props {
  state: ShowcaseBroadcastState
  /** Libellé déjà traduit par l'écran appelant. */
  label: string
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })

/**
 * LE BROUILLON EST GRIS, PAS JAUNE. Le jaune signale « ce qui demande
 * attention » ; un brouillon ne demande rien tant que l'éditeur n'a pas décidé,
 * et l'aide de l'écran le dit elle-même — « Personne ne la voit ». C'est aussi
 * ce que font les trois autres listes du back-office qui affichent un brouillon
 * (`admin/events/EditionsTable.vue`, `admin/proposals/Table.vue`,
 * `admin/review/Header.vue`) : le gris est la majorité, et un cinquième écran
 * n'a pas à la renverser. L'icône `edit` continue de porter la forme — la
 * couleur ne dit jamais seule ce qu'est un état.
 */
const INTENTS: Record<ShowcaseBroadcastState, 'neutral' | 'info' | 'success' | 'warning'> = {
  live: 'success',
  scheduled: 'info',
  draft: 'neutral',
  expired: 'neutral',
  archived: 'neutral',
}

const ICONS: Record<ShowcaseBroadcastState, string> = {
  live: 'check-circle',
  scheduled: 'calendar',
  draft: 'edit',
  expired: 'clock',
  archived: 'ban',
}
</script>

<template>
  <UiBadge
    :intent="INTENTS[props.state]"
    :icon="ICONS[props.state]"
    :size="props.size"
    :solid="props.state === 'live'"
  >
    {{ props.label }}
  </UiBadge>
</template>
