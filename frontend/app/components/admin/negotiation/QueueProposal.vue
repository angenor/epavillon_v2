<script setup lang="ts">
import type { AdminProposal, GlossaryProposalStatus } from '~/types/admin-negotiation-queue'
import type { Uuid } from '~/types/shared'
import type { Intent } from '~/types/ui'

// Aucun auteur : l'API rend les contextes seuls (R9).
const props = withDefaults(
  defineProps<{
    proposal: AdminProposal
    rejecting?: boolean
    error?: string | null
    /** Sur la page de la proposition, l'acceptation est le formulaire lui-même. */
    acceptLink?: boolean
  }>(),
  { rejecting: false, error: null, acceptLink: true },
)

const emit = defineEmits<{ reject: [id: Uuid, reason: string] }>()

const { t } = useI18n()
const localePath = useLocalePath()
const { dateTime, zoneLabel } = useDateTime()

const ETAT: Record<GlossaryProposalStatus, Intent> = {
  pending: 'warning',
  accepted: 'success',
  rejected: 'neutral',
}

const motif = ref('')
const refus = ref<string | null>(null)

const zone = (instant: string): string =>
  t('admin.negociations.file.zoned', {
    date: dateTime(instant, KNOWLEDGE_TIMEZONE),
    zone: zoneLabel(KNOWLEDGE_TIMEZONE),
  })

function rejeter(): void {
  if (!motif.value.trim()) {
    refus.value = t('admin.negociations.file.proposal.reject.required')
    return
  }
  refus.value = null
  emit('reject', props.proposal.id, motif.value.trim())
}
</script>

<template>
  <article class="rounded-lg border border-border bg-surface p-4 sm:p-5">
    <header class="flex flex-wrap items-center gap-x-3 gap-y-1 text-sm">
      <UiBadge
        :intent="ETAT[props.proposal.status]"
        size="sm"
        solid
        :label="t(`admin.negociations.file.proposal.state.${props.proposal.status}`)"
      />
      <span class="text-text-muted">
        {{ t('admin.negociations.file.proposal.sent', { date: zone(props.proposal.created_at) }) }}
      </span>
      <span class="text-text-muted">
        {{ t('admin.negociations.file.proposal.authors', { count: props.proposal.authors_count }, props.proposal.authors_count) }}
      </span>
    </header>

    <p class="mt-3 text-lg leading-snug font-semibold break-words" lang="en">
      <i>{{ props.proposal.term }}</i>
    </p>

    <section class="mt-3">
      <h3 class="text-sm font-semibold">{{ t('admin.negociations.file.proposal.contexts') }}</h3>
      <ul class="mt-1 space-y-1">
        <li v-for="(c, i) in props.proposal.contexts" :key="i" class="break-words whitespace-pre-line text-text-secondary">
          {{ c.context ? t('admin.negociations.file.details', { text: c.context }) : t('admin.negociations.file.proposal.noContext') }}
        </li>
      </ul>
    </section>

    <section class="mt-3">
      <h3 class="text-sm font-semibold">{{ t('admin.negociations.file.proposal.nearby') }}</h3>
      <p v-if="!props.proposal.nearby.length" class="mt-1 text-sm text-text-muted">
        {{ t('admin.negociations.file.proposal.nearbyNone') }}
      </p>
      <ul v-else class="mt-1 flex flex-wrap gap-2">
        <li v-for="e in props.proposal.nearby" :key="e.id" class="flex items-center gap-2">
          <NuxtLink :to="localePath(`/admin/negociations/lexique/${e.id}`)" class="text-accent underline underline-offset-4" lang="en">
            {{ e.term }}
          </NuxtLink>
          <UiBadge
            :intent="KNOWLEDGE_STATUS_INTENT[e.status]"
            size="sm"
            :label="t(`admin.negociations.lexique.state.${e.status}`)"
          />
        </li>
      </ul>
    </section>

    <UiAlert v-if="props.error" class="mt-4" intent="danger" live :message="props.error" />

    <template v-if="props.proposal.status === 'pending'">
      <UiButton
        v-if="props.acceptLink"
        class="mt-4"
        icon-trailing="arrow-right"
        :to="localePath(`/admin/negociations/file/proposition/${props.proposal.id}`)"
      >
        {{ t('admin.negociations.file.proposal.accept') }}
      </UiButton>

      <form class="mt-4 space-y-3" @submit.prevent="rejeter">
        <UiTextarea
          v-model="motif"
          :label="t('admin.negociations.file.proposal.reject.label')"
          :hint="t('admin.negociations.file.proposal.reject.hint')"
          :error="refus ?? undefined"
          :rows="2"
          auto-grow
          required
          block
        />
        <UiButton type="submit" variant="secondary" :loading="props.rejecting">
          {{ t('admin.negociations.file.proposal.reject.submit') }}
        </UiButton>
      </form>
    </template>

    <p v-else-if="props.proposal.rejection_reason" class="mt-3 text-sm text-text-secondary">
      {{ t('admin.negociations.file.proposal.rejected', { reason: props.proposal.rejection_reason }) }}
    </p>
    <UiButton
      v-else-if="props.proposal.glossary_entry_id"
      class="mt-4"
      variant="secondary"
      icon-trailing="arrow-right"
      :to="localePath(`/admin/negociations/lexique/${props.proposal.glossary_entry_id}`)"
    >
      {{ t('admin.negociations.file.proposal.open') }}
    </UiButton>
  </article>
</template>
