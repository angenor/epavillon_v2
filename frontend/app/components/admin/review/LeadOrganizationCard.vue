<script setup lang="ts">
import type { ProposalDashboard, ProposalOrganizationEntry } from '~/types/admin-review'

/**
 * L'ORGANISATION PORTEUSE, en carte latérale : sa fiche, son historique en une
 * ligne, ses co-organisatrices. « Modifier la fiche » ouvre le panneau partagé
 * avec la page de l'organisation ; « Changer » ne touche que le RATTACHEMENT.
 */

interface Props {
  entries: ProposalOrganizationEntry[]
  dashboard: ProposalDashboard
  canChange: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ change: []; edit: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const localePath = useLocalePath()

const lead = computed(() => props.entries.find((entry) => entry.link.role === 'lead') ?? null)
const others = computed(() => props.entries.filter((entry) => entry.link.role !== 'lead'))
const sheetTo = computed(() => localePath(`/admin/organisations/${props.dashboard.lead_organization_id}`))

const subtitle = computed(() =>
  [
    lead.value?.organization?.acronym,
    props.dashboard.lead_country_name ? tr(props.dashboard.lead_country_name) : null,
    props.dashboard.lead_type_label ? tr(props.dashboard.lead_type_label) : null,
  ]
    .filter(Boolean)
    .join(' · '),
)

const LINK = 'flex min-h-(--target-min) items-center justify-between gap-2 border-b border-border-subtle text-sm no-underline'
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised p-5">
    <span class="text-xs font-semibold tracking-caps text-text-subtle uppercase">
      {{ t('admin.proposal.dashboard.lead.eyebrow') }}
    </span>

    <div class="mt-3 flex items-center gap-4">
      <span class="flex size-18 shrink-0 items-center justify-center overflow-hidden rounded-md border border-border bg-surface p-1">
        <UiImage
          v-if="props.dashboard.lead_logo"
          :image="props.dashboard.lead_logo"
          ratio="1 / 1"
          sizes="72px"
          class="size-full object-contain"
        />
        <span v-else class="text-sm font-bold text-text-secondary">
          {{ lead?.organization?.acronym || initialsOf(lead?.organization?.legal_name ?? '') }}
        </span>
      </span>
      <div class="min-w-0">
        <p class="text-lg leading-snug font-semibold text-text">{{ lead?.organization?.legal_name ?? '' }}</p>
        <p v-if="subtitle" class="text-sm text-text-muted">{{ subtitle }}</p>
      </div>
    </div>

    <p v-if="lead?.track_record" class="mt-3 text-sm text-text-secondary">
      {{ t('admin.proposal.review.organizations.record.submitted', lead.track_record.propositions_deposees) }}
      <template v-if="lead.track_record.propositions_deposees > 0">
        ·
        {{ t('admin.proposal.review.organizations.record.accepted', { count: lead.track_record.propositions_acceptees }) }}
        ·
        {{ t('admin.proposal.review.organizations.record.editions', lead.track_record.evenements_couverts) }}
      </template>
    </p>

    <div class="mt-3 flex flex-col border-t border-border-subtle">
      <button type="button" :class="[LINK, 'cursor-pointer text-start text-accent']" @click="emit('edit')">
        {{ t('admin.proposal.dashboard.lead.edit') }}
        <UiIcon name="arrow-right" size="1rem" />
      </button>
      <NuxtLink :to="sheetTo" :class="LINK">
        {{ t('admin.proposal.dashboard.lead.open') }}
        <UiIcon name="external-link" size="1rem" />
      </NuxtLink>
      <button
        v-if="props.canChange"
        type="button"
        :class="[LINK, 'cursor-pointer text-start text-accent']"
        @click="emit('change')"
      >
        {{ t('admin.proposal.dashboard.lead.change') }}
        <UiIcon name="arrow-right" size="1rem" />
      </button>
    </div>

    <ul v-if="others.length" class="mt-3 text-sm">
      <li
        v-for="entry in others"
        :key="entry.link.organization_id"
        class="flex flex-wrap justify-between gap-x-2 gap-y-1 border-t border-border-subtle py-2"
      >
        <span class="font-semibold text-text">{{ entry.organization?.legal_name ?? '' }}</span>
        <span class="text-text-muted">
          {{ t(`admin.proposal.review.organizations.role.${entry.link.role}`) }}
          <template v-if="entry.link.confirmed_at === null">
            · {{ t('admin.proposal.dashboard.lead.pending') }}
          </template>
        </span>
      </li>
    </ul>
  </section>
</template>
