<script setup lang="ts">
import type { EffectivePermission } from '~/types/identity'
import type { FrancophoneMeetingInput } from '~/types/negotiation-meetings'
import type { PublicEditionRow } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'
import type { MeetingFormFailure } from '~/components/admin/negotiation/MeetingForm.vue'

// La réunion naît en brouillon ; elle se publie depuis sa fiche.

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationMeetings', to: '/admin/negociations/reunions' },
    { labelKey: 'admin.negociations.reunions.new.title' },
  ],
})

const { t, locale } = useI18n()
const api = useApi()
const auth = useAuthStore()
const route = useRoute()
const localePath = useLocalePath()
const { zoneOf } = useDateTime()

useHead(() => ({ title: t('admin.negociations.reunions.new.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)
const canManage = computed(() => hasPermission(granted.value, 'negotiation.meeting.manage'))

const { data: editions, status: editionsStatus, error: editionsError, refresh: refreshEditions } = await useAsyncData<PublicEditionRow[]>(
  'admin-negotiation-cop-editions',
  () => api.events.publicList(),
  { default: () => [], lazy: true },
)
const choixEditions = computed(() => editionsDeCop(editions.value, locale.value))
const slug = computed(() =>
  typeof route.query.edition === 'string' && route.query.edition
    ? route.query.edition
    : (choixEditions.value.parDefaut ?? ''),
)
const edition = computed(() => editions.value.find((e) => e.slug === slug.value) ?? null)
const libelleEdition = computed(() => choixEditions.value.options.find((o) => o.value === slug.value)?.label ?? slug.value)
const fuseau = computed(() => (edition.value?.timezone ?? 'UTC') as TimeZoneName)
const zone = computed(() => zoneOf(edition.value?.city?.trim() || timeZoneCityLabel(fuseau.value)))
const retour = computed(() => localePath({ path: '/admin/negociations/reunions', query: { edition: slug.value } }))

const submitting = ref(false)
const failure = ref<MeetingFormFailure | null>(null)
const forbidden = ref(false)

async function creer(entree: FrancophoneMeetingInput): Promise<void> {
  if (submitting.value) return
  submitting.value = true
  failure.value = null
  try {
    const creee = await api.adminNegotiations.creerUneReunion({ ...entree, edition: slug.value })
    await navigateTo(localePath(`/admin/negociations/reunions/${creee.id}`))
  } catch (erreur) {
    if (isForbiddenError(erreur)) forbidden.value = true
    const refus = normalizeApiError(erreur)
    failure.value = {
      message: apiErrorMessage(erreur, t),
      field: refus instanceof ApiRequestError ? refus.field : null,
    }
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-4xl">
    <UiForbiddenState
      v-if="forbidden || (!canManage && permissionStatus !== 'pending')"
      :required-scope="t('admin.negociations.reunions.forbidden.scope')"
      :description="t('admin.negociations.reunions.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <UiErrorState v-else-if="editionsError" :retry-label="t('common.actions.retry')" @retry="refreshEditions()" />

    <UiLoadingState
      v-else-if="permissionStatus === 'pending' || editionsStatus !== 'success'"
      variant="form"
    />

    <UiEmptyState
      v-else-if="!edition"
      icon="calendar"
      :title="t('admin.negociations.reunions.noEdition.title')"
      :description="t('admin.negociations.reunions.noEdition.description')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.negociations.reunions.new.title') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.negociations.reunions.new.subtitle', { edition: libelleEdition }) }}
        </p>
      </header>

      <AdminNegotiationMeetingForm
        class="mt-8"
        :timezone="fuseau"
        :zone-label="zone"
        :submitting="submitting"
        :failure="failure"
        @submit="creer"
      >
        <template #actions>
          <UiButton variant="ghost" :to="retour">{{ t('admin.negociations.reunions.new.cancel') }}</UiButton>
        </template>
      </AdminNegotiationMeetingForm>
    </template>
  </div>
</template>
