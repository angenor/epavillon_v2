<script setup lang="ts">
import type { PlannerSession } from '~/types/admin-planner'
import type { ScheduleConflict } from '~/types/programme/session'
import type { TimeZoneName, Uuid } from '~/types/shared'

/**
 * LE BANDEAU DES CONFLITS — permanent, jamais refermable.
 *
 * IL NE BLOQUE RIEN. Il recense ce que `detect_conflicts()` a trouvé et donne un
 * chemin vers chaque cas ; l'équipe arbitre. Un bandeau qu'on peut chasser d'un
 * clic est un bandeau qu'on chasse — et les chevauchements réapparaîtraient le
 * jour de la publication, quand il est trop tard pour les discuter.
 *
 * DEUX GRAVITÉS, DEUX COULEURS, ET LEURS MOTS :
 *
 *   BLOQUANT (rouge) — matériellement impossible : deux activités de la MÊME
 *   édition qui occupent le stand en même temps, deux directs simultanés (une
 *   seule équipe technique), une salle physique réservée deux fois. Deux
 *   activités de DEUX ÉVÉNEMENTS distincts ne se gênent pas — sauf pour la
 *   diffusion, ressource unique de la plateforme.
 *
 *   AVERTISSEMENT (jaune) — gênant mais possible : un intervenant attendu à deux
 *   endroits, une organisation programmée deux fois. L'équipe juge.
 *
 * La couleur ne porte jamais seule : chaque ligne écrit la nature du conflit, et
 * le compteur l'annonce en toutes lettres.
 */

interface Props {
  conflicts: ScheduleConflict[]
  /** Pour situer chaque cas : titre, salle, jour. */
  sessions: PlannerSession[]
  timezone: TimeZoneName
  zoneLabel?: string
  loading?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  /** « Voir » : la page ouvre le jour et sélectionne le bloc. */
  focus: [sessionId: Uuid]
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, timeRange } = useDateTime()

const sessionById = computed(() => new Map(props.sessions.map((session) => [session.id, session])))

const counts = computed(() => countBySeverity(props.conflicts))
const total = computed(() => props.conflicts.length)

/** Les bloquants d'abord : ce sont eux qui retiendront la publication. */
const ordered = computed(() =>
  [...props.conflicts].sort((a, b) => {
    if (a.severity !== b.severity) return a.severity === 'blocking' ? -1 : 1
    return a.conflict_kind.localeCompare(b.conflict_kind)
  }),
)

const isOpen = ref(false)

/** Créneau du chevauchement, situé dans le fuseau du pavillon. */
function overlapLabel(conflict: ScheduleConflict): string {
  const a = sessionById.value.get(conflict.session_a)
  const b = sessionById.value.get(conflict.session_b)
  if (!a || !b) return ''

  const start = Math.max(Date.parse(a.starts_at), Date.parse(b.starts_at))
  const end = Math.min(Date.parse(a.ends_at), Date.parse(b.ends_at))
  return `${date(start, props.timezone)} · ${timeRange(start, end, props.timezone, props.zoneLabel)}`
}

function titleOf(sessionId: Uuid, fallback: string | null): string {
  const session = sessionById.value.get(sessionId)
  return session ? tr(session.title) : (fallback ?? '')
}
</script>

<template>
  <section
    class="rounded-lg border"
    :class="
      counts.blocking > 0
        ? 'border-danger-border bg-danger-surface'
        : counts.warning > 0
          ? 'border-warning-border bg-warning-surface'
          : 'border-success-border bg-success-surface'
    "
    aria-labelledby="planner-conflicts-title"
  >
    <div class="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
      <UiIcon
        :name="total > 0 ? 'warning' : 'check-circle'"
        size="1.25rem"
        :stroke-width="1.8"
        :class="counts.blocking > 0 ? 'text-danger' : counts.warning > 0 ? 'text-warning' : 'text-success'"
      />

      <div class="min-w-[12rem] flex-1">
        <h2 id="planner-conflicts-title" class="text-sm font-semibold text-text">
          <template v-if="props.loading">{{ t('admin.planner.conflict.loading') }}</template>
          <template v-else-if="total === 0">{{ t('admin.planner.conflict.none') }}</template>
          <template v-else>{{ t('admin.planner.conflict.total', total) }}</template>
        </h2>
        <p v-if="total > 0" class="mt-0.5 text-xs text-text-secondary">
          {{ t('admin.planner.conflict.breakdown', {
            blocking: counts.blocking,
            warning: counts.warning,
          }) }}
          — {{ t('admin.planner.conflict.neverBlocked') }}
        </p>
        <p v-else-if="!props.loading" class="mt-0.5 text-xs text-text-secondary">
          {{ t('admin.planner.conflict.noneDetail') }}
        </p>
      </div>

      <!-- Sur écran étroit, le bouton passe à la ligne : gardé sur la même, il
           comprimait le compteur au point de couper « 4 chevauchements » en deux
           (mesuré à 375 px). -->
      <UiButton
        v-if="total > 0"
        class="w-full sm:w-auto"
        size="sm"
        variant="ghost"
        :icon-trailing="isOpen ? 'chevron-up' : 'chevron-down'"
        :aria-expanded="isOpen"
        aria-controls="planner-conflict-list"
        @click="isOpen = !isOpen"
      >
        {{ isOpen ? t('admin.planner.conflict.collapse') : t('admin.planner.conflict.expand') }}
      </UiButton>
    </div>

    <ul v-show="isOpen" id="planner-conflict-list" class="border-t border-border-strong/30 px-4 py-2">
      <li
        v-for="(conflict, index) in ordered"
        :key="`${conflict.conflict_kind}-${conflict.session_a}-${conflict.session_b}-${index}`"
        class="flex flex-wrap items-start gap-x-3 gap-y-1 border-b border-border-strong/20 py-2 last:border-b-0"
      >
        <UiBadge
          size="sm"
          :intent="conflict.severity === 'blocking' ? 'danger' : 'warning'"
          solid
          :label="t(`admin.planner.conflict.severity.${conflict.severity}`)"
        />

        <div class="min-w-0 flex-1">
          <p class="text-sm text-text">
            <span class="font-medium">{{ t(`admin.planner.conflict.kind.${conflict.conflict_kind}`) }}</span>
            <span v-if="conflict.subject_label"> — {{ conflict.subject_label }}</span>
          </p>
          <p class="text-xs text-text-secondary">
            {{ titleOf(conflict.session_a, conflict.session_a_title) }}
            ↔ {{ titleOf(conflict.session_b, conflict.session_b_title) }}
          </p>
          <p class="text-xs text-text-muted">{{ overlapLabel(conflict) }}</p>
        </div>

        <div class="flex shrink-0 gap-1">
          <UiButton size="sm" variant="ghost" @click="emit('focus', conflict.session_a)">
            {{ t('admin.planner.conflict.showFirst') }}
          </UiButton>
          <UiButton size="sm" variant="ghost" @click="emit('focus', conflict.session_b)">
            {{ t('admin.planner.conflict.showSecond') }}
          </UiButton>
        </div>
      </li>
    </ul>
  </section>
</template>
