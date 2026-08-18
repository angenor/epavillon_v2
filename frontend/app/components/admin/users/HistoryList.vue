<script setup lang="ts">
import type { AssignmentHistoryEntry } from '~/types/admin-users'
import type { TimeZoneName } from '~/types/shared'

/**
 * L'HISTORIQUE DES ATTRIBUTIONS ET DES RÉVOCATIONS.
 *
 * UNE LIGNE DE TABLE, DEUX ÉVÉNEMENTS. Un octroi et un retrait ont deux dates,
 * deux auteurs et deux motifs : les fondre en une entrée « attribué puis retiré »
 * obligerait à choisir laquelle des deux dates classer, et l'historique cesserait
 * d'être chronologique — la seule chose qu'on lui demande.
 *
 * IL SE LIT DANS LA TABLE, PAS DANS L'AUDIT. `platform.audit_log` existe et le
 * trigger l'alimente, mais il porte des différences champ par champ :
 * « revoked_at : null → 2026-07-20 » n'apprend rien à personne, quand
 * `role_assignments` dit déjà qui, quand et pourquoi.
 *
 * LE MOTIF MANQUANT SE DIT. Les attributions antérieures à cet écran n'en ont
 * pas ; l'afficher vide serait moins clair que d'écrire qu'il n'a pas été saisi.
 */

interface Props {
  entries: AssignmentHistoryEntry[]
  timezone: TimeZoneName
  loading?: boolean
}

defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()
</script>

<template>
  <div>
    <div v-if="loading" class="space-y-3">
      <UiSkeletonLoader v-for="n in 3" :key="n" height="3.5rem" />
    </div>

    <UiEmptyState
      v-else-if="entries.length === 0"
      compact
      icon="clock"
      :title="t('admin.user.detail.history.empty.title')"
      :description="t('admin.user.detail.history.empty.description')"
    />

    <ol v-else class="relative space-y-4 border-s border-border ps-5">
      <li v-for="(entry, index) in entries" :key="`${entry.assignment_id}-${entry.kind}-${index}`">
        <!-- La pastille de la chronologie : verte pour un octroi, rouge pour un
             retrait — confirmé d'un côté, suppression de l'autre. -->
        <span
          class="absolute -start-[5px] mt-2 size-[9px] rounded-full"
          :class="entry.kind === 'granted' ? 'bg-success-solid' : 'bg-danger-solid'"
          aria-hidden="true"
        />

        <p class="text-sm text-text-muted">
          {{ dateTime(entry.occurred_at, timezone) }}
          <template v-if="entry.actor_name"> · {{ entry.actor_name }}</template>
        </p>

        <p class="mt-0.5">
          <span class="font-medium">
            {{ t(`admin.user.detail.history.kind.${entry.kind}`) }}
          </span>
          —
          {{ tr(entry.role_label) }}
          <span class="text-text-muted">
            ·
            {{
              entry.scope.scope_type === 'global'
                ? t('admin.user.roles.scope.global')
                : entry.scope.scope_label
                  ? tr(entry.scope.scope_label)
                  : t('admin.user.roles.scope.dangling')
            }}
          </span>
        </p>

        <p v-if="entry.valid_until && entry.kind === 'granted'" class="text-sm text-text-muted">
          {{ t('admin.user.detail.history.until', { date: dateTime(entry.valid_until, timezone) }) }}
        </p>

        <p v-if="entry.reason" class="mt-1 max-w-(--measure) text-sm">« {{ entry.reason }} »</p>
        <p v-else class="mt-1 text-sm text-text-subtle italic">
          {{ t('admin.user.detail.history.noReason') }}
        </p>
      </li>
    </ol>
  </div>
</template>
