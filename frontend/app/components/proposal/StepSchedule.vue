<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'
import type { DraftIssue, ProposalDraft } from '~/types/proposal-form'
import { TEXT_LIMITS } from '~/types/proposal-form'
import type { SelectOption } from '~/types/ui'

/**
 * ÉTAPE 5 — CRÉNEAU SOUHAITÉ ET DURÉE.
 *
 * LA RÈGLE MÉTIER LA PLUS CONTRE-INTUITIVE DE LA PLATEFORME SE JOUE ICI : les
 * chevauchements ne sont JAMAIS bloqués. Une organisation propose le créneau qui
 * l'arrange, sans se soucier de ce que les autres ont demandé — c'est l'IFDD qui
 * arbitre ensuite, par glisser-déposer (A9). Cet écran ne consulte donc aucune
 * disponibilité, n'affiche aucun conflit et ne grise aucune heure. Une
 * disponibilité affichée ici serait un mensonge : le créneau retenu vit sur la
 * SESSION, pas sur le dossier (`proposals.preferred_*` contre
 * `sessions.starts_at`).
 *
 * LA MENTION EST EXPLICITE ET EN TÊTE, pas en note de bas de page : sans elle,
 * une organisation croit réserver, et découvre le contraire par un courriel de
 * programmation qu'elle prend pour une erreur.
 *
 * L'HEURE SAISIE EST CELLE DU PAVILLON. Le champ porte le fuseau de l'édition —
 * « heure de Belém » — et la conversion en instant se fait avec ce fuseau-là
 * (`instantFromWallClock`). Sans cela, un dossier rempli depuis Dakar
 * proposerait 14:30 heure de Dakar, soit 11:30 sur place, et personne ne s'en
 * apercevrait avant la publication du programme.
 *
 * LES OCCURRENCES (`requested_sessions`) NE SONT PAS UN DÉTAIL : un cycle de
 * webinaires en annonce plusieurs dès le dépôt. La v1 ne savait pas l'exprimer
 * et a dû rattraper le cas PACO par une colonne ajoutée dans les inscriptions.
 *
 * CRÉNEAU ET DURÉE SONT OBLIGATOIRES depuis le 17/08 (arbitrage du
 * commanditaire), et les bornes viennent de l'APPEL : durée entre
 * `min_duration_minutes` et `max_duration_minutes`, activité comprise entre
 * `daily_start_time` et `daily_end_time`. Ces quatre colonnes ont été ajoutées au
 * modèle le même jour ; les écrire en dur ici aurait figé « 9 h – 17 h » pour
 * toutes les éditions à venir, y compris celles qui n'ont pas de stand.
 *
 * UNE PLAGE D'OUVERTURE N'EST PAS UN CHEVAUCHEMENT. Refuser 16 h pour une séance
 * de deux heures ne contredit pas la règle métier n° 2 : on ne compare le
 * créneau à aucun autre dossier, seulement à l'amplitude d'ouverture du stand —
 * un fait matériel, comme le fait qu'il n'y ait qu'un seul lieu.
 */

const draft = defineModel<ProposalDraft>({ required: true })

interface Props {
  call: CallForProposals
  edition: EventEdition
  issues: DraftIssue[]
}

const props = defineProps<Props>()

const { t } = useI18n()
const { date, dateTime } = useDateTime()

const zone = computed(() => props.edition.timezone)
const zoneLabel = computed(() => props.edition.city ?? timeZoneCityLabel(props.edition.timezone))

function errorOf(field: string): string | undefined {
  const issue = props.issues.find((entry) => entry.field === field && entry.severity === 'error')
  return issue ? t(issue.messageKey, issue.params ?? {}, Number(issue.params?.count ?? 1)) : undefined
}

// ---------------------------------------------------------------------------
// Bornes du champ de date : les jours de l'édition, en heure MURALE du pavillon
// ---------------------------------------------------------------------------

const minWallClock = computed(() =>
  wallClockInZone(props.edition.starts_at, zone.value).replace(' ', 'T'),
)
const maxWallClock = computed(() =>
  wallClockInZone(props.edition.ends_at, zone.value).replace(' ', 'T'),
)

const editionRange = computed(() =>
  t('common.datetime.dateRange', {
    start: date(props.edition.starts_at, zone.value),
    end: date(props.edition.ends_at, zone.value),
  }),
)

// ---------------------------------------------------------------------------
// Durée
// ---------------------------------------------------------------------------

/**
 * LES DURÉES PROPOSÉES SORTENT DE L'APPEL, pas d'une liste écrite ici :
 * `min_duration_minutes` et `max_duration_minutes` sont des colonnes depuis le
 * 17/08. On égrène des quarts d'heure entre les deux bornes — un pas de quinze
 * minutes est celui d'une grille de programmation, et il évite le champ libre où
 * l'on saisit « 1h37 ».
 */
const durationOptions = computed<SelectOption[]>(() => {
  const values: number[] = []
  for (
    let minutes = props.call.min_duration_minutes;
    minutes <= props.call.max_duration_minutes;
    minutes += 15
  ) {
    values.push(minutes)
  }
  // La borne haute tombe rarement sur un quart d'heure du pas : on l'ajoute.
  if (!values.includes(props.call.max_duration_minutes)) values.push(props.call.max_duration_minutes)
  // Une durée reprise d'un brouillon plus ancien doit rester sélectionnable,
  // même si l'appel a resserré ses bornes depuis.
  if (draft.value.duration_minutes && !values.includes(draft.value.duration_minutes)) {
    values.push(draft.value.duration_minutes)
  }

  return values
    .sort((a, b) => a - b)
    .map((minutes) => ({
      value: String(minutes),
      label: t('proposal.form.step-schedule.duration.value', { minutes }),
      description:
        minutes === props.call.default_duration_minutes
          ? t('proposal.form.step-schedule.duration.default')
          : undefined,
    }))
})

/** « 09:00 » — les deux bornes de l'appel, sans leurs secondes. */
const dailyWindow = computed(() => ({
  open: props.call.daily_start_time.slice(0, 5),
  close: props.call.daily_end_time.slice(0, 5),
}))

/** Fin déduite du début et de la durée — `preferred_end_at` n'est pas saisi. */
const endLabel = computed(() => {
  if (!draft.value.preferred_start_at || !draft.value.duration_minutes) return ''
  const start = instantFromWallClock(draft.value.preferred_start_at, zone.value)
  if (!start) return ''
  const end = new Date(Date.parse(start) + draft.value.duration_minutes * 60_000)
  return dateTime(end.toISOString(), zone.value)
})
</script>

<template>
  <div class="grid gap-6">
    <header>
      <h2 class="font-display text-xl text-text">
        {{ t('proposal.form.step-schedule.title') }}
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('proposal.form.step-schedule.description') }}
      </p>
    </header>

    <!-- LA MENTION, en tête et sans détour. -->
    <UiAlert intent="info" :title="t('proposal.form.step-schedule.notice.title')">
      {{ t('proposal.form.step-schedule.notice.body') }}
    </UiAlert>

    <UiDatePicker
      v-model="draft.preferred_start_at"
      with-time
      required
      :label="t('proposal.form.step-schedule.start.label')"
      :hint="t('proposal.form.step-schedule.start.hint', {
        range: editionRange,
        open: dailyWindow.open,
        close: dailyWindow.close,
      })"
      :error="errorOf('preferred_start_at')"
      :min="minWallClock"
      :max="maxWallClock"
      :timezone-label="zoneLabel"
    />

    <div class="grid gap-4 sm:grid-cols-2">
      <UiSelect
        :model-value="draft.duration_minutes === null ? null : String(draft.duration_minutes)"
        :options="durationOptions"
        :label="t('proposal.form.step-schedule.duration.label')"
        required
        :hint="t('proposal.form.step-schedule.duration.hint', {
          min: props.call.min_duration_minutes,
          max: props.call.max_duration_minutes,
        })"
        :error="errorOf('duration_minutes')"
        :placeholder="t('proposal.form.step-schedule.duration.placeholder')"
        @update:model-value="draft.duration_minutes = Number($event)"
      />

      <UiInput
        :model-value="draft.requested_sessions"
        type="number"
        :min="1"
        :max="50"
        :label="t('proposal.form.step-schedule.sessions.label')"
        :hint="t('proposal.form.step-schedule.sessions.hint')"
        :error="errorOf('requested_sessions')"
        required
        @update:model-value="draft.requested_sessions = Number($event)"
      />
    </div>

    <!-- La fin est DÉDUITE, jamais saisie : deux champs de date à tenir cohérents
         sont deux occasions de se contredire, et `ck_proposals_preferred_period`
         refuserait la ligne. -->
    <p v-if="endLabel" class="flex items-start gap-2 rounded-md bg-surface-sunken px-3 py-2 text-sm text-text-secondary">
      <UiIcon name="clock" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
      {{ t('proposal.form.step-schedule.endsAt', { end: endLabel }) }}
    </p>

    <UiTextarea
      v-model="draft.scheduling_constraints"
      :label="t('proposal.form.step-schedule.constraints.label')"
      :hint="t('proposal.form.step-schedule.constraints.hint')"
      :error="errorOf('scheduling_constraints')"
      :maxlength="TEXT_LIMITS.scheduling_constraints"
      :rows="3"
      auto-grow
    />
  </div>
</template>
