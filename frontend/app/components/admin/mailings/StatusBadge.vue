<script setup lang="ts">
import type { MailingRecipientStatus, MailingStatus } from '~/types/admin-mailings'

/** L'état d'un envoi, ou d'un de ses destinataires. Couleurs du guide : jaune en cours, vert parti, rouge échec, gris clos. */
interface Props {
  of: 'mailing' | 'recipient'
  status: MailingStatus | MailingRecipientStatus
}

const props = defineProps<Props>()
const { t } = useI18n()

const intent = computed(() =>
  props.of === 'mailing'
    ? mailingStatusIntent(props.status as MailingStatus)
    : recipientStatusIntent(props.status as MailingRecipientStatus),
)
const label = computed(() =>
  props.of === 'mailing'
    ? t(`admin.mailing.common.status.${props.status}`)
    : t(`admin.mailing.common.recipientStatus.${props.status}`),
)
</script>

<template>
  <UiBadge :intent="intent" size="sm" :label="label" />
</template>
