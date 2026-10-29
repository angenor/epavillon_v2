<script setup lang="ts">
import type { AttachableRoleRule } from '~/types/media'
import type { AssetId, Uuid } from '~/types/shared'
import type { ProposalFileGap } from '~/utils/proposal-file'

/**
 * CE QUI MANQUE, en tête de la fiche (29/10).
 *
 * Le logo de l'organisation à part, en ROUGE : sans lui l'activité paraît sans
 * signature sur la page publique et sur l'image de couverture. Seul un référent
 * le pose (garde du module média) : les autres membres lisent qui le peut.
 * Les autres manques en JAUNE, chacun avec un lien vers sa rubrique.
 */

interface Props {
  gaps: ProposalFileGap[]
  hasLogo: boolean
  organizationId: Uuid
  organizationName: string
  canSetLogo: boolean
  logoRule: AttachableRoleRule | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ logoSet: [] }>()

const { t } = useI18n()
const api = useApi()

const addingLogo = ref(false)
const logoAssetId = ref<AssetId | null>(null)
const logoError = ref<string | null>(null)

async function attachLogo(assetId: AssetId | null): Promise<void> {
  logoAssetId.value = assetId
  if (!assetId) return
  logoError.value = null
  try {
    await api.media.attach({
      owner_schema: 'org',
      owner_table: 'organizations',
      owner_id: props.organizationId,
      assignments: [{ role: 'logo', asset_id: assetId }],
    })
    addingLogo.value = false
    emit('logoSet')
  } catch (thrown) {
    logoError.value = apiErrorMessage(thrown, (key) => t(key))
  }
}

const ANCHORS: Record<ProposalFileGap['kind'], string> = {
  bio: '#intervenants',
  photo: '#intervenants',
  cover: '#couverture',
}

function labelOf(gap: ProposalFileGap): string {
  const key = 'organization.workspace.proposal.gaps'
  if (gap.kind === 'cover') return t(`${key}.cover`)
  return t(`${key}.${gap.kind}`, { names: gap.names.join(', ') }, gap.names.length)
}
</script>

<template>
  <div class="flex flex-col gap-4">
    <div
      v-if="!props.hasLogo"
      id="logo"
      role="alert"
      class="scroll-mt-24 rounded-lg border-2 border-danger bg-danger-surface p-4 sm:p-5"
    >
      <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div class="flex items-start gap-3">
          <UiIcon name="warning" class="mt-0.5 shrink-0 text-danger" size="1.25rem" />
          <div>
            <p class="font-semibold text-danger">{{ t('organization.workspace.proposal.gaps.logo.title') }}</p>
            <p class="mt-1 max-w-(--measure) text-sm text-text-secondary">
              {{
                props.canSetLogo
                  ? t('organization.workspace.proposal.gaps.logo.description')
                  : t('organization.workspace.proposal.gaps.logo.askManager', { organization: props.organizationName })
              }}
            </p>
          </div>
        </div>
        <UiButton v-if="props.canSetLogo && !addingLogo" variant="danger" class="shrink-0" @click="addingLogo = true">
          {{ t('organization.workspace.proposal.gaps.logo.action') }}
        </UiButton>
      </div>
      <MediaImageField
        v-if="addingLogo"
        class="mt-4 max-w-md"
        role="logo"
        :asset-id="logoAssetId"
        :label="t('organization.workspace.proposal.gaps.logo.field')"
        :rule="props.logoRule"
        :owner="{ schema: 'org', table: 'organizations', id: props.organizationId }"
        @update:asset-id="attachLogo"
      />
      <UiAlert v-if="logoError" class="mt-3" intent="danger" :message="logoError" live />
    </div>

    <div v-if="props.gaps.length > 0" role="status" class="rounded-lg border border-warning-border bg-warning-surface p-4 sm:p-5">
      <p class="font-semibold text-warning">
        {{ t('organization.workspace.proposal.gaps.title', props.gaps.length) }}
      </p>
      <ul class="mt-2 flex flex-col">
        <li v-for="gap in props.gaps" :key="gap.kind" class="border-t border-warning-border/40 first:border-t-0">
          <a
            :href="ANCHORS[gap.kind]"
            class="flex min-h-(--target-min) items-center justify-between gap-4 text-sm text-text no-underline hover:text-accent"
          >
            <span>{{ labelOf(gap) }}</span>
            <span class="shrink-0 font-semibold text-accent">{{ t(`organization.workspace.proposal.gaps.cta.${gap.kind}`) }}</span>
          </a>
        </li>
      </ul>
    </div>
  </div>
</template>
