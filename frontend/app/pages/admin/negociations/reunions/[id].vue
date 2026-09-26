<script setup lang="ts">
import type { EffectivePermission } from '~/types/identity'
import type { AdminFrancophoneMeeting, FrancophoneMeetingInput } from '~/types/negotiation-meetings'
import type { PublicEditionRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'
import type { MeetingFormFailure } from '~/components/admin/negotiation/MeetingForm.vue'

/**
 * LA FICHE D'UNE RÉUNION : la saisie, la publication, l'annulation, le lien au
 * Pavillon et les inscrites. Publier porte sur la dernière version enregistrée ;
 * un refus de publication nomme son champ, que le formulaire pointe.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationMeetings', to: '/admin/negociations/reunions' },
    { labelKey: 'admin.negociations.reunions.detail.breadcrumb' },
  ],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()
const { date, timeRange, zoneOf } = useDateTime()

const id = computed(() => String(route.params.id ?? ''))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)
const canManage = computed(() => hasPermission(granted.value, 'negotiation.meeting.manage'))

const { data: reunion, status, error, refresh } = await useAsyncData<AdminFrancophoneMeeting | null>(
  'admin-negotiation-meeting',
  () => api.adminNegotiations.reunion(id.value),
  { default: () => null, lazy: true, watch: [id] },
)

const { data: editions } = await useAsyncData<PublicEditionRow[]>(
  'admin-negotiation-cop-editions',
  () => api.events.publicList(),
  { default: () => [], lazy: true },
)

const isForbidden = computed(
  () => (!canManage.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)

const titre = computed(() => (reunion.value ? resolveI18nText(reunion.value.title, locale.value) : ''))
useHead(() => ({ title: titre.value || t('admin.negociations.reunions.list.title') }))

const fuseau = computed(() => (reunion.value?.timezone ?? 'UTC') as TimeZoneName)
const ville = computed(
  () =>
    editions.value.find((e) => e.slug === reunion.value?.edition)?.city?.trim() || timeZoneCityLabel(fuseau.value),
)
const zone = computed(() => zoneOf(ville.value))
const retour = computed(() =>
  localePath({ path: '/admin/negociations/reunions', query: reunion.value ? { edition: reunion.value.edition } : {} }),
)

const figee = computed(() => reunion.value?.status === 'cancelled' || reunion.value?.status === 'completed')
const annulable = computed(() => Boolean(reunion.value) && !figee.value)

const revision = ref(0)
const occupe = ref<'save' | 'publish' | null>(null)
const failure = ref<MeetingFormFailure | null>(null)
const annonce = ref<string | null>(null)
const refusDePublication = ref<string | null>(null)
const annulationOuverte = ref(false)

function echecDe(e: unknown): MeetingFormFailure {
  const refus = normalizeApiError(e)
  return { message: apiErrorMessage(e, t), field: refus instanceof ApiRequestError ? refus.field : null }
}

async function enregistrer(entree: FrancophoneMeetingInput): Promise<void> {
  if (!reunion.value || occupe.value) return
  occupe.value = 'save'
  failure.value = null
  annonce.value = null
  refusDePublication.value = null
  try {
    reunion.value = await api.adminNegotiations.modifierUneReunion(reunion.value.id, entree)
    revision.value++
    annonce.value = t('admin.negociations.reunions.detail.saved')
  } catch (e) {
    failure.value = echecDe(e)
  } finally {
    occupe.value = null
  }
}

async function publier(): Promise<void> {
  if (!reunion.value || occupe.value) return
  occupe.value = 'publish'
  failure.value = null
  annonce.value = null
  refusDePublication.value = null
  try {
    reunion.value = await api.adminNegotiations.publierUneReunion(reunion.value.id)
    annonce.value = t('admin.negociations.reunions.detail.published')
  } catch (e) {
    // Le refus pointe son champ dans le formulaire ; on le dit aussi là où l'on a cliqué.
    const echec = echecDe(e)
    refusDePublication.value = echec.field
      ? t('admin.negociations.reunions.detail.publishRefused', { message: echec.message })
      : echec.message
    if (echec.field) failure.value = echec
  } finally {
    occupe.value = null
  }
}

function annulee(suivante: AdminFrancophoneMeeting): void {
  reunion.value = suivante
  revision.value++
  failure.value = null
  annonce.value = t('admin.negociations.reunions.detail.cancelled')
}

function relie(suivante: AdminFrancophoneMeeting): void {
  if (reunion.value) reunion.value = { ...reunion.value, pavilion_session_id: suivante.pavilion_session_id }
}
</script>

<template>
  <div class="mx-auto w-full max-w-4xl">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.reunions.forbidden.scope')"
      :description="t('admin.negociations.reunions.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <UiLoadingState v-if="status === 'pending'" variant="form" />

      <UiErrorState v-else-if="error" :retry-label="t('common.actions.retry')" @retry="refresh()" />

      <UiEmptyState
        v-else-if="!reunion"
        icon="search"
        :title="t('admin.negociations.reunions.detail.notFound.title')"
        :description="t('admin.negociations.reunions.detail.notFound.description')"
        :action-label="t('admin.negociations.reunions.detail.backToList')"
        :action-to="retour"
      />

      <template v-else>
        <header class="flex flex-wrap items-start justify-between gap-x-6 gap-y-4">
          <div class="min-w-0">
            <div class="flex flex-wrap items-center gap-3">
              <h1 class="text-3xl leading-tight font-semibold text-balance">{{ titre }}</h1>
              <AdminNegotiationMeetingStatusBadge :status="reunion.status" />
            </div>
            <p class="mt-1 text-text-muted">
              {{ reunion.type ? resolveI18nText(reunion.type.label, locale) : t('admin.negociations.reunions.list.noType') }}
              · {{ t('admin.negociations.reunions.detail.organizer', { organizer: reunion.organizer }) }}
            </p>
            <p class="mt-1 text-sm">
              {{ date(reunion.start_at, fuseau) }} · {{ timeRange(reunion.start_at, reunion.end_at, fuseau, ville) }}
            </p>
          </div>
          <div v-if="reunion.status === 'draft' || annulable" class="flex flex-wrap gap-3">
            <UiButton
              v-if="reunion.status === 'draft'"
              icon="broadcast"
              :loading="occupe === 'publish'"
              :disabled="occupe === 'save'"
              @click="publier"
            >
              {{ t('admin.negociations.reunions.detail.publish') }}
            </UiButton>
            <UiButton v-if="annulable" variant="danger" :disabled="occupe !== null" @click="annulationOuverte = true">
              {{ t('admin.negociations.reunions.detail.cancel') }}
            </UiButton>
          </div>
        </header>

        <p v-if="reunion.status === 'draft'" class="mt-3 max-w-(--measure) text-sm text-text-muted">
          {{ t('admin.negociations.reunions.detail.publishHint') }}
        </p>

        <UiAlert
          v-if="reunion.status === 'cancelled'"
          class="mt-6"
          intent="danger"
          :title="t('admin.negociations.reunions.detail.cancelledTitle')"
          :message="reunion.cancellation_reason ?? ''"
        />
        <UiAlert v-if="refusDePublication" class="mt-6" intent="danger" live :message="refusDePublication" />
        <UiAlert v-else-if="annonce" class="mt-6" intent="success" live dismissible :message="annonce" />

        <AdminNegotiationMeetingForm
          class="mt-8"
          :meeting="reunion"
          :timezone="fuseau"
          :zone-label="zone"
          :submitting="occupe === 'save'"
          :failure="failure"
          :readonly="figee"
          :revision="revision"
          @submit="enregistrer"
        >
          <template #actions>
            <UiButton variant="ghost" :to="retour">{{ t('admin.negociations.reunions.detail.backToList') }}</UiButton>
          </template>
        </AdminNegotiationMeetingForm>

        <AdminNegotiationMeetingPavilionPicker
          class="mt-10"
          :meeting="reunion"
          :timezone="fuseau"
          :zone-label="zone"
          :readonly="figee"
          @updated="relie"
        />

        <AdminNegotiationMeetingRegistrants
          class="mt-6"
          :meeting-id="reunion.id"
          :timezone="fuseau"
          :zone-label="zone"
          :revision="revision"
        />

        <AdminNegotiationMeetingCancelDialog
          v-model:open="annulationOuverte"
          :meeting="reunion"
          @cancelled="annulee"
        />
      </template>
    </template>
  </div>
</template>
