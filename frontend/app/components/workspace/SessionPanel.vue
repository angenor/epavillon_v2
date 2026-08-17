<script setup lang="ts">
import type { TrackedSession } from '~/types/organization-workspace'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES ACTIVITÉS PROGRAMMÉES D'UN DOSSIER RETENU : créneau, salle, inscrits, et
 * le calendrier des rappels.
 *
 * UN NOMBRE, JAMAIS UNE LISTE. L'organisation voit combien de personnes sont
 * inscrites à son activité, pas qui elles sont : ce sont les données
 * personnelles de tiers, et le seul écran qui les nomme est celui de l'IFDD,
 * derrière un périmètre d'administration. Le décompte suffit d'ailleurs à ce
 * qu'on en fait — préparer la salle et les documents.
 *
 * LA LISTE D'ATTENTE EST DITE À PART. Fondue dans le nombre d'inscrits, elle
 * ferait croire à une salle plus pleine qu'elle n'est ; tue, elle cacherait
 * qu'il y a plus de demande que de places — ce qui est précisément ce qu'une
 * organisation veut savoir avant de demander une plus grande salle.
 *
 * TOUTE DATE PORTE SON FUSEAU, celui de l'ÉDITION : « 14:30 — 16:00 (heure de
 * Belém, UTC−3) ». C'est la forme complète, celle qui fait foi pour un créneau.
 */

interface Props {
  sessions: TrackedSession[]
  /** Fuseau de l'édition. */
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeRangeFull } = useDateTime()
</script>

<template>
  <section aria-labelledby="workspace-sessions-title">
    <h2 id="workspace-sessions-title" class="text-xl font-semibold">
      {{ t('organization.workspace.proposal.sessions.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('organization.workspace.proposal.sessions.description') }}
    </p>

    <UiEmptyState
      v-if="props.sessions.length === 0"
      class="mt-6"
      icon="calendar"
      :title="t('organization.workspace.proposal.sessions.empty.title')"
      :description="t('organization.workspace.proposal.sessions.empty.description')"
    />

    <ol v-else class="mt-6 flex flex-col gap-5">
      <li
        v-for="tracked in props.sessions"
        :key="tracked.session.id"
        class="rounded-lg border border-border bg-surface-raised p-5"
      >
        <h3 class="text-lg leading-snug font-semibold text-heading">{{ tr(tracked.session.title) }}</h3>

        <dl class="mt-3 flex flex-col gap-2 text-sm sm:flex-row sm:flex-wrap sm:gap-x-6">
          <div class="flex items-center gap-2">
            <dt class="sr-only">{{ t('organization.workspace.proposal.sessions.slot') }}</dt>
            <UiIcon name="calendar" size="1rem" class="shrink-0 text-text-subtle" />
            <dd class="text-text-secondary">
              {{ date(tracked.session.starts_at, props.timezone) }} ·
              {{ timeRangeFull(tracked.session.starts_at, tracked.session.ends_at, props.timezone) }}
            </dd>
          </div>

          <div class="flex items-center gap-2">
            <dt class="sr-only">{{ t('organization.workspace.proposal.sessions.room') }}</dt>
            <UiIcon name="map-pin" size="1rem" class="shrink-0 text-text-subtle" />
            <dd class="text-text-secondary">
              {{
                tracked.room
                  ? tr(tracked.room.name)
                  : t('organization.workspace.proposal.sessions.online')
              }}
            </dd>
          </div>
        </dl>

        <div class="mt-4 flex flex-wrap items-center gap-x-5 gap-y-2 text-sm">
          <span class="inline-flex items-center gap-2 font-semibold text-text">
            <UiIcon name="users" size="1rem" class="text-text-subtle" />
            {{ t('organization.workspace.proposal.sessions.registered', tracked.registered_count) }}
          </span>
          <span v-if="tracked.waitlisted_count > 0" class="text-text-muted">
            {{ t('organization.workspace.proposal.sessions.waitlisted', tracked.waitlisted_count) }}
          </span>
          <span v-if="tracked.capacity !== null" class="text-text-muted">
            {{ t('organization.workspace.proposal.sessions.capacity', { count: tracked.capacity }) }}
          </span>
          <span v-if="tracked.session.attendee_count !== null" class="text-text-muted">
            {{ t('organization.workspace.proposal.sessions.attendance', { count: tracked.session.attendee_count }) }}
          </span>
        </div>

        <!-- Une activité tenue sans compte rendu : c'est la dernière chose que
             l'IFDD attend, et la plus facile à oublier une fois la COP finie. -->
        <UiAlert
          v-if="tracked.session.status === 'completed' && tracked.session.report_submitted_at === null"
          intent="warning"
          class="mt-4"
          compact
          :message="t('organization.workspace.proposal.sessions.reportMissing')"
        />

        <WorkspaceReminderSchedule
          class="mt-4"
          :reminders="tracked.reminders"
          :timezone="tracked.session.timezone"
        />
      </li>
    </ol>
  </section>
</template>
