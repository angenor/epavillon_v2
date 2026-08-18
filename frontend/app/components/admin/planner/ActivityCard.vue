<script setup lang="ts">
import type { PlannerSession } from '~/types/admin-planner'
import type { TimeZoneName } from '~/types/shared'

/**
 * UNE ACTIVITÉ RETENUE, EN ATTENTE DE PLACEMENT.
 *
 * Elle se prend au doigt (glisser-déposer vers la grille) ET au clavier (bouton
 * « Placer », qui ouvre le dialogue d'affectation). LES DEUX CHEMINS EXISTENT
 * TOUJOURS, sur tous les écrans : le glisser-déposer HTML5 ne fonctionne ni au
 * clavier, ni sur une tablette tactile, et une carte qu'on ne peut que traîner
 * est une carte inaccessible.
 *
 * CE QUE LA CARTE PORTE, ET POURQUOI. L'équipe place une activité en se posant
 * quatre questions : est-elle bien notée, combien de temps dure-t-elle, quand
 * l'organisation l'avait-elle demandée, et y a-t-il une contrainte déclarée. Le
 * reste — la présentation, les intervenants — appartient à la fiche du dossier,
 * qui reste à un clic.
 *
 * TROIS PASTILLES THÉMATIQUES AU PLUS, les suivantes repliées en « +N » : au
 * delà, elles cessent d'informer (règle d'usage du guide de style).
 */

interface Props {
  session: PlannerSession
  timezone: TimeZoneName
  zoneLabel?: string
  /** Faux sur écran étroit : le glisser-déposer n'y a pas de sens. */
  draggable?: boolean
  selected?: boolean
  disabled?: boolean
}

const props = withDefaults(defineProps<Props>(), { draggable: true })
const emit = defineEmits<{
  place: [session: PlannerSession]
  /** Le glissement commence : la page l'annonce à la grille. */
  dragstart: [session: PlannerSession]
  dragend: []
}>()

const { t } = useI18n()
const { tr } = useI18nText()
const { timeRange, date } = useDateTime()

const title = computed(() => tr(props.session.title))
const organization = computed(() =>
  props.session.organization_acronym?.trim() || props.session.organization_name || '',
)

const duration = computed(() => plannedDuration(props.session))

/** « 1 h 30 » ou « 45 min » — assemblé par i18n, jamais écrit en dur. */
const durationLabel = computed(() => {
  const parts = durationParts(duration.value)
  if (!parts) return ''
  if (parts.hours === 0) return t('admin.planner.card.durationMinutes', { minutes: parts.minutes })
  if (parts.minutes === 0) return t('admin.planner.card.durationHours', { hours: parts.hours })
  return t('admin.planner.card.durationHoursMinutes', { hours: parts.hours, minutes: parts.minutes })
})

/** Heure murale souhaitée au pavillon (`HH:MM`), à défaut celle déjà portée. */
const preferredTime = computed(() =>
  wallClockInZone(props.session.preferred_start_at ?? props.session.starts_at, props.timezone).slice(11),
)

const preferred = computed(() => {
  const start = props.session.preferred_start_at
  if (!start) return null
  return t('admin.planner.card.preferred', {
    date: date(start, props.timezone),
    range: timeRange(start, endOfSlot(start, duration.value), props.timezone, props.zoneLabel),
  })
})

const MAX_THEMES = 3
const shownThemes = computed(() => props.session.themes.slice(0, MAX_THEMES))
const hiddenThemeCount = computed(() => Math.max(0, props.session.themes.length - MAX_THEMES))

/**
 * LA CHARGE UTILE DU GLISSEMENT est celle que la bibliothèque de calendrier
 * attend (`dataTransfer` sous la clé `event`) : un titre, une durée en minutes,
 * et notre identifiant de séance. Elle crée le bloc à l'endroit du dépôt, et la
 * page traduit ensuite ce dépôt en écriture.
 */
function onDragStart(dragEvent: DragEvent): void {
  if (!props.draggable || props.disabled || !dragEvent.dataTransfer) return
  dragEvent.dataTransfer.dropEffect = 'move'
  dragEvent.dataTransfer.effectAllowed = 'move'
  dragEvent.dataTransfer.setData(
    'event',
    JSON.stringify({
      title: title.value,
      duration: duration.value,
      sessionId: props.session.id,
      // L'HEURE SOUHAITÉE VOYAGE AVEC LA CARTE. En vue mois, la grille n'a pas
      // d'axe horaire : c'est cette heure-là que prend le dépôt, et non une
      // valeur déduite de la position du curseur dans la case.
      preferredTime: preferredTime.value,
      class: 'planner-event planner-event--new',
    }),
  )
  emit('dragstart', props.session)
}
</script>

<template>
  <article
    class="rounded-lg border bg-surface-raised p-3 transition-colors"
    :class="[
      props.selected ? 'border-accent shadow-[var(--shadow-focus)]' : 'border-border hover:border-border-strong',
      props.draggable && !props.disabled ? 'cursor-grab active:cursor-grabbing' : '',
      props.disabled ? 'opacity-60' : '',
    ]"
    :draggable="props.draggable && !props.disabled"
    @dragstart="onDragStart"
    @dragend="emit('dragend')"
  >
    <div class="flex items-start justify-between gap-2">
      <h3 class="min-w-0 text-sm leading-snug font-semibold text-text">{{ title }}</h3>

      <!-- LA NOTE DU COMITÉ, en tête de carte : c'est par elle que le panneau se
           trie, et la première question devant un pavillon à remplir. Une
           activité programmée par l'IFDD n'en a pas — on n'écrit pas « 0 ». -->
      <span
        v-if="props.session.average_score !== null"
        class="shrink-0 rounded-md bg-surface-sunken px-2 py-0.5 font-mono text-xs font-bold tabular-nums text-text-secondary"
        :title="t('admin.planner.card.scoreTitle')"
      >{{ t('admin.planner.card.score', { score: props.session.average_score.toFixed(2) }) }}</span>
    </div>

    <p v-if="organization" class="mt-1 truncate text-xs text-text-muted">
      {{ organization }}
      <span v-if="props.session.organization_country_code">
        · {{ props.session.organization_country_code }}
      </span>
      <span v-if="props.session.reference_code"> · {{ props.session.reference_code }}</span>
    </p>

    <dl class="mt-2 space-y-1 text-xs text-text-secondary">
      <div class="flex items-center gap-1.5">
        <UiIcon name="clock" size="0.875rem" :stroke-width="1.8" />
        <dt class="sr-only">{{ t('admin.planner.card.durationLabel') }}</dt>
        <dd>{{ durationLabel }} · {{ t(`admin.planner.format.${props.session.format}`) }}</dd>
      </div>
      <div v-if="preferred" class="flex items-start gap-1.5">
        <UiIcon name="calendar" size="0.875rem" :stroke-width="1.8" class="mt-0.5 shrink-0" />
        <dt class="sr-only">{{ t('admin.planner.card.preferredLabel') }}</dt>
        <dd>{{ preferred }}</dd>
      </div>
      <!-- LES CONTRAINTES DÉCLARÉES AU DÉPÔT : « pas avant 14 h », « après la
           plénière ». Les cacher derrière la fiche du dossier reviendrait à
           placer sans les lire. -->
      <div v-if="props.session.scheduling_constraints" class="flex items-start gap-1.5">
        <UiIcon name="info" size="0.875rem" :stroke-width="1.8" class="mt-0.5 shrink-0" />
        <dt class="sr-only">{{ t('admin.planner.card.constraintsLabel') }}</dt>
        <dd class="text-text-muted">{{ props.session.scheduling_constraints }}</dd>
      </div>
    </dl>

    <div v-if="shownThemes.length" class="mt-2 flex flex-wrap items-center gap-1">
      <UiBadge
        v-for="theme in shownThemes"
        :key="theme.code"
        size="sm"
        :dot-color="theme.color"
        :label="tr(theme.label)"
      />
      <span v-if="hiddenThemeCount > 0" class="text-xs text-text-muted">
        {{ t('admin.planner.card.moreThemes', { count: hiddenThemeCount }) }}
      </span>
    </div>

    <div class="mt-3 flex items-center gap-2">
      <!-- LE CHEMIN SANS SOURIS. Ce bouton n'est pas un repli pour mobile : il
           est le seul moyen de placer une activité au clavier, et il porte les
           44 px de cible tactile de la charte. -->
      <UiButton size="sm" variant="secondary" icon="calendar" :disabled="props.disabled" @click="emit('place', props.session)">
        {{ t('admin.planner.card.place') }}
      </UiButton>
      <span v-if="props.draggable" class="text-xs text-text-subtle">
        {{ t('admin.planner.card.orDrag') }}
      </span>
    </div>
  </article>
</template>
