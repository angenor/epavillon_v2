<script setup lang="ts">
import type { AdminQuestion } from '~/types/admin-negotiation-queue'
import type { ExpertQuestionStatus } from '~/types/negotiation-savoir'
import type { Uuid } from '~/types/shared'
import type { Intent, SelectOption } from '~/types/ui'

// Aucune auteure ni aucun expert : l'API ne les rend pas (R9).
const props = defineProps<{
  question: AdminQuestion
  sections: SelectOption[]
  answering?: boolean
  error?: string | null
}>()

const emit = defineEmits<{
  answer: [id: Uuid, answer: string]
  promote: [id: Uuid, sectionCode: string]
}>()

const { t } = useI18n()
const localePath = useLocalePath()
const { dateTime, zoneLabel } = useDateTime()

const ETAT: Record<ExpertQuestionStatus, Intent> = {
  pending: 'warning',
  answered: 'success',
  added_to_faq: 'info',
}

const reponse = ref('')
const rubrique = ref('')
const refusReponse = ref<string | null>(null)

const zone = (instant: string): string =>
  t('admin.negociations.file.zoned', {
    date: dateTime(instant, KNOWLEDGE_TIMEZONE),
    zone: zoneLabel(KNOWLEDGE_TIMEZONE),
  })

function repondre(): void {
  if (!reponse.value.trim()) {
    refusReponse.value = t('admin.negociations.file.question.answer.required')
    return
  }
  refusReponse.value = null
  emit('answer', props.question.id, reponse.value.trim())
}

function promouvoir(): void {
  if (rubrique.value) emit('promote', props.question.id, rubrique.value)
}
</script>

<template>
  <article class="rounded-lg border border-border bg-surface p-4 sm:p-5">
    <header class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
      <UiBadge
        :intent="ETAT[props.question.status]"
        size="sm"
        solid
        :label="t(`admin.negociations.file.question.state.${props.question.status}`)"
      />
      <span class="font-semibold">{{ props.question.theme_label }}</span>
      <span class="text-text-muted">
        {{ t('admin.negociations.file.question.sent', { date: zone(props.question.created_at) }) }}
      </span>
    </header>

    <p class="mt-3 text-lg leading-snug break-words whitespace-pre-line">{{ props.question.body }}</p>
    <p class="mt-2 text-sm text-text-secondary">
      {{ t(`admin.negociations.file.question.consent.${props.question.consent_to_faq ? 'yes' : 'no'}`) }}
    </p>

    <UiAlert v-if="props.error" class="mt-4" intent="danger" live :message="props.error" />

    <form v-if="props.question.status === 'pending'" class="mt-4 space-y-3" @submit.prevent="repondre">
      <UiTextarea
        v-model="reponse"
        :label="t('admin.negociations.file.question.answer.label')"
        :hint="t('admin.negociations.file.question.answer.hint')"
        :error="refusReponse ?? undefined"
        :rows="5"
        auto-grow
        required
        block
      />
      <UiButton type="submit" :loading="props.answering">
        {{ t('admin.negociations.file.question.answer.submit') }}
      </UiButton>
    </form>

    <template v-else>
      <section class="mt-4 rounded-md border border-border bg-surface-raised p-3 sm:p-4">
        <h3 class="text-sm font-semibold">{{ t('admin.negociations.file.question.answer.title') }}</h3>
        <p class="mt-1 break-words whitespace-pre-line">{{ props.question.answer }}</p>
        <p v-if="props.question.answered_at" class="mt-1 text-xs text-text-muted">
          {{ t('admin.negociations.file.question.answered', { date: zone(props.question.answered_at) }) }}
        </p>
      </section>

      <UiButton
        v-if="props.question.faq_entry_id"
        class="mt-4"
        variant="secondary"
        icon-trailing="arrow-right"
        :to="localePath(`/admin/negociations/faq/${props.question.faq_entry_id}`)"
      >
        {{ t('admin.negociations.file.question.promote.open') }}
      </UiButton>

      <div v-else class="mt-4 flex flex-wrap items-end gap-3">
        <UiSelect
          v-model="rubrique"
          class="min-w-0 flex-[1_1_16rem]"
          :label="t('admin.negociations.file.question.promote.section')"
          :placeholder="t('admin.negociations.file.question.promote.placeholder')"
          :options="props.sections"
          :disabled="!props.question.consent_to_faq"
          :hint="props.question.consent_to_faq ? undefined : t('admin.negociations.file.question.promote.noConsent')"
          hide-optional
        />
        <UiButton variant="secondary" :disabled="!props.question.consent_to_faq || !rubrique" @click="promouvoir">
          {{ t('admin.negociations.file.question.promote.submit') }}
        </UiButton>
      </div>
    </template>
  </article>
</template>
