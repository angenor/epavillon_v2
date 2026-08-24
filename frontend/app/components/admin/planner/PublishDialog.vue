<script setup lang="ts">
import type { PublicationReadinessIssue } from '~/types/programme/session'
import type { PublishProgrammeResult } from '~/types/admin-planner'
import type { TimeZoneName, Uuid } from '~/types/shared'

/**
 * PUBLIER LA PROGRAMMATION — LE SEUL ENDROIT DE L'ÉCRAN OÙ UN CONTRÔLE BLOQUANT
 * A DU SENS.
 *
 * Tout le reste du planificateur laisse faire : les chevauchements s'écrivent,
 * les blocs se superposent, l'équipe arbitre. Ici, la programmation devient
 * PUBLIQUE — visible des délégations, imprimée dans les programmes, envoyée aux
 * organisations. Un point matériellement impossible qui passerait cette étape
 * deviendrait un problème le jour même, dans le pavillon.
 *
 * `publication_readiness()` rend la liste ; un seul point de gravité `blocking`
 * retient TOUTE la publication. Les avertissements l'accompagnent sans la
 * retenir — un intervenant attendu à deux endroits se règle par un appel
 * téléphonique, pas par un verrou.
 *
 * LE RÉCAPITULATIF EST MONTRÉ AVANT LE CLIC, pas découvert après : on ouvre ce
 * panneau, on lit ce qui reste, et le bouton dit d'avance s'il va agir.
 */

interface Props {
  open: boolean
  issues: PublicationReadinessIssue[]
  /** Fuseau de l'édition : chaque point est situé à l'heure du pavillon. */
  timezone: TimeZoneName
  zoneLabel?: string
  /**
   * Séances qui deviendront publiques si l'on publie maintenant, comptées sur le
   * prédicat de l'API : `planned` ou `scheduled`, pas encore publiées.
   */
  readyCount: number
  /** Déjà publiée une première fois ? Le libellé du bouton change. */
  publishedAt: string | null
  busy?: boolean
  error?: string | null
  /** Résultat du dernier envoi, à relire sans refermer. */
  result?: PublishProgrammeResult | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:open': [value: boolean]
  publish: []
  /** Aller voir le cas dans la grille. */
  focus: [sessionId: Uuid]
}>()

const { t } = useI18n()
const { date, time } = useDateTime()

/**
 * QUAND CELA SE PRODUIT, situé à l'heure du pavillon. La base rend un instant
 * (`occurs_at`) et non une phrase : c'est ici, et seulement ici, qu'il devient
 * une date lisible dans la langue du lecteur.
 */
function whenLabel(issue: PublicationReadinessIssue): string {
  if (!issue.occurs_at) return ''
  return `${date(issue.occurs_at, props.timezone)} · ${time(issue.occurs_at, props.timezone)}`
}

const blocking = computed(() => props.issues.filter((issue) => issue.severity === 'blocking'))
const warnings = computed(() => props.issues.filter((issue) => issue.severity === 'warning'))
const isBlocked = computed(() => blocking.value.length > 0)

/**
 * Les points identiques sont regroupés : « Séance sans lieu » quinze fois de
 * suite se lit moins bien qu'une ligne suivie de ses quinze séances.
 */
function groupOf(issues: PublicationReadinessIssue[]) {
  const groups = new Map<string, { issue: string; entries: PublicationReadinessIssue[] }>()
  for (const issue of issues) {
    const group = groups.get(issue.issue)
    if (group) group.entries.push(issue)
    else groups.set(issue.issue, { issue: issue.issue, entries: [issue] })
  }
  return [...groups.values()]
}

const blockingGroups = computed(() => groupOf(blocking.value))
const warningGroups = computed(() => groupOf(warnings.value))
</script>

<template>
  <UiModal
    :open="props.open"
    size="lg"
    :title="t('admin.planner.publish.title')"
    :description="t('admin.planner.publish.description')"
    @update:open="emit('update:open', $event)"
  >
    <div class="space-y-4">
      <!-- CE QUI SE PASSERA. Un décompte avant l'action, pas après. -->
      <UiAlert
        :intent="isBlocked ? 'danger' : 'info'"
        :title="isBlocked
          ? t('admin.planner.publish.blocked', blocking.length)
          : t('admin.planner.publish.ready', props.readyCount)"
      >
        {{ isBlocked
          ? t('admin.planner.publish.blockedDetail')
          : t('admin.planner.publish.readyDetail') }}
      </UiAlert>

      <section v-if="blockingGroups.length" class="space-y-3">
        <h3 class="text-sm font-semibold tracking-wide text-danger uppercase">
          {{ t('admin.planner.publish.mustFix', blocking.length) }}
        </h3>
        <div v-for="group in blockingGroups" :key="group.issue" class="rounded-lg border border-danger-border bg-danger-surface p-3">
          <!-- Le libellé vient de la BASE, déjà rédigé en français par
               `publication_readiness()` : le recopier dans un fichier i18n en
               ferait deux versions à maintenir, et la seconde diverge toujours. -->
          <p class="text-sm font-medium text-text">{{ group.issue }}</p>
          <ul class="mt-1 space-y-0.5">
            <li
              v-for="(entry, index) in group.entries"
              :key="`${entry.session_id}-${index}`"
              class="flex flex-wrap items-center gap-2 text-xs text-text-secondary"
            >
              <span class="min-w-0 flex-1">
                {{ entry.detail }}
                <span v-if="whenLabel(entry)" class="text-text-muted">— {{ whenLabel(entry) }}</span>
              </span>
              <button
                v-if="entry.session_id"
                type="button"
                class="cursor-pointer underline hover:text-text"
                @click="emit('focus', entry.session_id)"
              >
                {{ t('admin.planner.publish.show') }}
              </button>
            </li>
          </ul>
        </div>
      </section>

      <section v-if="warningGroups.length" class="space-y-3">
        <h3 class="text-sm font-semibold tracking-wide text-warning uppercase">
          {{ t('admin.planner.publish.shouldCheck', warnings.length) }}
        </h3>
        <div v-for="group in warningGroups" :key="group.issue" class="rounded-lg border border-warning-border bg-warning-surface p-3">
          <p class="text-sm font-medium text-text">{{ group.issue }}</p>
          <ul class="mt-1 space-y-0.5">
            <li
              v-for="(entry, index) in group.entries"
              :key="`${entry.session_id}-${index}`"
              class="flex flex-wrap items-center gap-2 text-xs text-text-secondary"
            >
              <span class="min-w-0 flex-1">
                {{ entry.detail }}
                <span v-if="whenLabel(entry)" class="text-text-muted">— {{ whenLabel(entry) }}</span>
              </span>
              <button
                v-if="entry.session_id"
                type="button"
                class="cursor-pointer underline hover:text-text"
                @click="emit('focus', entry.session_id)"
              >
                {{ t('admin.planner.publish.show') }}
              </button>
            </li>
          </ul>
        </div>
      </section>

      <UiEmptyState
        v-if="props.issues.length === 0"
        compact
        icon="check-circle"
        :title="t('admin.planner.publish.clean.title')"
        :description="t('admin.planner.publish.clean.description')"
      />

      <UiAlert
        v-if="props.result && !props.result.blocked"
        intent="success"
        live
        :title="t('admin.planner.publish.done', props.result.published_count)"
      />
      <UiAlert v-if="props.error" intent="danger" live :message="props.error" />
    </div>

    <template #footer>
      <div class="flex flex-wrap items-center justify-end gap-2">
        <UiButton variant="ghost" :disabled="props.busy" @click="emit('update:open', false)">
          {{ t('common.actions.close') }}
        </UiButton>
        <!-- Le bouton reste VISIBLE mais inerte quand un point bloque : le
             masquer laisserait croire que la publication n'existe pas. UN POINT
             BLOQUANT EST LE SEUL MOTIF : une édition sans aucune activité à
             publier s'estampille quand même, l'API le documente, et l'inerter
             sur un décompte nul laissait un bouton mort sans dire pourquoi. -->
        <UiButton
          icon="globe"
          :loading="props.busy"
          :disabled="isBlocked"
          @click="emit('publish')"
        >
          {{ props.publishedAt
            ? t('admin.planner.publish.actionAgain')
            : t('admin.planner.publish.action') }}
        </UiButton>
      </div>
    </template>
  </UiModal>
</template>
