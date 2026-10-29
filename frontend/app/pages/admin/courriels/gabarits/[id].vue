<script setup lang="ts">
import type { MailingTemplate, MailTemplate, MailTemplatePayload, MailTemplatePreview } from '~/types/admin-mailings'
import { MAILING_TEMPLATES } from '~/types/admin-mailings'
import type { AttachableRoleRule, AttachmentRole } from '~/types/media'
import type { AssetId } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * UN GABARIT DE COURRIEL, `/admin/courriels/gabarits/<id>` (29/10).
 *
 * Enregistré à chaque pause de frappe, puis relu en APERÇU : celui-là même que
 * le worker enverrait, sur un texte d'exemple et avec les visuels de l'édition
 * sous les siens. Ses deux images passent par l'outil de dépôt du site
 * (`MediaImageField`) : le fond recadré en 3:1, 1200 px au plus, JPEG
 * compressé ; le logo, 400 px au plus. Elles se rattachent par
 * `PUT /media/attachments`, comme les visuels d'une édition.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  key: (route) => route.fullPath,
  breadcrumb: [
    { labelKey: 'nav.admin.mailings', to: '/admin/courriels' },
    { labelKey: 'admin.mailing.templates.list.crumb', to: '/admin/courriels/gabarits' },
    { labelKey: 'admin.mailing.templates.edit.crumb' },
  ],
})

const SAVE_DELAY_MS = 800

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()

await adminScope.ensureLoaded()

const templateId = computed(() => String(route.params.id ?? ''))

const {
  data: template,
  status,
  error: loadError,
  refresh,
} = await useAsyncData<MailTemplate | null>(
  `admin-mail-template-${templateId.value}`,
  () => api.adminMailings.templates.byId(templateId.value),
  { lazy: true },
)

const { data: rules } = await useAsyncData<AttachableRoleRule[]>(
  'admin-mail-template-rules',
  () => api.media.roles('engagement', 'mail_templates'),
  { default: () => [], lazy: true },
)

useHead(() => ({ title: template.value?.name || t('admin.mailing.templates.edit.title') }))

type Scope = 'edition' | 'all'
interface Form {
  name: string
  layout: MailingTemplate
  title: string
  subtitle: string
  gradient: boolean
  scope: Scope
  preselect: boolean
}

const form = ref<Form | null>(null)
const saving = ref(false)
const saveError = ref<string | null>(null)
let lastSaved = ''
let saveTimer: ReturnType<typeof setTimeout> | null = null

const readOnly = computed(() => !template.value?.can_edit)

function formOf(m: MailTemplate): Form {
  return {
    name: m.name,
    layout: m.layout,
    title: m.title ?? '',
    subtitle: m.subtitle ?? '',
    gradient: m.gradient,
    scope: m.event_id ? 'edition' : 'all',
    preselect: m.preselect_for_submitters,
  }
}

function payloadOf(f: Form): MailTemplatePayload {
  const eventId = f.scope === 'edition' ? (template.value?.event_id ?? adminScope.currentEventId) : null
  return {
    name: f.name,
    layout: f.layout,
    title: f.title.trim() || null,
    subtitle: f.subtitle.trim() || null,
    gradient: f.gradient,
    event_id: eventId,
    preselect_for_submitters: Boolean(eventId) && f.preselect,
  }
}

watch(
  template,
  (m) => {
    if (m && !form.value) {
      form.value = formOf(m)
      lastSaved = JSON.stringify(payloadOf(form.value))
      void loadPreview()
    }
  },
  { immediate: true },
)

async function save(): Promise<void> {
  const f = form.value
  if (!f || readOnly.value || !f.name.trim()) return
  const payload = payloadOf(f)
  const snapshot = JSON.stringify(payload)
  if (snapshot === lastSaved) return
  saving.value = true
  saveError.value = null
  try {
    template.value = await api.adminMailings.templates.update(templateId.value, payload)
    lastSaved = snapshot
    void loadPreview()
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
  } finally {
    saving.value = false
  }
}

watch(
  form,
  () => {
    if (saveTimer) clearTimeout(saveTimer)
    saveTimer = setTimeout(() => void save(), SAVE_DELAY_MS)
  },
  { deep: true },
)
onBeforeUnmount(() => {
  if (saveTimer) clearTimeout(saveTimer)
  void save()
})

// --- Aperçu ------------------------------------------------------------------

const preview = ref<MailTemplatePreview | null>(null)
const previewError = ref<string | null>(null)

async function loadPreview(): Promise<void> {
  previewError.value = null
  try {
    preview.value = await api.adminMailings.templates.preview(templateId.value, adminScope.currentEventId ?? undefined)
  } catch (thrown) {
    previewError.value = apiErrorMessage(thrown, t)
  }
}

// --- Images ------------------------------------------------------------------

const imageError = ref<string | null>(null)
const owner = computed(() => ({ schema: 'engagement', table: 'mail_templates', id: templateId.value }))
const ruleOf = (role: AttachmentRole) => rules.value.find((rule) => rule.role === role) ?? null

async function setImage(role: 'background' | 'logo', assetId: AssetId | null): Promise<void> {
  imageError.value = null
  try {
    await api.media.attach({
      owner_schema: 'engagement',
      owner_table: 'mail_templates',
      owner_id: templateId.value,
      assignments: [{ role, asset_id: assetId }],
    })
    const fresh = await api.adminMailings.templates.byId(templateId.value)
    if (fresh) template.value = fresh
    void loadPreview()
  } catch (thrown) {
    imageError.value = apiErrorMessage(thrown, t)
  }
}

// --- Choix -------------------------------------------------------------------

const layouts = computed<SelectOption[]>(() =>
  MAILING_TEMPLATES.map((value) => ({
    value,
    label: t(`admin.mailing.composer.templates.${value}.name`),
    description: t(`admin.mailing.composer.templates.${value}.hint`),
  })),
)
const scopes = computed<SelectOption[]>(() => [
  { value: 'edition', label: t('admin.mailing.templates.edit.scope.edition') },
  { value: 'all', label: t('admin.mailing.templates.edit.scope.all') },
])
const usesImage = computed(() => form.value?.layout === 'banniere' || form.value?.layout === 'texte_sur_image')

// --- Suppression -------------------------------------------------------------

const deleteOpen = ref(false)
const deleting = ref(false)

async function remove(): Promise<void> {
  deleting.value = true
  saveError.value = null
  try {
    if (saveTimer) clearTimeout(saveTimer)
    form.value = null
    await api.adminMailings.templates.remove(templateId.value)
    deleteOpen.value = false
    await navigateTo(localePath('/admin/courriels/gabarits'))
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
    if (template.value) form.value = formOf(template.value)
  } finally {
    deleting.value = false
  }
}
</script>

<template>
  <div class="mx-auto w-full max-w-[100rem]">
    <UiForbiddenState
      v-if="!adminScope.isLoading && !adminScope.canAdminister"
      :required-scope="t('admin.mailing.list.forbidden')"
      action-to="/"
      :action-label="t('nav.admin.backToSite')"
    />

    <UiLoadingState v-else-if="status === 'pending' && !template" variant="text" :lines="6" />

    <UiErrorState v-else-if="loadError" :description="apiErrorMessage(loadError, t)" @retry="refresh()" />

    <UiEmptyState
      v-else-if="!template || !form"
      icon="search"
      :title="t('admin.mailing.templates.edit.notFound.title')"
      :description="t('admin.mailing.templates.edit.notFound.description')"
      :action-label="t('admin.mailing.templates.edit.back')"
      @action="navigateTo(localePath('/admin/courriels/gabarits'))"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance">{{ form.name || t('admin.mailing.templates.edit.title') }}</h1>
          <p class="mt-1 text-sm text-text-subtle" role="status">
            <template v-if="readOnly">{{ t('admin.mailing.templates.edit.readOnly') }}</template>
            <template v-else-if="saving">{{ t('admin.mailing.templates.edit.saving') }}</template>
            <template v-else>{{ t('admin.mailing.templates.edit.saved') }}</template>
          </p>
        </div>
        <UiButton v-if="!readOnly" variant="danger" icon="trash" @click="deleteOpen = true">
          {{ t('admin.mailing.templates.edit.delete') }}
        </UiButton>
      </header>

      <UiAlert v-if="saveError" class="mt-6" intent="danger" live :message="saveError" />

      <div class="mt-8 grid gap-10 xl:grid-cols-[minmax(0,1fr)_minmax(0,40rem)]">
        <fieldset class="min-w-0 space-y-6" :disabled="readOnly">
          <UiInput v-model="form.name" :label="t('admin.mailing.templates.edit.name')" required block :maxlength="120" />
          <UiSelect
            v-model="form.layout"
            :options="layouts"
            :label="t('admin.mailing.templates.edit.layout')"
            :hint="t(`admin.mailing.composer.templates.${form.layout}.hint`)"
            hide-optional
            block
          />
          <div class="grid gap-4 md:grid-cols-2">
            <UiInput
              v-model="form.title"
              :label="t('admin.mailing.templates.edit.titleField')"
              :hint="t('admin.mailing.templates.edit.titleHint')"
              :maxlength="200"
              block
            />
            <UiInput
              v-model="form.subtitle"
              :label="t('admin.mailing.templates.edit.subtitle')"
              :hint="t('admin.mailing.templates.edit.subtitleHint')"
              :maxlength="200"
              block
            />
          </div>
          <UiCheckbox
            v-model="form.gradient"
            :label="t('admin.mailing.templates.edit.gradient')"
            :hint="t('admin.mailing.templates.edit.gradientHint')"
          />
          <UiRadio
            v-model="form.scope"
            :options="scopes"
            :label="t('admin.mailing.templates.edit.scope.label')"
            :hint="t('admin.mailing.templates.edit.scope.hint')"
            inline
          />
          <UiCheckbox
            v-model="form.preselect"
            :disabled="form.scope !== 'edition'"
            :label="t('admin.mailing.templates.edit.preselect')"
            :hint="t('admin.mailing.templates.edit.preselectHint')"
          />

          <section>
            <h2 class="text-lg font-semibold">{{ t('admin.mailing.templates.edit.images') }}</h2>
            <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
              {{ usesImage ? t('admin.mailing.templates.edit.imagesHint') : t('admin.mailing.templates.edit.imagesUnused') }}
            </p>
            <UiAlert v-if="imageError" class="mt-3" intent="danger" compact live :message="imageError" />
            <div class="mt-4 grid items-start gap-5 sm:grid-cols-2">
              <MediaImageField
                role="background"
                :label="t('admin.mailing.templates.edit.background')"
                :use="t('admin.mailing.templates.edit.backgroundUse')"
                shape-label="3:1"
                :rule="ruleOf('background')"
                :image="template.background"
                :asset-id="template.background?.asset_id ?? null"
                :owner="owner"
                :disabled="readOnly"
                :max-width="1200"
                :formats="['jpeg', 'png']"
                :default-quality="70"
                @update:asset-id="(next: AssetId | null) => setImage('background', next)"
              />
              <MediaImageField
                role="logo"
                :label="t('admin.mailing.templates.edit.logo')"
                :use="t('admin.mailing.templates.edit.logoUse')"
                :rule="ruleOf('logo')"
                :image="template.logo"
                :asset-id="template.logo?.asset_id ?? null"
                :owner="owner"
                :disabled="readOnly"
                :max-width="400"
                :formats="['png', 'jpeg']"
                :default-quality="80"
                @update:asset-id="(next: AssetId | null) => setImage('logo', next)"
              />
            </div>
          </section>
        </fieldset>

        <section class="min-w-0">
          <h2 class="text-lg font-semibold">{{ t('admin.mailing.templates.edit.preview') }}</h2>
          <p class="mt-1 text-sm text-text-muted">{{ t('admin.mailing.templates.edit.previewHint') }}</p>
          <UiAlert v-if="previewError" class="mt-2" intent="danger" compact :message="previewError" />
          <iframe
            v-if="preview"
            class="mt-3 h-[40rem] w-full rounded-lg border border-border bg-white"
            :title="t('admin.mailing.templates.edit.preview')"
            sandbox=""
            :srcdoc="preview.html"
          />
          <UiSkeletonLoader v-else-if="!previewError" class="mt-3" variant="block" height="40rem" />
        </section>
      </div>

      <UiModal
        v-model:open="deleteOpen"
        :title="t('admin.mailing.templates.edit.deleteConfirm.title')"
        :description="t('admin.mailing.templates.edit.deleteConfirm.description')"
      >
        <template #footer>
          <UiButton variant="secondary" :disabled="deleting" @click="deleteOpen = false">
            {{ t('admin.mailing.common.back') }}
          </UiButton>
          <UiButton variant="danger" icon="trash" :loading="deleting" @click="remove">
            {{ t('admin.mailing.templates.edit.delete') }}
          </UiButton>
        </template>
      </UiModal>
    </template>
  </div>
</template>
