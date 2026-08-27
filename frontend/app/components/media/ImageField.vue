<script setup lang="ts">
import type { AssetId, I18nText, Uuid } from '~/types/shared'
import type { AttachableRoleRule, AttachedImage, AttachmentRole } from '~/types/media'

/**
 * UN EMPLACEMENT D'IMAGE, DU DISQUE JUSQU'À L'OBJET DÉPOSÉ.
 *
 * ── CE QU'IL REMPLACE ───────────────────────────────────────────────────────
 *
 * Un champ de texte où l'on saisissait un IDENTIFIANT D'OBJET. La route de
 * rattachement existait, celle du dépôt aussi ; il manquait l'écran entre les
 * deux, si bien qu'illustrer une édition supposait un accès à la base.
 *
 * ── CE QUI PART N'EST PAS CE QUI A ÉTÉ CHOISI ───────────────────────────────
 *
 * Le fichier du disque passe par `UiImageEditor`, qui le recadre au rapport
 * exigé par le rôle et le réencode au poids voulu. Ce qui traverse le réseau est
 * donc recevable par construction — la forme n'est plus apprise par le refus.
 *
 * ── LE TEXTE ALTERNATIF EST OBLIGATOIRE, ET IL SE SAISIT ICI ────────────────
 *
 * `ck_assets_alt_text_required` interdit à une image d'atteindre l'état servable
 * sans lui, et le dépôt le refuse AVANT de lire le flux. Le demander après le
 * téléversement ferait perdre le fichier ; le demander avant le recadrage ferait
 * décrire une image qu'on n'a pas encore cadrée. Il se saisit donc DANS
 * l'éditeur, à côté de l'aperçu de ce qui partira. C'est une DONNÉE multilingue,
 * pas une traduction d'interface.
 *
 * ── LE DÉPÔT ET LE RATTACHEMENT SONT DEUX GESTES ────────────────────────────
 *
 * Ce composant DÉPOSE et rend un identifiant d'objet ; c'est le formulaire qui
 * rattache, à l'enregistrement. La séparation n'est pas gratuite : à la création
 * d'une édition, l'entité à laquelle rattacher n'existe pas encore. Un objet
 * déposé puis abandonné n'est pas perdu pour autant — `media.find_orphan_assets()`
 * le retrouve, et le worker de purge s'en charge.
 */

const props = withDefaults(
  defineProps<{
    /** Le rôle visé — `banner`, `cover`, `thumbnail`… */
    role: AttachmentRole
    label: string
    /** À quoi sert cette déclinaison, en une ligne. */
    use?: string
    /** La règle du rôle, telle que `media.attachable_roles` la déclare. */
    rule?: AttachableRoleRule | null
    /** Forme annoncée à l'œil — « 32:9 ». Le rapport qui TRANCHE vient de `rule`. */
    shapeLabel?: string
    /** L'image déjà rattachée, telle que `media.attached_image()` la rend. */
    image?: AttachedImage | null
    /** L'objet retenu par le formulaire. `null` : l'emplacement est vide. */
    assetId: AssetId | null
    /**
     * L'entité porteuse, quand elle existe déjà. Absente à la création : le dépôt
     * a lieu quand même, et le rattachement suivra l'enregistrement.
     */
    owner?: { schema: string; table: string; id: Uuid } | null
    disabled?: boolean
  }>(),
  { rule: null, image: null, owner: null },
)

const emit = defineEmits<{ 'update:assetId': [value: AssetId | null] }>()

const { t, locale } = useI18n()
const api = useApi()

const input = ref<HTMLInputElement | null>(null)
const chosen = ref<File | null>(null)
const altText = ref<I18nText | null>(null)
const busy = ref(false)
const failure = ref<string | null>(null)

/** L'objet déposé pendant cette session d'écran, pas encore rattaché. */
const uploaded = ref<AttachedImage | null>(null)

/**
 * CE QUE L'EMPLACEMENT MONTRE. L'objet retenu commande : vidé par le formulaire,
 * l'aperçu disparaît sans qu'on ait à le lui dire, et un objet fraîchement
 * déposé l'emporte sur celui que l'API avait rendu au chargement.
 */
const shown = computed<AttachedImage | null>(() => {
  if (!props.assetId) return null
  if (uploaded.value?.asset_id === props.assetId) return uploaded.value
  return props.image?.asset_id === props.assetId ? props.image : null
})

/** Le rapport largeur ÷ hauteur exigé. Il traverse en texte — voir le modèle. */
const aspectRatio = computed<number | null>(() => {
  const declared = props.rule?.expected_aspect_ratio
  if (!declared) return null
  const value = Number(declared)
  return Number.isFinite(value) && value > 0 ? value : null
})

const accept = computed(() => {
  const prefixes = props.rule?.allowed_mime_prefixes ?? []
  return prefixes.length > 0 ? prefixes.join(',') : 'image/*'
})

/** Le type est-il accepté ? Même règle que la base : `*` y vaut « n'importe quoi ». */
function accepted(mime: string): boolean {
  const prefixes = props.rule?.allowed_mime_prefixes ?? []
  if (prefixes.length === 0) return mime.startsWith('image/')
  return prefixes.some((pattern) => {
    const [head, tail] = pattern.split('*')
    if (tail === undefined) return mime === pattern
    return mime.startsWith(head ?? '') && mime.endsWith(tail)
  })
}

function pick(): void {
  failure.value = null
  input.value?.click()
}

function onFileChosen(event: Event): void {
  const target = event.target as HTMLInputElement
  const file = target.files?.[0] ?? null
  // Le champ est REMIS À ZÉRO : sans cela, rechoisir le même fichier après une
  // annulation n'émettrait aucun événement, et le bouton semblerait mort.
  target.value = ''
  if (!file) return
  if (!accepted(file.type)) {
    failure.value = t('image-field.errors.mimeRejected')
    return
  }
  failure.value = null
  altText.value = props.image?.alt_text ?? null
  chosen.value = file
}

function cancelEditing(): void {
  chosen.value = null
  altText.value = null
}

async function upload(result: {
  blob: Blob
  width: number
  height: number
  mimeType: string
  filename: string
}): Promise<void> {
  const description = altText.value
  if (!description?.fr) return
  busy.value = true
  failure.value = null
  try {
    const asset = await api.media.upload({
      file: result.blob,
      filename: result.filename,
      mimeType: result.mimeType,
      altText: description,
      role: props.role,
      ownerSchema: props.owner?.schema,
      ownerTable: props.owner?.table,
      ownerId: props.owner?.id,
    })
    // LES DIMENSIONS VIENNENT DE L'ÉDITEUR, pas de la réponse : la base les
    // renseigne au traitement, et l'aperçu ne peut pas attendre le worker.
    uploaded.value = {
      asset_id: asset.id,
      url: asset.url,
      width: result.width,
      height: result.height,
      alt_text: asset.alt_text ?? description,
      caption: null,
      credit: null,
      sources: asset.sources,
    }
    emit('update:assetId', asset.id)
    cancelEditing()
  } catch (thrown) {
    failure.value = apiErrorMessage(thrown, t)
  } finally {
    busy.value = false
  }
}

function clear(): void {
  failure.value = null
  uploaded.value = null
  emit('update:assetId', null)
}

/**
 * LE MESSAGE NOMME LA LANGUE, et ce n'est pas un détail.
 *
 * Le champ de description a DEUX ONGLETS et le français est la langue de repli
 * du modèle : une description saisie en anglais SEULEMENT laisse la valeur nulle
 * et l'envoi fermé. Le composant de saisie ne remonte rien tant que le français
 * manque — l'écran ne peut donc pas distinguer « rien saisi » de « saisi dans
 * l'autre onglet ». Un message qui dirait « une description est obligatoire » à
 * quelqu'un qui vient d'en écrire une le ferait tourner en rond ; celui-ci dit
 * la langue, et couvre les deux cas.
 */
const maxLabel = computed(() =>
  props.rule?.max_byte_size ? formatByteSize(props.rule.max_byte_size, locale.value) : null,
)
</script>

<template>
  <section class="flex flex-col gap-3 rounded-lg border border-border-subtle bg-surface p-4">
    <header>
      <h3 class="text-sm font-semibold">{{ props.label }}</h3>
      <p class="mt-0.5 text-xs text-text-subtle">
        <template v-if="props.shapeLabel">
          {{ t('image-field.shape', { ratio: props.shapeLabel }) }}
        </template>
        <template v-if="props.use"> — {{ props.use }}</template>
        <template v-if="maxLabel"> — {{ t('image-field.max', { max: maxLabel }) }}</template>
      </p>
    </header>

    <UiImage
      v-if="shown"
      :image="shown"
      :ratio="aspectRatio ? String(aspectRatio) : 'auto'"
      rounded="rounded-md"
      sizes="(min-width: 1280px) 20rem, (min-width: 640px) 45vw, 90vw"
    />
    <!-- L'EMPLACEMENT VIDE GARDE LA FORME du fichier attendu : c'est ce qui fait
         comprendre « très large » ou « carré » sans avoir à le lire. -->
    <p
      v-else
      class="flex items-center justify-center rounded-md border border-dashed border-border px-3 text-center text-xs text-text-subtle"
      :style="{ aspectRatio: aspectRatio ?? 1.7778 }"
    >
      {{ t('image-field.empty') }}
    </p>

    <div class="flex flex-wrap items-center gap-2">
      <UiButton
        variant="secondary"
        size="sm"
        icon="upload"
        :disabled="props.disabled || busy"
        @click="pick"
      >
        {{ shown ? t('image-field.actions.replace') : t('image-field.actions.choose') }}
      </UiButton>
      <UiButton
        v-if="props.assetId"
        variant="ghost"
        size="sm"
        icon="trash"
        :disabled="props.disabled || busy"
        @click="clear"
      >
        {{ t('image-field.actions.remove') }}
      </UiButton>
    </div>

    <UiAlert v-if="failure" intent="danger" live compact :message="failure" />

    <input
      ref="input"
      type="file"
      class="sr-only"
      :accept="accept"
      :aria-label="props.label"
      @change="onFileChosen"
    >

    <!-- L'ÉDITEUR N'EST MONTÉ QU'À L'OUVERTURE : sa mesure de cadre a besoin
         d'une boîte de dialogue déjà visible, et un canevas dessiné dans une
         largeur nulle ne montre rien sans rien signaler. -->
    <UiImageEditor
      v-if="chosen"
      :file="chosen"
      :title="t('image-field.editor.title', { slot: props.label })"
      :description="t('image-field.editor.description')"
      :aspect-ratio="aspectRatio"
      :ratio-label="props.shapeLabel"
      :max-byte-size="props.rule?.max_byte_size ?? null"
      :busy="busy"
      :apply-disabled="!altText?.fr"
      :apply-hint="t('image-field.altText.required')"
      :apply-label="t('image-field.editor.apply')"
      @apply="upload"
      @cancel="cancelEditing"
    >
      <template #aside>
        <AdminEventsI18nField
          v-model="altText"
          :label="t('image-field.altText.label')"
          :hint="t('image-field.altText.hint')"
          :disabled="busy"
          required
          multiline
          :rows="3"
          :maxlength="300"
        />
      </template>
    </UiImageEditor>
  </section>
</template>
