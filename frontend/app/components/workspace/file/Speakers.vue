<script setup lang="ts">
import type { AttachableRoleRule } from '~/types/media'
import type { DraftSpeaker } from '~/types/proposal-form'
import type { AssetId } from '~/types/shared'

/**
 * LES INTERVENANTS EN LECTURE, avec leur photo (29/10).
 *
 * La photo se pose ici, sans ouvrir la rubrique : c'est le manque le plus
 * fréquent, et un geste à part d'un dossier qui s'enregistre en entier. Elle va
 * sur la LIGNE de l'intervenant (`programme.proposal_speakers`, rôle `avatar`),
 * que l'organisation peut toujours poser, même pour quelqu'un qui a un compte.
 */

interface Props {
  speakers: DraftSpeaker[]
  canSetPhoto: boolean
  photoRule: AttachableRoleRule | null
  /** Activité retenue : chaque intervenant peut être coché comme point focal. */
  focalSpeakerIds?: string[] | null
  focalBusy?: boolean
}

const props = withDefaults(defineProps<Props>(), { focalSpeakerIds: null, focalBusy: false })
const emit = defineEmits<{ photoSet: []; toggleFocal: [speakerId: string, on: boolean] }>()

const { t } = useI18n()
const api = useApi()

// Une case refusée (le dernier point focal) garde sa valeur : sans nouveau rendu, le navigateur la laisserait décochée.
const focalRender = ref(0)

function toggleFocal(speakerId: string, on: boolean): void {
  emit('toggleFocal', speakerId, on)
  focalRender.value += 1
}
const photoError = ref<string | null>(null)

const hasBio = (speaker: DraftSpeaker) => plainTextOf(speaker.bio).trim().length > 0

async function attachPhoto(speakerId: string, assetId: AssetId | null): Promise<void> {
  if (!assetId) return
  photoError.value = null
  try {
    await api.media.attach({
      owner_schema: 'programme',
      owner_table: 'proposal_speakers',
      owner_id: speakerId,
      assignments: [{ role: 'avatar', asset_id: assetId }],
    })
    emit('photoSet')
  } catch (thrown) {
    photoError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}
</script>

<template>
  <ul class="flex flex-col">
    <li
      v-for="speaker in props.speakers"
      :key="speaker.key"
      class="flex flex-col gap-3 border-t border-border-subtle py-4 first:border-t-0 first:pt-0 sm:flex-row sm:items-start sm:gap-4"
    >
      <UiImage
        v-if="speaker.photo_image"
        :image="speaker.photo_image"
        ratio="1 / 1"
        sizes="64px"
        rounded="rounded-full"
        class="w-16 shrink-0"
        aria-hidden="true"
      />
      <span
        v-else
        aria-hidden="true"
        class="grid size-16 shrink-0 place-items-center rounded-full border-2 border-dashed border-warning-border bg-warning-surface text-lg font-bold text-warning"
      >
        {{ initialsOf(speakerName(speaker)) }}
      </span>

      <div class="min-w-0 flex-1">
        <p class="font-semibold text-heading">
          {{ speakerName(speaker) }}
          <span class="font-normal text-text-muted">· {{ t(`proposal.form.step-speakers.roles.${speaker.role}.label`) }}</span>
        </p>
        <p v-if="speaker.job_title || speaker.organization_name" class="mt-0.5 text-sm text-text-muted">
          {{ [speaker.job_title, speaker.organization_name].filter(Boolean).join(', ') }}
        </p>
        <p v-if="hasBio(speaker)" class="mt-2 line-clamp-3 text-sm text-text-secondary">{{ plainTextOf(speaker.bio) }}</p>

        <p v-if="!hasBio(speaker) || !speaker.photo_image" class="mt-2 flex flex-wrap gap-2">
          <UiBadge v-if="!speaker.photo_image" intent="warning" :label="t('organization.workspace.proposal.speakers.photoMissing')" />
          <UiBadge v-if="!hasBio(speaker)" intent="warning" :label="t('organization.workspace.proposal.speakers.bioMissing')" />
        </p>

        <UiCheckbox
          v-if="props.focalSpeakerIds && speaker.speaker_id"
          :id="`focal-${speaker.speaker_id}`"
          :key="`focal-${speaker.speaker_id}-${focalRender}`"
          class="mt-3"
          :model-value="props.focalSpeakerIds.includes(speaker.speaker_id)"
          :disabled="props.focalBusy"
          :label="t('organization.workspace.proposal.focal.checkbox')"
          @update:model-value="(on) => toggleFocal(speaker.speaker_id as string, on)"
        />
      </div>

      <MediaImageField
        v-if="props.canSetPhoto && speaker.speaker_id"
        class="shrink-0 self-start"
        role="avatar"
        :asset-id="null"
        :label="t('organization.workspace.proposal.speakers.photoField', { name: speakerName(speaker) })"
        :button-label="speaker.photo_image ? t('organization.workspace.proposal.speakers.changePhoto') : t('organization.workspace.proposal.speakers.addPhoto')"
        :rule="props.photoRule"
        :owner="{ schema: 'programme', table: 'proposal_speakers', id: speaker.speaker_id }"
        @update:asset-id="(assetId) => attachPhoto(speaker.speaker_id as string, assetId)"
      />
    </li>
  </ul>
  <UiAlert v-if="photoError" class="mt-3" intent="danger" :message="photoError" live />
</template>
