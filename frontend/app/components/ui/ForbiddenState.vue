<script setup lang="ts">
/**
 * Accès refusé — QUATRIÈME des quatre états d'écran, et le plus négligé.
 *
 * POURQUOI IL EXISTE, alors qu'une liste vide « ferait l'affaire ». Elle ne la
 * fait pas : « aucune donnée » et « aucun droit » sont deux situations
 * différentes, et les confondre coûte des heures. C'est la même confusion que
 * celle corrigée dans `identity.administered_events()`, où l'absence
 * d'attribution renvoyait `NULL` — un filtre `x = ANY(NULL)` ne remontait alors
 * rien SANS lever d'erreur, ce qui se lisait comme une liste vide légitime.
 * `useApi()` lève `ForbiddenError` pour cette raison précise, et cet écran en
 * est la traduction visible.
 *
 * RÈGLE MÉTIER N° 8 : un administrateur peut n'avoir accès qu'à un seul
 * événement. Toute liste du back-office est filtrée par le périmètre
 * d'administration — y compris quand l'URL est forgée à la main. C'est
 * exactement le cas que cet écran rencontre.
 *
 * CE QU'IL DIT, ET CE QU'IL NE DIT PAS. Il indique quel périmètre serait
 * nécessaire et à qui s'adresser. Il ne révèle NI l'existence NI le contenu de
 * la ressource demandée : « vous n'avez pas accès à la proposition COP31-00147 »
 * apprend déjà que ce dossier existe.
 *
 * LE CONTRÔLE RÉEL EST CÔTÉ API. Cet écran sert l'ergonomie, pas la sécurité :
 * un refus qui ne repose que sur le front n'est pas un refus.
 *
 * IL EST EN AVERTISSEMENT, PAS EN NEUTRE. Le gris disait « rien ici », ce qui
 * est exactement la confusion décrite plus haut. Le jaune dit « quelque chose
 * vous attend » : une accréditation à demander, un périmètre à faire ouvrir. Et
 * il n'est pas rouge — un droit manquant n'est pas une panne, et rien n'est
 * cassé ; le rouge appellerait un « réessayer » qui n'aboutira jamais.
 */

interface Props {
  title?: string
  description?: string
  /** Périmètre attendu, en clair : « administrateur de la COP31 ». */
  requiredScope?: string
  /** Adresse ou service à contacter pour obtenir le droit. */
  contact?: string
  /** Destination du bouton de repli. */
  actionTo?: string
  actionLabel?: string
  compact?: boolean
}

const props = defineProps<Props>()
const { t } = useI18n()
const localePath = useLocalePath()
</script>

<template>
  <div
    class="flex flex-col items-center rounded-lg border border-warning-border bg-surface text-center"
    :class="props.compact ? 'px-4 py-6' : 'px-6 py-12'"
  >
    <UiIcon name="lock" :size="props.compact ? '1.5rem' : '2.75rem'" class="text-warning" />

    <p class="mt-3 font-display" :class="props.compact ? 'text-base' : 'text-xl'">
      {{ props.title ?? t('common.states.forbidden.title') }}
    </p>

    <p class="mt-1.5 max-w-(--measure) text-sm text-text-secondary">
      {{ props.description ?? t('common.states.forbidden.description') }}
    </p>

    <p
      v-if="props.requiredScope"
      class="mt-3 rounded-md border border-border bg-surface-raised px-3 py-2 text-sm text-text-muted"
    >
      {{ t('common.states.forbidden.scope', { scope: props.requiredScope }) }}
    </p>

    <p v-if="props.contact" class="mt-3 text-sm text-text-subtle">
      {{ t('common.states.forbidden.contact', { contact: props.contact }) }}
    </p>

    <div class="mt-5 flex flex-wrap justify-center gap-2">
      <slot name="actions">
        <UiButton variant="secondary" :to="localePath(props.actionTo ?? '/')" icon="arrow-left">
          {{ props.actionLabel ?? t('common.states.forbidden.action') }}
        </UiButton>
      </slot>
    </div>
  </div>
</template>
