<script setup lang="ts">
import type { ProposalFile } from '~/types/organization-workspace'
import type { ProposalStatus } from '~/types/programme/proposal'
import type { AttachedImage } from '~/types/media'
import type { TabItem } from '~/types/ui'

/**
 * Le bandeau d'un dossier (04/11), repris de celui d'une activité : photo de
 * l'édition voilée, état et format en pastilles, grand titre condensé, lieu et
 * date, organisation porteuse en tuile blanche ; les vues « Mon dossier » et
 * « Historique » s'y posent.
 */

interface Props {
  file: ProposalFile
  image: AttachedImage | null
  /** Logo clair de l'édition, en face du fil d'Ariane. */
  editionLogo: AttachedImage | null
  /** Nom de l'organisation porteuse. */
  organizationName: string
  /** Sigle, dans la tuile quand l'organisation n'a pas de logo. */
  organizationAcronym?: string | null
  /** Retenue puis annulée : la pastille dit « Annulée », c'est l'activité qui l'est. */
  activityCancelled?: boolean
  view: string
  panelId: string
}

const props = withDefaults(defineProps<Props>(), { organizationAcronym: null, activityCancelled: false })
const emit = defineEmits<{ 'update:view': [value: string] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const { intlLocale } = useDateTime()
const localePath = useLocalePath()

const tracking = computed(() => props.file.tracking)
const edition = computed(() => tracking.value.edition)
const editionName = computed(() => edition.value.acronym ?? tr(edition.value.title))

// « En évaluation » garde le cyan de l'information (arbitré le 04/11).
const DOTS: Record<ProposalStatus, string> = {
  draft: 'bg-text-on-inverse-muted',
  submitted: 'bg-accent-on-inverse',
  under_review: 'bg-accent-on-inverse',
  changes_requested: 'bg-warning-on-inverse',
  accepted: 'bg-success-solid',
  rejected: 'bg-danger-solid',
  withdrawn: 'bg-text-on-inverse-muted',
  cancelled: 'bg-text-on-inverse-muted',
}

const statusLabel = computed(() =>
  props.activityCancelled
    ? t('organization.workspace.proposals.timeline.activityCancelled')
    : t(`organization.workspace.proposals.timeline.${tracking.value.proposal.status}`),
)

const FORMAT_ICONS: Record<string, string> = { in_person: 'map-pin', online: 'globe', hybrid: 'globe' }

const first = computed(() => tracking.value.sessions[0] ?? null)

const place = computed(() => {
  const room = first.value?.room
  return room ? [tr(room.name), edition.value.city].filter(Boolean).join(', ') : ''
})

const day = computed(() =>
  first.value
    ? formatDate(first.value.session.starts_at, { locale: intlLocale.value, timeZone: edition.value.timezone, dateStyle: 'full' })
    : [editionName.value, edition.value.city].filter(Boolean).join(', '),
)

const logo = computed(() => {
  const asset = props.file.organization_logo
  return asset ? (asset.sources.thumb?.url ?? asset.url) : null
})

const tabs = computed<TabItem[]>(() => [
  { value: 'tracking', label: t('organization.workspace.proposal.tabs.tracking'), icon: 'document' },
  { value: 'history', label: t('organization.workspace.proposal.tabs.history'), icon: 'clock' },
])

const title = computed(() => tr(tracking.value.proposal.title))
</script>

<template>
  <!-- Bas réduit sur une colonne : rien n'y remonte (les cartes ne débordent qu'à partir de `lg`). -->
  <UiMediaBanner
    class="max-lg:pb-9!"
    :image="props.image"
    :back-to="localePath('/mon-organisation')"
    :back-label="t('organization.workspace.proposal.breadcrumb')"
    :trail="t('organization.workspace.proposal.hero.trail', { edition: editionName, code: tracking.proposal.reference_code })"
    :logo="props.editionLogo"
    :logo-alt="t('organization.workspace.proposal.hero.editionLogo', { edition: editionName })"
  >
    <div class="mt-5 flex flex-wrap gap-2">
      <UiBannerPill :dot="DOTS[tracking.proposal.status]">{{ statusLabel }}</UiBannerPill>
      <UiBannerPill v-if="tracking.proposal.format">
        <UiIcon :name="FORMAT_ICONS[tracking.proposal.format] ?? 'globe'" size="0.9375rem" :stroke-width="2" />
        {{ t(`organization.workspace.proposal.tracking.formatValue.${tracking.proposal.format}`) }}
      </UiBannerPill>
    </div>

    <h1
      class="mt-4 max-w-[21em] font-condensed text-[clamp(1.85rem,4.2vw,3.5rem)] leading-[1.02] font-semibold tracking-[0.005em] text-balance text-text-on-inverse"
      :class="{ 'line-through decoration-2': tracking.proposal.status === 'cancelled' }"
    >
      {{ title }}
    </h1>

    <ul class="mt-4.5 flex flex-wrap items-center gap-x-6.5 gap-y-2.5 text-[0.95rem] text-text-on-inverse">
      <li v-if="place" class="inline-flex items-center gap-2">
        <UiIcon name="map-pin" size="1rem" :stroke-width="2" class="text-accent-on-inverse" />
        {{ place }}
      </li>
      <li class="inline-flex items-center gap-2">
        <UiIcon name="calendar" size="1rem" :stroke-width="2" class="text-accent-on-inverse" />
        <span class="inline-block first-letter:uppercase">{{ day }}</span>
      </li>
    </ul>

    <div class="mt-5.5 flex items-center gap-3.5">
      <span
        class="flex h-16 max-w-[45%] min-w-16 shrink-0 items-center justify-center rounded-lg bg-surface-inverse-selected p-1.5 shadow-glass"
        aria-hidden="true"
      >
        <img v-if="logo" :src="logo" alt="" class="max-h-13 max-w-full object-contain sm:max-w-44">
        <span v-else class="px-2 font-condensed text-lg font-bold tracking-[0.02em] text-text-on-inverse-selected">
          {{ props.organizationAcronym ?? initialsOf(props.organizationName) }}
        </span>
      </span>
      <p class="min-w-0">
        <span class="block font-condensed text-[0.95rem] font-semibold tracking-[0.05em] text-text-on-inverse-muted uppercase">
          {{ t('organization.workspace.proposal.hero.ledBy') }}
        </span>
        <b class="block text-[1.02rem] font-bold">{{ props.organizationName }}</b>
      </p>
    </div>

    <UiTabs
      class="mt-7"
      tone="banner"
      :items="tabs"
      :model-value="props.view"
      :label="t('organization.workspace.proposal.tabs.label')"
      :panel-id="() => props.panelId"
      @update:model-value="(value) => emit('update:view', value)"
    />
  </UiMediaBanner>
</template>
