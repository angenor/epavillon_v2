<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { DraftIssue, DraftSpeaker, ProposalDraft } from '~/types/proposal-form'

/**
 * ÉTAPE 4 — LES INTERVENANTS.
 *
 * LES BORNES VIENNENT DE L'APPEL : `min_speakers` et `max_speakers`, données de
 * `event.calls_for_proposals`. Elles sont annoncées AVANT la saisie, pas
 * découvertes à l'envoi — un dossier qu'on croit fini et qui exige un cinquième
 * intervenant se solde par un intervenant inventé.
 *
 * ATTENTION, CE N'EST PAS LA BASE QUI LES TIENT : aucun trigger ne les vérifie
 * aujourd'hui. C'est l'écran qui les applique, et cela devra devenir une règle
 * de l'API (prompt B4). Écrit ici pour que la prochaine session ne croie pas la
 * contrainte déjà portée par le modèle.
 *
 * LA LISTE EST ORDONNÉE, ET L'ORDRE COMPTE : `proposal_speakers.sort_order`
 * décide de l'ordre d'affichage sur la fiche publique de l'activité. On le
 * modifie par deux boutons plutôt que par glisser-déposer — un glissé se rate au
 * doigt, et la moitié des dossiers se remplissent sur téléphone.
 *
 * CHAQUE LIGNE MONTRE CE QUI MANQUE, et ce qui manque BLOQUE : civilité,
 * fonction et organisation sont obligatoires depuis le 17/08 (arbitrage du
 * commanditaire). Aucune n'est `NOT NULL` en base — ce sont des exigences de
 * dossier : le programme annonce « Mme Awa Sow Fall, directrice exécutive,
 * ROAC », et une ligne amputée s'y voit immédiatement.
 *
 * LA PHOTO EST MONTRÉE, pas seulement annoncée. Une vignette dans la liste est
 * le seul endroit où l'on s'aperçoit qu'un portrait s'est glissé sur la mauvaise
 * personne — ce qu'un « Photo jointe » en toutes lettres ne dit jamais.
 */

const draft = defineModel<ProposalDraft>({ required: true })

interface Props {
  call: CallForProposals
  issues: DraftIssue[]
}

const props = defineProps<Props>()

const { t } = useI18n()

const isDialogOpen = ref(false)
const editing = ref<DraftSpeaker | null>(null)

const takenEmails = computed(() => draft.value.speakers.map((speaker) => speaker.email))

const countError = computed(() => {
  const issue = props.issues.find(
    (entry) => entry.field === 'speakers' && entry.severity === 'error',
  )
  return issue ? t(issue.messageKey, issue.params ?? {}, Number(issue.params?.count ?? 1)) : null
})

function issuesOfSpeaker(speaker: DraftSpeaker): DraftIssue[] {
  return props.issues.filter((entry) => entry.field.startsWith(`speakers.${speaker.key}.`))
}

function openAdd(): void {
  editing.value = null
  isDialogOpen.value = true
}

function openEdit(speaker: DraftSpeaker): void {
  editing.value = speaker
  isDialogOpen.value = true
}

function save(speaker: DraftSpeaker): void {
  const existing = draft.value.speakers.findIndex((entry) => entry.key === speaker.key)
  draft.value.speakers =
    existing >= 0
      ? draft.value.speakers.map((entry) => (entry.key === speaker.key ? speaker : entry))
      : [...draft.value.speakers, speaker]
  isDialogOpen.value = false
  editing.value = null
}

function remove(key: string): void {
  draft.value.speakers = draft.value.speakers.filter((entry) => entry.key !== key)
}

/** Déplacement d'un cran. `sort_order` se recompose à l'envoi, depuis l'ordre. */
function move(index: number, direction: -1 | 1): void {
  const target = index + direction
  if (target < 0 || target >= draft.value.speakers.length) return
  const next = [...draft.value.speakers]
  const moved = next[index]
  const swapped = next[target]
  if (!moved || !swapped) return
  next[index] = swapped
  next[target] = moved
  draft.value.speakers = next
}

function fullName(speaker: DraftSpeaker): string {
  const civility = speaker.civility
    ? t(`proposal.form.step-speakers.civilities.${speaker.civility}`)
    : ''
  return `${civility} ${speaker.first_name} ${speaker.last_name}`.trim()
}
</script>

<template>
  <div class="grid gap-5">
    <header>
      <h2 class="font-display text-xl text-text">
        {{ t('proposal.form.step-speakers.title') }}
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('proposal.form.step-speakers.description') }}
      </p>
    </header>

    <!-- Les bornes de l'appel, annoncées AVANT la saisie. -->
    <p class="flex items-start gap-2 rounded-md bg-surface-sunken px-3 py-2 text-sm text-text-secondary">
      <UiIcon name="users" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
      {{
        t('proposal.form.step-speakers.bounds', {
          min: props.call.min_speakers,
          max: props.call.max_speakers,
          count: draft.speakers.length,
        })
      }}
    </p>

    <p v-if="countError" role="alert" class="text-sm font-bold text-danger">{{ countError }}</p>

    <ul v-if="draft.speakers.length > 0" class="grid gap-3">
      <li
        v-for="(speaker, index) in draft.speakers"
        :key="speaker.key"
        class="rounded-md border bg-surface-raised px-4 py-3"
        :class="issuesOfSpeaker(speaker).some((i) => i.severity === 'error') ? 'border-danger-border' : 'border-border'"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <!-- La photo est MONTRÉE dans la liste, pas seulement annoncée : c'est
               le seul endroit où l'on vérifie d'un coup d'œil qu'aucun portrait
               ne s'est glissé sur la mauvaise personne. -->
          <img
            v-if="speaker.photo?.preview_url"
            :src="speaker.photo.preview_url"
            :alt="t('proposal.form.step-speakers.photo.previewAlt', { speaker: fullName(speaker) })"
            class="size-12 shrink-0 rounded-md border border-border object-cover"
          >

          <div class="min-w-0 flex-1">
            <p class="font-bold text-text">
              {{ fullName(speaker) }}
              <UiBadge size="sm" class="ml-1.5">
                {{ t(`proposal.form.step-speakers.roles.${speaker.role}.label`) }}
              </UiBadge>
              <!-- Profil de la plateforme : le déposant n'en modifie pas
                   l'identité, et il doit le savoir depuis la liste. -->
              <UiBadge v-if="speaker.has_account" size="sm" intent="success" icon="shield-check" class="ml-1.5">
                {{ t('proposal.form.step-speakers.accountBadge') }}
              </UiBadge>
            </p>
            <p class="text-sm text-text-muted">
              <span v-if="speaker.job_title">{{ speaker.job_title }}</span>
              <span v-if="speaker.job_title && speaker.organization_name"> · </span>
              <span v-if="speaker.organization_name">{{ speaker.organization_name }}</span>
            </p>
            <p class="font-mono text-sm text-text-subtle">{{ speaker.email }}</p>

            <ul v-if="issuesOfSpeaker(speaker).length > 0" class="mt-2 grid gap-1">
              <li
                v-for="(issue, issueIndex) in issuesOfSpeaker(speaker)"
                :key="issueIndex"
                class="flex items-start gap-1.5 text-sm"
                :class="issue.severity === 'error' ? 'text-danger' : 'text-warning'"
              >
                <UiIcon
                  :name="issue.severity === 'error' ? 'error' : 'warning'"
                  size="0.95rem"
                  class="mt-0.5 shrink-0"
                />
                {{ t(issue.messageKey, issue.params ?? {}, Number(issue.params?.count ?? 1)) }}
              </li>
            </ul>
          </div>

          <div class="flex shrink-0 items-center gap-1">
            <UiButton
              variant="ghost"
              size="sm"
              icon="chevron-up"
              icon-only
              :disabled="index === 0"
              :label="t('proposal.form.step-speakers.moveUp', { speaker: fullName(speaker) })"
              @click="move(index, -1)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="chevron-down"
              icon-only
              :disabled="index === draft.speakers.length - 1"
              :label="t('proposal.form.step-speakers.moveDown', { speaker: fullName(speaker) })"
              @click="move(index, 1)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="edit"
              icon-only
              :label="t('proposal.form.step-speakers.edit', { speaker: fullName(speaker) })"
              @click="openEdit(speaker)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="trash"
              icon-only
              :label="t('proposal.form.step-speakers.remove', { speaker: fullName(speaker) })"
              @click="remove(speaker.key)"
            />
          </div>
        </div>
      </li>
    </ul>

    <UiEmptyState
      v-else
      compact
      icon="users"
      :title="t('proposal.form.step-speakers.empty.title')"
      :description="t('proposal.form.step-speakers.empty.description')"
    />

    <div>
      <UiButton
        variant="secondary"
        icon="plus"
        :disabled="draft.speakers.length >= props.call.max_speakers"
        :label="t('proposal.form.step-speakers.add')"
        @click="openAdd()"
      />
      <p
        v-if="draft.speakers.length >= props.call.max_speakers"
        class="mt-2 text-sm text-text-muted"
      >
        {{ t('proposal.form.step-speakers.maxReached', { max: props.call.max_speakers }) }}
      </p>
    </div>

    <!-- LA CONFIRMATION DE L'INTERVENANT SE FAIT PLUS TARD, par jeton envoyé par
         courriel (`one_time_tokens`, usage `speaker_confirmation`). On le dit :
         citer quelqu'un dans un dossier l'engage, et il l'apprendra. -->
    <p class="flex items-start gap-2 text-sm text-text-muted">
      <UiIcon name="mail" size="1.05rem" class="mt-0.5 shrink-0" />
      {{ t('proposal.form.step-speakers.confirmationNotice') }}
    </p>

    <ProposalSpeakerDialog
      v-model:open="isDialogOpen"
      :speaker="editing"
      :taken-emails="takenEmails"
      @save="save"
    />
  </div>
</template>
