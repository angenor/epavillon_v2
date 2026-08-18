<script setup lang="ts">
import type { AssignmentState, RoleAssignmentView } from '~/types/admin-users'
import type { Intent } from '~/types/ui'

/**
 * UN RÔLE ET SA PORTÉE, EN UN SEUL OBJET — la brique de tout cet écran.
 *
 * « Administrateur » et « Administrateur de la COP31 » portent le MÊME
 * `role_code`. Un composant qui afficherait le rôle seul mentirait sur le seul
 * point qui compte, et c'est exactement ce que faisait la v1 avec son ENUM de
 * huit rôles globaux. Ce composant n'accepte donc pas un code de rôle : il exige
 * l'attribution entière, portée comprise.
 *
 * LA PORTÉE SE LIT AVANT LE RÔLE, EN LARGEUR. Un rôle global s'annonce par la
 * mention « toute la plateforme » ; un rôle porté ne s'annonce que par le nom de
 * sa cible. Les deux tiennent sur une ligne, dans cet ordre : le rôle d'abord —
 * c'est ce qu'on cherche —, la portée immédiatement après, dans la même pastille.
 * Deux pastilles séparées se liraient comme deux attributions.
 *
 * LES COULEURS SUIVENT LA CHARTE, PAS L'INTUITION. Vert pour ce qui est CONFIRMÉ
 * — l'attribution en cours ; cyan pour l'INFORMATION — celle qui prendra effet
 * plus tard ; gris pour ce qui est CLOS — l'attribution expirée ; rouge pour le
 * RETRAIT. Le jaune, qui dit « demande attention », n'a rien à faire ici : une
 * attribution n'attend jamais rien de personne.
 */

interface Props {
  assignment: RoleAssignmentView
  /** Rend l'état (en cours, à venir, expiré, retiré) à côté du libellé. */
  showState?: boolean
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { size: 'md' })

const { t } = useI18n()
const { tr } = useI18nText()

const STATE_INTENT: Record<AssignmentState, Intent> = {
  active: 'success',
  scheduled: 'info',
  expired: 'neutral',
  revoked: 'danger',
}

const intent = computed<Intent>(() => STATE_INTENT[props.assignment.state])

/**
 * Le nom de la cible.
 *
 * Une portée dont la cible a disparu se DIT — « portée introuvable » — plutôt que
 * de se taire : une attribution orpheline donne des droits sur rien, et il faut
 * pouvoir la repérer pour la retirer.
 */
const scopeLabel = computed(() => {
  const { scope_type, scope_label, is_dangling } = props.assignment
  if (scope_type === 'global') return t('admin.user.roles.scope.global')
  if (is_dangling || !scope_label) return t('admin.user.roles.scope.dangling')
  return tr(scope_label)
})

/**
 * L'infobulle porte le nom COMPLET de la cible. La pastille affiche le sigle —
 * « COP31 » plutôt que « COP31 — Conférence des Nations unies sur les
 * changements climatiques », qui chasserait les colonnes suivantes hors de
 * l'écran —, mais le nom entier doit rester atteignable sans quitter la liste.
 */
const title = computed(() =>
  t('admin.user.roles.badge.title', {
    role: tr(props.assignment.role_label),
    scope: props.assignment.scope_hint
      ? `${scopeLabel.value} (${props.assignment.scope_hint})`
      : scopeLabel.value,
    state: t(`admin.user.roles.state.${props.assignment.state}`),
  }),
)
</script>

<template>
  <span
    class="inline-flex max-w-full items-center gap-1.5 rounded-md border px-2 py-1 text-xs leading-tight"
    :class="[
      intent === 'success' && 'border-success-border bg-success-surface text-success',
      intent === 'info' && 'border-info-border bg-info-surface text-info',
      intent === 'neutral' && 'border-border bg-surface-sunken text-text-muted',
      intent === 'danger' && 'border-danger-border bg-danger-surface text-danger',
      size === 'md' && 'sm:text-sm',
    ]"
    :title="title"
  >
    <span class="truncate font-medium">{{ tr(assignment.role_label) }}</span>

    <!-- LA PORTÉE, DANS LA MÊME PASTILLE. Le séparateur est un point médian et
         non un tiret : un tiret se lit comme une plage. -->
    <span aria-hidden="true" class="opacity-50">·</span>
    <span class="truncate" :class="assignment.is_dangling && 'italic'">{{ scopeLabel }}</span>

    <span v-if="showState && assignment.state !== 'active'" class="opacity-70">
      ({{ t(`admin.user.roles.state.${assignment.state}`) }})
    </span>
  </span>
</template>
