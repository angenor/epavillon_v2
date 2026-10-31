<script setup lang="ts">
import type { MailText, MailTextPayload } from '~/types/admin-mailings'
import type { SelectOption } from '~/types/ui'

/**
 * UN TEXTE TYPE, `/admin/courriels/textes/<id>` (01/11), sur le modèle de la
 * page d'un gabarit : enregistré à chaque pause de frappe.
 *
 * Les variables du destinataire valent partout ; celles du dossier ne valent
 * que dans un courriel écrit depuis un dossier, et un envoi groupé qui les cite
 * est refusé. Une date limite ne s'écrit pas en dur : on laisse un passage à
 * compléter entre crochets (« [date limite] »), que la fenêtre de rédaction
 * signale tant qu'il reste.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  key: (route) => route.fullPath,
  breadcrumb: [
    { labelKey: 'nav.admin.mailings', to: '/admin/courriels' },
    { labelKey: 'admin.mailing.texts.list.crumb', to: '/admin/courriels/textes' },
    { labelKey: 'admin.mailing.texts.edit.crumb' },
  ],
})

const SAVE_DELAY_MS = 800

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
const route = useRoute()
const localePath = useLocalePath()

await adminScope.ensureLoaded()

const textId = computed(() => String(route.params.id ?? ''))

const {
  data: text,
  status,
  error: loadError,
  refresh,
} = await useAsyncData<MailText | null>(`admin-mail-text-${textId.value}`, () => api.adminMailings.texts.byId(textId.value), {
  lazy: true,
})

useHead(() => ({ title: text.value?.name || t('admin.mailing.texts.edit.title') }))

type Scope = 'edition' | 'all'
interface Form {
  name: string
  category: string
  sortOrder: string
  subject: string
  body: string
  scope: Scope
}

const form = ref<Form | null>(null)
const saving = ref(false)
const saveError = ref<string | null>(null)
let lastSaved = ''
let saveTimer: ReturnType<typeof setTimeout> | null = null

const readOnly = computed(() => !text.value?.can_edit)

function formOf(m: MailText): Form {
  return {
    name: m.name,
    category: m.category ?? '',
    sortOrder: String(m.sort_order),
    subject: m.subject,
    body: m.body_html,
    scope: m.event_id ? 'edition' : 'all',
  }
}

function payloadOf(f: Form): MailTextPayload {
  const order = Number.parseInt(f.sortOrder, 10)
  return {
    name: f.name,
    category: f.category.trim() || null,
    subject: f.subject,
    body_html: f.body,
    sort_order: Number.isFinite(order) ? Math.max(-32768, Math.min(32767, order)) : 0,
    event_id: f.scope === 'edition' ? (text.value?.event_id ?? adminScope.currentEventId) : null,
  }
}

watch(
  text,
  (m) => {
    if (m && !form.value) {
      form.value = formOf(m)
      lastSaved = JSON.stringify(payloadOf(form.value))
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
    text.value = await api.adminMailings.texts.update(textId.value, payload)
    lastSaved = snapshot
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

const scopes = computed<SelectOption[]>(() => [
  { value: 'edition', label: t('admin.mailing.texts.edit.scope.edition') },
  { value: 'all', label: t('admin.mailing.texts.edit.scope.all') },
])

const tokenOf = variableToken
const cited = computed(() => (form.value ? citedVariables(`${form.value.subject} ${form.value.body}`) : []))
const unknownVariables = computed(() =>
  cited.value.filter((name) => !MAILING_VARIABLES.includes(name) && !MAILING_PROPOSAL_VARIABLES.includes(name)),
)
const placeholders = computed(() =>
  form.value ? placeholdersOf(`${form.value.subject} ${richTextToPlain(form.value.body)}`) : [],
)

// --- Suppression -------------------------------------------------------------

const deleteOpen = ref(false)
const deleting = ref(false)

async function remove(): Promise<void> {
  deleting.value = true
  saveError.value = null
  try {
    if (saveTimer) clearTimeout(saveTimer)
    form.value = null
    await api.adminMailings.texts.remove(textId.value)
    deleteOpen.value = false
    await navigateTo(localePath('/admin/courriels/textes'))
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
    if (text.value) form.value = formOf(text.value)
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

    <UiLoadingState v-else-if="status === 'pending' && !text" variant="text" :lines="6" />

    <UiErrorState v-else-if="loadError" :description="apiErrorMessage(loadError, t)" @retry="refresh()" />

    <UiEmptyState
      v-else-if="!text || !form"
      icon="search"
      :title="t('admin.mailing.texts.edit.notFound.title')"
      :description="t('admin.mailing.texts.edit.notFound.description')"
      :action-label="t('admin.mailing.texts.edit.back')"
      @action="navigateTo(localePath('/admin/courriels/textes'))"
    />

    <template v-else>
      <header class="flex flex-wrap items-end justify-between gap-x-6 gap-y-3">
        <div class="min-w-0">
          <h1 class="text-3xl leading-tight font-semibold text-balance break-words">{{ form.name || t('admin.mailing.texts.edit.title') }}</h1>
          <p class="mt-1 text-sm text-text-subtle" role="status">
            <template v-if="readOnly">{{ t('admin.mailing.texts.edit.readOnly') }}</template>
            <template v-else-if="saving">{{ t('admin.mailing.texts.edit.saving') }}</template>
            <template v-else>{{ t('admin.mailing.texts.edit.saved') }}</template>
          </p>
        </div>
        <UiButton v-if="!readOnly" variant="danger" icon="trash" @click="deleteOpen = true">
          {{ t('admin.mailing.texts.edit.delete') }}
        </UiButton>
      </header>

      <UiAlert v-if="saveError" class="mt-6" intent="danger" live :message="saveError" />

      <div class="mt-8 grid gap-10 xl:grid-cols-[minmax(0,1fr)_minmax(0,24rem)]">
        <fieldset class="min-w-0 space-y-6" :disabled="readOnly">
          <div class="grid gap-4 md:grid-cols-[minmax(0,2fr)_minmax(0,2fr)_minmax(0,1fr)]">
            <UiInput v-model="form.name" :label="t('admin.mailing.texts.edit.name')" required block :maxlength="120" />
            <UiInput
              v-model="form.category"
              :label="t('admin.mailing.texts.edit.category')"
              :hint="t('admin.mailing.texts.edit.categoryHint')"
              block
              :maxlength="60"
            />
            <UiInput
              v-model="form.sortOrder"
              type="number"
              inputmode="numeric"
              :label="t('admin.mailing.texts.edit.sortOrder')"
              :hint="t('admin.mailing.texts.edit.sortOrderHint')"
              block
            />
          </div>
          <UiRadio
            v-model="form.scope"
            :options="scopes"
            :label="t('admin.mailing.texts.edit.scope.label')"
            :hint="t('admin.mailing.texts.edit.scope.hint')"
            inline
          />
          <UiInput v-model="form.subject" :label="t('admin.mailing.texts.edit.subject')" block :maxlength="250" />
          <ClientOnly>
            <UiRichText
              v-model="form.body"
              :label="t('admin.mailing.texts.edit.body')"
              :disabled="readOnly"
              :rows="14"
              buttons
            />
          </ClientOnly>
          <div class="space-y-2" role="status">
            <UiAlert
              v-if="unknownVariables.length"
              intent="danger"
              compact
              :message="t('admin.mailing.composer.alerts.variables', { names: unknownVariables.join(', ') })"
            />
            <UiAlert
              v-if="placeholders.length"
              intent="info"
              compact
              :message="t('admin.mailing.texts.edit.placeholders', { list: placeholders.join(', ') })"
            />
          </div>
        </fieldset>

        <aside class="min-w-0 space-y-6">
          <section>
            <h2 class="text-lg font-semibold">{{ t('admin.mailing.texts.edit.variables') }}</h2>
            <p class="mt-1 text-sm text-text-muted">{{ t('admin.mailing.texts.edit.variablesHint') }}</p>
            <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
              <template v-for="name in MAILING_VARIABLES" :key="name">
                <dt class="font-mono text-text-secondary">{{ tokenOf(name) }}</dt>
                <dd>{{ t(`admin.mailing.common.variable.${name}`) }}</dd>
              </template>
            </dl>
          </section>
          <section>
            <h2 class="text-lg font-semibold">{{ t('admin.mailing.texts.edit.proposalVariables') }}</h2>
            <p class="mt-1 text-sm text-text-muted">{{ t('admin.mailing.texts.edit.proposalVariablesHint') }}</p>
            <dl class="mt-3 grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 text-sm">
              <template v-for="name in MAILING_PROPOSAL_VARIABLES" :key="name">
                <dt class="font-mono text-text-secondary">{{ tokenOf(name) }}</dt>
                <dd>{{ t(`admin.mailing.common.variable.${name}`) }}</dd>
              </template>
            </dl>
          </section>
        </aside>
      </div>

      <UiModal
        v-model:open="deleteOpen"
        :title="t('admin.mailing.texts.edit.deleteConfirm.title')"
        :description="t('admin.mailing.texts.edit.deleteConfirm.description')"
      >
        <template #footer>
          <UiButton variant="secondary" :disabled="deleting" @click="deleteOpen = false">
            {{ t('admin.mailing.common.back') }}
          </UiButton>
          <UiButton variant="danger" icon="trash" :loading="deleting" @click="remove">
            {{ t('admin.mailing.texts.edit.delete') }}
          </UiButton>
        </template>
      </UiModal>
    </template>
  </div>
</template>
