<script setup lang="ts">
import type { NextWorkspaceActivity } from '~/utils/workspace-editions'

/**
 * « Votre prochaine activité » (04/11) : le billet à droite de « À faire »,
 * tiré des dossiers déjà chargés. La date en aplat, l'heure en grand avec son
 * fuseau, le titre, la salle, et le chemin vers le dossier.
 */

interface Props {
  activity: NextWorkspaceActivity
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { time, zoneLabel, zoneOffsetShort } = useDateTime()
const { duration } = useProgrammeSession()
const localePath = useLocalePath()

const session = computed(() => props.activity.session.session)
const edition = computed(() => props.activity.tracking.edition)

const hours = computed(() =>
  t('organization.workspace.next.hours', {
    start: time(session.value.starts_at, edition.value.timezone),
    end: time(session.value.ends_at, edition.value.timezone),
  }),
)

const zone = computed(() =>
  t('organization.workspace.next.zone', {
    zone: zoneLabel(edition.value.timezone, edition.value.city ?? undefined),
    offset: zoneOffsetShort(edition.value.timezone, session.value.starts_at),
  }),
)

const room = computed(() => (props.activity.session.room ? tr(props.activity.session.room.name) : t('organization.workspace.next.online')))
</script>

<template>
  <UiTicket :label="t('organization.workspace.next.label')">
    <template #band>
      <EventTicketDate :date="session.starts_at" :timezone="edition.timezone" :caption="t('organization.workspace.next.label')">
        <template #icon>
          <UiIcon name="calendar" size="1.75rem" :stroke-width="1.6" class="text-accent-on-inverse" />
        </template>
      </EventTicketDate>
    </template>

    <p class="flex flex-wrap items-baseline gap-x-2.5">
      <span class="font-condensed text-[2.4rem] leading-none font-bold text-heading tabular-nums">{{ hours }}</span>
      <span class="font-condensed text-[1.1rem] font-semibold text-text-secondary">{{ duration(session) }}</span>
    </p>
    <p class="mt-1.5 text-sm text-text-secondary first-letter:uppercase">{{ zone }}</p>
    <p class="mt-2.5 font-condensed text-[1.3rem] leading-[1.12] font-semibold text-heading">
      {{ tr(props.activity.tracking.proposal.title) }}
    </p>
    <p class="mt-1.5 flex items-center gap-2 text-sm text-text-secondary">
      <UiIcon name="map-pin" size="1rem" class="shrink-0 text-text-subtle" />
      {{ room }}
    </p>
    <UiButton
      class="mt-4"
      variant="secondary"
      block
      :to="localePath(`/mon-organisation/dossiers/${props.activity.tracking.proposal.id}`)"
    >
      {{ t('organization.workspace.next.open') }}
    </UiButton>
  </UiTicket>
</template>
