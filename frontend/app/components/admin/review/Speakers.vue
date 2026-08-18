<script setup lang="ts">
import type { ProposalSpeakerEntry } from '~/types/admin-review'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES INTERVENANTS ANNONCÉS.
 *
 * LA CONFIRMATION EST L'INFORMATION PRINCIPALE, pas la biographie. Un panel de
 * cinq noms dont aucun n'a répondu ne vaut pas un panel de trois confirmés :
 * c'est ce que le critère de faisabilité juge, et `proposal_speakers.confirmed_at`
 * le porte. Trois états, trois messages — confirmé, invité sans réponse, pas
 * encore invité.
 *
 * LA FONCTION ET L'ORGANISATION AFFICHÉES SONT CELLES DU DOSSIER
 * (`job_title_snapshot`, `organization_snapshot`), et non celles du profil : une
 * personne change d'employeur, et l'archive d'une COP ne doit pas se réécrire
 * pour autant. C'est aussi ce qui permet de repérer qu'un intervenant est
 * annoncé sous une casquette qu'il n'a plus.
 */

interface Props {
  entries: ProposalSpeakerEntry[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()
</script>

<template>
  <section aria-labelledby="review-speakers-title">
    <h3 id="review-speakers-title" class="text-sm font-semibold tracking-wide uppercase">
      {{ t('admin.proposal.review.speakers.title') }}
    </h3>

    <p v-if="props.entries.length === 0" class="mt-3 text-sm text-text-muted">
      {{ t('admin.proposal.review.speakers.empty') }}
    </p>

    <ul v-else class="mt-3 flex flex-col divide-y divide-border-subtle">
      <li v-for="entry in props.entries" :key="entry.speaker.id" class="py-3">
        <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <span class="font-medium text-text">
            {{ entry.person?.display_name ?? '' }}
          </span>
          <UiBadge
            size="sm"
            :label="t(`admin.proposal.review.speakers.role.${entry.speaker.role}`)"
          />
          <UiBadge
            v-if="entry.speaker.confirmed_at"
            size="sm"
            intent="success"
            icon="check"
            :label="
              t('admin.proposal.review.speakers.confirmed', {
                date: date(entry.speaker.confirmed_at, props.timezone),
              })
            "
          />
          <UiBadge
            v-else-if="entry.speaker.confirmation_sent_at"
            size="sm"
            intent="warning"
            :label="
              t('admin.proposal.review.speakers.invited', {
                date: date(entry.speaker.confirmation_sent_at, props.timezone),
              })
            "
          />
          <UiBadge
            v-else
            size="sm"
            intent="neutral"
            :label="t('admin.proposal.review.speakers.notInvited')"
          />
        </div>

        <p class="mt-1 text-sm text-text-muted">
          <span v-if="entry.speaker.job_title_snapshot">{{ entry.speaker.job_title_snapshot }}</span>
          <span v-if="entry.speaker.job_title_snapshot && entry.speaker.organization_snapshot"> — </span>
          <span v-if="entry.speaker.organization_snapshot">{{ entry.speaker.organization_snapshot }}</span>
        </p>

        <p v-if="entry.speaker.bio" class="mt-1 max-w-(--measure) text-sm text-text-secondary">
          {{ tr(entry.speaker.bio) }}
        </p>

        <p v-if="entry.speaker.is_available_for_questions" class="mt-1 text-xs text-text-subtle">
          {{ t('admin.proposal.review.speakers.questions') }}
        </p>
      </li>
    </ul>
  </section>
</template>
