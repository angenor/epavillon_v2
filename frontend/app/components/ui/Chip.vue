<script setup lang="ts">
/**
 * Jeton de filtre actif — retirable.
 *
 * À NE PAS CONFONDRE AVEC `UiBadge` : une pastille informe, un jeton AGIT. Le
 * jeton dit « ce filtre est appliqué, voici comment l'enlever ». Il n'apparaît
 * donc jamais seul : il matérialise l'état d'une liste filtrée, au-dessus de
 * cette liste.
 *
 * DEUX ZONES CLIQUABLES DISTINCTES. Le corps du jeton peut ouvrir le filtre pour
 * le modifier (`clickable`), la croix le retire. Elles sont séparées dans le
 * balisage : un bouton dans un bouton n'est pas du HTML valide, et le lecteur
 * d'écran n'y comprend rien. Sans `clickable`, le corps n'est pas focalisable et
 * seule la croix l'est.
 *
 * NOM ACCESSIBLE DE LA CROIX : « Retirer le filtre Adaptation », pas « Retirer ».
 * Dix croix identiques dans une barre de filtres, ce sont dix boutons
 * indiscernables à l'oreille.
 */

interface Props {
  label: string
  /** Critère filtré, affiché en tête : « Thématique : Adaptation ». */
  facet?: string
  /** Couleur venue de la base, rendue en point — voir l'en-tête de `UiBadge`. */
  dotColor?: string | null
  /** Le corps ouvre le filtre. Sans cela, seule la croix est focalisable. */
  clickable?: boolean
  disabled?: boolean
  /** Le jeton n'est pas retirable : la croix disparaît (filtre imposé par le périmètre). */
  fixed?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ remove: []; click: [] }>()

const { t } = useI18n()

const bodyClasses = computed(() => [
  'inline-flex items-center gap-1.5 rounded-full border border-border bg-surface-raised py-1 text-xs text-text transition-colors',
  props.fixed ? 'pr-3 pl-2.5' : 'pr-1 pl-2.5',
  props.clickable && !props.disabled ? 'hover:bg-surface-hover hover:border-border-strong' : '',
  props.disabled ? 'opacity-60' : '',
])
</script>

<template>
  <span :class="bodyClasses">
    <component
      :is="props.clickable && !props.disabled ? 'button' : 'span'"
      :type="props.clickable && !props.disabled ? 'button' : undefined"
      class="inline-flex items-center gap-1.5 rounded-full"
      @click="props.clickable && !props.disabled ? emit('click') : undefined"
    >
      <span
        v-if="props.dotColor"
        class="size-2 shrink-0 rounded-full ring-1 ring-black/10"
        :style="{ backgroundColor: props.dotColor }"
        aria-hidden="true"
      />
      <span v-if="props.facet" class="text-text-subtle">{{ props.facet }} :</span>
      <span class="font-medium">{{ props.label }}</span>
    </component>

    <button
      v-if="!props.fixed"
      type="button"
      class="ml-0.5 inline-flex size-5 items-center justify-center rounded-full text-text-subtle transition-colors hover:bg-surface-sunken hover:text-danger disabled:cursor-not-allowed disabled:hover:bg-transparent disabled:hover:text-text-subtle"
      :disabled="props.disabled"
      @click="emit('remove')"
    >
      <span class="sr-only">{{ t('common.actions.remove') }} — {{ props.label }}</span>
      <UiIcon name="close" size="0.85rem" :stroke-width="2.2" />
    </button>
  </span>
</template>
