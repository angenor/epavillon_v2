<script setup lang="ts">
import type { MailingAttachment } from '~/types/admin-mailings'
import type { AttachableRoleRule } from '~/types/media'
import type { Uuid } from '~/types/shared'

/**
 * LES PIÈCES JOINTES D'UN ENVOI : déposées par `useDepotMedia` (même contrôle
 * de type, même envoi que les autres champs de fichier), rattachées une à une
 * au rôle `attachment` de l'envoi. L'objet est privé, la garde du module media
 * l'impose. Le poids cumulé est vérifié avant l'envoi du fichier ; l'API le
 * revérifie.
 *
 * En lecture seule (`editable` faux), la liste seule : la fiche d'un envoi
 * parti, l'étape de vérification.
 */
interface Props {
  mailingId: Uuid
  attachments: MailingAttachment[]
  totalBytes: number
  maxBytes: number | null
  editable?: boolean
}

const props = withDefaults(defineProps<Props>(), { editable: false })
const emit = defineEmits<{ changed: [] }>()

const { t, locale } = useI18n()
const api = useApi()

const rule = ref<AttachableRoleRule | null>(null)
onMounted(async () => {
  if (!props.editable) return
  try {
    const rules = await api.media.roles('engagement', 'mailings')
    rule.value = rules.find((r) => r.role === 'attachment') ?? null
  } catch {
    rule.value = null
  }
})

const { enCours, echec, accept, accepte, deposer } = useDepotMedia({
  motifs: () => rule.value?.allowed_mime_prefixes ?? [],
  repli: 'application/pdf',
})

const input = ref<HTMLInputElement | null>(null)
const removing = ref<Uuid | null>(null)

function size(bytes: number): string {
  return formatByteSize(bytes, locale.value)
}

const usage = computed(() =>
  props.maxBytes
    ? t('admin.mailing.compose.attachments.usage', { used: size(props.totalBytes), max: size(props.maxBytes) })
    : t('admin.mailing.compose.attachments.usageNoMax', { used: size(props.totalBytes) }),
)

function choose(): void {
  echec.value = null
  input.value?.click()
}

async function onFile(event: Event): Promise<void> {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0] ?? null
  target.value = ''
  if (!file) return
  if (!accepte(file.type)) {
    echec.value = t('admin.mailing.compose.attachments.typeRejected')
    return
  }
  if (props.maxBytes && props.totalBytes + file.size > props.maxBytes) {
    echec.value = t('admin.mailing.compose.attachments.tooHeavy', { max: size(props.maxBytes) })
    return
  }
  const asset = await deposer({
    file,
    filename: file.name,
    mimeType: file.type,
    // Une image exige un texte alternatif ; le nom du fichier en tient lieu.
    altText: file.type.startsWith('image/') ? { fr: file.name } : undefined,
    ownerSchema: 'engagement',
    ownerTable: 'mailings',
    ownerId: props.mailingId,
    role: 'attachment',
  })
  if (!asset) return
  try {
    await api.media.attachOne({
      owner_schema: 'engagement',
      owner_table: 'mailings',
      owner_id: props.mailingId,
      role: 'attachment',
      asset_id: asset.id,
    })
    emit('changed')
  } catch (thrown) {
    echec.value = apiErrorMessage(thrown, t)
  }
}

async function remove(attachment: MailingAttachment): Promise<void> {
  removing.value = attachment.attachment_id
  echec.value = null
  try {
    await api.media.detach(attachment.attachment_id)
    emit('changed')
  } catch (thrown) {
    echec.value = apiErrorMessage(thrown, t)
  } finally {
    removing.value = null
  }
}
</script>

<template>
  <section class="space-y-3" :aria-label="t('admin.mailing.compose.attachments.title')">
    <div class="flex flex-wrap items-end justify-between gap-2">
      <div>
        <h3 class="text-sm font-semibold text-text">{{ t('admin.mailing.compose.attachments.title') }}</h3>
        <p class="mt-0.5 text-xs text-text-subtle">{{ usage }}</p>
      </div>
      <UiButton v-if="props.editable" variant="secondary" icon="upload" :loading="enCours" @click="choose">
        {{ t('admin.mailing.compose.attachments.add') }}
      </UiButton>
      <input
        v-if="props.editable"
        ref="input"
        type="file"
        class="sr-only"
        :accept="accept"
        tabindex="-1"
        aria-hidden="true"
        @change="onFile"
      >
    </div>

    <ul v-if="props.attachments.length" class="divide-y divide-border rounded-lg border border-border bg-surface-raised">
      <li v-for="a in props.attachments" :key="a.attachment_id" class="flex min-h-(--target-min) items-center gap-3 px-3 py-2">
        <UiIcon name="document" class="shrink-0 text-text-subtle" />
        <span class="min-w-0 flex-1 text-sm text-text [overflow-wrap:anywhere]">{{ a.filename }}</span>
        <UiBadge
          v-if="a.status !== 'ready'"
          :intent="a.status === 'quarantined' ? 'danger' : 'warning'"
          size="sm"
          :label="t(a.status === 'quarantined' ? 'admin.mailing.compose.attachments.refused' : 'admin.mailing.compose.attachments.processing')"
        />
        <span class="shrink-0 text-xs text-text-subtle tabular-nums">{{ size(a.byte_size) }}</span>
        <UiButton
          v-if="props.editable"
          variant="ghost"
          icon="trash"
          icon-only
          :label="t('admin.mailing.compose.attachments.remove', { name: a.filename })"
          :loading="removing === a.attachment_id"
          @click="remove(a)"
        />
      </li>
    </ul>
    <p v-else class="rounded-md border border-dashed border-border px-3 py-2 text-sm text-text-subtle">
      {{ t('admin.mailing.compose.attachments.none') }}
    </p>

    <UiAlert v-if="echec" intent="danger" compact live :message="echec" />
  </section>
</template>
