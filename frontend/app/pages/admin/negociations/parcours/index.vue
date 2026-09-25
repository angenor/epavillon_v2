<script setup lang="ts">
import type { AdminPathway } from '~/types/admin-negotiation-savoir'
import type { EffectivePermission } from '~/types/identity'

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [{ labelKey: 'nav.admin.negotiationPathway' }],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()

useHead(() => ({ title: t('admin.negociations.parcours.title') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canRead = computed(
  () =>
    hasPermission(granted.value, 'negotiation.knowledge.publish') ||
    hasPermission(granted.value, 'negotiation.knowledge.review'),
)

const { data: pathway, status, error, refresh } = await useAsyncData<AdminPathway | null>(
  'admin-negotiation-pathway',
  () => api.adminNegotiationSavoir.parcours(),
  { lazy: true, default: () => null },
)

const isForbidden = computed(
  () => (!canRead.value && permissionStatus.value !== 'pending') || isForbiddenError(error.value),
)
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <UiForbiddenState
      v-if="isForbidden"
      :required-scope="t('admin.negociations.parcours.forbidden.scope')"
      :description="t('admin.negociations.parcours.forbidden.description')"
      action-to="/admin"
      :action-label="t('nav.admin.title')"
    />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">{{ t('admin.negociations.parcours.title') }}</h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">{{ t('admin.negociations.parcours.subtitle') }}</p>
      </header>

      <UiErrorState
        v-if="error"
        class="mt-8"
        :description="apiErrorMessage(error, t)"
        :request-id="incidentReference(error) ?? undefined"
        :retry-label="t('common.actions.retry')"
        @retry="refresh()"
      />

      <UiLoadingState v-else-if="status === 'pending' && !pathway" class="mt-8" />

      <template v-else-if="pathway">
        <UiAlert
          v-if="!pathway.can_publish"
          class="mt-6"
          intent="info"
          compact
          :message="t('admin.negociations.parcours.readonly')"
        />
        <AdminNegotiationPathwayEditor v-model="pathway" class="mt-8" />
      </template>
    </template>
  </div>
</template>
