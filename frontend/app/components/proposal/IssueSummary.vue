<script setup lang="ts">
import type { DraftIssue, ProposalFormStep } from '~/types/proposal-form'

/**
 * LES DÉFAUTS DU DOSSIER, REGROUPÉS EN TÊTE.
 *
 * DEUX SIGNALEMENTS POUR UN MÊME DÉFAUT, et ce n'est pas une redondance : le
 * champ dit CE QUI ne va pas, la liste dit COMBIEN il en reste et où ils sont.
 * Un formulaire de sept étapes dont l'envoi échoue sans récapitulatif oblige à
 * rouvrir chaque étape pour chercher le champ rouge.
 *
 * CHAQUE LIGNE EST UN LIEN VERS L'ÉTAPE CONCERNÉE. C'est là toute son utilité :
 * la liste ne se contente pas d'énumérer, elle emmène. Le champ visé reçoit le
 * focus une fois l'étape ouverte — c'est l'écran qui s'en charge, ce composant
 * ne fait qu'émettre la destination.
 *
 * ERREURS ET AVERTISSEMENTS NE SE MÉLANGENT PAS. Les premières empêchent
 * l'envoi ; les seconds décrivent un dossier affaibli — pas de résumé, aucun
 * public visé, aucune thématique. Les rendre dans le même rouge apprendrait à
 * ignorer le rouge, et c'est l'erreur qui en pâtirait.
 *
 * `role="alert"` sur le bloc d'erreurs : il est annoncé dès qu'il apparaît, sans
 * attendre qu'un champ reprenne le focus.
 */

interface Props {
  issues: DraftIssue[]
  /** Titre des étapes, déjà traduit : la liste dit « Étape 4 — Intervenants ». */
  stepLabels: Record<ProposalFormStep, string>
  /** Les avertissements sont-ils affichés ? Faux tant qu'on n'a pas tenté d'envoyer. */
  showWarnings?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ goTo: [step: ProposalFormStep, field: string] }>()

const { t } = useI18n()

const errors = computed(() => props.issues.filter((issue) => issue.severity === 'error'))
const warnings = computed(() => props.issues.filter((issue) => issue.severity === 'warning'))
</script>

<template>
  <div v-if="errors.length > 0 || (props.showWarnings && warnings.length > 0)" class="grid gap-4">
    <section
      v-if="errors.length > 0"
      role="alert"
      class="rounded-md border-(length:--border-medium) border-danger-border bg-danger-surface px-4 py-4"
    >
      <h2 class="flex items-center gap-2 font-bold text-danger">
        <UiIcon name="error" size="1.1rem" />
        {{ t('validation.summary.title', { count: errors.length }, errors.length) }}
      </h2>
      <p class="mt-1 text-sm text-text-secondary">{{ t('validation.summary.description') }}</p>

      <ul class="mt-3 grid gap-2">
        <li v-for="(issue, index) in errors" :key="`${issue.field}-${index}`">
          <button
            type="button"
            class="cursor-pointer text-start text-sm text-text-link underline decoration-from-font underline-offset-2"
            @click="emit('goTo', issue.step, issue.field)"
          >
            <span class="font-bold">{{ props.stepLabels[issue.step] }}</span>
            <span aria-hidden="true"> — </span>
            <span>{{ t(issue.messageKey, issue.params ?? {}, Number(issue.params?.count ?? 1)) }}</span>
          </button>
        </li>
      </ul>
    </section>

    <section
      v-if="props.showWarnings && warnings.length > 0"
      class="rounded-md border border-warning-border bg-warning-surface px-4 py-4"
    >
      <h2 class="flex items-center gap-2 font-bold text-warning">
        <UiIcon name="warning" size="1.1rem" />
        {{ t('proposal.form.issues.warningsTitle', { count: warnings.length }, warnings.length) }}
      </h2>
      <p class="mt-1 text-sm text-text-secondary">
        {{ t('proposal.form.issues.warningsDescription') }}
      </p>

      <ul class="mt-3 grid gap-2">
        <li v-for="(issue, index) in warnings" :key="`${issue.field}-${index}`">
          <button
            type="button"
            class="cursor-pointer text-start text-sm text-text-link underline decoration-from-font underline-offset-2"
            @click="emit('goTo', issue.step, issue.field)"
          >
            <span class="font-bold">{{ props.stepLabels[issue.step] }}</span>
            <span aria-hidden="true"> — </span>
            <span>{{ t(issue.messageKey, issue.params ?? {}, Number(issue.params?.count ?? 1)) }}</span>
          </button>
        </li>
      </ul>
    </section>
  </div>
</template>
