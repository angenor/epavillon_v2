<script setup lang="ts">
import type { AdminDocumentInput } from '~/types/admin-negotiation-documents'
import type { EffectivePermission } from '~/types/identity'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

// Le brouillon naît sans source : son PDF se dépose ensuite, depuis sa fiche.

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  breadcrumb: [
    { labelKey: 'nav.admin.negotiationDocuments', to: '/admin/negociations/documents' },
    { labelKey: 'admin.negociations.documents.form.newTitle' },
  ],
})

const { t } = useI18n()
const api = useApi()
const auth = useAuthStore()
const localePath = useLocalePath()

useHead(() => ({ title: t('admin.negociations.documents.form.newTitle') }))

const { data: granted, status: permissionStatus } = await useAsyncData<EffectivePermission[]>(
  'admin-negotiation-permissions',
  async () => (auth.person ? api.identity.permissions(auth.person.id) : []),
  { default: () => [], lazy: true },
)

const canPublish = computed(() => hasPermission(granted.value, 'negotiation.document.publish'))

const submitting = ref(false)
const failure = ref<DocumentFormFailure | null>(null)
const forbidden = ref(false)

async function creer(entree: AdminDocumentInput): Promise<void> {
  if (submitting.value) return
  submitting.value = true
  failure.value = null
  try {
    const cree = await api.adminNegotiationDocuments.creer(entree)
    await navigateTo(localePath(`/admin/negociations/documents/${cree.id}`))
  } catch (erreur) {
    if (isForbiddenError(erreur)) forbidden.value = true
    failure.value = {
      message: apiErrorMessage(erreur, t),
      field: erreur instanceof ApiRequestError ? erreur.field : null,
    }
  } finally {
    submitting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-5xl">
    <UiForbiddenState
      v-if="forbidden || (!canPublish && permissionStatus !== 'pending')"
      :required-scope="t('admin.negociations.documents.form.forbidden.scopePublish')"
      :description="t('admin.negociations.documents.form.forbidden.description')"
      action-to="/admin/negociations/documents"
      :action-label="t('admin.negociations.documents.form.backToList')"
    />

    <UiLoadingState v-else-if="permissionStatus === 'pending'" />

    <template v-else>
      <header class="min-w-0">
        <h1 class="text-3xl leading-tight font-semibold text-balance">
          {{ t('admin.negociations.documents.form.newTitle') }}
        </h1>
        <p class="mt-1 max-w-(--measure) text-text-muted">
          {{ t('admin.negociations.documents.form.newSubtitle') }}
        </p>
      </header>

      <AdminNegotiationDocumentForm
        class="mt-8"
        :document="null"
        :submitting="submitting"
        :failure="failure"
        @submit="creer"
      >
        <template #actions>
          <UiButton variant="ghost" :to="localePath('/admin/negociations/documents')">
            {{ t('admin.negociations.documents.form.cancel') }}
          </UiButton>
        </template>
      </AdminNegotiationDocumentForm>
    </template>
  </div>
</template>
