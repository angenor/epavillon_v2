<script setup lang="ts">
/** Une adresse électronique lisible, sélectionnable et copiable d'un geste. */

const props = defineProps<{ email: string | null }>()

const { t } = useI18n()
const copied = ref(false)

async function copy(): Promise<void> {
  if (!props.email) return
  await navigator.clipboard.writeText(props.email)
  copied.value = true
  setTimeout(() => (copied.value = false), 2500)
}
</script>

<template>
  <span v-if="props.email" class="inline-flex min-w-0 items-center gap-1 font-mono text-sm text-text">
    <span class="min-w-0 break-all">{{ props.email }}</span>
    <UiButton
      variant="ghost"
      :icon="copied ? 'check' : 'copy'"
      icon-only
      :label="copied ? t('admin.proposal.dashboard.email.copied') : t('admin.proposal.dashboard.email.copy')"
      @click="copy"
    />
    <span class="sr-only" aria-live="polite">{{ copied ? t('admin.proposal.dashboard.email.copied') : '' }}</span>
  </span>
  <span v-else class="text-sm text-text-muted">{{ t('admin.proposal.dashboard.email.none') }}</span>
</template>
