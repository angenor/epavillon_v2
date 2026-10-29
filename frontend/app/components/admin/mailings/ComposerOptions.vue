<script setup lang="ts">
import type { ComposerOptionsValue } from '~/types/admin-mailings'
import type { Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'

/**
 * « PLUS D'OPTIONS » de la fenêtre de rédaction : réponse à, édition,
 * variables et leurs valeurs de secours, envoi d'essai. Gabarits, bouton
 * Répondre et aperçu sont en vue dans la fenêtre depuis le 29/10.
 */

interface Props {
  modelValue: ComposerOptionsValue
  mailingId: Uuid | null
  testing: boolean
  testMessage: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:modelValue': [value: ComposerOptionsValue]
  test: []
}>()

const { t } = useI18n()
const { tr } = useI18nText()
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

const tokenOf = variableToken
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

    <div class="flex flex-wrap items-center gap-3">
      <UiButton variant="secondary" icon="mail" :loading="props.testing" :disabled="!props.mailingId" @click="emit('test')">
        {{ t('admin.mailing.composer.options.test') }}
      </UiButton>
      <span class="text-xs text-text-subtle">{{ t('admin.mailing.composer.options.testHint') }}</span>
    </div>
    <UiAlert v-if="props.testMessage" intent="success" compact live :message="props.testMessage" />
  </div>
</template>
