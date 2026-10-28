<script setup lang="ts">
/**
 * LE MESSAGE TEL QU'UN DESTINATAIRE LE REÇOIT : en-têtes, objet et corps, les
 * variables remplacées par ses valeurs (même substitution que le worker,
 * valeurs échappées). Le corps est déjà assaini par l'API.
 */
interface Props {
  subject: string
  bodyHtml: string
  to: string
  cc: string[]
  bcc: string[]
  replyTo: string | null
  values: Record<string, string>
}

const props = defineProps<Props>()
const { t } = useI18n()

const subject = computed(() => renderMailingTemplate(props.subject, props.values, false))
const body = computed(() => renderMailingTemplate(props.bodyHtml, props.values, true))

const headers = computed(() =>
  [
    { key: 'to', value: props.to },
    { key: 'cc', value: props.cc.join(', ') },
    { key: 'bcc', value: props.bcc.join(', ') },
    { key: 'replyTo', value: props.replyTo ?? '' },
  ].filter((h) => h.value),
)
</script>

<template>
  <article class="overflow-hidden rounded-lg border border-border bg-surface-raised">
    <dl class="grid grid-cols-[auto_1fr] gap-x-4 gap-y-1 border-b border-border bg-surface-sunken px-4 py-3 text-sm">
      <template v-for="h in headers" :key="h.key">
        <dt class="text-text-subtle">{{ t(`admin.mailing.common.header.${h.key}`) }}</dt>
        <dd class="min-w-0 text-text [overflow-wrap:anywhere]">{{ h.value }}</dd>
      </template>
      <dt class="text-text-subtle">{{ t('admin.mailing.common.header.subject') }}</dt>
      <dd class="min-w-0 font-semibold text-text">{{ subject || t('admin.mailing.common.noSubject') }}</dd>
    </dl>
    <div class="px-4 py-5 sm:px-6">
      <UiRichContent :html="body" />
    </div>
  </article>
</template>
