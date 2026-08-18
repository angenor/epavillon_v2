<script setup lang="ts">
import type { RoleAssignmentView } from '~/types/admin-users'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES ATTRIBUTIONS EN COURS D'UNE PERSONNE, et le geste pour les retirer.
 *
 * UNE CARTE PAR ATTRIBUTION, jamais une ligne de tableau. Chacune porte quatre
 * choses que l'opérateur lit ensemble — le rôle, sa PORTÉE, son terme éventuel,
 * le motif de l'octroi — et un tableau les aurait éclatées en colonnes dont trois
 * seraient vides la plupart du temps.
 *
 * L'ATTRIBUTION ORPHELINE EST SIGNALÉE, pas masquée. Une édition supprimée laisse
 * une attribution qui donne des droits sur rien : elle se voit, avec de quoi la
 * retirer. La taire reviendrait à laisser une ligne inaccessible en base.
 *
 * UN RÔLE SYSTÈME SE RETIRE COMME UN AUTRE. `roles.is_system` protège le RÔLE de
 * la suppression du catalogue, pas ses attributions — confondre les deux
 * empêcherait de retirer « Administrateur » à quelqu'un qui part.
 */

interface Props {
  assignments: RoleAssignmentView[]
  timezone: TimeZoneName
  /** L'acteur peut-il retirer ? Faux : la liste reste consultable. */
  canRevoke?: boolean
  loading?: boolean
}

defineProps<Props>()
const emit = defineEmits<{ revoke: [assignment: RoleAssignmentView] }>()

const { t } = useI18n()
const { date } = useDateTime()
</script>

<template>
  <div>
    <div v-if="loading" class="space-y-3">
      <UiSkeletonLoader v-for="n in 2" :key="n" height="5.5rem" />
    </div>

    <UiEmptyState
      v-else-if="assignments.length === 0"
      compact
      icon="users"
      :title="t('admin.user.roles.empty.title')"
      :description="t('admin.user.roles.empty.description')"
    />

    <ul v-else class="space-y-3">
      <li
        v-for="assignment in assignments"
        :key="assignment.id"
        class="rounded-lg border border-border bg-surface-raised p-4"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0 space-y-2">
            <AdminUsersRoleBadge :assignment="assignment" show-state />

            <dl class="space-y-1 text-sm text-text-muted">
              <div class="flex flex-wrap gap-x-2">
                <dt>{{ t('admin.user.roles.card.granted') }}</dt>
                <dd>
                  {{ date(assignment.granted_at, timezone) }}
                  <template v-if="assignment.granted_by_name">
                    · {{ t('admin.user.roles.card.by', { name: assignment.granted_by_name }) }}
                  </template>
                </dd>
              </div>

              <!-- LE TERME EST FACULTATIF, et son absence se dit : sans cette
                   ligne, on ne distingue pas un rôle permanent d'un rôle dont
                   personne n'a pensé à borner la durée. -->
              <div class="flex flex-wrap gap-x-2">
                <dt>{{ t('admin.user.roles.card.until') }}</dt>
                <dd :class="assignment.valid_until ? 'text-text' : ''">
                  {{
                    assignment.valid_until
                      ? date(assignment.valid_until, timezone)
                      : t('admin.user.roles.card.noEnd')
                  }}
                </dd>
              </div>

              <div v-if="assignment.state === 'scheduled'" class="flex flex-wrap gap-x-2 text-info">
                <dt>{{ t('admin.user.roles.card.from') }}</dt>
                <dd>{{ date(assignment.valid_from, timezone) }}</dd>
              </div>
            </dl>

            <p v-if="assignment.note" class="max-w-(--measure) text-sm text-text">
              « {{ assignment.note }} »
            </p>

            <UiAlert
              v-if="assignment.is_dangling"
              intent="warning"
              compact
              :message="t('admin.user.roles.card.dangling')"
            />
          </div>

          <UiButton
            v-if="canRevoke"
            variant="ghost"
            size="sm"
            icon="ban"
            @click="emit('revoke', assignment)"
          >
            {{ t('admin.user.roles.card.revoke') }}
          </UiButton>
        </div>

        <!-- CE QUE CE RÔLE APPORTE, replié dans la carte : la question « pourquoi
             cette personne peut-elle faire ça ? » se pose ici autant que dans
             l'écran des permissions. -->
        <details class="mt-3">
          <summary class="cursor-pointer text-sm text-accent">
            {{ t('admin.user.roles.card.permissions', { count: assignment.role_permissions.length }) }}
          </summary>
          <ul class="mt-2 flex flex-wrap gap-1.5">
            <li v-for="code in assignment.role_permissions" :key="code">
              <UiBadge size="sm" :label="code" />
            </li>
          </ul>
        </details>
      </li>
    </ul>
  </div>
</template>
