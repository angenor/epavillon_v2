<script setup lang="ts">
/**
 * Carte des écrans d'authentification.
 *
 * Distincte de `UiCard`, qui sert les contenus d'une page — une carte de liste,
 * un bloc de tableau de bord. Celle-ci EST la page : elle porte le `<h1>`, elle
 * est seule à l'écran, et sa largeur est calée sur un formulaire à un champ par
 * ligne, pas sur une grille.
 *
 * UN SEUL `<h1>` PAR ÉCRAN, et c'est lui. Les cinq pages d'A1 n'ont pas d'autre
 * titre de niveau 1 : le titre de la carte est le titre du document, ce qui
 * évite le classique « ePavillon » en `<h1>` suivi d'un formulaire orphelin.
 *
 * LE PIED EST DANS LA CARTE, pas sous elle : « Pas encore de compte ? Créer un
 * compte » appartient au parcours, et un lien posé hors du cadre se lit comme
 * une note de bas de page.
 */

interface Props {
  title: string
  /** Phrase d'orientation sous le titre. Une seule, courte. */
  description?: string
  /** Surtitre — sert à situer une étape (« Étape 2 sur 2 »). */
  eyebrow?: string
  /** Nom d'icône affichée au-dessus du titre, pour les écrans de confirmation. */
  icon?: string
  /** Intention de l'icône : `success` après une vérification, `info` par défaut. */
  iconIntent?: 'info' | 'success' | 'warning' | 'danger'
}

const props = withDefaults(defineProps<Props>(), { iconIntent: 'info' })

const ICON_TONES: Record<NonNullable<Props['iconIntent']>, string> = {
  info: 'border-info-border bg-info-surface text-info',
  success: 'border-success-border bg-success-surface text-success',
  warning: 'border-warning-border bg-warning-surface text-warning',
  danger: 'border-danger-border bg-danger-surface text-danger',
}

defineSlots<{
  default: () => unknown
  /** Liens de parcours, sous un trait de séparation. */
  footer?: () => unknown
}>()

const slots = useSlots()
</script>

<template>
  <section
    class="rounded-lg border border-border bg-surface-raised p-6 shadow-(--shadow-sm) sm:p-8"
  >
    <div
      v-if="props.icon"
      class="mb-5 flex h-12 w-12 items-center justify-center rounded-full border border-(length:--border-thin)"
      :class="ICON_TONES[props.iconIntent]"
    >
      <UiIcon :name="props.icon" size="1.5rem" />
    </div>

    <p
      v-if="props.eyebrow"
      class="mb-1.5 text-xs font-bold tracking-(--tracking-caps) text-text-subtle uppercase"
    >
      {{ props.eyebrow }}
    </p>

    <h1 class="font-display text-2xl leading-tight tracking-(--tracking-title) text-text">
      {{ props.title }}
    </h1>

    <p v-if="props.description" class="mt-2 max-w-(--measure) text-sm text-text-muted">
      {{ props.description }}
    </p>

    <div class="mt-6">
      <slot />
    </div>

    <div v-if="slots.footer" class="mt-6 border-t border-border-subtle pt-5 text-sm text-text-muted">
      <slot name="footer" />
    </div>
  </section>
</template>
