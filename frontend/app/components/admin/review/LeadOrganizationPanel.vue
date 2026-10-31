<script setup lang="ts">
import type { SimilarOrganization } from '~/types/org'
import type { OrganizationId } from '~/types/shared'

/**
 * CHANGER L'ORGANISATION DU DOSSIER : il a été déposé au nom de la mauvaise
 * fiche. La recherche est celle de la plateforme, toutes dénominations
 * confondues (règle n° 1). Les co-organisatrices ne changent pas.
 *
 * Pas de case « rattacher aussi la déposante » : le module des organisations
 * n'offre à l'équipe aucune route pour poser une adhésion, et ce panneau n'en
 * invente pas une.
 */

interface Props {
  open: boolean
  currentId: OrganizationId
  currentName: string
  busy?: boolean
  error?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:open': [value: boolean]; submit: [organizationId: OrganizationId] }>()

const { t } = useI18n()

const chosen = ref<OrganizationId | null>(null)
const chosenName = ref('')

watch(
  () => props.open,
  (open) => {
    if (!open) return
    chosen.value = null
    chosenName.value = ''
  },
)

function onChosen(match: SimilarOrganization): void {
  chosenName.value = match.legal_name
}

const isCurrent = computed(() => chosen.value === props.currentId)
</script>

<template>
  <UiDrawer
    :open="props.open"
    :title="t('admin.proposal.dashboard.orgPanel.title')"
    width="40rem"
    @update:open="emit('update:open', $event)"
  >
    <div class="flex flex-col gap-5">
      <p class="text-sm text-text-secondary">{{ t('admin.proposal.dashboard.orgPanel.intro') }}</p>

      <p class="flex flex-wrap items-baseline justify-between gap-2 border-y border-border-subtle py-3 text-sm">
        <span class="font-semibold text-text">{{ props.currentName }}</span>
        <span class="text-text-muted">{{ t('admin.proposal.dashboard.orgPanel.current') }}</span>
      </p>

      <AdminNegotiationMeetingOrganizerPicker
        v-model="chosen"
        :current-name="chosenName"
        :label="t('admin.proposal.dashboard.orgPanel.search')"
        :hint="t('admin.proposal.dashboard.orgPanel.searchHint')"
        :disabled="props.busy"
        @chosen="onChosen"
      />

      <UiAlert v-if="isCurrent" intent="info" compact :message="t('admin.proposal.dashboard.orgPanel.same')" />
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />
      <p class="text-xs text-text-muted">{{ t('admin.proposal.dashboard.orgPanel.membershipNote') }}</p>
    </div>

    <template #footer>
      <div class="flex flex-wrap justify-end gap-2">
        <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
          {{ t('common.actions.cancel') }}
        </UiButton>
        <UiButton
          :loading="props.busy"
          :disabled="!chosen || isCurrent"
          @click="chosen && emit('submit', chosen)"
        >
          {{ t('admin.proposal.dashboard.orgPanel.confirm') }}
        </UiButton>
      </div>
    </template>
  </UiDrawer>
</template>
