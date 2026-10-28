<script setup lang="ts">
import type {
  ComposerOptionsValue,
  MailingAudienceEntry,
  MailingPreview,
  MailingTemplate,
} from '~/types/admin-mailings'
import type { Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * « PLUS D'OPTIONS » de la fenêtre de rédaction : réponse à, édition, gabarit
 * et son aperçu, variables et leurs valeurs de secours, bouton Répondre,
 * bouton d'action de l'annonce, envoi d'essai.
 *
 * L'aperçu est rendu par l'API, par la fonction même du worker : ce qu'on voit
 * est ce qui part. Il se relit après chaque enregistrement (`refreshKey`).
 */

interface Props {
  modelValue: ComposerOptionsValue
  mailingId: Uuid | null
  entries: MailingAudienceEntry[]
  refreshKey: number
  testing: boolean
  testMessage: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: ComposerOptionsValue]; test: [] }>()

const { t } = useI18n()
const { tr } = useI18nText()
const api = useApi()
const adminScope = useAdminScopeStore()

function set<K extends keyof ComposerOptionsValue>(key: K, value: ComposerOptionsValue[K]): void {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}

function setFallback(name: string, value: string): void {
  set('fallbacks', { ...props.modelValue.fallbacks, [name]: value })
}

const editions = computed<SelectOption[]>(() =>
  adminScope.events.map((e) => ({ value: e.id, label: e.acronym ? `${e.acronym}, ${tr(e.title)}` : tr(e.title) })),
)

const TEMPLATES: MailingTemplate[] = ['sobre', 'institutionnel', 'annonce']
const tokenOf = variableToken

// « Vu par » : une personne avec compte et, s'il y en a, une sans, pour voir le secours.
const viewers = computed<SelectOption[]>(() => {
  const sendable = props.entries.filter((e) => !e.is_suppressed)
  const withAccount = sendable.filter((e) => e.person_id).slice(0, 30)
  const without = sendable.filter((e) => !e.person_id).slice(0, 30)
  return [...without.slice(0, 1), ...withAccount, ...without.slice(1)].map((e) => ({
    value: e.email,
    label: [e.first_name, e.last_name].filter(Boolean).join(' ') || e.email,
    description: e.person_id ? e.email : t('admin.mailing.composer.options.noAccount', { email: e.email }),
  }))
})
const viewer = ref('')
watch(viewers, (list) => {
  if (!list.some((v) => v.value === viewer.value)) viewer.value = list[0]?.value ?? ''
}, { immediate: true })

const preview = ref<MailingPreview | null>(null)
const previewError = ref<string | null>(null)
watch(
  () => [props.mailingId, viewer.value, props.refreshKey] as const,
  async ([id, email]) => {
    if (!id) return
    try {
      previewError.value = null
      preview.value = await api.adminMailings.preview(id, email || undefined)
    } catch (thrown) {
      previewError.value = apiErrorMessage(thrown, t)
    }
  },
  { immediate: true },
)
</script>

<template>
  <div class="space-y-6 border-t border-border pt-4">
    <div class="grid gap-4 md:grid-cols-2">
      <UiInput
        :model-value="props.modelValue.replyTo"
        type="email"
        :label="t('admin.mailing.composer.options.replyTo')"
        :hint="t('admin.mailing.composer.options.replyToHint')"
        block
        @update:model-value="set('replyTo', $event)"
      />
      <UiSelect
        :model-value="props.modelValue.eventId"
        :options="editions"
        :label="t('admin.mailing.composer.options.edition')"
        :hint="t('admin.mailing.composer.options.editionHint')"
        hide-optional
        block
        @update:model-value="set('eventId', $event)"
      />
    </div>

    <fieldset>
      <legend class="text-sm font-semibold text-text">{{ t('admin.mailing.composer.options.template') }}</legend>
      <div class="mt-2 grid gap-2 sm:grid-cols-3">
        <label
          v-for="tpl in TEMPLATES"
          :key="tpl"
          class="flex min-h-(--target-min) cursor-pointer flex-col rounded-lg border p-3"
          :class="props.modelValue.template === tpl ? 'border-accent bg-accent-surface' : 'border-border bg-surface-raised'"
        >
          <span class="flex items-center gap-2 text-sm font-semibold">
            <input
              type="radio"
              name="mailing-template"
              class="cursor-pointer"
              :value="tpl"
              :checked="props.modelValue.template === tpl"
              @change="set('template', tpl)"
            >
            {{ t(`admin.mailing.composer.templates.${tpl}.name`) }}
          </span>
          <span class="mt-1 text-xs text-text-subtle">{{ t(`admin.mailing.composer.templates.${tpl}.hint`) }}</span>
        </label>
      </div>
    </fieldset>

    <div v-if="props.modelValue.template === 'annonce'" class="grid gap-4 md:grid-cols-2">
      <UiInput
        :model-value="props.modelValue.ctaLabel"
        :label="t('admin.mailing.composer.options.ctaLabel')"
        block
        @update:model-value="set('ctaLabel', $event)"
      />
      <UiInput
        :model-value="props.modelValue.ctaUrl"
        type="url"
        :label="t('admin.mailing.composer.options.ctaUrl')"
        placeholder="https://"
        block
        @update:model-value="set('ctaUrl', $event)"
      />
    </div>

    <UiCheckbox
      :model-value="props.modelValue.replyButton"
      :label="t('admin.mailing.composer.options.replyButton')"
      :hint="t('admin.mailing.composer.options.replyButtonHint')"
      @update:model-value="set('replyButton', $event)"
    />

    <fieldset>
      <legend class="text-sm font-semibold text-text">{{ t('admin.mailing.composer.options.variables') }}</legend>
      <p class="mt-0.5 text-xs text-text-subtle">{{ t('admin.mailing.composer.options.variablesHint') }}</p>
      <div class="mt-2 grid gap-3 sm:grid-cols-2">
        <UiInput
          v-for="name in MAILING_VARIABLES"
          :key="name"
          :model-value="props.modelValue.fallbacks[name] ?? ''"
          :label="tokenOf(name)"
          :hint="t('admin.mailing.composer.options.fallbackHint', { name: t(`admin.mailing.common.variable.${name}`) })"
          :placeholder="t('admin.mailing.composer.options.noFallback')"
          block
          @update:model-value="setFallback(name, $event)"
        />
      </div>
    </fieldset>

    <section>
      <div class="flex flex-wrap items-end justify-between gap-3">
        <h3 class="text-sm font-semibold text-text">{{ t('admin.mailing.composer.options.preview') }}</h3>
        <UiSelect
          v-if="viewers.length > 1"
          v-model="viewer"
          class="w-full sm:w-80"
          :options="viewers"
          :label="t('admin.mailing.composer.options.seenBy')"
          hide-optional
        />
      </div>
      <p v-if="preview && !preview.has_account" class="mt-2 text-xs text-text-subtle">
        {{ t('admin.mailing.composer.options.previewNoAccount') }}
      </p>
      <UiAlert v-if="previewError" class="mt-2" intent="danger" compact :message="previewError" />
      <iframe
        v-if="preview"
        class="mt-2 h-[28rem] w-full rounded-lg border border-border bg-white"
        :title="t('admin.mailing.composer.options.preview')"
        sandbox=""
        :srcdoc="preview.html"
      />
    </section>

    <div class="flex flex-wrap items-center gap-3">
      <UiButton variant="secondary" icon="mail" :loading="props.testing" :disabled="!props.mailingId" @click="emit('test')">
        {{ t('admin.mailing.composer.options.test') }}
      </UiButton>
      <span class="text-xs text-text-subtle">{{ t('admin.mailing.composer.options.testHint') }}</span>
    </div>
    <UiAlert v-if="props.testMessage" intent="success" compact live :message="props.testMessage" />
  </div>
</template>
