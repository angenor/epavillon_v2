<script setup lang="ts">
import VueCal from 'vue-cal'
import 'vue-cal/dist/vuecal.css'
import type { PlannerDay, PlannerRoom, PlannerSession } from '~/types/admin-planner'
import type { SessionConflictMark } from '~/utils/planner'
import type { IsoDate, IsoDateTime, RoomId, TimeZoneName, Uuid } from '~/types/shared'

/** Les quatre vues du planificateur, dans l'ordre où on les parcourt. */
export type PlannerCalendarView = 'day' | 'week' | 'month' | 'year'

const VIEWS: PlannerCalendarView[] = ['day', 'week', 'month', 'year']

/**
 * LA GRILLE SALLE × HEURE — le lieu de l'arbitrage.
 *
 * Même bibliothèque que la programmation publique (`vue-cal` 4), et c'est
 * délibéré : deux calendriers pour les mêmes créneaux donneraient deux lectures
 * du même programme, et l'équipe finirait par vérifier dans l'un ce qu'elle a
 * arbitré dans l'autre. Ici, tout ce qui modifie est OUVERT — glisser-déposer et
 * redimensionnement — là-bas, tout est coupé.
 *
 * ── UN SEUL STAND, DONC AUCUNE COLONNE DE SALLE ─────────────────────────────
 *
 * Le pavillon est un espace unique (règle métier n° 3, confirmée par le
 * commanditaire : « tout se passe dans la même salle »). Découper la journée en
 * colonnes de salles, comme le permet `split-days`, laisserait croire que deux
 * activités peuvent s'y tenir de front — exactement ce que le terrain interdit.
 * La grille du jour ne montre donc qu'une colonne, et deux blocs simultanés s'y
 * chevauchent VISIBLEMENT : c'est ainsi qu'un conflit doit se lire.
 *
 * Les rares séances tenues EN LIGNE, dans une salle virtuelle, n'occupent pas le
 * stand. Elles ne méritent pas une colonne pour autant : le bloc porte la mention
 * de sa salle, et `detect_conflicts()` ne leur oppose aucun conflit de stand.
 *
 * ── QUATRE VUES, ET DEUX D'ENTRE ELLES SE REGARDENT SANS SE TOUCHER ─────────
 *
 * Le JOUR sert à poser les créneaux à l'heure près. La SEMAINE sert à voir
 * l'équilibre du programme — une journée surchargée face à une autre presque
 * vide se voit d'un coup d'œil, jamais en feuilletant douze jours un par un. Le
 * MOIS situe l'édition dans son calendrier, et l'ANNÉE range les éditions et les
 * webinaires d'un cycle, dispersés sur douze mois.
 *
 * Le MOIS et l'ANNÉE ne montrent pas les blocs : ils COMPTENT. Un nombre par
 * jour, un nombre par mois — c'est ce que l'on vient y chercher, et trente titres
 * tronqués à trois lettres n'apprendraient rien de plus. Le décompte prend la
 * couleur du pire conflit de la journée : on repère la journée à reprendre sans
 * ouvrir quoi que ce soit, puis on clique pour y descendre.
 *
 * ── DÉPOSER EN VUE MOIS : LE JOUR CHANGE, L'HEURE NON ───────────────────────
 *
 * On peut faire glisser une activité du panneau jusqu'à une case du mois. Mais la
 * grille du mois n'a PAS d'axe horaire, et la bibliothèque déduit l'heure d'un
 * dépôt de la position VERTICALE du curseur : le bloc prendrait une heure
 * calculée sur la hauteur de la case, sans erreur et sans avertissement. On ne
 * garde donc de son calcul que la DATE, et l'heure vient d'ailleurs — celle que
 * l'organisation a demandée dans son dossier, à défaut celle que la séance porte
 * déjà, à défaut 9 h. Déposer en vue mois veut dire « ce jour-là », et l'heure
 * s'ajuste ensuite au jour ou dans le panneau.
 *
 * L'ANNÉE, elle, reste en lecture : une case y vaut un mois entier, et aucune
 * date ne s'en déduit sans l'inventer.
 *
 * ── AUCUN DÉPÔT N'EST REFUSÉ ────────────────────────────────────────────────
 *
 * Poser un bloc sur un créneau occupé FONCTIONNE. Le conflit devient alors
 * visible — liseré épais, marque, décompte au bandeau — mais rien n'est
 * empêché : les organisations ont proposé sans se coordonner, et un planificateur
 * travaille par déplacements successifs en passant par des états incohérents
 * (décision structurante n° 1 de `075_programme_sessions.sql`).
 *
 * ── `editable` FAIT PARTIE DE LA CLÉ DU COMPOSANT, ET CE N'EST PAS UN DÉTAIL ─
 *
 * vue-cal ne charge son module de glisser-déposer QU'À SA CRÉATION, et seulement
 * si l'édition est déjà ouverte (`created()` → `loadDragAndDrop()`). Or le droit
 * d'arbitrer et la détection du pointeur fin arrivent après le premier rendu :
 * la grille naissait donc en lecture seule et le restait — blocs immobiles,
 * poignées inertes, sans le moindre message. Inclure `editable` dans la clé force
 * un remontage au moment où le droit apparaît. Mesuré au navigateur, pas déduit.
 *
 * ── LE FUSEAU, PIÈGE PRINCIPAL, DANS LES DEUX SENS ──────────────────────────
 *
 * vue-cal ne connaît pas les fuseaux : il place ses blocs à l'heure de la
 * MACHINE. On convertit donc vers l'heure du PAVILLON avant de lui donner quoi
 * que ce soit, et — nouveauté de cet écran — on reconvertit en sens inverse ce
 * qu'il rend après un dépôt. Sans le second passage, un déplacement fait depuis
 * Dakar décalerait le créneau de trois heures sans qu'aucune erreur ne soit
 * levée.
 */

interface Props {
  sessions: PlannerSession[]
  rooms: PlannerRoom[]
  days: PlannerDay[]
  timezone: TimeZoneName
  zoneLabel?: string
  /** Jour affiché (`AAAA-MM-JJ`), porté par l'URL de la page. */
  selectedDate: IsoDate
  /** Vue active. Le jour et la semaine se modifient ; le mois et l'année se lisent. */
  view?: PlannerCalendarView
  /** Conflits par séance : c'est la grille qui les rend visibles. */
  marks: Map<Uuid, SessionConflictMark>
  selectedId?: string | null
  /** Faux tant que la personne n'a pas le droit d'arbitrer : grille en lecture. */
  editable?: boolean
  busy?: boolean
}

const props = withDefaults(defineProps<Props>(), { editable: true, view: 'day' })

const emit = defineEmits<{
  'update:selectedDate': [date: IsoDate]
  'update:view': [view: PlannerCalendarView]
  /** Une séance a été posée ou déplacée : salle et créneau retenus. */
  schedule: [payload: { sessionId: Uuid; roomId: RoomId | null; startsAt: IsoDateTime; endsAt: IsoDateTime }]
  /** Le bloc a été ouvert : la page montre le panneau de réglages. */
  open: [session: PlannerSession]
}>()

const { t, locale } = useI18n()
const { tr } = useI18nText()

const sessionById = computed(() => new Map(props.sessions.map((session) => [session.id, session])))

/**
 * LA SALLE DU STAND — celle dans laquelle un bloc déposé atterrit.
 *
 * Sans colonnes, le dépôt ne peut plus désigner une salle : c'est le stand, et
 * il n'y en a qu'un. On retient donc la première salle PHYSIQUE de l'édition ; à
 * défaut — une édition entièrement en ligne, comme le cycle PACO — la première
 * salle déclarée. Une séance déjà posée en salle virtuelle y RESTE : la déplacer
 * dans la grille ne la fait pas entrer au pavillon.
 */
const standRoomId = computed<RoomId | null>(
  () => (props.rooms.find((room) => !room.is_virtual) ?? props.rooms[0])?.id ?? null,
)

/**
 * L'ARBITRAGE N'EST OFFERT QU'AU JOUR ET À LA SEMAINE — voir l'en-tête : hors
 * d'une grille horaire, l'heure d'un dépôt serait déduite d'une hauteur qui ne
 * représente plus le temps.
 */
const isTimeGrid = computed(() => props.view === 'day' || props.view === 'week')
const canEdit = computed(() => props.editable && props.view !== 'year')

/** Salle d'une séance après un dépôt : la sienne si elle en a une, sinon le stand. */
function roomAfterDrop(sessionId: string): RoomId | null {
  return sessionById.value.get(sessionId)?.room_id ?? standRoomId.value
}

/** Bornes du calendrier : les jours de l'édition, jamais au-delà. */
const minDate = computed(() => props.days[0]?.day_date)
const maxDate = computed(() => props.days[props.days.length - 1]?.day_date)

const events = computed(() =>
  props.sessions
    .filter((session) => session.room_id !== null)
    .map((session) => {
      const mark = props.marks.get(session.id)
      return {
        start: wallClockInZone(session.starts_at, props.timezone),
        end: wallClockInZone(session.ends_at, props.timezone),
        title: tr(session.title),
        sessionId: session.id,
        class: [
          'planner-event',
          `planner-event--${session.status}`,
          mark ? `planner-event--conflict-${mark.severity}` : '',
          props.selectedId === session.id ? 'planner-event--selected' : '',
        ]
          .filter(Boolean)
          .join(' '),
        // Une séance publiée reste déplaçable — un programme public se corrige —
        // mais elle est marquée, pour qu'on sache qu'on touche à du visible.
        draggable: props.editable,
        resizable: props.editable,
      }
    }),
)

/**
 * Bornes horaires, déduites du contenu du JOUR AFFICHÉ et non de la semaine
 * entière : ouvrir de minuit à minuit laisserait les deux tiers de la hauteur
 * vides sur une journée qui commence à 9 h.
 */
const bounds = computed(() => {
  const minutes = sessionsInView.value
    .flatMap((session) => [
      toMinutes(wallClockInZone(session.starts_at, props.timezone).slice(11)),
      toMinutes(wallClockInZone(session.ends_at, props.timezone).slice(11)),
    ])

  if (!minutes.length) return { from: 8 * 60, to: 20 * 60 }
  return {
    from: Math.max(0, Math.floor(Math.min(...minutes) / 60) * 60 - 60),
    to: Math.min(24 * 60, Math.ceil(Math.max(...minutes) / 60) * 60 + 60),
  }
})

/**
 * Les séances de la période AFFICHÉE — le jour, ou les sept jours de la semaine.
 *
 * Elle sert deux fois : à cadrer les bornes horaires de la grille (ouvrir de
 * minuit à minuit sur une journée qui commence à 9 h laisse défiler pour rien) et
 * à dire que la période est vide.
 */
const sessionsInView = computed(() => {
  const keys = daysInView.value
  const exact = new Set(keys)
  return props.sessions.filter((session) => {
    const day = wallClockInZone(session.starts_at, props.timezone).slice(0, 10)
    // En vue année, les clés sont des mois (`2027-11`) : on compare le préfixe.
    return props.view === 'year' ? keys.some((key) => day.startsWith(key)) : exact.has(day)
  })
})

/**
 * Les jours couverts par la vue : un seul, la semaine du lundi au dimanche, le
 * mois entier ou l'année.
 *
 * Les dates sont construites par leurs COMPOSANTES et relues de même : passer par
 * un instant ferait changer de semaine — ou de mois — selon le fuseau du poste.
 */
const daysInView = computed<string[]>(() => {
  const [year, month, day] = props.selectedDate.split('-').map(Number)
  if (!year || !month || !day) return [props.selectedDate]

  if (props.view === 'day') return [props.selectedDate]

  if (props.view === 'week') {
    const weekday = (new Date(year, month - 1, day).getDay() + 6) % 7 // lundi = 0
    return Array.from({ length: 7 }, (_, index) =>
      wallClockFromLocalDate(new Date(year, month - 1, day - weekday + index)).slice(0, 10),
    )
  }

  if (props.view === 'month') {
    const days = new Date(year, month, 0).getDate()
    return Array.from({ length: days }, (_, index) =>
      wallClockFromLocalDate(new Date(year, month - 1, index + 1)).slice(0, 10),
    )
  }

  // Année : douze préfixes suffisent, on ne compare que le début de la date.
  return Array.from({ length: 12 }, (_, index) => `${year}-${String(index + 1).padStart(2, '0')}`)
})

/** Aucune séance placée sur la période affichée. */
const dayIsEmpty = computed(() => sessionsInView.value.length === 0)

function toMinutes(hhmm: string): number {
  const [hours, mins] = hhmm.split(':')
  return Number.parseInt(hours ?? '0', 10) * 60 + Number.parseInt(mins ?? '0', 10)
}

function sessionOf(id: unknown): PlannerSession | undefined {
  return sessionById.value.get(String(id))
}

function timeOf(sessionId: unknown): string {
  const session = sessionOf(sessionId)
  return session ? wallClockInZone(session.starts_at, props.timezone).slice(11) : ''
}

/**
 * Nom de la salle, affiché sur le bloc SEULEMENT quand ce n'est pas le stand.
 *
 * Sans colonnes de salles, plus rien ne dirait qu'une séance se tient en ligne —
 * or c'est ce qui explique qu'elle ne produise aucun conflit de stand face à sa
 * voisine. L'écrire sur toutes les séances du pavillon serait en revanche du
 * bruit : elles s'y tiennent toutes.
 */
function roomLabel(sessionId: unknown): string {
  const session = sessionOf(sessionId)
  if (!session || session.room_id === standRoomId.value) return ''
  return session.room_name ? tr(session.room_name) : ''
}

/**
 * Gravité affichée par le décompte d'une cellule : le pire conflit qu'elle porte.
 * `null` quand la journée est saine — le nombre reste alors neutre.
 */
function countClass(cellEvents: { sessionId?: unknown }[]): string {
  let worst: 'blocking' | 'warning' | null = null
  for (const entry of cellEvents) {
    const mark = props.marks.get(String(entry.sessionId ?? ''))
    if (!mark) continue
    if (mark.severity === 'blocking') return 'planner-count--blocking'
    worst = 'warning'
  }
  return worst ? 'planner-count--warning' : ''
}

/** Marque d'un bloc : le nombre de conflits qui le concernent, et leur nature. */
function markOf(sessionId: unknown): SessionConflictMark | undefined {
  return props.marks.get(String(sessionId))
}

function markLabel(sessionId: unknown): string {
  const mark = markOf(sessionId)
  if (!mark) return ''
  return mark.kinds.map((kind) => t(`admin.planner.conflict.kind.${kind}`)).join(' · ')
}

/**
 * DÉPÔT D'UN BLOC — celui d'une séance déjà placée comme celui d'une carte venue
 * du panneau latéral (`external`). Les deux se traduisent par la même écriture,
 * parce que la base n'en distingue pas : `room_id`, `starts_at`, `ends_at`.
 *
 * La durée est celle du bloc déposé ; vue-cal l'a déjà appliquée à `event.end`,
 * qu'on relit plutôt que de la recalculer.
 */
function onEventDrop(payload: {
  event: { start: Date; end: Date; sessionId?: unknown }
  originalEvent?: { preferredTime?: unknown; duration?: unknown }
}): void {
  const sessionId = String(payload.event.sessionId ?? '')
  if (!sessionId) return

  const startsAt = isTimeGrid.value
    ? instantFromDroppedDate(payload.event.start, props.timezone)
    : instantOnDay(payload.event.start, sessionId, payload.originalEvent)
  if (!startsAt) return

  const endsAt = isTimeGrid.value
    ? instantFromDroppedDate(payload.event.end, props.timezone)
    : endOfSlot(startsAt, durationOfDrop(sessionId, payload.originalEvent))
  if (!endsAt) return

  emit('schedule', { sessionId, roomId: roomAfterDrop(sessionId), startsAt, endsAt })
}

/**
 * Instant d'un dépôt fait HORS grille horaire : la date vient de la case, l'heure
 * du dossier ou de la séance — jamais de la position du curseur (voir l'en-tête).
 */
function instantOnDay(
  cellDate: Date,
  sessionId: string,
  transferred?: { preferredTime?: unknown },
): IsoDateTime | null {
  const day = wallClockFromLocalDate(cellDate).slice(0, 10)
  const session = sessionById.value.get(sessionId)

  const time =
    (typeof transferred?.preferredTime === 'string' && transferred.preferredTime) ||
    (session ? wallClockInZone(session.starts_at, props.timezone).slice(11) : '') ||
    DEFAULT_DROP_TIME

  return instantFromWallClock(`${day} ${time}`, props.timezone)
}

/** Durée à donner au bloc déposé : la sienne, celle transportée, ou 90 minutes. */
function durationOfDrop(sessionId: string, transferred?: { duration?: unknown }): number {
  const session = sessionById.value.get(sessionId)
  if (session) return durationOf(session)
  return typeof transferred?.duration === 'number' ? transferred.duration : 90
}

/** 9 h — l'heure d'ouverture du pavillon, faute de mieux. Ajustable ensuite. */
const DEFAULT_DROP_TIME = '09:00'

/** REDIMENSIONNEMENT — même écriture, seule la fin change. */
function onDurationChange(payload: { event: { start: Date; end: Date; sessionId?: unknown } }): void {
  const sessionId = String(payload.event.sessionId ?? '')
  if (!sessionId) return

  const startsAt = instantFromDroppedDate(payload.event.start, props.timezone)
  const endsAt = instantFromDroppedDate(payload.event.end, props.timezone)
  if (!startsAt || !endsAt) return

  emit('schedule', { sessionId, roomId: roomAfterDrop(sessionId), startsAt, endsAt })
}

function onViewChange(view: { startDate?: Date; view?: string }): void {
  // Cliquer une case de mois ou d'année FAIT DESCENDRE la bibliothèque d'un cran.
  // Sans reprendre sa vue, notre propriété la ramenait aussitôt en arrière : le
  // clic changeait la date et rien d'autre, ce qui se lit comme une panne.
  if (view.view && view.view !== props.view && VIEWS.includes(view.view as PlannerCalendarView)) {
    emit('update:view', view.view as PlannerCalendarView)
  }
  if (!view.startDate) return
  // La date rendue par vue-cal est LOCALE : on la relit par ses composantes,
  // sans passer par un instant, faute de quoi un poste à l'est de Greenwich
  // changerait de jour en changeant de fuseau.
  const next = wallClockFromLocalDate(view.startDate).slice(0, 10)
  if (next !== props.selectedDate) emit('update:selectedDate', next)
}
</script>

<template>
  <div class="min-w-0">
    <div class="mb-2 flex flex-wrap items-center justify-between gap-x-4 gap-y-2">
      <p class="text-sm text-text-muted">
        {{ t('admin.planner.calendar.zoneNotice', { zone: props.zoneLabel || props.timezone }) }}
      </p>

      <!-- LE JOUR POUR POSER, LA SEMAINE POUR VOIR L'ÉQUILIBRE. Deux boutons
           plutôt que le sélecteur natif de la bibliothèque : celui-ci offre le
           mois et l'année, qui n'ont aucun sens sur douze jours d'édition. -->
      <div class="flex items-center gap-1 rounded-md border border-border p-0.5" role="group" :aria-label="t('admin.planner.calendar.viewLabel')">
        <button
          v-for="entry in (['day', 'week', 'month', 'year'] as const)"
          :key="entry"
          type="button"
          class="cursor-pointer rounded px-3 py-1.5 text-sm transition-colors"
          :class="props.view === entry ? 'bg-accent-solid text-accent-contrast' : 'text-text-secondary hover:bg-surface-hover'"
          :aria-pressed="props.view === entry"
          @click="emit('update:view', entry)"
        >
          {{ t(`admin.planner.calendar.view.${entry}`) }}
        </button>
      </div>

      <!-- L'indication CHANGE avec la vue : promettre le glisser-déposer sur une
           grille qui ne l'accepte pas se paie en essais infructueux. -->
      <p v-if="props.editable" class="text-xs text-text-subtle">
        {{ t(isTimeGrid
          ? 'admin.planner.calendar.hint'
          : props.view === 'month'
            ? 'admin.planner.calendar.hintMonth'
            : 'admin.planner.calendar.hintReadOnly') }}
      </p>
    </div>

    <!-- PÉRIODE VIDE : dit UNE FOIS, au-dessus de la grille. Le message par
         défaut de la bibliothèque se répète dans chaque colonne — sept fois en
         vue semaine, pour une seule information. -->
    <p
      v-if="dayIsEmpty"
      class="mb-2 rounded-md border border-dashed border-border-strong px-4 py-3 text-center text-sm text-text-muted"
    >
      {{ t(`admin.planner.calendar.noEventIn.${props.view}`) }}
    </p>

    <div class="planner-calendar overflow-hidden rounded-lg border border-border bg-surface-raised">
      <VueCal
        :key="`${props.selectedDate}-${props.view}-${locale}-${canEdit}`"
        :events="events"
        :selected-date="props.selectedDate"
        :min-date="minDate"
        :max-date="maxDate"
        :active-view="props.view"
        :disable-views="['years']"
        hide-view-selector
        events-count-on-year-view
        :click-to-navigate="!isTimeGrid"
        :time-from="bounds.from"
        :time-to="bounds.to"
        :time-step="30"
        :time-cell-height="64"
        :snap-to-time="15"
        :locale="locale"
        :editable-events="canEdit ? { title: false, drag: true, resize: true, delete: false, create: false } : false"
        :drag-to-create-event="false"
        :dblclick-to-navigate="false"
        :style="{ height: props.view === 'day' ? '40rem' : '44rem' }"
        @event-drop="onEventDrop"
        @event-duration-change="onDurationChange"
        @view-change="onViewChange"
      >
        <!-- LE DÉCOMPTE D'UNE JOURNÉE (vue mois) OU D'UN MOIS (vue année), teinté
             par le pire conflit qu'il contient. Le nombre seul dirait la densité ;
             la couleur dit où l'arbitrage n'est pas fini. -->
        <template #events-count="{ events: cellEvents }">
          <span class="planner-count" :class="countClass(cellEvents)">
            {{ cellEvents.length }}
            <span class="sr-only">{{ t('admin.planner.calendar.countLabel', cellEvents.length) }}</span>
          </span>
        </template>

        <template #event="{ event }">
          <!-- Le corps du bloc est un vrai bouton : les blocs de vue-cal ne sont
               pas atteignables au clavier, et l'arbitrage ne peut pas dépendre
               d'une souris. Il ouvre le panneau de réglages, d'où l'on change
               salle, créneau, journée spéciale et diffusion sans rien traîner. -->
          <button
            type="button"
            class="planner-event__body"
            @click.stop="sessionOf(event.sessionId) && emit('open', sessionOf(event.sessionId)!)"
          >
            <span class="planner-event__meta">
              <span class="planner-event__time">{{ timeOf(event.sessionId) }}</span>
              <span v-if="markOf(event.sessionId)" class="planner-event__mark">
                <UiIcon name="warning" size="0.75rem" :stroke-width="2" />
                {{ markOf(event.sessionId)!.count }}
                <span class="sr-only">{{ markLabel(event.sessionId) }}</span>
              </span>
            </span>
            <span class="planner-event__title">{{ event.title }}</span>
            <!-- La salle n'est écrite que si ce n'est PAS le stand : une séance
                 en ligne n'occupe pas le pavillon, et c'est ce qui explique
                 qu'elle ne produise aucun conflit face à sa voisine. -->
            <span v-if="roomLabel(event.sessionId) && isTimeGrid" class="planner-event__room">
              {{ roomLabel(event.sessionId) }}
            </span>
          </button>
        </template>
      </VueCal>
    </div>
  </div>
</template>

<style scoped>
/*
 * vue-cal apporte une feuille de style écrite pour un thème clair et des
 * couleurs en dur. On ne la remplace pas : on redéfinit ce qui doit suivre les
 * JETONS de la plateforme — grille, barre de titre, colonnes de salle, blocs.
 * Sans cela, le planificateur reste blanc en thème sombre.
 */
.planner-calendar :deep(.vuecal) {
  box-shadow: none;
  color: var(--color-text);
  font-family: inherit;
}

.planner-calendar :deep(.vuecal__title-bar) {
  background-color: var(--color-surface-sunken);
  font-size: 1rem;
  font-family: var(--font-display, inherit);
  min-height: var(--target-min);
}

.planner-calendar :deep(.vuecal__arrow),
.planner-calendar :deep(.vuecal__title button) {
  color: var(--color-text);
  min-height: var(--target-min);
  min-width: var(--target-min);
  cursor: pointer;
}

/* EN VUE SEMAINE, les en-têtes de jours : ils portent le seul repère qui
   distingue une colonne d'une autre. */
.planner-calendar :deep(.vuecal__heading) {
  color: var(--color-text-secondary);
  font-size: 0.8125rem;
  font-weight: 600;
}

.planner-calendar :deep(.vuecal__heading.today) {
  color: var(--color-accent);
}

/* LES TRAITS DE LA GRILLE — `--color-border`, et non `--color-separator`.
   En thème sombre, le filet intérieur vaut gris-800 sur un fond presque aussi
   sombre : la grille disparaissait, et l'on plaçait des blocs sans voir les
   heures ni les jours (signalé en revue). Le trait de séparation ordinaire, lui,
   se voit dans les deux thèmes. */
.planner-calendar :deep(.vuecal__cell:before),
.planner-calendar :deep(.vuecal__time-column .vuecal__time-cell-line:before) {
  border-color: var(--color-border);
}

/* La barre des jours suit la même règle : la feuille de la bibliothèque y pose
   un gris clair, invisible sur fond sombre. */
.planner-calendar :deep(.vuecal__weekdays-headings),
.planner-calendar :deep(.vuecal__all-day-text) {
  border-color: var(--color-border);
}

/* Le jour retenu et le jour courant, distingués par un fond TEINTÉ plutôt que
   par les gris pâles de la bibliothèque, illisibles en thème sombre. */
.planner-calendar :deep(.vuecal__cell--selected) {
  background-color: color-mix(in oklab, var(--color-accent) 12%, transparent);
}

.planner-calendar :deep(.vuecal__cell--out-of-scope),
.planner-calendar :deep(.vuecal__cell--out-of-scope .vuecal__cell-date) {
  color: var(--color-text-subtle);
}

.planner-calendar :deep(.vuecal__time-column .vuecal__time-cell) {
  color: var(--color-text-subtle);
}

.planner-calendar :deep(.vuecal__cell--highlighted) {
  background-color: color-mix(in oklab, var(--color-accent) 10%, transparent);
}

.planner-calendar :deep(.vuecal__now-line) {
  color: var(--color-live);
}

/* vue-cal écrit « Aucun événement » DANS CHAQUE COLONNE de salle vide : trois
   salles donnaient trois fois la même phrase pour une seule information. Le
   message du jour vide est rendu une fois, au-dessus de la grille. */
.planner-calendar :deep(.vuecal__no-event) {
  display: none;
}

.planner-calendar :deep(.vuecal__event) {
  background: none;
  border-radius: 6px;
  overflow: visible;
}

.planner-calendar :deep(.vuecal__event.vuecal__event--dragging) {
  opacity: 0.5;
}

.planner-calendar :deep(.planner-event__body) {
  display: flex;
  flex-direction: column;
  gap: 2px;
  width: 100%;
  height: 100%;
  padding: 4px 6px;
  text-align: start;
  overflow: hidden;
  cursor: pointer;
  border: 1px solid var(--color-info-border);
  border-left: 4px solid var(--color-info-border);
  border-radius: 6px;
  background-color: var(--color-info-surface);
  color: var(--color-text);
  font: inherit;
}

.planner-calendar :deep(.planner-event__body:focus-visible) {
  outline: none;
  box-shadow: var(--shadow-focus);
}

.planner-calendar :deep(.planner-event--selected .planner-event__body) {
  border-color: var(--color-accent);
  box-shadow: var(--shadow-focus);
}

/* UNE SÉANCE PROGRAMMÉE MAIS PAS ENCORE PUBLIQUE reste neutre ; c'est l'état de
   travail, pas un problème. */
.planner-calendar :deep(.planner-event--planned .planner-event__body) {
  background-color: var(--color-neutral-surface);
  border-color: var(--color-border-strong);
  border-left-color: var(--color-border-strong);
}

/* LE CONFLIT SE VOIT SUR LE BLOC, pas seulement au bandeau : rouge pour ce qui
   est matériellement impossible, jaune pour ce qui demande attention. La couleur
   ne porte jamais seule — le bloc affiche aussi le nombre de conflits, et sa
   nature est énoncée aux lecteurs d'écran. */
.planner-calendar :deep(.planner-event--conflict-blocking .planner-event__body) {
  background-color: var(--color-danger-surface);
  border-color: var(--color-danger-border);
  border-left-color: var(--color-danger);
  border-left-width: 6px;
}

.planner-calendar :deep(.planner-event--conflict-warning .planner-event__body) {
  background-color: var(--color-warning-surface);
  border-color: var(--color-warning-border);
  border-left-color: var(--color-warning);
  border-left-width: 6px;
}

.planner-calendar :deep(.planner-event--live .planner-event__body) {
  border-color: var(--color-live);
  border-width: var(--border-medium);
}

.planner-calendar :deep(.planner-event__meta) {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.375rem;
  min-width: 0;
}

.planner-calendar :deep(.planner-event__time) {
  font-size: 0.6875rem;
  font-variant-numeric: tabular-nums;
  color: var(--color-text-muted);
}

.planner-calendar :deep(.planner-event__mark) {
  display: inline-flex;
  align-items: center;
  gap: 0.125rem;
  font-size: 0.6875rem;
  font-weight: 700;
  color: var(--color-danger);
}

.planner-calendar :deep(.planner-event--conflict-warning .planner-event__mark) {
  color: var(--color-warning);
}

.planner-calendar :deep(.planner-event__room) {
  font-size: 0.625rem;
  text-transform: uppercase;
  letter-spacing: var(--tracking-caps);
  color: var(--color-text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.planner-calendar :deep(.planner-event__title) {
  font-size: 0.8125rem;
  font-weight: 600;
  line-height: 1.25;
  overflow: hidden;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
}

.planner-calendar :deep(.vuecal--month-view .vuecal__cell-content) {
  justify-content: flex-start;
  align-items: stretch;
  padding: 2px;
}

.planner-calendar :deep(.vuecal--month-view .vuecal__cell-date) {
  padding: 2px 4px;
  font-size: 0.75rem;
  color: var(--color-text-secondary);
}

.planner-calendar :deep(.vuecal--month-view .vuecal__event),
.planner-calendar :deep(.vuecal--year-view .vuecal__event) {
  margin-bottom: 2px;
}

/* LE DÉCOMPTE — une pastille par jour en vue mois, par mois en vue année. */
.planner-calendar :deep(.vuecal__cell-events-count) {
  background: none;
  padding: 0;
}

.planner-calendar :deep(.planner-count) {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 1.75rem;
  padding: 0.125rem 0.5rem;
  border-radius: 999px;
  background-color: var(--color-accent-solid);
  color: var(--color-accent-contrast);
  font-size: 0.8125rem;
  font-weight: 700;
  font-variant-numeric: tabular-nums;
}

/* La couleur dit où l'arbitrage n'est pas fini — jamais seule : le bandeau les
   nomme, et la journée s'ouvre d'un clic. */
.planner-calendar :deep(.planner-count--blocking) {
  background-color: var(--color-danger);
  color: var(--color-danger-contrast, #fff);
}

.planner-calendar :deep(.planner-count--warning) {
  background-color: var(--color-warning);
  color: var(--color-warning-contrast, #1a1a1a);
}

/* Les mois et les jours hors de l'édition restent LISIBLES : la bibliothèque les
   efface presque, et une vue année où l'on ne lit plus « Janvier » ne situe
   plus rien. */
.planner-calendar :deep(.vuecal__cell--disabled) {
  color: var(--color-text-subtle);
  opacity: 1;
}

.planner-calendar :deep(.vuecal__cell--disabled .vuecal__cell-date) {
  color: var(--color-text-subtle);
}

.planner-calendar :deep(.vuecal--month-view .vuecal__cell-date),
.planner-calendar :deep(.vuecal--year-view .vuecal__cell-date) {
  color: var(--color-text-secondary);
  font-size: 0.8125rem;
}

/* Hors grille horaire, la cellule se clique pour descendre d'un cran. */
.planner-calendar :deep(.vuecal--click-to-navigate .vuecal__cell-content) {
  cursor: pointer;
}

/* La poignée de redimensionnement de vue-cal, rendue visible : sans elle,
   personne ne devine que le bloc s'étire. */
.planner-calendar :deep(.vuecal__event-resize-handle) {
  height: 8px;
  background-color: color-mix(in oklab, var(--color-text) 18%, transparent);
  border-radius: 0 0 6px 6px;
}
</style>
