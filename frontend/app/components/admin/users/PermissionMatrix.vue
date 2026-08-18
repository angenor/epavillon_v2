<script setup lang="ts">
import type { EffectivePermissionsView } from '~/types/admin-users'

/**
 * « VOICI CE QUE CETTE PERSONNE PEUT FAIRE, ET OÙ » — l'écran d'explication.
 *
 * IL RÉPOND À UNE QUESTION QUI EST PRESQUE TOUJOURS NÉGATIVE. On n'ouvre pas cet
 * écran pour admirer une liste de droits : on l'ouvre parce que quelqu'un dit
 * « je ne vois pas le bouton », ou parce qu'on se demande pourquoi il l'a vu. Il
 * montre donc les deux moitiés — ce qui est accordé, et ce qui ne l'est pas.
 *
 * CHAQUE PERMISSION DIT D'OÙ ELLE VIENT. `effective_permissions()` rend des
 * lignes (permission, portée) : suffisant pour autoriser, muet pour expliquer. La
 * réponse utile n'est pas « elle a `programme.proposal.decide` sur `event:01a…` »
 * mais « elle est administratrice de la COP31 » — donc le rôle, et la portée,
 * pour chaque octroi.
 *
 * LE PÉRIMÈTRE D'ADMINISTRATION EST DIT EN TÊTE, en une phrase. C'est ce que tout
 * le back-office lit avant d'afficher quoi que ce soit
 * (`identity.administered_events()`), et le déduire de vingt-quatre lignes serait
 * un exercice que personne ne fera.
 *
 * GROUPÉ PAR MODULE, dans l'ordre du jalon et non par ordre alphabétique — qui
 * mettrait « Analytique » en tête et « Programmation » en huitième position.
 */

interface Props {
  view: EffectivePermissionsView
  /** Noms des éditions administrées, résolus par la page. */
  administeredLabels: string[]
  loading?: boolean
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const showMissing = ref(false)

/** La phrase du périmètre, en toutes lettres. Trois cas, trois lectures. */
const scopeSentence = computed(() => {
  const { is_global, event_ids } = props.view.administered
  if (is_global) return t('admin.user.detail.permissions.scope.global')
  if (event_ids.length === 0) return t('admin.user.detail.permissions.scope.none')
  return t('admin.user.detail.permissions.scope.events', {
    count: event_ids.length,
    list: props.administeredLabels.join(', '),
  })
})

const scopeIntent = computed(() => {
  const { is_global, event_ids } = props.view.administered
  if (is_global) return 'warning' as const
  return event_ids.length === 0 ? ('neutral' as const) : ('info' as const)
})
</script>

<template>
  <div class="space-y-6">
    <div v-if="loading" class="space-y-3">
      <UiSkeletonLoader v-for="n in 3" :key="n" height="6rem" />
    </div>

    <template v-else>
      <!-- LE PÉRIMÈTRE D'ABORD. C'est la phrase que le back-office lit en premier. -->
      <UiAlert
        :intent="scopeIntent"
        :title="t('admin.user.detail.permissions.scope.title')"
        :message="scopeSentence"
      />

      <UiEmptyState
        v-if="view.total === 0"
        compact
        icon="lock"
        :title="t('admin.user.detail.permissions.empty.title')"
        :description="t('admin.user.detail.permissions.empty.description')"
      />

      <template v-else>
        <p class="text-text-muted">
          {{ t('admin.user.detail.permissions.summary', { count: view.total, modules: view.groups.length }) }}
        </p>

        <section v-for="group in view.groups" :key="group.module_code" class="space-y-2">
          <h3 class="font-display text-sm tracking-wide text-text-subtle uppercase">
            {{ tr(group.module_label) }}
            <span class="text-text-muted normal-case">({{ group.rows.length }})</span>
          </h3>

          <ul class="divide-y divide-border overflow-hidden rounded-lg border border-border bg-surface-raised">
            <li v-for="row in group.rows" :key="row.permission_code" class="p-3">
              <div class="flex flex-wrap items-start justify-between gap-3">
                <div class="min-w-0">
                  <p class="font-medium">{{ tr(row.label) }}</p>
                  <p class="font-mono text-xs text-text-subtle">{{ row.permission_code }}</p>
                </div>

                <UiBadge
                  v-if="row.is_global"
                  intent="warning"
                  size="sm"
                  :label="t('admin.user.detail.permissions.everywhere')"
                />
              </div>

              <!-- D'OÙ ELLE VIENT — un octroi, ou plusieurs. -->
              <ul class="mt-2 flex flex-wrap gap-1.5">
                <li v-for="grant in row.grants" :key="`${grant.assignment_id}-${grant.role_code}`">
                  <!-- LE RÔLE D'ABORD, LA PORTÉE ENSUITE — le même ordre que
                       partout ailleurs dans l'écran. Deux ordres de lecture pour
                       la même paire obligeraient à relire chaque pastille. -->
                  <UiChip
                    fixed
                    :facet="tr(grant.role_label)"
                    :label="
                      grant.scope_type === 'global'
                        ? t('admin.user.roles.scope.global')
                        : grant.scope_label
                          ? tr(grant.scope_label)
                          : t('admin.user.roles.scope.dangling')
                    "
                  />
                </li>
              </ul>
            </li>
          </ul>
        </section>
      </template>

      <!-- L'AUTRE MOITIÉ DE LA RÉPONSE. Repliée, parce qu'elle est longue et
           qu'on ne la consulte que lorsqu'un bouton manque — mais présente,
           parce que sans elle l'écran laisse croire qu'il n'existe que ce
           qu'il montre. -->
      <section v-if="view.missing.length">
        <button
          type="button"
          class="cursor-pointer text-sm text-accent underline underline-offset-2"
          :aria-expanded="showMissing"
          @click="showMissing = !showMissing"
        >
          {{
            showMissing
              ? t('admin.user.detail.permissions.missing.hide')
              : t('admin.user.detail.permissions.missing.show', { count: view.missing.length })
          }}
        </button>

        <ul v-if="showMissing" class="mt-3 flex flex-wrap gap-1.5">
          <li v-for="entry in view.missing" :key="entry.permission_code">
            <UiBadge size="sm" :label="tr(entry.label)" :title="entry.permission_code" />
          </li>
        </ul>
      </section>
    </template>
  </div>
</template>
