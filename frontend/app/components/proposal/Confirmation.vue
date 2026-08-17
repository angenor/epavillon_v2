<script setup lang="ts">
import type { EventEdition } from '~/types/event/edition'

/**
 * APRÈS L'ENVOI — le numéro de dossier, et la suite des opérations.
 *
 * LE NUMÉRO EST LE SEUL OBJET DE CET ÉCRAN. C'est lui qu'on cite dans un
 * courriel, au téléphone, dans une relance : `programme.proposals.reference_code`
 * existe pour cela et n'est jamais réutilisé. Il est donc rendu grand, en chasse
 * fixe, et COPIABLE — recopier « COP31-00041 » à la main depuis un écran est le
 * genre de geste où l'on intervertit deux chiffres.
 *
 * LA COPIE A UN RETOUR VISIBLE. Un bouton qui ne dit rien laisse cliquer trois
 * fois. On confirme, et l'on revient à l'état initial au bout de deux secondes.
 * Là où le presse-papiers n'est pas accessible — contexte non sécurisé, refus du
 * navigateur —, le numéro reste sélectionnable : il n'est pas dans un bouton.
 *
 * LA SUITE DES OPÉRATIONS EST ÉCRITE, pas sous-entendue : combien de membres du
 * comité liront le dossier, quand les résultats sont annoncés, ce qu'on peut
 * encore modifier. Une organisation qui ne sait pas ce qui l'attend écrit à
 * l'IFDD pour le demander — et c'est autant de courriels que la v1 recevait.
 */

interface Props {
  referenceCode: string
  submittedAt: string
  requiredReviews: number
  /** `calls_for_proposals.results_expected_at` — une DATE, jamais une heure. */
  resultsExpectedAt: string | null
  edition: EventEdition
  /** Destination de l'espace organisation (A5), quand il existera. */
  organizationSpaceTo: string | null
  /**
   * Le dossier a été RENVOYÉ après correction, non déposé pour la première fois.
   *
   * Le distinguer n'est pas un raffinement : « Votre dossier est déposé » est
   * faux pour un dossier qui l'était déjà depuis un mois, et ferait craindre un
   * second dossier — alors que le numéro, lui, est bien le même.
   */
  resubmitted?: boolean
  eventTo: string
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()

const zone = computed(() => props.edition.timezone)

const submittedLabel = computed(() => dateTime(props.submittedAt, zone.value))
const resultsLabel = computed(() =>
  props.resultsExpectedAt ? date(`${props.resultsExpectedAt}T12:00:00Z`, zone.value) : null,
)

const copied = ref(false)
let timer: ReturnType<typeof setTimeout> | null = null

async function copyReference(): Promise<void> {
  try {
    await navigator.clipboard.writeText(props.referenceCode)
    copied.value = true
    if (timer) clearTimeout(timer)
    timer = setTimeout(() => {
      copied.value = false
    }, 2_000)
  } catch {
    // Presse-papiers indisponible : le numéro reste sélectionnable à la souris,
    // ce qui est la raison pour laquelle il n'est pas rendu dans un bouton.
    copied.value = false
  }
}

onBeforeUnmount(() => {
  if (timer) clearTimeout(timer)
})
</script>

<template>
  <section class="mx-auto grid w-full max-w-180 gap-6">
    <header class="text-center">
      <span class="inline-flex size-14 items-center justify-center rounded-full bg-success-surface text-success">
        <UiIcon name="check-circle" size="2rem" :stroke-width="2" />
      </span>
      <h1 class="mt-4 font-display text-2xl leading-tight text-text sm:text-3xl">
        {{
          props.resubmitted
            ? t('proposal.form.confirmation.resubmittedTitle')
            : t('proposal.form.confirmation.title')
        }}
      </h1>
      <p class="mt-2 text-text-muted">
        {{
          props.resubmitted
            ? t('proposal.form.confirmation.resubmittedDescription')
            : t('proposal.form.confirmation.description', { edition: tr(props.edition.title) })
        }}
      </p>
    </header>

    <!-- LE NUMÉRO DE DOSSIER -->
    <div class="rounded-lg border-(length:--border-medium) border-accent bg-info-surface px-5 py-6 text-center">
      <p class="text-xs uppercase text-text-subtle" :style="{ letterSpacing: 'var(--tracking-caps)' }">
        {{ t('proposal.form.confirmation.reference') }}
      </p>
      <p class="mt-2 font-mono text-3xl font-bold tracking-wide break-all text-text select-all">
        {{ props.referenceCode }}
      </p>

      <UiButton
        class="mt-4"
        variant="secondary"
        :icon="copied ? 'check' : 'copy'"
        :label="copied ? t('proposal.form.confirmation.copied') : t('proposal.form.confirmation.copy')"
        @click="copyReference()"
      />
      <!-- La confirmation est ANNONCÉE, pas seulement dessinée. -->
      <p class="sr-only" role="status">
        {{ copied ? t('proposal.form.confirmation.copiedAnnouncement', { code: props.referenceCode }) : '' }}
      </p>

      <p class="mt-4 text-sm text-text-secondary">
        {{ t('proposal.form.confirmation.submittedAt', { datetime: submittedLabel }) }}
        <span class="block text-text-muted">
          {{ t('common.datetime.zoneOf', { zone: props.edition.city ?? props.edition.timezone }) }}
        </span>
      </p>
    </div>

    <!-- LA SUITE DES OPÉRATIONS -->
    <section class="rounded-lg border border-border px-5 py-5">
      <h2 class="font-display text-lg text-text">{{ t('proposal.form.confirmation.next.title') }}</h2>
      <ol class="mt-3 grid gap-4">
        <li class="flex items-start gap-3">
          <span class="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full bg-neutral-surface font-mono text-xs font-bold text-text-secondary">1</span>
          <span class="text-sm text-text-secondary">
            {{ t('proposal.form.confirmation.next.acknowledgement') }}
          </span>
        </li>
        <li class="flex items-start gap-3">
          <span class="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full bg-neutral-surface font-mono text-xs font-bold text-text-secondary">2</span>
          <span class="text-sm text-text-secondary">
            {{ t('proposal.form.confirmation.next.review', { count: props.requiredReviews }, props.requiredReviews) }}
          </span>
        </li>
        <li class="flex items-start gap-3">
          <span class="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full bg-neutral-surface font-mono text-xs font-bold text-text-secondary">3</span>
          <span class="text-sm text-text-secondary">
            <template v-if="resultsLabel">
              {{ t('proposal.form.confirmation.next.resultsOn', { date: resultsLabel }) }}
            </template>
            <template v-else>
              {{ t('proposal.form.confirmation.next.results') }}
            </template>
          </span>
        </li>
        <li class="flex items-start gap-3">
          <span class="mt-0.5 flex size-6 shrink-0 items-center justify-center rounded-full bg-neutral-surface font-mono text-xs font-bold text-text-secondary">4</span>
          <span class="text-sm text-text-secondary">
            {{ t('proposal.form.confirmation.next.changes') }}
          </span>
        </li>
      </ol>
    </section>

    <div class="flex flex-col gap-3 sm:flex-row">
      <UiButton
        v-if="props.organizationSpaceTo"
        variant="primary"
        size="lg"
        :to="props.organizationSpaceTo"
        :label="t('proposal.form.confirmation.actions.followUp')"
      />
      <UiButton
        variant="secondary"
        size="lg"
        :to="props.eventTo"
        :label="t('proposal.form.confirmation.actions.backToEvent')"
      />
    </div>
  </section>
</template>
