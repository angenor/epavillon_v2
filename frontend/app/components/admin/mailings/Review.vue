<script setup lang="ts">
import type { MailingAudience, MailingAudienceEntry, MailingRecipientRow } from '~/types/admin-mailings'
import type { SelectOption } from '~/types/ui'

/**
 * ÉTAPE 3, LA VÉRIFICATION : ce qui partira, à qui, et ce que ça coûtera.
 *
 * Le décompte vient de la base (`mailing_audience()`), sur le brouillon
 * enregistré : la liste qui partira est celle-ci, figée au moment de l'envoi.
 * Trois mises en garde, chiffrées : les adresses écartées (liste de
 * suppression), les destinataires sans prénom connu quand le corps salue par
 * le prénom, et le nombre de copies que recevra chaque adresse en copie.
 */
interface Props {
  audience: MailingAudience | null
  loading: boolean
  error: string | null
  subject: string
  bodyHtml: string
  cc: string[]
  bcc: string[]
  replyTo: string | null
  editionTitle: string
  timezone: string
  zone: string
  testing: boolean
  testMessage: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ test: []; retry: [] }>()

const { t } = useI18n()

const sendable = computed(() => (props.audience?.entries ?? []).filter((e) => !e.is_suppressed))
const previewEmail = ref<string>('')
watch(
  sendable,
  (rows) => {
    if (!rows.some((r) => r.email === previewEmail.value)) previewEmail.value = rows[0]?.email ?? ''
  },
  { immediate: true },
)

const previewOptions = computed<SelectOption[]>(() =>
  sendable.value.slice(0, 200).map((e) => ({
    value: e.email,
    label: [e.first_name, e.last_name].filter(Boolean).join(' ') || e.email,
    description: e.email,
  })),
)

function valuesFor(entry: MailingAudienceEntry | undefined): Record<string, string> {
  return {
    prenom: entry?.first_name ?? '',
    nom: entry?.last_name ?? '',
    organisation: entry?.organization ?? '',
    email: entry?.email ?? '',
    edition: props.editionTitle,
  }
}

const previewEntry = computed(() => sendable.value.find((e) => e.email === previewEmail.value))

const usesFirstName = computed(() => /\{\{\s*prenom\s*\}\}/.test(props.subject + props.bodyHtml))


const rows = computed<MailingRecipientRow[]>(() =>
  (props.audience?.entries ?? []).map((e) => ({
    email: e.email,
    name: [e.first_name, e.last_name].filter(Boolean).join(' ') || null,
    organization: e.organization,
    sources: e.sources,
    status: null,
    suppressed: e.is_suppressed,
    error: null,
    sentAt: null,
  })),
)
</script>

<template>
  <section class="space-y-8" :aria-label="t('admin.mailing.compose.steps.review')">
    <UiErrorState v-if="props.error" compact :description="props.error" @retry="emit('retry')" />

    <template v-else>
      <UiLoadingState v-if="props.loading && !props.audience" variant="text" :lines="4" />

      <div v-else-if="props.audience" class="space-y-3">
        <p class="font-display text-2xl leading-tight font-semibold">
          {{ t('admin.mailing.compose.review.count', { count: props.audience.sendable }, props.audience.sendable) }}
        </p>
        <p v-if="props.audience.sendable > 0" class="text-text-secondary">
          {{ t('admin.mailing.compose.review.messages', { count: props.audience.envelope_count }, props.audience.envelope_count) }}
        </p>

        <UiAlert
          v-if="props.audience.sendable === 0"
          intent="warning"
          :message="t('admin.mailing.compose.review.none')"
        />
        <UiAlert
          v-if="props.audience.suppressed > 0"
          intent="danger"
          :message="t('admin.mailing.compose.review.suppressed', { count: props.audience.suppressed }, props.audience.suppressed)"
        />
        <UiAlert
          v-if="usesFirstName && props.audience.without_first_name > 0"
          intent="warning"
          :message="t('admin.mailing.compose.review.noFirstName', { count: props.audience.without_first_name }, props.audience.without_first_name)"
        />
        <UiAlert
          v-if="props.cc.length + props.bcc.length > 0 && props.audience.sendable > 1"
          intent="info"
          :message="t('admin.mailing.compose.review.copies', { count: props.audience.sendable, addresses: [...props.cc, ...props.bcc].join(', ') })"
        />
      </div>

      <slot />

      <div>
        <div class="flex flex-wrap items-end justify-between gap-3">
          <h3 class="font-display text-xl font-semibold">{{ t('admin.mailing.compose.review.preview') }}</h3>
          <UiSelect
            v-if="previewOptions.length > 1"
            v-model="previewEmail"
            class="w-full sm:w-80"
            :options="previewOptions"
            :label="t('admin.mailing.compose.review.previewAs')"
            hide-optional
          />
        </div>
        <AdminMailingsPreview
          class="mt-3"
          :subject="props.subject"
          :body-html="props.bodyHtml"
          :to="previewEntry?.email ?? t('admin.mailing.compose.review.previewNobody')"
          :cc="props.cc"
          :bcc="props.bcc"
          :reply-to="props.replyTo"
          :values="valuesFor(previewEntry)"
        />

        <div class="mt-4 flex flex-wrap items-center gap-3">
          <UiButton variant="secondary" icon="mail" :loading="props.testing" @click="emit('test')">
            {{ t('admin.mailing.compose.review.test') }}
          </UiButton>
          <span class="text-sm text-text-subtle">{{ t('admin.mailing.compose.review.testHint') }}</span>
        </div>
        <UiAlert v-if="props.testMessage" class="mt-3" intent="success" compact live :message="props.testMessage" />
      </div>

      <div v-if="props.audience">
        <h3 class="font-display text-xl font-semibold">{{ t('admin.mailing.compose.review.recipients') }}</h3>
        <p class="mt-1 text-sm text-text-subtle">{{ t('admin.mailing.compose.review.recipientsHint') }}</p>
        <AdminMailingsRecipientList
          class="mt-3"
          :rows="rows"
          :caption="t('admin.mailing.compose.review.recipients')"
          :timezone="props.timezone"
          :zone="props.zone"
          :loading="props.loading"
        />
      </div>
    </template>
  </section>
</template>
