<script setup lang="ts">
import type { IncidentSeverity } from '~/types/live'
import type { PublicScheduleRow } from '~/types/views'

/**
 * Section « Motifs transversaux » — ce qui traverse tous les écrans et que
 * personne ne pense à traiter :
 *
 *   1. les quatre états d'écran, en composants de plein droit ;
 *   2. une date avec son fuseau ;
 *   3. le bandeau d'incident, sur toutes ses gravités ;
 *   4. la pastille « en direct » et sa règle d'usage.
 *
 * CE SONT EUX QU'ON OUBLIE, et ils font mauvaise impression en production
 * précisément parce qu'ils n'apparaissent que lorsque quelque chose ne va pas —
 * donc jamais pendant le développement, où tout va bien.
 */

interface Props {
  /** Une séance réelle, pour démontrer l'affichage des dates. */
  session?: PublicScheduleRow | null
  zoneLabel: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { liveSessionId } = useLiveSession()

/** Quatre gravités — le modèle en déclare quatre, pas trois. */
const SEVERITIES: IncidentSeverity[] = ['info', 'warning', 'error', 'critical']

const retrying = ref(false)
function simulateRetry(): void {
  retrying.value = true
  setTimeout(() => (retrying.value = false), 1800)
}

/** Dates de démonstration : un créneau ordinaire, un créneau qui franchit minuit. */
const sameDay = { start: '2027-11-14T14:30:00-03:00', end: '2027-11-14T16:00:00-03:00' }
const overnight = { start: '2027-11-14T22:00:00-03:00', end: '2027-11-15T01:30:00-03:00' }
</script>

<template>
  <StyleGuideSection
    id="motifs"
    :title="t('style-guide.patterns.title')"
    :description="t('style-guide.patterns.description')"
  >
    <!-- 1. LES QUATRE ÉTATS D'ÉCRAN -->
    <StyleGuideDemo
      :title="t('style-guide.patterns.states.title')"
      :note="t('style-guide.patterns.states.note')"
      surface
    >
      <div class="grid gap-4 lg:grid-cols-2">
        <!-- Chargement -->
        <div>
          <p class="mb-2 font-mono text-xs text-text-subtle">1 — {{ t('style-guide.patterns.states.loading') }}</p>
          <div class="rounded-lg border border-border bg-surface-raised p-4" aria-busy="true">
            <UiSkeletonLoader width="45%" height="1.4rem" />
            <div class="mt-3">
              <UiSkeletonLoader variant="text" :lines="3" />
            </div>
            <div class="mt-4 flex gap-2">
              <UiSkeletonLoader width="6rem" height="2.25rem" />
              <UiSkeletonLoader width="6rem" height="2.25rem" />
            </div>
          </div>
        </div>

        <!-- Vide -->
        <div>
          <p class="mb-2 font-mono text-xs text-text-subtle">2 — {{ t('style-guide.patterns.states.empty') }}</p>
          <UiEmptyState
            icon="document"
            :title="t('style-guide.patterns.states.emptyTitle')"
            :description="t('style-guide.patterns.states.emptyDescription')"
            :action-label="t('style-guide.patterns.states.emptyAction')"
            compact
          />
        </div>

        <!-- Erreur -->
        <div>
          <p class="mb-2 font-mono text-xs text-text-subtle">3 — {{ t('style-guide.patterns.states.error') }}</p>
          <UiErrorState
            compact
            :request-id="'01K3Q7YV9M2B4T8F'"
            :retrying="retrying"
            @retry="simulateRetry"
          />
        </div>

        <!-- Accès refusé -->
        <div>
          <p class="mb-2 font-mono text-xs text-text-subtle">4 — {{ t('style-guide.patterns.states.forbidden') }}</p>
          <UiForbiddenState
            compact
            :required-scope="t('style-guide.patterns.states.forbiddenScope')"
            contact="ifdd-support@francophonie.org"
          />
        </div>
      </div>

      <UiAlert intent="info" class="mt-4">
        {{ t('style-guide.patterns.states.forbiddenNote') }}
      </UiAlert>
    </StyleGuideDemo>

    <!-- 2. UNE DATE AVEC SON FUSEAU -->
    <StyleGuideDemo
      :title="t('style-guide.patterns.datetime.title')"
      :note="t('style-guide.patterns.datetime.note')"
    >
      <dl class="space-y-3 text-sm">
        <div class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-4">
          <dt class="w-44 shrink-0 font-mono text-xs text-text-subtle">format="full"</dt>
          <dd>
            <UiZonedTime
              :start="sameDay.start"
              :end="sameDay.end"
              timezone="America/Belem"
              :zone-label="props.zoneLabel"
              icon
            />
          </dd>
        </div>

        <div class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-4">
          <dt class="w-44 shrink-0 font-mono text-xs text-text-subtle">format="short"</dt>
          <dd>
            <UiZonedTime
              :start="sameDay.start"
              :end="sameDay.end"
              timezone="America/Belem"
              :zone-label="props.zoneLabel"
              format="short"
            />
          </dd>
        </div>

        <div class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-4">
          <dt class="w-44 shrink-0 font-mono text-xs text-text-subtle">format="withDate"</dt>
          <dd>
            <UiZonedTime
              :start="sameDay.start"
              :end="sameDay.end"
              timezone="America/Belem"
              :zone-label="props.zoneLabel"
              format="withDate"
            />
          </dd>
        </div>

        <div class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-4">
          <dt class="w-44 shrink-0 font-mono text-xs text-text-subtle">
            {{ t('style-guide.patterns.datetime.overnight') }}
          </dt>
          <dd>
            <UiZonedTime
              :start="overnight.start"
              :end="overnight.end"
              timezone="America/Belem"
              :zone-label="props.zoneLabel"
            />
          </dd>
        </div>

        <div class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-4">
          <dt class="w-44 shrink-0 font-mono text-xs text-text-subtle">
            {{ t('style-guide.patterns.datetime.otherZone') }}
          </dt>
          <dd>
            <UiZonedTime
              :start="sameDay.start"
              :end="sameDay.end"
              timezone="Europe/Paris"
              zone-label="Paris"
            />
          </dd>
        </div>

        <div v-if="props.session" class="flex flex-col gap-1 sm:flex-row sm:items-baseline sm:gap-4">
          <dt class="w-44 shrink-0 font-mono text-xs text-text-subtle">
            {{ t('style-guide.patterns.datetime.real') }}
          </dt>
          <dd>
            <UiZonedTime
              :start="props.session.starts_at"
              :end="props.session.ends_at"
              :timezone="props.session.timezone"
              :zone-label="props.zoneLabel"
              format="withDate"
            />
          </dd>
        </div>
      </dl>

      <UiAlert intent="warning" class="mt-4">
        {{ t('style-guide.patterns.datetime.rule') }}
      </UiAlert>
    </StyleGuideDemo>

    <!-- 3. BANDEAU D'INCIDENT -->
    <StyleGuideDemo
      :title="t('style-guide.patterns.incidents.title')"
      :note="t('style-guide.patterns.incidents.note')"
      surface
      flush
    >
      <div class="space-y-px">
        <UiIncidentBanner
          v-for="severity in SEVERITIES"
          :key="severity"
          :severity="severity"
          :scope="severity === 'info' ? 'global' : severity === 'critical' ? 'event' : 'session'"
          :title="{ fr: t(`style-guide.patterns.incidents.samples.${severity}.title`) }"
          :message="{ fr: t(`style-guide.patterns.incidents.samples.${severity}.message`) }"
          :action-url="severity === 'warning' ? 'https://ifdd.francophonie.org' : null"
          dismissible
          :display-until="severity === 'info' ? '2027-11-20T23:59:00-03:00' : null"
          timezone="America/Belem"
        />
      </div>
    </StyleGuideDemo>

    <StyleGuideDemo :title="t('style-guide.patterns.incidents.rulesTitle')">
      <ul class="space-y-2 text-sm text-text-muted">
        <li v-for="index in 4" :key="index" class="flex gap-2">
          <UiIcon name="check" size="1rem" class="mt-0.5 shrink-0 text-success" />
          {{ t(`style-guide.patterns.incidents.rules.r${index}`) }}
        </li>
      </ul>
    </StyleGuideDemo>

    <!-- 4. LA PASTILLE « EN DIRECT » -->
    <StyleGuideDemo
      :title="t('style-guide.patterns.live.title')"
      :note="t('style-guide.patterns.live.note')"
    >
      <div class="flex flex-wrap items-center gap-4">
        <UiLiveBadge force />
        <UiLiveBadge force size="sm" />
        <span class="text-sm text-text-muted">
          {{
            liveSessionId
              ? t('style-guide.patterns.live.current', { id: liveSessionId })
              : t('style-guide.patterns.live.none')
          }}
        </span>
      </div>

      <ul class="mt-4 space-y-2 text-sm text-text-muted">
        <li v-for="index in 4" :key="index" class="flex gap-2">
          <UiIcon name="check" size="1rem" class="mt-0.5 shrink-0 text-success" />
          {{ t(`style-guide.patterns.live.rules.r${index}`) }}
        </li>
      </ul>

      <UiAlert intent="danger" class="mt-4" :title="t('style-guide.patterns.live.warningTitle')">
        {{ t('style-guide.patterns.live.warning') }}
      </UiAlert>
    </StyleGuideDemo>
  </StyleGuideSection>
</template>
