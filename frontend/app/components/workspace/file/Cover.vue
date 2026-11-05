<script setup lang="ts">
import type { AttachableRoleRule, AttachedImage } from '~/types/media'
import type { AssetId, Uuid } from '~/types/shared'

/**
 * L'IMAGE DE COUVERTURE, première rubrique de « Votre dossier » (04/11 : elle ne
 * doit pas être négligée). Changer le fichier, rouvrir l'atelier, créer à partir
 * d'un modèle.
 *
 * Une couverture qui vient d'être déposée n'est servie qu'une fois ses formats
 * préparés, quelques secondes plus tard : on montre l'original en attendant,
 * on le dit, et la rubrique bascule seule sur la version finale (01/11).
 */

interface Props {
  proposalId: Uuid
  title: string
  cover: AttachedImage | null
  canEdit: boolean
  rule: AttachableRoleRule | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ changed: [] }>()

const { t } = useI18n()
const api = useApi()
const localePath = useLocalePath()

const coverError = ref<string | null>(null)

async function attachCover(assetId: AssetId | null): Promise<void> {
  if (!assetId) return
  coverError.value = null
  try {
    await api.media.attach({
      owner_schema: 'programme',
      owner_table: 'proposals',
      owner_id: props.proposalId,
      assignments: [{ role: 'cover', asset_id: assetId }],
    })
    emit('changed')
    await rereadPreparingCover()
  } catch (thrown) {
    coverError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

const { data: preparingCover, refresh: rereadPreparingCover } = useAsyncData(
  () => `proposal-file-cover-preparing-${props.proposalId}`,
  async () => {
    const [latest] = await api.media.attached({ schema: 'programme', table: 'proposals', id: props.proposalId }, 'cover')
    return latest && ['uploaded', 'scanning', 'processing'].includes(latest.status) ? latest : null
  },
  { lazy: true, server: false, watch: [() => props.proposalId], getCachedData: () => undefined },
)
const preparingPreviewFailed = ref(false)
let preparingTimer: ReturnType<typeof setTimeout> | null = null
watch(preparingCover, (pending, previous) => {
  if (preparingTimer) clearTimeout(preparingTimer)
  if (pending) preparingTimer = setTimeout(() => rereadPreparingCover(), 3000)
  else if (previous) emit('changed')
})
onBeforeUnmount(() => {
  if (preparingTimer) clearTimeout(preparingTimer)
})

/** L'atelier a-t-il déjà servi ? « Modifier » le rouvre là où on l'a laissé. */
const { data: coverStudio } = useAsyncData(
  () => `proposal-file-cover-studio-${props.proposalId}`,
  () => api.proposals.coverStudio(props.proposalId),
  { lazy: true, server: false, watch: [() => props.proposalId] },
)

const studioTo = computed(() => localePath(`/mon-organisation/dossiers/${props.proposalId}/couverture`))
</script>

<template>
  <section
    id="couverture"
    aria-labelledby="couverture-title"
    class="grid scroll-mt-24 grid-cols-1 items-start gap-6 border-t border-border-subtle py-7 first-of-type:border-t-0 md:grid-cols-[minmax(0,1.25fr)_minmax(0,1fr)]"
  >
    <div class="overflow-hidden rounded-lg bg-surface-sunken">
      <div v-if="preparingCover && !preparingPreviewFailed" class="relative">
        <img
          :src="preparingCover.url"
          :alt="props.title"
          class="aspect-video w-full object-cover"
          @error="preparingPreviewFailed = true"
        >
        <p
          role="status"
          class="absolute inset-x-3 bottom-3 flex items-center gap-2 rounded-md bg-surface-raised px-3 py-2 text-sm font-medium text-text shadow-sm"
        >
          <UiSpinner size="1rem" />
          {{ t('organization.workspace.proposal.cover.preparing') }}
        </p>
      </div>
      <UiImage
        v-else-if="props.cover"
        :image="props.cover"
        ratio="16 / 9"
        sizes="(min-width: 1024px) 520px, 100vw"
        loading="eager"
      />
      <div v-else class="grid aspect-video place-items-center text-text-subtle">
        <UiIcon name="image" size="2.5rem" />
      </div>
    </div>

    <div>
      <h3 id="couverture-title" class="font-condensed text-[1.45rem] leading-tight font-semibold text-heading">
        {{ t('organization.workspace.proposal.cover.title') }}
      </h3>
      <p class="mt-1 text-[0.9rem] text-text-muted">{{ t('organization.workspace.proposal.cover.description') }}</p>
      <div v-if="props.canEdit" class="mt-4 flex flex-wrap gap-2.5">
        <MediaImageField
          role="cover"
          :asset-id="null"
          :label="t('organization.workspace.proposal.cover.title')"
          :alt-fallback="props.title"
          :button-label="t(props.cover ? 'organization.workspace.proposal.cover.change' : 'organization.workspace.proposal.cover.add')"
          button-icon="upload"
          shape-label="16:9"
          button-variant="secondary"
          :rule="props.rule"
          :owner="{ schema: 'programme', table: 'proposals', id: props.proposalId }"
          @update:asset-id="attachCover"
        />
        <UiButton v-if="coverStudio?.design" variant="secondary" icon="edit" :to="studioTo">
          {{ t('organization.workspace.proposal.cover.edit') }}
        </UiButton>
        <UiButton variant="primary" :to="`${studioTo}?nouveau=1`">
          {{ t('organization.workspace.proposal.cover.create') }}
        </UiButton>
      </div>
      <UiAlert v-if="coverError" class="mt-3" intent="danger" :message="coverError" live />
    </div>
  </section>
</template>
