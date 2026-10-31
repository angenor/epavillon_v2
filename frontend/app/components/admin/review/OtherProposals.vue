<script setup lang="ts">
import type { OrganizationOtherProposal } from '~/types/admin-review'
import type { OrganizationId } from '~/types/shared'

/**
 * LES AUTRES DOSSIERS DE L'ORGANISATION PORTEUSE, comme en v1 : titre, édition,
 * statut, dates, déposant avec sa photo et son adresse. Ceux d'une édition hors
 * périmètre ne sont pas envoyés (règle 8). Tableau sur grand écran, cartes
 * empilées sur téléphone.
 */

interface Props {
  entries: OrganizationOtherProposal[]
  organizationId: OrganizationId
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { timeWithZone, date } = useDateTime()
const localePath = useLocalePath()

function editionLabel(entry: OrganizationOtherProposal): string {
  return entry.event_acronym || tr(entry.event_title)
}

function zoneOf(entry: OrganizationOtherProposal): string {
  return entry.city?.trim() || entry.timezone
}

/** « 14 novembre 2025, 11:00, heure de Belém » : toute date porte son fuseau. */
function stamp(value: string, entry: OrganizationOtherProposal): string {
  return t('admin.proposal.dashboard.others.stamp', {
    date: date(value, entry.timezone),
    time: timeWithZone(value, entry.timezone, zoneOf(entry)),
  })
}

function retained(entry: OrganizationOtherProposal): string {
  if (entry.retained_starts_at) return stamp(entry.retained_starts_at, entry)
  return entry.status === 'accepted'
    ? t('admin.proposal.dashboard.others.notPlaced')
    : t('admin.proposal.dashboard.others.noSlot')
}

const linkTo = (entry: OrganizationOtherProposal) => localePath(`/admin/propositions/${entry.id}`)
const organizationTo = computed(() => localePath(`/admin/organisations/${props.organizationId}`))
</script>

<template>
  <section class="@container" aria-labelledby="dashboard-others-title">
    <h2
      id="dashboard-others-title"
      class="flex flex-wrap items-baseline gap-x-3 gap-y-1 border-b border-border pb-2 text-lg font-semibold"
    >
      {{ t('admin.proposal.dashboard.others.title') }}
      <small class="text-sm font-normal text-text-muted">{{ props.entries.length }}</small>
      <NuxtLink :to="organizationTo" class="ms-auto text-sm font-normal">
        {{ t('admin.proposal.dashboard.others.all') }}
      </NuxtLink>
    </h2>

    <p v-if="props.entries.length === 0" class="mt-3 text-sm text-text-muted">
      {{ t('admin.proposal.dashboard.others.empty') }}
    </p>

    <template v-else>
      <!-- Grand écran : un tableau, la densité est assumée. -->
      <table class="mt-3 hidden w-full border-collapse text-sm @xl:table">
        <thead>
          <tr class="border-b border-border text-left text-text-muted">
            <th class="py-2 pe-3 font-semibold">{{ t('admin.proposal.dashboard.others.columns.proposal') }}</th>
            <th class="py-2 pe-3 font-semibold">{{ t('admin.proposal.dashboard.others.columns.status') }}</th>
            <th class="py-2 pe-3 font-semibold">{{ t('admin.proposal.dashboard.others.columns.dates') }}</th>
            <th class="py-2 font-semibold">{{ t('admin.proposal.dashboard.others.columns.submitter') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="entry in props.entries" :key="entry.id" class="border-b border-border-subtle align-top">
            <td class="py-3 pe-3">
              <NuxtLink :to="linkTo(entry)" class="font-semibold">{{ tr(entry.title) }}</NuxtLink>
              <p class="mt-0.5 text-text-muted">
                <span class="font-mono">{{ entry.reference_code }}</span> · {{ editionLabel(entry) }}
              </p>
            </td>
            <td class="py-3 pe-3">
              <UiBadge
                size="sm"
                :intent="PROPOSAL_STATUS_INTENT[entry.status]"
                :label="t(`admin.proposal.review.status.${entry.status}`)"
              />
            </td>
            <td class="py-3 pe-3 text-text-secondary">
              <p>{{ t('admin.proposal.dashboard.others.createdAt', { date: stamp(entry.created_at, entry) }) }}</p>
              <p class="mt-0.5">{{ t('admin.proposal.dashboard.others.retainedAt', { slot: retained(entry) }) }}</p>
            </td>
            <td class="py-3">
              <div v-if="entry.submitter" class="flex items-start gap-3">
                <AdminReviewPersonAvatar :name="entry.submitter.display_name" :photo="entry.submitter.photo" size="2.25rem" />
                <div class="min-w-0">
                  <p class="font-semibold text-text">{{ entry.submitter.display_name }}</p>
                  <AdminReviewEmailAddress :email="entry.submitter.email" />
                </div>
              </div>
            </td>
          </tr>
        </tbody>
      </table>

      <!-- Téléphone : une carte par dossier, sans défilement horizontal. -->
      <ul class="mt-3 flex flex-col gap-3 @xl:hidden">
        <li
          v-for="entry in props.entries"
          :key="entry.id"
          class="flex flex-col gap-2 rounded-lg border border-border bg-surface-raised p-4"
        >
          <NuxtLink :to="linkTo(entry)" class="font-semibold">{{ tr(entry.title) }}</NuxtLink>
          <p class="text-sm text-text-muted">
            <span class="font-mono">{{ entry.reference_code }}</span> · {{ editionLabel(entry) }}
          </p>
          <div v-if="entry.submitter" class="flex items-center gap-3">
            <AdminReviewPersonAvatar :name="entry.submitter.display_name" :photo="entry.submitter.photo" size="2.25rem" />
            <p class="min-w-0 font-semibold text-text">{{ entry.submitter.display_name }}</p>
          </div>
          <AdminReviewEmailAddress v-if="entry.submitter" :email="entry.submitter.email" />
          <div>
            <UiBadge
              size="sm"
              :intent="PROPOSAL_STATUS_INTENT[entry.status]"
              :label="t(`admin.proposal.review.status.${entry.status}`)"
            />
          </div>
          <p class="text-sm text-text-secondary">
            {{ t('admin.proposal.dashboard.others.createdAt', { date: stamp(entry.created_at, entry) }) }}
          </p>
          <p class="text-sm text-text-secondary">
            {{ t('admin.proposal.dashboard.others.retainedAt', { slot: retained(entry) }) }}
          </p>
        </li>
      </ul>
    </template>
  </section>
</template>
