<script setup lang="ts">
import type { AssetId, Uuid } from '~/types/shared'
import type { AttachableRoleRule, AttachedImage, EditionImageRole } from '~/types/media'
import { EDITION_IMAGE_RATIO, EDITION_IMAGE_ROLES } from '~/types/media'

/**
 * LES TROIS DÉCLINAISONS D'UNE ÉDITION — `banner` 32:9, `cover` 16:9,
 * `thumbnail` 1:1.
 *
 * ── POURQUOI TROIS FICHIERS ET NON UN SEUL RECADRÉ ──────────────────────────
 *
 * Un bandeau 32:9 rogné automatiquement dans une photographie de conférence
 * décapite les intervenants ; un carré tiré du même fichier ne garde qu'une
 * épaule. Les trois recadrages sont donc CHOISIS À LA MAIN — c'est la personne
 * qui sait où est le sujet, pas l'algorithme. L'éditeur lui donne la grille des
 * tiers et verrouille le rapport ; il ne cadre pas à sa place.
 *
 * Les trois sont INDÉPENDANTES et FACULTATIVES : on peut n'en fournir qu'une,
 * et vider un emplacement retire cette déclinaison sans toucher aux deux autres.
 * Chaque écran porte déjà son repli — l'accueil se rabat du 16:9 sur le 32:9,
 * la fiche affiche son en-tête sobre sans aucune image.
 *
 * ── LA FORME VIENT DE LA BASE, PAS D'UNE CONSTANTE ──────────────────────────
 *
 * `GET /media/roles` rend `media.attachable_roles` : rapport attendu, tolérance,
 * types acceptés, poids maximal. C'est ce qui verrouille la poignée de
 * l'éditeur. `EDITION_IMAGE_RATIO` ne sert plus qu'à ÉCRIRE la forme — « 32:9 »
 * se lit, « 3,5556 » se calcule.
 *
 * ── LE DÉPÔT A REJOINT LE RATTACHEMENT ─────────────────────────────────────
 *
 * Jusqu'au 26/08, chaque emplacement demandait un IDENTIFIANT D'OBJET saisi à la
 * main : la route de rattachement existait, celle du dépôt aussi, et il manquait
 * l'écran entre les deux. Le fichier part maintenant d'ici, et ce panneau ne
 * transmet plus que ce qu'il a toujours transmis — un identifiant par rôle.
 */

interface Props {
  /** Les images déjà rattachées, telles que `media.attached_image()` les rend. */
  images: Record<EditionImageRole, AttachedImage | null>
  /** Ce que le formulaire enverra — un identifiant d'objet par rôle. */
  modelValue: Record<EditionImageRole, AssetId | null>
  /**
   * Les règles des rôles, telles que `media.attachable_roles` les déclare.
   * Chargées par la PAGE : un composant de cet écran ne va pas chercher ses
   * données lui-même, et les trois emplacements n'ont pas à faire trois appels.
   */
  rules: AttachableRoleRule[]
  /** L'édition, quand elle existe déjà. Absente à la création. */
  eventId?: Uuid | null
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), { eventId: null })

const emit = defineEmits<{ 'update:modelValue': [value: Record<EditionImageRole, AssetId | null>] }>()

const { t } = useI18n()

/**
 * La règle d'un rôle. ABSENTE, l'éditeur recadre librement et le dépôt s'en
 * remet au refus de la base : une table de référence muette ne doit pas mettre
 * le panneau en panne, elle doit le rendre moins précis.
 */
const ruleOf = (role: EditionImageRole): AttachableRoleRule | null =>
  props.rules.find((rule) => rule.role === role) ?? null

/** L'entité porteuse du dépôt. Nulle à la création : elle n'existe pas encore. */
const owner = computed(() =>
  props.eventId ? { schema: 'event', table: 'events', id: props.eventId } : null,
)

function setRole(role: EditionImageRole, assetId: AssetId | null): void {
  emit('update:modelValue', { ...props.modelValue, [role]: assetId })
}
</script>

<template>
  <!-- `items-start` : sans lui, les trois emplacements s'étirent à la hauteur
       du plus haut — le carré — et le bandeau 32:9 se retrouve suivi de
       300 px de vide. Trois hauteurs différentes sont ici la conséquence
       normale de trois formes différentes. -->
  <div class="grid items-start gap-5 sm:grid-cols-2 xl:grid-cols-3">
    <MediaImageField
      v-for="role in EDITION_IMAGE_ROLES"
      :key="role"
      :role="role"
      :label="t(`admin.event.form.fields.images.${role}.label`)"
      :use="t(`admin.event.form.fields.images.${role}.use`)"
      :shape-label="EDITION_IMAGE_RATIO[role].replace(' / ', ':')"
      :rule="ruleOf(role)"
      :image="props.images[role]"
      :asset-id="props.modelValue[role]"
      :owner="owner"
      :disabled="props.disabled"
      @update:asset-id="(next: AssetId | null) => setRole(role, next)"
    />
  </div>
</template>
