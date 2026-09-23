<script setup lang="ts">
import type { AssetId, Uuid } from '~/types/shared'
import type { UploadedAsset } from '~/types/media'

/**
 * Un emplacement de fichier qui n'est pas une image — le PDF d'un document de
 * négociation, d'abord. Il dépose par `useDepotMedia`, comme `MediaImageField` :
 * même contrôle du type, même envoi, mêmes messages. Ni recadrage ni texte
 * alternatif, qu'un document n'a pas.
 *
 * Il dépose et rend l'objet ; c'est l'écran qui le rattache.
 */

const props = withDefaults(
  defineProps<{
    label: string
    /** Les types acceptés, en motifs MIME — `application/pdf`. */
    types: readonly string[]
    assetId: AssetId | null
    /** Le fichier déjà retenu, pour le nommer. */
    current?: { filename: string; byteSize: number } | null
    owner?: { schema: string; table: string; id: Uuid } | null
    maxByteSize?: number | null
    hint?: string
    disabled?: boolean
  }>(),
  { current: null, owner: null, maxByteSize: null, hint: undefined },
)

const emit = defineEmits<{
  'update:assetId': [value: AssetId | null]
  uploaded: [asset: UploadedAsset]
}>()

const { t, locale } = useI18n()
const { enCours, echec, accept, accepte, deposer } = useDepotMedia({
  motifs: () => props.types,
  repli: props.types[0] ?? 'application/octet-stream',
})

const input = ref<HTMLInputElement | null>(null)
const envoye = ref<{ filename: string; byteSize: number } | null>(null)
const enVol = ref<{ filename: string; byteSize: number } | null>(null)

const affiche = computed(() => {
  if (!props.assetId) return null
  return envoye.value ?? props.current
})

const typesLisibles = computed(() =>
  props.types.map((type) => (type.split('/')[1] ?? type).toUpperCase()).join(', '),
)
const plafond = computed(() =>
  props.maxByteSize ? formatByteSize(props.maxByteSize, locale.value) : null,
)

function poids(octets: number): string {
  return formatByteSize(octets, locale.value)
}

function choisir(): void {
  echec.value = null
  input.value?.click()
}

async function onFichier(event: Event): Promise<void> {
  const cible = event.target as HTMLInputElement
  const fichier = cible.files?.[0] ?? null
  // Remis à zéro : rechoisir le même fichier après un refus doit relancer.
  cible.value = ''
  if (!fichier) return
  if (!accepte(fichier.type)) {
    echec.value = t('file-field.errors.mimeRejected')
    return
  }
  if (props.maxByteSize && fichier.size > props.maxByteSize) {
    echec.value = t('file-field.errors.tooLarge', { max: plafond.value })
    return
  }
  enVol.value = { filename: fichier.name, byteSize: fichier.size }
  const asset = await deposer({
    file: fichier,
    filename: fichier.name,
    mimeType: fichier.type,
    ownerSchema: props.owner?.schema,
    ownerTable: props.owner?.table,
    ownerId: props.owner?.id,
  })
  enVol.value = null
  if (!asset) return
  envoye.value = { filename: asset.original_filename ?? fichier.name, byteSize: asset.byte_size }
  emit('update:assetId', asset.id)
  emit('uploaded', asset)
}

function retirer(): void {
  echec.value = null
  envoye.value = null
  emit('update:assetId', null)
}
</script>

<template>
  <section class="flex flex-col gap-3 rounded-lg border border-border-subtle bg-surface p-4">
    <header>
      <h3 class="text-sm font-semibold">{{ props.label }}</h3>
      <p class="mt-0.5 text-xs text-text-subtle">
        {{ t('file-field.types', { types: typesLisibles }) }}
        <template v-if="plafond"> — {{ t('file-field.max', { max: plafond }) }}</template>
        <template v-if="props.hint"> — {{ props.hint }}</template>
      </p>
    </header>

    <p
      v-if="enVol"
      class="flex items-center gap-2 rounded-md border border-border px-3 py-2 text-sm"
      aria-live="polite"
    >
      <UiSpinner />
      {{ t('file-field.sending', { name: enVol.filename, size: poids(enVol.byteSize) }) }}
    </p>
    <p
      v-else-if="affiche"
      class="flex items-center gap-2 rounded-md border border-border px-3 py-2 text-sm"
    >
      <UiIcon name="document" />
      <span class="min-w-0 truncate">{{ affiche.filename }}</span>
      <span class="shrink-0 text-text-subtle">{{ poids(affiche.byteSize) }}</span>
    </p>
    <p
      v-else
      class="rounded-md border border-dashed border-border px-3 py-2 text-sm text-text-subtle"
    >
      {{ t('file-field.empty') }}
    </p>

    <div class="flex flex-wrap items-center gap-2">
      <UiButton
        variant="secondary"
        size="sm"
        icon="upload"
        :loading="enCours"
        :disabled="props.disabled"
        @click="choisir"
      >
        {{ affiche ? t('file-field.actions.replace') : t('file-field.actions.choose') }}
      </UiButton>
      <UiButton
        v-if="props.assetId"
        variant="ghost"
        size="sm"
        icon="trash"
        :disabled="props.disabled || enCours"
        @click="retirer"
      >
        {{ t('file-field.actions.remove') }}
      </UiButton>
    </div>

    <UiAlert v-if="echec" intent="danger" live compact :message="echec" />

    <input
      ref="input"
      type="file"
      class="sr-only"
      :accept="accept"
      :aria-label="props.label"
      @change="onFichier"
    >
  </section>
</template>
