<script setup lang="ts">
import VueCal from 'vue-cal'
import 'vue-cal/dist/vuecal.css'
import type { PublicScheduleRow } from '~/types/views'
import type { IsoDate, TimeZoneName } from '~/types/shared'
import type { SessionDisplayState } from '~/components/ui/StatusBadge.vue'

/**
 * VUE CALENDRIER — les mêmes créneaux, placés dans le temps, EN LECTURE SEULE.
 *
 * C'est la même bibliothèque que le planificateur du back-office (A9), et c'est
 * délibéré : deux calendriers différents pour les mêmes créneaux donneraient
 * deux lectures du même programme, et l'équipe finirait par vérifier dans l'un
 * ce qu'elle a arbitré dans l'autre. Ici, tout ce qui modifie est coupé —
 * `editable-events` reste faux, aucune création par glissement, aucun
 * redimensionnement.
 *
 * ── LE FUSEAU, PIÈGE PRINCIPAL ──────────────────────────────────────────────
 *
 * vue-cal ne connaît pas les fuseaux : il place les blocs à l'heure locale de la
 * MACHINE. Lui passer l'instant brut afficherait 14 h de Belém à 18 h pour qui
 * consulte depuis Dakar, et toute la grille du pavillon serait fausse. Les
 * bornes sont donc converties une fois, en amont, vers le fuseau de l'ÉDITION
 * (`wallClockInZone()`), et le fuseau est rappelé au-dessus de la grille — sans
 * quoi le visiteur croirait lire ses propres heures.
 *
 * ── LA COULEUR NE SUFFIT JAMAIS ─────────────────────────────────────────────
 *
 * Chaque bloc porte trois marques : sa couleur d'état, le NOM de l'état écrit
 * en toutes lettres, et un liseré à gauche pour la journée spéciale quand
 * l'activité en fait partie. La légende est affichée en permanence par la
 * section. Un bloc n'est jamais un simple aplat coloré.
 *
 * ── ACCESSIBILITÉ ───────────────────────────────────────────────────────────
 *
 * Le contenu de chaque bloc est un vrai `<button>` : atteignable au clavier,
 * annoncé avec l'heure et le titre. Les blocs de vue-cal, eux, ne le sont pas.
 * La vue grille reste de toute façon l'équivalent lisible et complet de cette
 * vue — c'est elle qui est proposée par défaut.
 */

interface Props {
  sessions: PublicScheduleRow[]
  timezone: TimeZoneName
  zoneLabel?: string
  /** Jour affiché (`AAAA-MM-JJ`) — partagé avec le filtre de la vue grille. */
  selectedDate: IsoDate
  /** Bornes de navigation : premier et dernier jour porteur d'activités. */
  minDate?: IsoDate
  maxDate?: IsoDate
  selectedId?: string | null
}

const props = defineProps<Props>()
const emit = defineEmits<{
  select: [session: PublicScheduleRow]
  /** Le jour affiché a changé : la section met le filtre à jour. */
  'update:selectedDate': [date: IsoDate]
}>()

const { t, locale } = useI18n()
const { tr } = useI18nText()
const { isLive } = useLiveSession()

const sessionById = computed(() => new Map(props.sessions.map((session) => [session.id, session])))

/** L'état AFFICHÉ : le direct l'emporte sur l'état temporel, comme sur la carte. */
function displayState(session: PublicScheduleRow): SessionDisplayState {
  return isLive(session.id) ? 'live' : session.temporal_state
}

const events = computed(() =>
  props.sessions.map((session) => ({
    start: wallClockInZone(session.starts_at, props.timezone),
    end: wallClockInZone(session.ends_at, props.timezone),
    title: tr(session.title),
    sessionId: session.id,
    class: `programme-event programme-event--${displayState(session)}`,
  })),
)

/**
 * Bornes horaires de la grille, déduites des activités : afficher minuit à
 * minuit sur une journée qui commence à 9 h laisse les deux tiers de la hauteur
 * vides, et le visiteur défile pour rien.
 */
const bounds = computed(() => {
  const minutes = props.sessions.flatMap((session) => {
    const start = wallClockInZone(session.starts_at, props.timezone).slice(11)
    const end = wallClockInZone(session.ends_at, props.timezone).slice(11)
    return [toMinutes(start), toMinutes(end)]
  })
  if (!minutes.length) return { from: 8 * 60, to: 20 * 60 }
  return {
    from: Math.max(0, Math.floor(Math.min(...minutes) / 60) * 60 - 60),
    to: Math.min(24 * 60, Math.ceil(Math.max(...minutes) / 60) * 60 + 60),
  }
})

/**
 * Heure de début du bloc, relue depuis la SÉANCE et non depuis l'objet de
 * vue-cal : la bibliothèque expose ses dates par des méthodes greffées sur
 * `Date.prototype`, et s'appuyer dessus lierait notre affichage à un détail
 * d'implémentation qu'une option (`disable-date-prototypes`) peut retirer.
 */
function startTime(sessionId: string): string {
  const session = sessionById.value.get(sessionId)
  return session ? wallClockInZone(session.starts_at, props.timezone).slice(11) : ''
}

/** Le nom de l'état, traduit — « À venir », « Reportée », « En direct ». */
function stateLabel(sessionId: string): string {
  const session = sessionById.value.get(sessionId)
  if (!session) return ''
  const state = displayState(session)
  return t(state === 'live' ? 'live-badge.label' : `session-card.state.${session.temporal_state}`)
}

function toMinutes(hhmm: string): number {
  const [hours, mins] = hhmm.split(':')
  return Number.parseInt(hours ?? '0', 10) * 60 + Number.parseInt(mins ?? '0', 10)
}

/** Couleur de la journée spéciale, en liseré — jamais en fond : voir l'en-tête. */
function trackColor(sessionId: string): string | undefined {
  const session = sessionById.value.get(sessionId)
  const specialDay = session?.tracks.find((track) => track.kind === 'special_day' && track.color)
  return specialDay?.color ?? undefined
}

function onViewChange(view: { startDate?: Date }): void {
  if (!view.startDate) return
  // La date rendue par vue-cal est une date LOCALE de la machine : on la relit
  // par ses composantes, sans passer par un instant, faute de quoi un visiteur
  // à l'est de Greenwich changerait de jour en changeant de fuseau.
  const year = view.startDate.getFullYear()
  const month = String(view.startDate.getMonth() + 1).padStart(2, '0')
  const day = String(view.startDate.getDate()).padStart(2, '0')
  const next = `${year}-${month}-${day}`
  if (next !== props.selectedDate) emit('update:selectedDate', next)
}
</script>

<template>
  <div>
    <p class="mb-2 text-sm text-text-muted">
      {{ t('programme.calendar.zoneNotice', {
        zone: props.zoneLabel || props.timezone,
      }) }}
    </p>

    <!-- `time-cell-height` vaut 72 px et non la valeur par défaut : c'est la
         hauteur qu'il faut pour qu'un créneau d'une heure porte son horaire, son
         état et deux lignes de titre sans rien tronquer. En dessous, le titre —
         la seule des trois informations qui ne se devine pas — passait à la
         trappe. -->
    <div class="programme-calendar overflow-hidden rounded-lg border border-border bg-surface-raised">
      <VueCal
        :key="`${props.selectedDate}-${locale}`"
        :events="events"
        :selected-date="props.selectedDate"
        :min-date="props.minDate"
        :max-date="props.maxDate"
        active-view="day"
        :disable-views="['years', 'year', 'month', 'week']"
        hide-view-selector
        :time-from="bounds.from"
        :time-to="bounds.to"
        :time-step="60"
        :time-cell-height="72"
        :locale="locale"
        :editable-events="false"
        :drag-to-create-event="false"
        :dblclick-to-navigate="false"
        style="height: 34rem"
        @view-change="onViewChange"
      >
        <template #no-event>
          <p class="px-4 py-8 text-center text-sm text-text-muted">
            {{ t('programme.calendar.noEvent') }}
          </p>
        </template>

        <template #event="{ event }">
          <button
            type="button"
            class="programme-event__body"
            :class="props.selectedId === event.sessionId ? 'programme-event__body--selected' : ''"
            :style="trackColor(String(event.sessionId))
              ? { borderLeftColor: trackColor(String(event.sessionId)), borderLeftWidth: '4px' }
              : undefined"
            @click="emit('select', sessionById.get(String(event.sessionId))!)"
          >
            <!-- L'heure et l'état sur une même ligne : un créneau d'une heure ne
                 fait que 56 px de haut, et trois lignes empilées tronqueraient
                 le titre — le seul des trois qu'on ne peut pas deviner.
                 LE NOM DE L'ÉTAT EST ÉCRIT : la couleur ne porte jamais seule
                 une information (règle d'usage du guide de style). -->
            <span class="programme-event__meta">
              <span class="programme-event__time">{{ startTime(String(event.sessionId)) }}</span>
              <span class="programme-event__state">{{ stateLabel(String(event.sessionId)) }}</span>
            </span>
            <span class="programme-event__title">{{ event.title }}</span>
          </button>
        </template>
      </VueCal>
    </div>
  </div>
</template>

<style scoped>
/*
 * vue-cal apporte sa propre feuille de style, écrite pour un thème clair et des
 * couleurs en dur. On ne la remplace pas — on redéfinit ce qui doit suivre les
 * JETONS de la plateforme, et rien de plus : grille, barre de titre, blocs.
 * Sans cela, le calendrier reste blanc en thème sombre.
 */
.programme-calendar :deep(.vuecal) {
  box-shadow: none;
  color: var(--color-text);
  font-family: inherit;
}

.programme-calendar :deep(.vuecal__title-bar) {
  background-color: var(--color-surface-sunken);
  font-size: 1rem;
  font-family: var(--font-display, inherit);
  min-height: var(--target-min);
}

.programme-calendar :deep(.vuecal__arrow),
.programme-calendar :deep(.vuecal__title button) {
  color: var(--color-text);
  min-height: var(--target-min);
  min-width: var(--target-min);
  cursor: pointer;
}

.programme-calendar :deep(.vuecal__cell:before),
.programme-calendar :deep(.vuecal__time-column .vuecal__time-cell-line:before) {
  border-color: var(--color-separator);
}

.programme-calendar :deep(.vuecal__time-column .vuecal__time-cell) {
  color: var(--color-text-subtle);
}

.programme-calendar :deep(.vuecal__cell--today),
.programme-calendar :deep(.vuecal__cell--current) {
  background-color: color-mix(in oklab, var(--color-accent) 6%, transparent);
}

.programme-calendar :deep(.vuecal__now-line) {
  color: var(--color-live);
}

.programme-calendar :deep(.vuecal__event) {
  background: none;
  border-radius: 6px;
  overflow: visible;
}

/* Le corps du bloc : un vrai bouton, donc atteignable au clavier. */
.programme-calendar :deep(.programme-event__body) {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  height: 100%;
  padding: 4px 6px;
  text-align: start;
  overflow: hidden;
  cursor: pointer;
  border: 1px solid var(--color-border);
  border-left: 4px solid var(--color-border-strong);
  border-radius: 6px;
  background-color: var(--color-surface-raised);
  color: var(--color-text);
  font: inherit;
}

.programme-calendar :deep(.programme-event__body:focus-visible) {
  outline: none;
  box-shadow: var(--shadow-focus);
}

.programme-calendar :deep(.programme-event__body--selected) {
  border-color: var(--color-accent);
  box-shadow: var(--shadow-focus);
}

.programme-calendar :deep(.programme-event__meta) {
  display: flex;
  align-items: baseline;
  gap: 0.5rem;
  min-width: 0;
}

.programme-calendar :deep(.programme-event__time) {
  font-size: 0.6875rem;
  font-variant-numeric: tabular-nums;
  color: var(--color-text-muted);
}

.programme-calendar :deep(.programme-event__title) {
  font-size: 0.8125rem;
  font-weight: 600;
  line-height: 1.25;
  overflow: hidden;
  /* Deux lignes au plus : au-delà, le bloc d'une heure déborderait sur le
     suivant, et un titre coupé au milieu d'un mot ne se lit pas mieux. */
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.programme-calendar :deep(.programme-event__state) {
  font-size: 0.625rem;
  text-transform: uppercase;
  letter-spacing: var(--tracking-caps);
  color: var(--color-text-muted);
  white-space: nowrap;
}

/*
 * LES SIX ÉTATS, avec les couleurs du guide de style — et pas d'autres.
 * Le jaune dit « en cours » (ce qui demande attention, pas une réussite), le
 * violet dit « reporté » (déjà arbitré), le rouge dit « annulé » et « en
 * direct », le gris dit « terminé ».
 */
.programme-calendar :deep(.programme-event--upcoming .programme-event__body) {
  background-color: var(--color-info-surface);
  border-color: var(--color-info-border);
}

.programme-calendar :deep(.programme-event--ongoing .programme-event__body) {
  background-color: var(--color-warning-surface);
  border-color: var(--color-warning-border);
}

.programme-calendar :deep(.programme-event--live .programme-event__body) {
  background-color: var(--color-danger-surface);
  border-color: var(--color-live);
  border-width: var(--border-medium);
}

.programme-calendar :deep(.programme-event--past .programme-event__body) {
  background-color: var(--color-neutral-surface);
  border-color: var(--color-border);
}

.programme-calendar :deep(.programme-event--postponed .programme-event__body) {
  background-color: var(--color-postponed-surface);
  border-color: var(--color-postponed-border);
}

.programme-calendar :deep(.programme-event--cancelled .programme-event__body) {
  background-color: var(--color-danger-surface);
  border-color: var(--color-danger-border);
}

.programme-calendar :deep(.programme-event--cancelled .programme-event__title) {
  text-decoration: line-through;
}
</style>
