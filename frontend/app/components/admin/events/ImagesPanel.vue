<script setup lang="ts">
import type { AssetId } from '~/types/shared'
import type { AttachedImage, EditionImageRole } from '~/types/media'
import { EDITION_IMAGE_RATIO, EDITION_IMAGE_ROLES } from '~/types/media'

/**
 * LES TROIS DÉCLINAISONS D'UNE ÉDITION — `banner` 32:9, `cover` 16:9,
 * `thumbnail` 1:1.
 *
 * ── POURQUOI TROIS FICHIERS ET NON UN SEUL RECADRÉ ──────────────────────────
 *
 * Un bandeau 32:9 rogné automatiquement dans une photographie de conférence
 * décapite les intervenants ; un carré tiré du même fichier ne garde qu'une
 * épaule. Les trois recadrages sont donc TÉLÉVERSÉS À LA MAIN — c'est la
 * personne qui sait où est le sujet, pas l'algorithme.
 *
 * Les trois sont INDÉPENDANTES et FACULTATIVES : on peut n'en fournir qu'une,
 * et vider un champ retire cette déclinaison sans toucher aux deux autres.
 * Chaque écran porte déjà son repli — l'accueil se rabat du 16:9 sur le 32:9,
 * la fiche affiche son en-tête sobre sans aucune image.
 *
 * ── LA FORME EST ANNONCÉE ICI, ELLE EST EXIGÉE EN BASE ──────────────────────
 *
 * `media.attachable_roles.expected_aspect_ratio` porte le rapport attendu et le
 * trigger de `media.attachments` refuse ce qui s'en écarte de plus de 2 % : un
 * 16:9 déposé en bandeau est REJETÉ, pas rogné. Cet écran ne revérifie donc
 * rien — il ANNONCE, pour que le refus n'arrive pas en surprise. Un contrôle
 * réécrit ici finirait par accepter ce que la base refuse, ou l'inverse.
 *
 * ── LE TÉLÉVERSEMENT N'EXISTE PAS ENCORE ────────────────────────────────────
 *
 * Le module Média n'est pas raccordé : le champ transmet un identifiant d'objet
 * et l'écran le DIT, plutôt que d'offrir un bouton qui ne ferait rien. Même
 * traitement que la vitrine (`AdminShowcaseMediaPanel`).
 */

interface Props {
  /** Les images déjà rattachées, telles que `media.attached_image()` les rend. */
  images: Record<EditionImageRole, AttachedImage | null>
  /** Ce que le formulaire enverra — un identifiant d'objet par rôle. */
  modelValue: Record<EditionImageRole, AssetId | null>
}

const props = defineProps<Props>()

const emit = defineEmits<{ 'update:modelValue': [value: Record<EditionImageRole, AssetId | null>] }>()

const { t } = useI18n()

function setRole(role: EditionImageRole, raw: string): void {
  emit('update:modelValue', { ...props.modelValue, [role]: (raw.trim() || null) as AssetId | null })
}
</script>

<template>
  <!-- `items-start` : sans lui, les trois emplacements s'étirent à la hauteur
       du plus haut — le carré — et le bandeau 32:9 se retrouve suivi de
       300 px de vide. Trois hauteurs différentes sont ici la conséquence
       normale de trois formes différentes. -->
  <div class="grid items-start gap-5 sm:grid-cols-2 xl:grid-cols-3">
    <section
      v-for="role in EDITION_IMAGE_ROLES"
      :key="role"
      class="flex flex-col gap-3 rounded-lg border border-border-subtle bg-surface p-4"
    >
      <header>
        <h3 class="text-sm font-semibold">
          {{ t(`admin.event.form.fields.images.${role}.label`) }}
        </h3>
        <!-- LA FORME EXIGÉE, ÉCRITE. Elle vient de `EDITION_IMAGE_RATIO`, qui
             reprend `attachable_roles.expected_aspect_ratio` : deux endroits,
             une seule vérité, et celle qui tranche est en base. -->
        <p class="mt-0.5 text-xs text-text-subtle">
          {{ t('admin.event.form.fields.images.shape', {
            ratio: EDITION_IMAGE_RATIO[role].replace(' / ', ':'),
          }) }}
          — {{ t(`admin.event.form.fields.images.${role}.use`) }}
        </p>
      </header>

      <UiImage
        v-if="props.images[role]"
        :image="props.images[role]"
        :ratio="EDITION_IMAGE_RATIO[role]"
        rounded="rounded-md"
        sizes="(min-width: 1280px) 20rem, (min-width: 640px) 45vw, 90vw"
      />
      <!-- L'EMPLACEMENT VIDE GARDE LA FORME du fichier attendu : c'est ce qui
           fait comprendre « très large » ou « carré » sans le lire. -->
      <p
        v-else
        class="flex items-center justify-center rounded-md border border-dashed border-border px-3 text-center text-xs text-text-subtle"
        :style="{ aspectRatio: EDITION_IMAGE_RATIO[role] }"
      >
        {{ t('admin.event.form.fields.images.none') }}
      </p>

      <UiInput
        :model-value="props.modelValue[role] ?? ''"
        :label="t('admin.event.form.fields.images.assetId')"
        @update:model-value="(next: string) => setRole(role, next)"
      />
    </section>
  </div>
</template>
