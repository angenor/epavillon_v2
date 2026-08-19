<script setup lang="ts">
import type { ShowcaseMediaSlot } from '~/types/admin-showcase'

/**
 * LES TROIS EMPLACEMENTS DE MÉDIA D'UNE DIAPOSITIVE — `banner`, `video`, `cover`.
 *
 * LES MÉDIAS NE SONT PAS DES COLONNES (ADR-08) : ils passent par
 * `media.attachments`, sous les trois rôles déclarés pour
 * `('content','highlights')` en § 5 de `115_content.sql`. Ce panneau ne saisit
 * donc rien du formulaire — il MONTRE ce qui est rattaché, et la contrainte de
 * chaque rôle.
 *
 * LE TÉLÉVERSEMENT ARRIVE EN PHASE B ; LA CONTRAINTE S'ANNONCE MAINTENANT. Types
 * acceptés et poids maximal sont lus de `media.attachable_roles`, pas écrits
 * ici : c'est la base qui refusera un fichier de 300 Mio, et un chiffre recopié
 * dans un template mentirait dès la première modification de la règle.
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
  media: ShowcaseMediaSlot[]
  /** Le formulaire travaille : rien n'est offert pendant l'enregistrement. */
  disabled?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

/** « image, vidéo » — les préfixes MIME rendus lisibles, sans jargon. */
function typesOf(slot: ShowcaseMediaSlot): string {
  return slot.allowed_mime_prefixes
    .map((prefix) => {
      const family = prefix.split('/')[0] ?? prefix
      return t(`admin.showcase.form.media.family.${family}`)
    })
    .join(', ')
}
</script>

<template>
  <div class="grid gap-4 sm:grid-cols-2 xl:grid-cols-3">
    <div
      v-for="slot in props.media"
      :key="slot.role"
      class="flex flex-col rounded-lg border border-border bg-surface-sunken p-4"
    >
      <p class="flex items-center gap-2 font-semibold text-text">
        <UiIcon :name="SHOWCASE_MEDIA_ICON[slot.role]" size="1.05rem" aria-hidden="true" />
        {{ tr(slot.label) }}
      </p>

      <p class="mt-1 text-xs text-text-subtle">
        {{ t('common.file.types', { types: typesOf(slot) }) }} ·
        {{ t('admin.showcase.form.media.maxSize', { size: mebibytes(slot.max_byte_size) }) }}
      </p>

      <!-- CE QUI EST RATTACHÉ. L'image prêtée par la base, à la même proportion
           que le bandeau : c'est ce qui permet de voir un recadrage malheureux. -->
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

      <!-- LE BOUTON EXISTE, DÉSACTIVÉ, ET IL DIT POURQUOI. Le masquer ferait
           croire qu'un média se remplace ailleurs ; le laisser actif ferait
           croire qu'il fonctionne. -->
      <!-- EMPILÉ, PAS EN LIGNE. Les trois emplacements se rangent en grille de
           trois colonnes : posés côte à côte, le bouton et sa mention
           débordaient de la carte dès que la colonne descendait sous ~230 px.
           C'est aussi la disposition qui tient à 375 px, où la carte occupe
           toute la largeur. -->
      <div class="mt-3 flex flex-col items-start gap-2">
        <!-- 44 px et non 40 : c'est l'unique action de cette carte, et la carte
             se rend à 375 px. Les 40 px compacts sont réservés aux barres
             d'outils sur écran large (règle d'usage n° 11). Le bouton est
             désactivé aujourd'hui, il portera le téléversement en phase B :
             autant qu'il ait déjà la bonne taille. -->
        <UiButton
          variant="secondary"
          icon="upload"
          disabled
          :title="t('admin.showcase.form.media.soon')"
        >
          {{ t('common.actions.upload') }}
        </UiButton>
        <span class="text-xs text-balance text-text-subtle">
          {{ t('admin.showcase.form.media.soon') }}
        </span>
      </div>
    </div>
  </div>
</template>
