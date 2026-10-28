<script setup lang="ts">
import type { Mailing } from '~/types/admin-mailings'
import type { ShowcaseSessionOption } from '~/types/admin-showcase'

/**
 * UN ENVOI, `/admin/courriels/<id>`.
 *
 * Parti : la fiche (contenu, critères, pièces jointes, état de chaque
 * destinataire), avec « Dupliquer » et « Arrêter ». Brouillon : il s'ouvre dans
 * la fenêtre de rédaction (28/10), qui remplace les quatre étapes.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  // Dupliquer mène d'un envoi à un autre sur la même page : la remonter.
  key: (route) => route.fullPath,
  breadcrumb: [{ labelKey: 'nav.admin.mailings', to: '/admin/courriels' }, { labelKey: 'admin.mailing.compose.crumb' }],
})

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
const composer = useMailComposerStore()
const route = useRoute()
const localePath = useLocalePath()

await adminScope.ensureLoaded()

const mailingId = computed(() => String(route.params.id ?? ''))

const {
  data: mailing,
  status,
  error: loadError,
  refresh,
} = await useAsyncData<Mailing | null>(`admin-mailing-${mailingId.value}`, () => api.adminMailings.byId(mailingId.value), {
  lazy: true,
})

useHead(() => ({ title: mailing.value?.subject || t('admin.mailing.compose.title') }))

const isDraft = computed(() => mailing.value?.status === 'draft')
watch(
  isDraft,
  (draft) => {
    if (draft && import.meta.client) composer.open(mailingId.value)
  },
  { immediate: true },
)
watch(() => composer.version, () => void refresh())

const edition = computed(() => adminScope.events.find((e) => e.id === mailing.value?.event_id) ?? null)
const timezone = computed(() => edition.value?.timezone ?? 'UTC')
const zone = computed(() => edition.value?.city || timeZoneCityLabel(timezone.value))

const sessions = ref<ShowcaseSessionOption[]>([])
watch(
  () => mailing.value?.event_id,
  async (eventId) => {
    if (!eventId) return
    try {
      sessions.value = await api.adminShowcase.sessionsFor(eventId, adminScope.scope)
    } catch {
      sessions.value = []
    }
  },
  { immediate: true },
)

const busy = ref(false)
const actionError = ref<string | null>(null)
const cancelOpen = ref(false)

async function duplicate(): Promise<void> {
  const source = mailing.value
  if (!source) return
  busy.value = true
  actionError.value = null
  try {
    const copy = await api.adminMailings.duplicate(source.id)
    // Les pièces jointes sont reprises : les mêmes objets, rattachés à la copie.
    for (const piece of source.attachments) {
      await api.media.attachOne({
        owner_schema: 'engagement',
        owner_table: 'mailings',
        owner_id: copy.id,
        role: 'attachment',
        asset_id: piece.asset_id,
      })
    }
    composer.open(copy.id)
  } catch (thrown) {
    actionError.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}

async function cancelSending(): Promise<void> {
  if (!mailing.value) return
  busy.value = true
  actionError.value = null
  try {
    mailing.value = await api.adminMailings.cancel(mailing.value.id)
    cancelOpen.value = false
  } catch (thrown) {
    actionError.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <UiLoadingState v-if="status === 'pending' && !mailing" variant="text" :lines="6" />

    <UiErrorState v-else-if="loadError" :description="apiErrorMessage(loadError, t)" @retry="refresh()" />

    <UiForbiddenState
      v-else-if="!mailing"
      :title="t('admin.mailing.compose.notFound.title')"
      :description="t('admin.mailing.compose.notFound.description')"
      action-to="/admin/courriels"
      :action-label="t('admin.mailing.compose.backToList')"
    />

    <template v-else>
      <header class="space-y-2">
        <NuxtLink :to="localePath('/admin/courriels')" class="inline-flex min-h-(--target-min) items-center gap-2 text-sm text-text-secondary no-underline hover:text-text">
          <UiIcon name="arrow-left" size="1rem" />
          {{ t('admin.mailing.compose.backToList') }}
        </NuxtLink>
        <h1 class="text-3xl leading-tight font-semibold text-balance break-words">
          {{ mailing.subject || t('admin.mailing.common.noSubject') }}
        </h1>
      </header>

      <UiAlert v-if="actionError" class="mt-6" intent="danger" live :message="actionError" />

      <div v-if="isDraft" class="mt-8">
        <UiAlert intent="info" :message="t('admin.mailing.composer.draftHere')" />
        <UiButton class="mt-4" icon="mail" @click="composer.open(mailing.id)">
          {{ t('admin.mailing.composer.openDraft') }}
        </UiButton>
      </div>

      <AdminMailingsRecord
        v-else
        class="mt-8"
        :mailing="mailing"
        :timezone="timezone"
        :zone="zone"
        :sessions="sessions"
        :busy="busy"
        @duplicate="duplicate"
        @cancel="cancelOpen = true"
        @refresh="refresh()"
      />

      <UiModal v-model:open="cancelOpen" :title="t('admin.mailing.record.cancelConfirm.title')" :description="t('admin.mailing.record.cancelConfirm.description')">
        <template #footer>
          <UiButton variant="secondary" :disabled="busy" @click="cancelOpen = false">
            {{ t('admin.mailing.common.back') }}
          </UiButton>
          <UiButton variant="danger" icon="ban" :loading="busy" @click="cancelSending">
            {{ t('admin.mailing.record.cancel') }}
          </UiButton>
        </template>
      </UiModal>
    </template>
  </div>
</template>
