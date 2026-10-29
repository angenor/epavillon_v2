<script setup lang="ts">
import type { ProposalDocumentEntry } from '~/types/admin-review'
import type { AssetId, Uuid } from '~/types/shared'
import type { UploadedAsset } from '~/types/media'

/**
 * LES PRÉSENTATIONS PROJETÉES pendant l'activité (29/10) : des documents du
 * dossier de type `presentation`. Le fichier part d'abord (`MediaFileField`),
 * puis se rattache au dossier ; c'est la page qui fait l'appel et recharge.
 */

interface Props {
  proposalId: Uuid
  documents: ProposalDocumentEntry[]
  canEdit: boolean
  busy: boolean
  error: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ add: [assetId: AssetId, title: string]; remove: [documentId: Uuid] }>()

const { t, locale } = useI18n()
const { tr } = useI18nText()

const TYPES = [
  'application/pdf',
  'application/vnd.ms-powerpoint',
  'application/vnd.openxmlformats-officedocument.presentationml.presentation',
] as const

const presentations = computed(() =>
  props.documents.filter((entry) => entry.document.document_type_code === 'presentation'),
)

const fieldKey = ref(0)

function onUploaded(asset: UploadedAsset): void {
  emit('add', asset.id, asset.original_filename ?? '')
  // Champ remis à neuf : la présentation suivante part d'un emplacement vide.
  fieldKey.value += 1
}
</script>

<template>
  <section
    id="presentations"
    aria-labelledby="presentations-title"
    class="scroll-mt-24 rounded-lg border border-border bg-surface-raised p-5 sm:p-6"
  >
    <h2 id="presentations-title" class="text-xl font-semibold text-heading">
      {{ t('organization.workspace.proposal.presentations.title') }}
    </h2>
    <p class="mt-1 text-sm text-text-muted">{{ t('organization.workspace.proposal.presentations.description') }}</p>

    <ul v-if="presentations.length > 0" class="mt-4 flex flex-col gap-2">
      <li
        v-for="entry in presentations"
        :key="entry.document.id"
        class="flex flex-wrap items-center gap-3 rounded-md border border-border p-3"
      >
        <UiIcon name="document" size="1.5rem" class="shrink-0 text-text-subtle" />
        <div class="min-w-0 flex-1">
          <a v-if="entry.url" :href="entry.url" target="_blank" rel="noopener" class="font-semibold break-words">
            {{ tr(entry.document.title) }}
          </a>
          <p v-else class="font-semibold break-words text-text">{{ tr(entry.document.title) }}</p>
          <p v-if="entry.asset" class="text-sm text-text-muted">{{ formatByteSize(entry.asset.byte_size, locale) }}</p>
        </div>
        <UiButton
          v-if="props.canEdit"
          variant="ghost"
          icon="trash"
          :disabled="props.busy"
          @click="emit('remove', entry.document.id)"
        >
          {{ t('organization.workspace.proposal.presentations.remove') }}
        </UiButton>
      </li>
    </ul>
    <p v-else class="mt-4 text-text-muted">{{ t('organization.workspace.proposal.presentations.empty') }}</p>

    <MediaFileField
      v-if="props.canEdit"
      :key="fieldKey"
      class="mt-4"
      :label="t('organization.workspace.proposal.presentations.add')"
      :types="TYPES"
      :asset-id="null"
      :owner="{ schema: 'programme', table: 'proposals', id: props.proposalId }"
      :disabled="props.busy"
      @uploaded="onUploaded"
    />
    <UiAlert v-if="props.error" class="mt-3" intent="danger" :message="props.error" live />
  </section>
</template>
