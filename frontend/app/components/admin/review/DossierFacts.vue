<script setup lang="ts">
import type { Proposal } from '~/types/programme/proposal'
import type { ScheduleThemeBadge } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/**
 * LA FICHE SIGNALÉTIQUE DU DOSSIER, en marge du texte : thématiques, créneau
 * souhaité, langues, pays, déposant, contraintes de programmation.
 *
 * LE CRÉNEAU SOUHAITÉ PORTE SON FUSEAU, celui de l'ÉDITION : une session demandée
 * « à 14 h » l'est à Belém, pas à Québec.
 *
 * LES CONTRAINTES DE PROGRAMMATION SONT UN AVERTISSEMENT, pas un paragraphe de
 * plus : elles décideront du créneau retenu, et le planificateur les relira.
 */

interface Props {
  proposal: Proposal
  themes: ScheduleThemeBadge[]
  timezone: TimeZoneName
  /** Nom de ville de l'édition, qui NOMME le fuseau (« heure de Belém »). */
  zoneLabel: string
  countryName: string | null
  submitterName: string | null
  contactName: string | null
}

const props = defineProps<Props>()

const { t } = useI18n()

const hasSlot = computed(
  () => props.proposal.preferred_start_at !== null && props.proposal.preferred_end_at !== null,
)

const TERM = 'text-xs tracking-caps text-text-subtle uppercase'
</script>

<template>
  <section aria-labelledby="dossier-facts">
    <h2 id="dossier-facts" class="sr-only">{{ t('admin.proposal.review.dossier.facts') }}</h2>

    <dl class="flex flex-col divide-y divide-border-subtle">
      <div v-if="props.themes.length > 0" class="pb-3">
        <dt :class="TERM">{{ t('admin.proposal.review.dossier.themes') }}</dt>
        <dd class="mt-1.5">
          <UiThemeTagList :themes="props.themes" :max="6" size="sm" />
        </dd>
      </div>

      <div class="py-3 first:pt-0">
        <dt :class="TERM">{{ t('admin.proposal.review.dossier.slot') }}</dt>
        <dd class="mt-1">
          <UiZonedTime
            v-if="hasSlot"
            :start="props.proposal.preferred_start_at!"
            :end="props.proposal.preferred_end_at"
            :timezone="props.timezone"
            :zone-label="props.zoneLabel"
            format="withDate"
          />
          <span v-else class="text-text-muted">{{ t('admin.proposal.review.dossier.noSlot') }}</span>
          <p v-if="props.proposal.duration_minutes" class="text-sm text-text-subtle">
            {{ t('admin.proposal.review.dossier.duration', { count: props.proposal.duration_minutes }) }}
            <template v-if="props.proposal.requested_sessions > 1">
              ·
              {{ t('admin.proposal.review.dossier.sessionsRequested', props.proposal.requested_sessions) }}
            </template>
          </p>
        </dd>
      </div>

      <div class="py-3">
        <dt :class="TERM">{{ t('admin.proposal.review.dossier.languages') }}</dt>
        <dd class="mt-1 text-text-secondary">
          {{ props.proposal.language_codes.join(' · ').toUpperCase() }}
        </dd>
      </div>

      <div v-if="props.countryName" class="py-3">
        <dt :class="TERM">{{ t('admin.proposal.review.dossier.country') }}</dt>
        <dd class="mt-1 text-text-secondary">{{ props.countryName }}</dd>
      </div>

      <div v-if="props.submitterName" class="py-3">
        <dt :class="TERM">{{ t('admin.proposal.review.dossier.submitter') }}</dt>
        <dd class="mt-1 text-text-secondary">
          {{ props.submitterName }}
          <span
            v-if="props.contactName && props.contactName !== props.submitterName"
            class="block text-sm text-text-subtle"
          >
            {{ t('admin.proposal.review.dossier.contact', { name: props.contactName }) }}
          </span>
        </dd>
      </div>
    </dl>

    <UiAlert
      v-if="props.proposal.scheduling_constraints"
      class="mt-3"
      intent="warning"
      icon="calendar"
      :title="t('admin.proposal.review.dossier.constraints')"
      :message="props.proposal.scheduling_constraints"
    />
  </section>
</template>
