<script setup lang="ts">
import type { TrackedSession } from '~/types/organization-workspace'
import type { TimeZoneName } from '~/types/shared'

/**
 * « QUAND ET OÙ » : le billet d'un dossier retenu, qui remonte sur le bandeau
 * (04/11). La date en aplat, l'heure en grand avec son fuseau, la salle et sa
 * jauge, la page publique et son QR, puis « Écrire à l'équipe » et, tant
 * qu'aucune séance n'a commencé, « Annuler l'activité ».
 *
 * LE NOMBRE D'INSCRITS N'EST PAS MONTRÉ à l'organisation (décision du
 * commanditaire, 02/11) ; jamais leur liste non plus, ce sont les données de
 * tiers. Le calendrier des rappels a rejoint « Le jour J ».
 *
 * Retenu mais pas encore programmé : le billet dit ce qui va venir.
 */

interface Props {
  sessions: TrackedSession[]
  /** Fuseau de l'édition. */
  timezone: TimeZoneName
  /** Ville de l'édition, qui nomme le fuseau. */
  city?: string | null
  /** Adresse de la page publique, une fois le programme publié. */
  publicUrl?: string | null
  /** Nom du QR téléchargé : le numéro du dossier. */
  fileName: string
  canCancel?: boolean
}

const props = withDefaults(defineProps<Props>(), { city: null, publicUrl: null, canCancel: false })
const emit = defineEmits<{ writeTeam: []; cancel: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dayLong, time, zoneLabel, zoneOffsetShort } = useDateTime()
const { duration } = useProgrammeSession()

const first = computed(() => props.sessions[0] ?? null)

function hoursOf(tracked: TrackedSession): string {
  return t('organization.workspace.proposal.sessions.hours', {
    start: time(tracked.session.starts_at, props.timezone),
    end: time(tracked.session.ends_at, props.timezone),
  })
}

function zoneOfSession(tracked: TrackedSession): string {
  return t('organization.workspace.proposal.sessions.zone', {
    zone: zoneLabel(props.timezone, props.city ?? undefined),
    offset: zoneOffsetShort(props.timezone, tracked.session.starts_at),
  })
}
</script>

<template>
  <UiTicket id="quand-ou" class="scroll-mt-24" :label="t('organization.workspace.proposal.sessions.title')">
    <template v-if="first" #band>
      <EventTicketDate :date="first.session.starts_at" :timezone="props.timezone" :caption="t('organization.workspace.proposal.sessions.title')">
        <template #icon>
          <UiIcon name="calendar" size="1.75rem" :stroke-width="1.6" class="text-accent-on-inverse" />
        </template>
      </EventTicketDate>
    </template>

    <template v-if="!first">
      <p class="font-condensed text-[1.05rem] font-bold tracking-[0.05em] text-text-secondary uppercase">
        {{ t('organization.workspace.proposal.sessions.title') }}
      </p>
      <p class="mt-1.5 font-bold text-text">{{ t('organization.workspace.proposal.sessions.empty.title') }}</p>
      <p class="mt-1.5 text-sm text-text-secondary">{{ t('organization.workspace.proposal.sessions.empty.description') }}</p>
    </template>

    <div
      v-for="(tracked, index) in props.sessions"
      :key="tracked.session.id"
      :class="index > 0 ? 'mt-4 border-t border-border-subtle pt-3.5' : ''"
    >
      <p v-if="props.sessions.length > 1" class="mb-1 text-sm font-bold text-text first-letter:uppercase">
        {{ dayLong(tracked.session.starts_at, props.timezone) }}
      </p>
      <p class="flex flex-wrap items-baseline gap-x-2.5">
        <span class="font-condensed text-[2.4rem] leading-none font-bold text-heading tabular-nums">{{ hoursOf(tracked) }}</span>
        <span class="font-condensed text-[1.1rem] font-semibold text-text-secondary">{{ duration(tracked.session) }}</span>
      </p>
      <p class="mt-1.5 text-sm text-text-secondary first-letter:uppercase">{{ zoneOfSession(tracked) }}</p>
      <p class="mt-3.5 flex items-center gap-2 text-[0.9rem] text-text-secondary">
        <UiIcon name="map-pin" size="1rem" class="shrink-0 text-text-subtle" />
        {{ tracked.room ? tr(tracked.room.name) : t('organization.workspace.proposal.sessions.online') }}
      </p>
      <p v-if="tracked.capacity !== null" class="mt-1 flex items-center gap-2 text-[0.9rem] text-text-secondary">
        <UiIcon name="users" size="1rem" class="shrink-0 text-text-subtle" />
        {{ t('organization.workspace.proposal.sessions.capacity', { count: tracked.capacity }) }}
      </p>
      <p v-if="tracked.session.attendee_count !== null" class="mt-1 flex items-center gap-2 text-[0.9rem] text-text-secondary">
        <UiIcon name="check-circle" size="1rem" class="shrink-0 text-text-subtle" />
        {{ t('organization.workspace.proposal.sessions.attendance', { count: tracked.session.attendee_count }) }}
      </p>
      <!-- Une activité tenue sans compte rendu : la dernière chose que l'IFDD
           attend, et la plus facile à oublier une fois la COP finie. -->
      <UiAlert
        v-if="tracked.session.status === 'completed' && tracked.session.report_submitted_at === null"
        intent="warning"
        class="mt-3"
        compact
        :message="t('organization.workspace.proposal.sessions.reportMissing')"
      />
    </div>
    <p v-if="first" class="mt-2 text-[0.8rem] text-text-muted">{{ t('organization.workspace.proposal.sessions.description') }}</p>

    <div v-if="props.publicUrl" class="mt-4 border-t border-border-subtle pt-3.5">
      <p class="mb-2 font-condensed text-[1.05rem] font-bold tracking-[0.05em] text-text-secondary uppercase">
        {{ t('organization.workspace.proposal.sessions.publicPage') }}
      </p>
      <WorkspaceFilePublicLink :url="props.publicUrl" :file-name="props.fileName" />
    </div>

    <div class="mt-4 flex flex-col gap-1 border-t border-border-subtle pt-3.5">
      <UiButton variant="secondary" icon="chat" block @click="emit('writeTeam')">
        {{ t('organization.workspace.proposal.sessions.writeTeam') }}
      </UiButton>
      <UiButton v-if="props.canCancel" variant="danger-link" icon="minus" block @click="emit('cancel')">
        {{ t('organization.workspace.proposal.cancel.action') }}
      </UiButton>
    </div>
  </UiTicket>
</template>
