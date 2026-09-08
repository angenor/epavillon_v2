<script setup lang="ts">
import type { ShowcaseMediaSlot } from '~/types/admin-showcase'
import type { HighlightId } from '~/types/content'
import type { AttachableRoleRule, AttachedImage } from '~/types/media'

/**
 * LES TROIS EMPLACEMENTS DE MÉDIA D'UNE DIAPOSITIVE — `banner`, `video`, `cover`.
 *
 * LES MÉDIAS NE SONT PAS DES COLONNES (ADR-08) : ils passent par
 * `media.attachments`, sous les trois rôles déclarés pour
 * `('content','highlights')` en § 5 de `115_content.sql`. Le formulaire ne les
 * enregistre donc pas avec le reste — c'est `PUT /media/attachments` qui les
 * pose, une fois la diapositive créée.
 *
 * ── LE TÉLÉVERSEMENT EST CELUI DES ÉDITIONS, PAS UN SECOND ─────────────────
 *
 * Ce panneau n'a plus de bouton à lui : chaque emplacement d'IMAGE est un
 * `MediaImageField`, le même composant que les trois déclinaisons d'une édition.
 * Il apporte avec lui ce qu'un bouton écrit ici n'aurait pas eu — le recadrage
 * au rapport du rôle, le réencodage au poids voulu, le texte alternatif exigé
 * par `ck_assets_alt_text_required`, et le dépôt lui-même. Jusqu'au 05/09 le
 * bouton était désactivé sous la mention « bientôt » : la chaîne existait
 * pourtant, à un écran de distance.
 *
 * LE FOND VIDÉO RESTE EN ATTENTE, ET IL EST LE SEUL. 200 Mio ne traversent pas
 * un envoi d'un seul tenant sans reprise ni progression ; l'emplacement garde
 * donc son bouton fermé et dit pourquoi. Il est aussi ABSENT du lot envoyé —
 * `PUT /media/attachments` vide tout rôle qu'on nomme, et le nommer sans savoir
 * le remplir détacherait une vidéo posée par ailleurs.
 *
 * ── LA FORME VIENT DE `media.attachable_roles`, PAS D'UNE CONSTANTE ────────
 *
 * `rules` est chargé par la PAGE (`GET /media/roles`) : types acceptés, poids
 * maximal, rapport attendu. La vitrine n'en déclare aucun — le recadrage y est
 * donc libre, et l'éditeur le dit. Un chiffre recopié dans un template mentirait
 * dès la première modification de la règle.
 *
 * « AUCUNE VIDÉO » ET « LA VIDÉO ARRIVE » NE SE RENDENT PAS PAREIL. Un objet
 * rattaché mais non `ready` — en traitement, en quarantaine, en échec — n'est
 * pas servi : `current` est nul alors qu'un fichier existe. Sans `is_pending`,
 * l'emplacement paraîtrait vide et l'éditeur téléverserait une seconde fois.
 *
 * LE LIBELLÉ DU RÔLE EST UNE DONNÉE (`media.attachable_roles.label`,
 * `platform.i18n_text`), résolue par `tr()` — jamais une clé i18n.
 */

interface Props {
  /**
   * Les emplacements ET leur contenu courant : c'est l'ÉTAT du panneau, pas une
   * lecture figée. Un dépôt remplace le `current` du rôle, et le formulaire en
   * dérive à la fois ce qu'il enverra et ce que son aperçu montre.
   */
  modelValue: ShowcaseMediaSlot[]
  /** Ce que chaque rôle exige, tel que `media.attachable_roles` le déclare. */
  rules: AttachableRoleRule[]
  /** La diapositive, quand elle existe déjà. Nulle à la création. */
  highlightId?: HighlightId | null
  /** Le formulaire travaille : rien n'est offert pendant l'enregistrement. */
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), { highlightId: null })

const emit = defineEmits<{ 'update:modelValue': [value: ShowcaseMediaSlot[]] }>()

const { t } = useI18n()
const { tr } = useI18nText()

/** L'entité porteuse du dépôt. Nulle à la création : elle n'existe pas encore. */
const owner = computed(() =>
  props.highlightId ? { schema: 'content', table: 'highlights', id: props.highlightId } : null,
)

/**
 * UN EMPLACEMENT D'IMAGE se téléverse ; les autres non. La question se pose aux
 * PRÉFIXES MIME du rôle et non à son nom : `video` s'appelle ainsi aujourd'hui,
 * et rien n'interdit qu'un rôle de document rejoigne la table demain.
 */
function isImageSlot(slot: ShowcaseMediaSlot): boolean {
  const prefixes = slot.allowed_mime_prefixes
  return prefixes.length > 0 && prefixes.every((prefix) => prefix.startsWith('image/'))
}

const ruleOf = (slot: ShowcaseMediaSlot): AttachableRoleRule | null =>
  props.rules.find((rule) => rule.role === slot.role) ?? null

/** « image, vidéo » — les préfixes MIME rendus lisibles, sans jargon. */
function typesOf(slot: ShowcaseMediaSlot): string {
  return slot.allowed_mime_prefixes
    .map((prefix) => {
      const family = prefix.split('/')[0] ?? prefix
      return t(`admin.showcase.form.media.family.${family}`)
    })
    .join(', ')
}

/**
 * Ce qui s'écrit sous le titre de l'emplacement : à quoi il sert, et ce qu'il
 * accepte. Le poids maximal, lui, est annoncé par le champ depuis la règle.
 */
function useOf(slot: ShowcaseMediaSlot): string {
  return t('common.file.types', { types: typesOf(slot) })
}

/**
 * Le dépôt a produit une image : elle devient le contenu de l'emplacement.
 * `is_pending` retombe — l'objet est là, et l'original est servi tant que le
 * worker n'a pas produit ses déclinaisons.
 */
function setImage(slot: ShowcaseMediaSlot, image: AttachedImage | null): void {
  emit(
    'update:modelValue',
    props.modelValue.map((entry) =>
      entry.role === slot.role ? { ...entry, current: image, is_pending: false } : entry,
    ),
  )
}
</script>

<template>
  <!-- `items-start` : sans lui, les emplacements s'étirent à la hauteur du plus
       haut, et un fond photographique se retrouve suivi du vide de la vignette. -->
  <div class="grid items-start gap-5 sm:grid-cols-2 xl:grid-cols-3">
    <template v-for="slot in props.modelValue" :key="slot.role">
      <MediaImageField
        v-if="isImageSlot(slot)"
        :role="slot.role"
        :label="tr(slot.label)"
        :use="useOf(slot)"
        :rule="ruleOf(slot)"
        :image="slot.current"
        :asset-id="slot.current?.asset_id ?? null"
        :owner="owner"
        :disabled="props.disabled"
        @update:image="(next: AttachedImage | null) => setImage(slot, next)"
      />

      <!-- LE FOND VIDÉO : montré, annoncé, pas encore téléversable. Voir l'en-tête. -->
      <section v-else class="flex flex-col rounded-lg border border-border-subtle bg-surface p-4">
        <h3 class="flex items-center gap-2 text-sm font-semibold">
          <UiIcon :name="SHOWCASE_MEDIA_ICON[slot.role]" size="1.05rem" aria-hidden="true" />
          {{ tr(slot.label) }}
        </h3>

        <p class="mt-0.5 text-xs text-text-subtle">
          {{ useOf(slot) }} —
          {{ t('admin.showcase.form.media.maxSize', { size: mebibytes(slot.max_byte_size) }) }}
        </p>

        <UiImage
          v-if="slot.current"
          class="mt-3"
          :image="slot.current"
          ratio="16 / 9"
          rounded="rounded-md"
        />

        <!-- EN TRAITEMENT. Ni vide, ni prêt : l'objet existe, il n'est pas servi. -->
        <p
          v-else-if="slot.is_pending"
          class="mt-3 flex items-center gap-2 rounded-md border border-warning-border bg-warning-surface px-3 py-2 text-xs text-warning"
        >
          <UiIcon name="clock" size="0.95rem" aria-hidden="true" />
          {{ t('admin.showcase.form.media.pending') }}
        </p>

        <p
          v-else
          class="mt-3 rounded-md border border-dashed border-border px-3 py-6 text-center text-xs text-text-subtle"
        >
          {{ t('admin.showcase.form.media.empty') }}
        </p>

        <!-- 44 px et non 40 : c'est l'unique action de cette carte, et la carte
             se rend à 375 px (règle d'usage n° 11). Le bouton existe, désactivé,
             et il dit pourquoi : le masquer ferait croire qu'un fond vidéo se
             pose ailleurs, le laisser actif ferait croire qu'il fonctionne. -->
        <div class="mt-3 flex flex-col items-start gap-2">
          <UiButton
            variant="secondary"
            icon="upload"
            disabled
            :title="t('admin.showcase.form.media.videoSoon')"
          >
            {{ t('common.actions.upload') }}
          </UiButton>
          <span class="text-xs text-balance text-text-subtle">
            {{ t('admin.showcase.form.media.videoSoon') }}
          </span>
        </div>
      </section>
    </template>
  </div>
</template>
