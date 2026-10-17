<script setup lang="ts">
import type { AdminSessionMeeting, MeetingStatus, MeetingSyncStatus } from '~/types/live'
import type { Intent } from '~/types/ui'
import type { Uuid } from '~/types/shared'

/**
 * La réunion Zoom d'une séance diffusée : son état chez Zoom, celui des inscrits
 * qu'on y pousse, et le geste qui la crée ou la resynchronise. Un numéro saisi
 * rattache une réunion déjà créée dans Zoom au lieu d'en ouvrir une autre.
 * La portée d'administration est vérifiée par l'API, sur l'édition de la séance.
 */

interface Props {
  sessionId: Uuid
  editable?: boolean
}

const props = withDefaults(defineProps<Props>(), { editable: true })

const { t } = useI18n()
const api = useApi()

const meeting = ref<AdminSessionMeeting | null>(null)
const loading = ref(false)
const loadError = ref<string | null>(null)
const forbidden = ref(false)
const syncing = ref(false)
const syncError = ref<string | null>(null)
const externalId = ref('')

const STATUS_INTENT: Record<MeetingStatus, Intent> = {
  draft: 'neutral',
  scheduled: 'info',
  started: 'warning',
  ended: 'neutral',
  cancelled: 'neutral',
  failed: 'danger',
}
const SYNC_INTENT: Record<MeetingSyncStatus, Intent> = {
  not_required: 'neutral',
  pending: 'warning',
  in_progress: 'warning',
  synced: 'success',
  failed: 'danger',
  abandoned: 'neutral',
}

async function load(): Promise<void> {
  loading.value = true
  loadError.value = null
  forbidden.value = false
  try {
    meeting.value = await api.meetings.adminForSession(props.sessionId)
  } catch (failure) {
    meeting.value = null
    if (isForbiddenError(failure)) forbidden.value = true
    else loadError.value = apiErrorMessage(failure, t)
  } finally {
    loading.value = false
  }
}

async function sync(): Promise<void> {
  syncing.value = true
  syncError.value = null
  try {
    meeting.value = await api.meetings.sync(props.sessionId, { external_id: externalId.value.replace(/\s/g, '') || null })
    externalId.value = ''
  } catch (failure) {
    syncError.value = apiErrorMessage(failure, t)
  } finally {
    syncing.value = false
  }
}

watch(
  () => props.sessionId,
  () => {
    externalId.value = ''
    syncError.value = null
    void load()
  },
  { immediate: true },
)
</script>

<template>
  <section class="space-y-3 rounded-lg border border-border p-3" aria-labelledby="reunion-zoom-titre">
    <h4 id="reunion-zoom-titre" class="text-sm font-semibold text-text">{{ t('admin-session-meeting.title') }}</h4>

    <div v-if="loading && !meeting" class="space-y-2" aria-busy="true">
      <UiSkeletonLoader variant="text" :lines="3" />
    </div>

    <p v-else-if="forbidden" class="text-sm text-text-muted">{{ t('admin-session-meeting.forbidden') }}</p>

    <UiAlert v-else-if="loadError" intent="danger" compact :message="loadError">
      <template #actions>
        <UiButton variant="link" @click="load">{{ t('admin-session-meeting.retry') }}</UiButton>
      </template>
    </UiAlert>

    <p v-else-if="!meeting" class="text-sm text-text-muted">{{ t('admin-session-meeting.empty') }}</p>

    <template v-else>
      <div class="flex flex-wrap gap-2">
        <UiBadge size="sm" solid :intent="STATUS_INTENT[meeting.status]" :label="t(`admin-session-meeting.status.${meeting.status}`)" />
        <UiBadge size="sm" :intent="SYNC_INTENT[meeting.sync_status]" :label="t(`admin-session-meeting.sync.${meeting.sync_status}`)" />
      </div>
      <dl class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs">
        <div>
          <dt class="text-text-muted">{{ t('admin-session-meeting.number') }}</dt>
          <dd class="font-mono text-text tabular-nums">{{ meeting.external_id ?? t('admin-session-meeting.none') }}</dd>
        </div>
        <div>
          <dt class="text-text-muted">{{ t('admin-session-meeting.participants') }}</dt>
          <dd class="text-text tabular-nums">
            {{ t('admin-session-meeting.synced', { synced: meeting.participants_synced, total: meeting.participants_total }) }}
            <span v-if="meeting.participants_failed" class="text-danger">
              · {{ t('admin-session-meeting.failed', meeting.participants_failed) }}
            </span>
          </dd>
        </div>
      </dl>
      <UiAlert v-if="meeting.last_sync_error" intent="danger" compact :message="meeting.last_sync_error" />
      <UiButton v-if="meeting.join_url" variant="link" icon="external-link" :href="meeting.join_url">
        {{ t('admin-session-meeting.open') }}
      </UiButton>
    </template>

    <div v-if="props.editable && !forbidden && !loading" class="space-y-2 border-t border-border-subtle pt-3">
      <UiInput
        v-model="externalId"
        :label="t('admin-session-meeting.attach')"
        :hint="t('admin-session-meeting.attachHint')"
        inputmode="numeric"
        autocomplete="off"
        :disabled="syncing"
      />
      <UiButton variant="secondary" icon="refresh" :loading="syncing" @click="sync">
        {{ t('admin-session-meeting.sync.action') }}
      </UiButton>
      <UiAlert v-if="syncError" intent="danger" live compact :message="syncError" />
    </div>
  </section>
</template>
