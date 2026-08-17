<script setup lang="ts">
import type { DraftIssue, ProposalDraft } from '~/types/proposal-form'
import { TEXT_LIMITS } from '~/types/proposal-form'

/**
 * ÉTAPE 2 — CE QUE L'ACTIVITÉ EST : titre, résumé, objectifs, présentation
 * détaillée, résultats attendus, public visé.
 *
 * TROIS CHAMPS SEULEMENT SONT OBLIGATOIRES, et ce sont exactement les trois
 * colonnes `NOT NULL` du dossier : `title`, `objectives`,
 * `detailed_presentation`. Les trois autres sont facultatives en base et le
 * restent ici — mais leur absence est signalée comme un affaiblissement du
 * dossier, pas comme une erreur. Le comité note sur six critères, dont l'impact
 * attendu et l'inclusion : un dossier qui ne dit ni ce qu'il produit ni à qui il
 * s'adresse part avec un handicap que personne ne lui a annoncé.
 *
 * LA SAISIE SE FAIT EN FRANÇAIS, quelle que soit la langue de l'interface, et ce
 * n'est pas une préférence : `platform.i18n_text` exige la clé `fr` non vide
 * (`platform.is_i18n_text()`). Un dossier rédigé en anglais seul serait refusé
 * par la base. L'aide contextuelle le dit une fois, en tête d'étape — le
 * découvrir champ par champ serait une punition.
 *
 * COMPTEURS DE CARACTÈRES PARTOUT, et la saisie n'est JAMAIS coupée : le texte
 * au-delà reste visible, le compteur passe en rouge, l'envoi refuse avec un
 * message. Couper la frappe à mi-mot fait croire à un clavier cassé. Les limites
 * sont des règles d'écran (`TEXT_LIMITS`), pas des contraintes de la base : un
 * résumé de mille signes ne tient pas sur une carte de programmation, et
 * personne ne le raccourcira après coup.
 */

const draft = defineModel<ProposalDraft>({ required: true })

/** Longueur d'un public visé : c'est une étiquette, pas une phrase. */
const AUDIENCE_MAX = 80

interface Props {
  issues: DraftIssue[]
}

const props = defineProps<Props>()

const { t } = useI18n()

function errorOf(field: string): string | undefined {
  const issue = props.issues.find((entry) => entry.field === field && entry.severity === 'error')
  if (!issue) return undefined
  return t(issue.messageKey, issue.params ?? {}, Number(issue.params?.count ?? 1))
}

function warningOf(field: string): string | undefined {
  const issue = props.issues.find((entry) => entry.field === field && entry.severity === 'warning')
  return issue ? t(issue.messageKey) : undefined
}

/**
 * L'aide contextuelle d'un champ facultatif porte l'avertissement quand il est
 * vide. Le champ garde ainsi UN seul message — pas un rouge qui n'en est pas un.
 */
function hintOf(field: string, base: string): string {
  const warning = warningOf(field)
  return warning ?? base
}

// ---------------------------------------------------------------------------
// Publics visés — une liste, saisie un par un
// ---------------------------------------------------------------------------

/**
 * POURQUOI UNE LISTE ET NON UNE PHRASE. « Ministères, ONG, journalistes » écrit
 * d'un trait ne se réaffiche pas : il s'imprime tel quel, ne se compte pas, ne
 * se filtre pas, et finit découpé à la virgule par le premier gabarit qui
 * essaie — ce que faisait la v1. Une entrée par public permet de les rendre en
 * pastilles sur la fiche publique, et de les compter dans les statistiques.
 * Le modèle suit : `proposals.target_audiences` est un tableau depuis le 17/08.
 */
const audienceInput = ref('')

const audienceError = computed(() => {
  const value = audienceInput.value.trim()
  if (value.length === 0) return undefined
  if (value.length > AUDIENCE_MAX) {
    return t('validation.maxLength', { max: AUDIENCE_MAX }, AUDIENCE_MAX)
  }
  if (draft.value.target_audiences.some((entry) => entry.toLowerCase() === value.toLowerCase())) {
    return t('proposal.form.step-presentation.fields.target_audience.duplicate')
  }
  return undefined
})

function addAudience(): void {
  const value = audienceInput.value.trim()
  if (value.length === 0 || audienceError.value) return
  draft.value.target_audiences = [...draft.value.target_audiences, value]
  audienceInput.value = ''
}

function removeAudience(value: string): void {
  draft.value.target_audiences = draft.value.target_audiences.filter((entry) => entry !== value)
}

/**
 * La VIRGULE valide l'entrée, comme la touche Entrée. C'est le geste que fait
 * naturellement quelqu'un qui a l'habitude d'écrire une énumération, et le
 * contrarier produit « Ministères, ONG » dans une seule pastille.
 */
function onAudienceKey(event: KeyboardEvent): void {
  if (event.key === 'Enter' || event.key === ',') {
    event.preventDefault()
    addAudience()
  }
}
</script>

<template>
  <div class="grid gap-6">
    <header>
      <h2 class="font-display text-xl text-text">
        {{ t('proposal.form.step-presentation.title') }}
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('proposal.form.step-presentation.description') }}
      </p>
      <!-- La langue de rédaction, dite une seule fois et en tête. -->
      <p class="mt-3 flex max-w-(--measure) items-start gap-2 rounded-md bg-surface-sunken px-3 py-2 text-sm text-text-secondary">
        <UiIcon name="info" size="1.05rem" class="mt-0.5 shrink-0 text-text-muted" />
        {{ t('proposal.form.step-presentation.frenchNotice') }}
      </p>
    </header>

    <UiInput
      id="proposal-title"
      v-model="draft.title"
      :label="t('proposal.form.step-presentation.fields.title.label')"
      :hint="t('proposal.form.step-presentation.fields.title.hint')"
      :error="errorOf('title')"
      :maxlength="TEXT_LIMITS.title"
      show-counter
      required
    />

    <UiTextarea
      id="proposal-summary"
      v-model="draft.summary"
      :label="t('proposal.form.step-presentation.fields.summary.label')"
      :hint="hintOf('summary', t('proposal.form.step-presentation.fields.summary.hint'))"
      :error="errorOf('summary')"
      :maxlength="TEXT_LIMITS.summary"
      :rows="3"
      auto-grow
    />

    <UiTextarea
      id="proposal-objectives"
      v-model="draft.objectives"
      :label="t('proposal.form.step-presentation.fields.objectives.label')"
      :hint="t('proposal.form.step-presentation.fields.objectives.hint')"
      :error="errorOf('objectives')"
      :maxlength="TEXT_LIMITS.objectives"
      :rows="4"
      auto-grow
      required
    />

    <!-- LA SEULE ZONE DE TEXTE RICHE DU DOSSIER. Structure seulement : ni police,
         ni couleur, ni taille — la charte décide de l'apparence, et c'est ce qui
         permet de rendre ce texte dans les deux thèmes, dans un courriel et dans
         un programme imprimé sans le réécrire. -->
    <ClientOnly>
      <UiRichText
        id="proposal-detailed_presentation"
        v-model="draft.detailed_presentation"
        :label="t('proposal.form.step-presentation.fields.detailed_presentation.label')"
        :hint="t('proposal.form.step-presentation.fields.detailed_presentation.hint')"
        :error="errorOf('detailed_presentation')"
        :maxlength="TEXT_LIMITS.detailed_presentation"
        :rows="10"
        required
      />
      <!-- Au rendu serveur, un cadre de la bonne hauteur : l'éditeur manipule le
           DOM et n'existe que dans le navigateur. -->
      <template #fallback>
        <UiTextarea
          :label="t('proposal.form.step-presentation.fields.detailed_presentation.label')"
          :hint="t('proposal.form.step-presentation.fields.detailed_presentation.hint')"
          :rows="10"
          readonly
        />
      </template>
    </ClientOnly>

    <UiTextarea
      id="proposal-expected_outcomes"
      v-model="draft.expected_outcomes"
      :label="t('proposal.form.step-presentation.fields.expected_outcomes.label')"
      :hint="hintOf('expected_outcomes', t('proposal.form.step-presentation.fields.expected_outcomes.hint'))"
      :error="errorOf('expected_outcomes')"
      :maxlength="TEXT_LIMITS.expected_outcomes"
      :rows="4"
      auto-grow
    />

    <!-- PUBLICS VISÉS — ajoutés un par un, rendus en jetons retirables. -->
    <div>
      <UiInput
        id="proposal-target_audiences"
        v-model="audienceInput"
        :label="t('proposal.form.step-presentation.fields.target_audience.label')"
        :hint="hintOf('target_audiences', t('proposal.form.step-presentation.fields.target_audience.hint'))"
        :error="audienceError"
        :maxlength="AUDIENCE_MAX"
        :placeholder="t('proposal.form.step-presentation.fields.target_audience.placeholder')"
        @keydown="onAudienceKey"
      >
        <template #suffix>
          <UiButton
            variant="ghost"
            size="sm"
            icon="plus"
            icon-only
            :disabled="audienceInput.trim().length === 0 || Boolean(audienceError)"
            :label="t('proposal.form.step-presentation.fields.target_audience.add')"
            @click="addAudience()"
          />
        </template>
      </UiInput>

      <ul v-if="draft.target_audiences.length > 0" class="mt-3 flex flex-wrap gap-2">
        <li v-for="audience in draft.target_audiences" :key="audience">
          <UiChip
            :label="audience"
            @remove="removeAudience(audience)"
          />
        </li>
      </ul>
    </div>
  </div>
</template>
