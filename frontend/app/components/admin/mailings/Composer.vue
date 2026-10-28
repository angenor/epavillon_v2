<script setup lang="ts">
import type {
  ComposerOptionsValue,
  Mailing,
  MailingAudience,
  MailingCriterion,
  MailingPayload,
} from '~/types/admin-mailings'
import type { ShowcaseSessionOption } from '~/types/admin-showcase'

/**
 * LA FENÊTRE DE RÉDACTION D'UN COURRIEL (28/10), sur le modèle de Gmail : une
 * fenêtre, pas d'étapes. Destinataires, objet, message, pièces jointes et
 * Envoyer d'emblée ; le reste sous « Plus d'options ».
 *
 * BROUILLON TOUJOURS ENREGISTRÉ. La fenêtre crée le brouillon à l'ouverture
 * (l'API y propose la copie de l'équipe), l'enregistre à chaque pause de frappe,
 * et le supprime à la fermeture s'il est resté vide. Fermer ne perd rien.
 *
 * VÉRIFICATION EN DIRECT. Après chaque enregistrement, la liste est relue
 * (`mailing_audience()`) : une ligne de synthèse sous les destinataires, et des
 * alertes en ligne. L'envoi part directement ; au-delà de 20 destinataires, une
 * seule confirmation, dans le bouton lui-même.
 */

const CONFIRM_ABOVE = 20
const SAVE_DELAY_MS = 800

const { t, locale } = useI18n()
const api = useApi()
const store = useMailComposerStore()
const adminScope = useAdminScopeStore()

interface Form {
  addresses: string[]
  groups: MailingCriterion[]
  cc: string[]
  bcc: string[]
  subject: string
  body: string
  options: ComposerOptionsValue
}

const mailing = ref<Mailing | null>(null)
const form = ref<Form | null>(null)
const loadError = ref<string | null>(null)
const saveError = ref<string | null>(null)
const saving = ref(false)
const lastSaved = ref('')
const showCc = ref(false)
const showBcc = ref(false)
const showOptions = ref(false)
const showAttachments = ref(false)
const audience = ref<MailingAudience | null>(null)
const refreshKey = ref(0)
const sessions = ref<ShowcaseSessionOption[]>([])

function formOf(m: Mailing): Form {
  return {
    addresses: [...m.extra_addresses],
    groups: [...m.audience],
    cc: [...m.cc],
    bcc: [...m.bcc],
    subject: m.subject,
    body: m.body_html,
    options: {
      replyTo: m.reply_to ?? '',
      eventId: m.event_id,
      template: m.template,
      fallbacks: { ...m.variable_fallbacks },
      replyButton: m.reply_button,
      ctaLabel: m.cta_label ?? '',
      ctaUrl: m.cta_url ?? '',
    },
  }
}

function payloadOf(f: Form): MailingPayload {
  const valid = (list: string[]) => list.filter((a) => isEmailAddress(a))
  const reply = f.options.replyTo.trim()
  return {
    subject: f.subject,
    body_html: f.body,
    audience: f.groups,
    extra_addresses: valid(f.addresses),
    cc: valid(f.cc),
    bcc: valid(f.bcc),
    reply_to: isEmailAddress(reply) ? reply : '',
    event_id: f.options.eventId,
    template: f.options.template,
    variable_fallbacks: f.options.fallbacks,
    reply_button: f.options.replyButton,
    cta_label: f.options.ctaLabel,
    cta_url: /^https?:\/\/\S+$/.test(f.options.ctaUrl.trim()) ? f.options.ctaUrl : '',
  }
}

async function loadSessions(eventId: string): Promise<void> {
  try {
    sessions.value = await api.adminShowcase.sessionsFor(eventId, adminScope.scope)
  } catch {
    sessions.value = []
  }
}

async function start(): Promise<void> {
  loadError.value = null
  saveError.value = null
  audience.value = null
  mailing.value = null
  form.value = null
  showOptions.value = false
  confirming.value = false
  testMessage.value = null
  try {
    let m: Mailing | null
    if (store.mailingId) {
      m = await api.adminMailings.byId(store.mailingId)
    } else {
      const eventId = adminScope.currentEventId
      if (!eventId) {
        loadError.value = t('admin.mailing.composer.noEdition')
        return
      }
      m = await api.adminMailings.create({ event_id: eventId })
    }
    if (!m) {
      loadError.value = t('admin.mailing.compose.notFound.description')
      return
    }
    if (m.status !== 'draft') {
      loadError.value = t('admin.mailing.composer.alreadySent')
      return
    }
    mailing.value = m
    form.value = formOf(m)
    lastSaved.value = JSON.stringify(payloadOf(form.value))
    showCc.value = m.cc.length > 0
    showBcc.value = m.bcc.length > 0
    void loadSessions(m.event_id)
    void refreshAudience()
  } catch (thrown) {
    loadError.value = apiErrorMessage(thrown, t)
  }
}


// ---------------------------------------------------------------------------
// Enregistrement automatique et vérification
// ---------------------------------------------------------------------------

let saveTimer: ReturnType<typeof setTimeout> | null = null

async function save(): Promise<boolean> {
  if (!form.value || !mailing.value) return false
  const payload = payloadOf(form.value)
  const snapshot = JSON.stringify(payload)
  if (snapshot === lastSaved.value) return true
  saving.value = true
  saveError.value = null
  try {
    const eventChanged = payload.event_id !== mailing.value.event_id
    mailing.value = await api.adminMailings.update(mailing.value.id, payload)
    lastSaved.value = snapshot
    if (eventChanged) void loadSessions(mailing.value.event_id)
    void refreshAudience()
    return true
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
    return false
  } finally {
    saving.value = false
  }
}

watch(
  form,
  () => {
    if (saveTimer) clearTimeout(saveTimer)
    confirming.value = false
    saveTimer = setTimeout(() => void save(), SAVE_DELAY_MS)
  },
  { deep: true },
)

async function refreshAudience(): Promise<void> {
  if (!mailing.value) return
  try {
    audience.value = await api.adminMailings.audience(mailing.value.id)
  } catch {
    audience.value = null
  }
  refreshKey.value += 1
}

async function reloadAttachments(): Promise<void> {
  if (!mailing.value) return
  const fresh = await api.adminMailings.byId(mailing.value.id)
  if (fresh && mailing.value) {
    mailing.value = {
      ...mailing.value,
      attachments: fresh.attachments,
      attachments_total_bytes: fresh.attachments_total_bytes,
    }
  }
}

const size = (bytes: number) => formatByteSize(bytes, locale.value)

const unknownVariables = computed(() => {
  if (!form.value) return []
  const found = [...`${form.value.subject} ${form.value.body}`.matchAll(/\{\{\s*([a-z_][a-z0-9_]*)\s*\}\}/g)]
    .map((match) => match[1] ?? '')
    .filter((name) => name && !MAILING_VARIABLES.includes(name))
  return [...new Set(found)]
})
const invalidAddresses = computed(() =>
  form.value ? [...form.value.addresses, ...form.value.cc, ...form.value.bcc].filter((a) => !isEmailAddress(a)) : [],
)
const tooHeavy = computed(
  () =>
    Boolean(mailing.value?.attachments_max_bytes) &&
    (mailing.value?.attachments_total_bytes ?? 0) > (mailing.value?.attachments_max_bytes ?? 0),
)
const subjectEmpty = computed(() => Boolean(form.value) && !form.value?.subject.trim())
const sendable = computed(() => audience.value?.sendable ?? 0)
const blocked = computed(
  () =>
    subjectEmpty.value ||
    unknownVariables.value.length > 0 ||
    invalidAddresses.value.length > 0 ||
    tooHeavy.value ||
    sendable.value === 0 ||
    richTextToPlain(form.value?.body).trim().length === 0,
)

// ---------------------------------------------------------------------------
// Envoi, essai, fermeture
// ---------------------------------------------------------------------------

const sending = ref(false)
const confirming = ref(false)
const testing = ref(false)
const testMessage = ref<string | null>(null)

async function flush(): Promise<boolean> {
  if (saveTimer) clearTimeout(saveTimer)
  return save()
}

async function send(): Promise<void> {
  if (!mailing.value || blocked.value) return
  if (!(await flush())) return
  await refreshAudience()
  if (sendable.value > CONFIRM_ABOVE && !confirming.value) {
    confirming.value = true
    return
  }
  sending.value = true
  saveError.value = null
  try {
    await api.adminMailings.dispatch(mailing.value.id)
    mailing.value = null
    store.close()
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
  } finally {
    sending.value = false
    confirming.value = false
  }
}

async function sendTest(): Promise<void> {
  if (!mailing.value || !(await flush())) return
  testing.value = true
  testMessage.value = null
  try {
    const result = await api.adminMailings.test(mailing.value.id)
    testMessage.value = t('admin.mailing.compose.review.testSent', { to: result.to })
  } catch (thrown) {
    saveError.value = apiErrorMessage(thrown, t)
  } finally {
    testing.value = false
  }
}

function isEmpty(f: Form): boolean {
  return (
    !f.subject.trim() &&
    !richTextToPlain(f.body).trim() &&
    f.addresses.length === 0 &&
    f.groups.length === 0 &&
    (mailing.value?.attachments.length ?? 0) === 0
  )
}

async function close(): Promise<void> {
  if (saveTimer) clearTimeout(saveTimer)
  const m = mailing.value
  const f = form.value
  if (m && f) {
    if (isEmpty(f)) await api.adminMailings.remove(m.id).catch(() => undefined)
    else await save()
  }
  mailing.value = null
  form.value = null
  store.close()
}

const summary = computed(() => {
  const a = audience.value
  if (!a) return ''
  const parts = [t('admin.mailing.composer.summary.count', { count: a.sendable }, a.sendable)]
  if (a.suppressed) parts.push(t('admin.mailing.composer.summary.suppressed', { count: a.suppressed }, a.suppressed))
  if (a.without_account) parts.push(t('admin.mailing.composer.summary.noAccount', { count: a.without_account }, a.without_account))
  if (a.fallback_count) parts.push(t('admin.mailing.composer.summary.fallback', { count: a.fallback_count }, a.fallback_count))
  if (mailing.value?.attachments.length) {
    parts.push(t('admin.mailing.composer.summary.attachments', { size: size(mailing.value.attachments_total_bytes) }))
  }
  return parts.join(' · ')
})

const sendLabel = computed(() =>
  confirming.value
    ? t('admin.mailing.composer.confirm', { count: sendable.value }, sendable.value)
    : t('admin.mailing.composer.send'),
)

// En dernier : `start()` touche des états déclarés plus haut, et l'appel
// immédiat les trouverait sinon non initialisés.
watch(
  () => store.isOpen,
  (open) => {
    if (open) void start()
  },
  { immediate: true },
)
</script>

<template>
  <UiModal
    :open="store.isOpen"
    :title="form?.subject.trim() || t('admin.mailing.composer.title')"
    size="xl"
    fullscreen-mobile
    @update:open="(open: boolean) => { if (!open) void close() }"
  >
    <UiErrorState v-if="loadError" compact :description="loadError" />
    <UiLoadingState v-else-if="!form || !mailing" variant="text" :lines="5" />

    <div v-else class="space-y-1">
      <AdminMailingsAddressField
        v-model:addresses="form.addresses"
        v-model:groups="form.groups"
        :label="t('admin.mailing.composer.to')"
        :sessions="sessions"
      >
        <template #end>
          <button v-if="!showCc" type="button" class="min-h-(--target-min) cursor-pointer px-2 text-sm text-text-link" @click="showCc = true">
            {{ t('admin.mailing.composer.cc') }}
          </button>
          <button v-if="!showBcc" type="button" class="min-h-(--target-min) cursor-pointer px-2 text-sm text-text-link" @click="showBcc = true">
            {{ t('admin.mailing.composer.bcc') }}
          </button>
        </template>
      </AdminMailingsAddressField>
      <AdminMailingsAddressField v-if="showCc" v-model:addresses="form.cc" :label="t('admin.mailing.composer.cc')" />
      <AdminMailingsAddressField v-if="showBcc" v-model:addresses="form.bcc" :label="t('admin.mailing.composer.bcc')" />

      <p class="py-1.5 text-xs text-text-subtle" aria-live="polite">
        {{ summary || t('admin.mailing.composer.summary.none') }}
      </p>

      <div class="flex items-center gap-2 border-b border-border">
        <label for="composer-subject" class="w-12 shrink-0 text-sm text-text-subtle">{{ t('admin.mailing.composer.subject') }}</label>
        <input
          id="composer-subject"
          v-model="form.subject"
          type="text"
          maxlength="250"
          class="min-h-(--target-min) flex-1 bg-transparent text-sm font-semibold text-text outline-none"
        >
      </div>

      <div class="pt-3">
        <ClientOnly>
          <UiRichText v-model="form.body" :label="t('admin.mailing.composer.body')" required :rows="10" buttons />
        </ClientOnly>
      </div>

      <div class="space-y-2 pt-2" role="status">
        <UiAlert v-if="subjectEmpty" intent="warning" compact :message="t('admin.mailing.composer.alerts.subject')" />
        <UiAlert
          v-if="unknownVariables.length"
          intent="danger"
          compact
          :message="t('admin.mailing.composer.alerts.variables', { names: unknownVariables.join(', ') })"
        />
        <UiAlert
          v-if="invalidAddresses.length"
          intent="danger"
          compact
          :message="t('admin.mailing.composer.alerts.addresses', { list: invalidAddresses.join(', ') })"
        />
        <UiAlert v-if="tooHeavy" intent="danger" compact :message="t('admin.mailing.composer.alerts.heavy')" />
        <UiAlert v-if="saveError" intent="danger" compact live :message="saveError" />
      </div>

      <div v-if="showAttachments || mailing.attachments.length" class="pt-3">
        <AdminMailingsAttachments
          :mailing-id="mailing.id"
          :attachments="mailing.attachments"
          :total-bytes="mailing.attachments_total_bytes"
          :max-bytes="mailing.attachments_max_bytes"
          editable
          @changed="reloadAttachments"
        />
      </div>

      <div class="pt-3">
        <button
          type="button"
          class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-2 text-sm text-text-secondary hover:text-text"
          :aria-expanded="showOptions"
          @click="showOptions = !showOptions"
        >
          <UiIcon :name="showOptions ? 'chevron-up' : 'chevron-down'" size="1rem" />
          {{ t('admin.mailing.composer.more') }}
        </button>
        <AdminMailingsComposerOptions
          v-if="showOptions"
          v-model="form.options"
          :mailing-id="mailing.id"
          :entries="audience?.entries ?? []"
          :refresh-key="refreshKey"
          :testing="testing"
          :test-message="testMessage"
          @test="sendTest"
        />
      </div>
    </div>

    <template v-if="form && mailing" #footer>
      <span class="mr-auto text-xs text-text-subtle">
        {{ saving ? t('admin.mailing.compose.saving') : t('admin.mailing.compose.saved') }}
      </span>
      <UiButton
        variant="ghost"
        icon="paperclip"
        icon-only
        :label="t('admin.mailing.composer.attach')"
        @click="showAttachments = !showAttachments"
      />
      <UiButton icon="mail" :loading="sending" :disabled="blocked" @click="send">
        {{ sendLabel }}
      </UiButton>
    </template>
  </UiModal>
</template>
