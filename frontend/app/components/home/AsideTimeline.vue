<script setup lang="ts">
import type { PublicEditionRow, PublicScheduleRow } from '~/types/views'
import type { AsideFastening } from './AsideSession.vue'

/**
 * LES PROCHAINES ACTIVITÉS RETENUES du panneau « À venir », dans l'ordre du
 * temps. `programme.v_public_schedule` ne rend que des séances publiées : aucun
 * filtre à rejouer ici.
 *
 * Une édition commune est nommée par le panneau, avec son fuseau ; quand la
 * liste en mêle plusieurs, chaque ligne dit d'où elle vient et à quelle heure.
 */

interface Props {
  sessions: PublicScheduleRow[]
  /** L'historique complet : nomme l'édition d'une séance et la relie. */
  editions: PublicEditionRow[]
  /** L'édition commune à toutes les séances, déjà nommée par le panneau. */
  singleEdition: PublicEditionRow | null
  fastening: AsideFastening
}

const props = defineProps<Props>()

const localePath = useLocalePath()
const { link } = useProgrammeSession()

/** La vue publique n'expose que `event_id` : la correspondance vient des éditions déjà en main. */
const editionByEvent = computed(() => new Map(props.editions.map((edition) => [edition.id, edition])))

/** Comme dans le programme : la page de l'activité, ou sa journée spéciale à sa place dans le déroulé. */
function sessionTo(session: PublicScheduleRow): string | undefined {
  const edition = editionByEvent.value.get(session.event_id)
  if (!edition) return undefined
  const special = session.tracks.find((badge) => badge.kind === 'special_day')
  if (special) {
    return localePath({
      name: 'track-edition-track',
      params: { edition: edition.slug, track: special.slug },
      hash: `#${trackActivityAnchor(session.slug)}`,
    })
  }
  return link(edition.slug, session)
}

function sessionEdition(session: PublicScheduleRow): PublicEditionRow | null {
  return editionByEvent.value.get(session.event_id) ?? null
}
</script>

<template>
  <ol class="m-0 list-none p-0">
    <li
      v-for="(session, index) in props.sessions"
      :key="session.id"
      class="border-t border-border-on-inverse lg:border-glass-border"
    >
      <HomeAsideSession
        :session="session"
        :to="sessionTo(session)"
        :zone-label="sessionEdition(session)?.city ?? undefined"
        :edition-label="props.singleEdition ? null : (sessionEdition(session)?.edition_label ?? sessionEdition(session)?.acronym ?? null)"
        :show-zone="!props.singleEdition"
        :fastening="props.fastening"
        :index="index"
      />
    </li>
  </ol>
</template>
