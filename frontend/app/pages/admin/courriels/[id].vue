<script setup lang="ts">
import type { Mailing, MailingAudience, MailingCriterion, MailingPayload } from '~/types/admin-mailings'
import type { ShowcaseSessionOption } from '~/types/admin-showcase'
import type { StepItem } from '~/types/ui'

/**
 * UN ENVOI, `/admin/courriels/<id>`.
 *
 * Brouillon : quatre étapes, Destinataires, Message, Vérification, Envoi.
 * Changer d'étape enregistre le brouillon ; on peut le quitter et y revenir.
 * La vérification lit la liste dans la base, sur le brouillon enregistré : le
 * décompte affiché est celui qui partira.
 *
 * Parti : la fiche (contenu, critères, état de chaque destinataire), avec
 * « Dupliquer » pour en préparer un autre et « Arrêter » tant qu'il est en
 * cours.
 */

definePageMeta({
  layout: 'admin',
  middleware: ['auth'],
  // Dupliquer mène d'un envoi à un autre sur la même page : la remonter remet
  // l'étape et le formulaire à zéro.
  key: (route) => route.fullPath,
  breadcrumb: [{ labelKey: 'nav.admin.mailings', to: '/admin/courriels' }, { labelKey: 'admin.mailing.compose.crumb' }],
})

type Step = 'audience' | 'message' | 'review' | 'send'
const STEPS: Step[] = ['audience', 'message', 'review', 'send']

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
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
  watch: [mailingId],
  lazy: true,
})

useHead(() => ({ title: mailing.value?.subject || t('admin.mailing.compose.title') }))

const edition = computed(() => adminScope.events.find((e) => e.id === mailing.value?.event_id) ?? null)
const timezone = computed(() => edition.value?.timezone ?? 'UTC')
const zone = computed(() => edition.value?.city || timeZoneCityLabel(timezone.value))
/** `{{edition}}` est rendu en français par le worker (`platform.t(title, 'fr')`) : l'aperçu fait de même. */
const editionTitle = computed(() => resolveI18nText(edition.value?.title, 'fr'))

const sessions = ref<ShowcaseSessionOption[]>([])
watch(
  () => mailing.value?.event_id,
  async (eventId) => {
    if (!eventId) return
    try {
      sessions.value = await api.adminShowcase.sessionsFor(eventId, adminScope.scope)
    } catch {
      // Sans la liste des activités, les critères restent utilisables sur toute l'édition.
      sessions.value = []
    }
  },
  { immediate: true },
)

// ---------------------------------------------------------------------------
// Le formulaire du brouillon
// ---------------------------------------------------------------------------

interface Form {
  audience: MailingCriterion[]
  extraText: string
  subject: string
  body: string
  ccText: string
  bccText: string
  replyTo: string
}

function formOf(m: Mailing): Form {
  return {
    audience: m.audience,
    extraText: m.extra_addresses.join('\n'),
    subject: m.subject,
    body: m.body_html,
    ccText: m.cc.join(', '),
    bccText: m.bcc.join(', '),
    replyTo: m.reply_to ?? '',
  }
}

const form = ref<Form | null>(null)
const saved = ref('')
watch(
  mailing,
  (m) => {
    if (!m || m.status !== 'draft') return
    if (form.value && saved.value === JSON.stringify(form.value)) {
      form.value = formOf(m)
      saved.value = JSON.stringify(form.value)
    } else if (!form.value) {
      form.value = formOf(m)
      saved.value = JSON.stringify(form.value)
    }
  },
  { immediate: true },
)
const isDirty = computed(() => Boolean(form.value) && JSON.stringify(form.value) !== saved.value)

type FieldKey = 'subject' | 'body' | 'ccText' | 'bccText' | 'replyTo' | 'extraText'
const errors = ref<Partial<Record<FieldKey, string>>>({})
const saveError = ref<string | null>(null)
const saving = ref(false)

const API_FIELDS: Record<string, FieldKey> = {
  subject: 'subject',
  body_html: 'body',
  cc: 'ccText',
  bcc: 'bccText',
  reply_to: 'replyTo',
  extra_addresses: 'extraText',
}

function addresses(text: string, key: FieldKey): string[] | null {
  const list = parseAddressList(text)
  const wrong = list.find((a) => !isEmailAddress(a))
  if (wrong) {
    errors.value[key] = t('admin.mailing.compose.invalidAddress', { address: wrong })
    return null
  }
  return list
}

function payloadOf(f: Form): MailingPayload | null {
  errors.value = {}
  const extra = addresses(f.extraText, 'extraText')
  const cc = addresses(f.ccText, 'ccText')
  const bcc = addresses(f.bccText, 'bccText')
  const reply = f.replyTo.trim()
  if (reply && !isEmailAddress(reply)) errors.value.replyTo = t('admin.mailing.compose.invalidAddress', { address: reply })
  if (!extra || !cc || !bcc || errors.value.replyTo) return null
  return {
    subject: f.subject,
    body_html: f.body,
    audience: f.audience,
    extra_addresses: extra,
    cc,
    bcc,
    reply_to: reply,
  }
}

/** Enregistre si besoin. Rend faux si l'enregistrement a échoué (erreur affichée). */
async function save(): Promise<boolean> {
  if (!form.value || !mailing.value) return false
  if (!isDirty.value) return true
  const payload = payloadOf(form.value)
  if (!payload) return false
  saving.value = true
  saveError.value = null
  try {
    const next = await api.adminMailings.update(mailing.value.id, payload)
    const snapshot = JSON.stringify(form.value)
    mailing.value = next
    saved.value = snapshot
    return true
  } catch (thrown) {
    const normalized = normalizeApiError(thrown)
    const field = normalized instanceof ApiRequestError ? normalized.field : null
    const key = field ? API_FIELDS[field] : undefined
    if (key) errors.value[key] = apiErrorMessage(thrown, t)
    else saveError.value = apiErrorMessage(thrown, t)
    return false
  } finally {
    saving.value = false
  }
}

// ---------------------------------------------------------------------------
// Étapes
// ---------------------------------------------------------------------------

const step = ref<Step>('audience')

const stepItems = computed<StepItem[]>(() => {
  const current = STEPS.indexOf(step.value)
  return STEPS.map((value, index) => ({
    value,
    label: t(`admin.mailing.compose.steps.${value}`),
    state: index < current ? 'done' : index === current ? 'current' : 'upcoming',
  }))
})

const audience = ref<MailingAudience | null>(null)
const audienceLoading = ref(false)
const audienceError = ref<string | null>(null)

async function loadAudience(): Promise<void> {
  if (!mailing.value) return
  audienceLoading.value = true
  audienceError.value = null
  try {
    audience.value = await api.adminMailings.audience(mailing.value.id)
  } catch (thrown) {
    audienceError.value = apiErrorMessage(thrown, t)
  } finally {
    audienceLoading.value = false
  }
}

async function goTo(next: Step): Promise<void> {
  if (!(await save())) return
  step.value = next
  if (next === 'review' || next === 'send') await loadAudience()
  if (import.meta.client) window.scrollTo({ top: 0, behavior: 'smooth' })
}

function nextStep(): void {
  const index = STEPS.indexOf(step.value)
  const next = STEPS[index + 1]
  if (next) void goTo(next)
}

function previousStep(): void {
  const index = STEPS.indexOf(step.value)
  const previous = STEPS[index - 1]
  if (previous) void goTo(previous)
}

const messageFields = computed({
  get: () => ({
    subject: form.value?.subject ?? '',
    body: form.value?.body ?? '',
    ccText: form.value?.ccText ?? '',
    bccText: form.value?.bccText ?? '',
    replyTo: form.value?.replyTo ?? '',
  }),
  set: (value) => {
    if (form.value) form.value = { ...form.value, ...value }
  },
})

const previewCc = computed(() => parseAddressList(form.value?.ccText ?? ''))
const previewBcc = computed(() => parseAddressList(form.value?.bccText ?? ''))

// ---------------------------------------------------------------------------
// Essai, envoi, brouillon
// ---------------------------------------------------------------------------

const testing = ref(false)
const testMessage = ref<string | null>(null)

async function sendTest(): Promise<void> {
  if (!mailing.value || !(await save())) return
  testing.value = true
  testMessage.value = null
  saveError.value = null
  try {
    const result = await api.adminMailings.test(mailing.value.id)
    testMessage.value = t('admin.mailing.compose.review.testSent', { to: result.to })
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
  } finally {
    testing.value = false
  }
}

const confirmOpen = ref(false)
const busy = ref(false)
const actionError = ref<string | null>(null)

async function dispatch(): Promise<void> {
  if (!mailing.value) return
  busy.value = true
  actionError.value = null
  try {
    mailing.value = await api.adminMailings.dispatch(mailing.value.id)
    confirmOpen.value = false
  } catch (thrown) {
    actionError.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}

const deleteOpen = ref(false)

async function removeDraft(): Promise<void> {
  if (!mailing.value) return
  busy.value = true
  actionError.value = null
  try {
    await api.adminMailings.remove(mailing.value.id)
    await navigateTo(localePath('/admin/courriels'))
  } catch (thrown) {
    actionError.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}

async function duplicate(): Promise<void> {
  if (!mailing.value) return
  busy.value = true
  actionError.value = null
  try {
    const copy = await api.adminMailings.duplicate(mailing.value.id)
    await navigateTo(localePath(`/admin/courriels/${copy.id}`))
  } catch (thrown) {
    actionError.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}

const cancelOpen = ref(false)

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

const isDraft = computed(() => mailing.value?.status === 'draft')
const sendable = computed(() => audience.value?.sendable ?? 0)
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
          {{ mailing.subject || (isDraft ? t('admin.mailing.compose.title') : t('admin.mailing.common.noSubject')) }}
        </h1>
        <p v-if="isDraft" class="text-sm text-text-subtle">
          {{ saving ? t('admin.mailing.compose.saving') : isDirty ? t('admin.mailing.compose.unsaved') : t('admin.mailing.compose.saved') }}
        </p>
      </header>

      <UiAlert v-if="actionError" class="mt-6" intent="danger" live :message="actionError" />

      <!-- LA FICHE D'UN ENVOI PARTI -->
      <AdminMailingsRecord
        v-if="!isDraft"
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

      <!-- LE BROUILLON -->
      <template v-else-if="form">
        <UiStepper
          class="mt-8"
          :model-value="step"
          :steps="stepItems"
          :label="t('admin.mailing.compose.steps.label')"
          allow-skip-ahead
          @update:model-value="goTo($event as Step)"
        />

        <UiAlert v-if="saveError" class="mt-6" intent="danger" live :message="saveError" />

        <div class="mt-8">
          <AdminMailingsAudienceEditor
            v-if="step === 'audience'"
            v-model:audience="form.audience"
            v-model:extra-text="form.extraText"
            :sessions="sessions"
            :extra-error="errors.extraText"
          />

          <AdminMailingsMessageEditor
            v-else-if="step === 'message'"
            v-model="messageFields"
            :variables="MAILING_VARIABLES"
            :errors="errors"
          />

          <AdminMailingsReview
            v-else-if="step === 'review'"
            :audience="audience"
            :loading="audienceLoading"
            :error="audienceError"
            :subject="mailing.subject"
            :body-html="mailing.body_html"
            :cc="previewCc"
            :bcc="previewBcc"
            :reply-to="mailing.reply_to"
            :edition-title="editionTitle"
            :timezone="timezone"
            :zone="zone"
            :testing="testing"
            :test-message="testMessage"
            @test="sendTest"
            @retry="loadAudience"
          />

          <section v-else class="space-y-5" :aria-label="t('admin.mailing.compose.steps.send')">
            <UiLoadingState v-if="audienceLoading && !audience" variant="text" :lines="3" />
            <template v-else>
              <p class="font-display text-2xl leading-tight font-semibold">
                {{ t('admin.mailing.compose.send.summary', { count: sendable }, sendable) }}
              </p>
              <ul class="space-y-1 text-text-secondary">
                <li>{{ t('admin.mailing.compose.send.subject', { subject: mailing.subject || t('admin.mailing.common.noSubject') }) }}</li>
                <li v-if="mailing.cc.length">{{ t('admin.mailing.compose.send.cc', { addresses: mailing.cc.join(', '), count: sendable }) }}</li>
                <li v-if="mailing.reply_to">{{ t('admin.mailing.compose.send.replyTo', { address: mailing.reply_to }) }}</li>
              </ul>
              <p class="max-w-(--measure) text-sm text-text-subtle">{{ t('admin.mailing.compose.send.explain') }}</p>
              <UiButton icon="mail" :disabled="sendable === 0 || busy" @click="confirmOpen = true">
                {{ t('admin.mailing.compose.send.action', { count: sendable }, sendable) }}
              </UiButton>
            </template>
          </section>
        </div>

        <footer class="mt-10 flex flex-wrap items-center justify-between gap-3 border-t border-border pt-6">
          <div class="flex flex-wrap gap-3">
            <UiButton v-if="step !== 'audience'" variant="secondary" icon="arrow-left" :disabled="saving" @click="previousStep">
              {{ t('admin.mailing.compose.previous') }}
            </UiButton>
            <UiButton v-if="step !== 'send'" icon-trailing="arrow-right" :loading="saving" @click="nextStep">
              {{ t('admin.mailing.compose.next') }}
            </UiButton>
          </div>
          <div class="flex flex-wrap gap-3">
            <UiButton variant="ghost" :disabled="!isDirty || saving" @click="save">
              {{ t('admin.mailing.compose.saveDraft') }}
            </UiButton>
            <UiButton variant="ghost" icon="trash" :disabled="busy" @click="deleteOpen = true">
              {{ t('admin.mailing.compose.delete') }}
            </UiButton>
          </div>
        </footer>
      </template>

      <UiModal
        v-model:open="confirmOpen"
        :title="t('admin.mailing.compose.confirm.title')"
        :description="t('admin.mailing.compose.confirm.description', { count: sendable }, sendable)"
      >
        <template #footer>
          <UiButton variant="secondary" :disabled="busy" @click="confirmOpen = false">
            {{ t('admin.mailing.common.cancel') }}
          </UiButton>
          <UiButton icon="mail" :loading="busy" @click="dispatch">
            {{ t('admin.mailing.compose.send.action', { count: sendable }, sendable) }}
          </UiButton>
        </template>
      </UiModal>

      <UiModal v-model:open="deleteOpen" :title="t('admin.mailing.compose.deleteConfirm.title')" :description="t('admin.mailing.compose.deleteConfirm.description')">
        <template #footer>
          <UiButton variant="secondary" :disabled="busy" @click="deleteOpen = false">
            {{ t('admin.mailing.common.cancel') }}
          </UiButton>
          <UiButton variant="danger" icon="trash" :loading="busy" @click="removeDraft">
            {{ t('admin.mailing.compose.delete') }}
          </UiButton>
        </template>
      </UiModal>

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
