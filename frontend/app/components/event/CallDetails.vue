<script setup lang="ts">
import type { CallForProposals } from '~/types/event/call'
import type { EventEdition } from '~/types/event/edition'

/**
 * LE DÉTAIL DE L'APPEL À PROPOSITIONS — ce qui reste de l'encart quand l'action
 * est montée dans le bandeau (19/08).
 *
 * ── CE QUI EST PARTI, CE QUI EST RESTÉ ──────────────────────────────────────
 *
 * Sont montés dans `EventHeroCall` : la pastille de phase, le rebours,
 * l'échéance et le bouton de dépôt. C'est le trio qui décide du clic, et il
 * demandait de défiler.
 *
 * Est resté ici tout ce qu'on lit une fois décidé — ce qu'on vient VÉRIFIER, et
 * non ce qu'on vient chercher : la description de l'appel, la prolongation
 * éventuelle, la date d'annonce des résultats, les conditions de dépôt, les
 * consignes.
 *
 * AUCUN BOUTON PRINCIPAL. Une page n'en porte qu'un, il est dans le bandeau, et
 * la barre d'ancres le rappelle une fois le bandeau passé. Un troisième
 * exemplaire ici ne ferait qu'user le signal.
 *
 * ── L'ÉTAT VIENT DU MODÈLE ──────────────────────────────────────────────────
 *
 * `utils/call.ts` reproduit `event.is_call_open()` et `event.effective_deadline()`,
 * prolongation comprise. L'échéance affichée est donc celle qui fait foi — et
 * quand l'appel a été prolongé, la date annoncée à l'origine reste lisible à
 * côté, ce que la v1 ne savait pas faire.
 *
 * ── L'ANCRE RESTE `appel-a-propositions` ────────────────────────────────────
 *
 * Le pied de page de chaque écran y renvoie. Elle ne bouge pas parce que le
 * contenu de la carte a changé.
 */

interface Props {
  /** `null` quand l'édition n'ouvre aucun appel — une COP sans pavillon, un
   *  cycle de webinaires. La carte n'est alors pas rendue du tout. */
  call: CallForProposals | null
  edition: EventEdition
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date, dateTime } = useDateTime()

const phase = computed<CallPhase | null>(() => (props.call ? callPhase(props.call) : null))
const deadline = computed(() => (props.call ? effectiveDeadline(props.call) : null))

const zone = computed(() => props.edition.timezone)

const deadlineLabel = computed(() => (deadline.value ? dateTime(deadline.value, zone.value) : ''))
const opensLabel = computed(() => (props.call ? dateTime(props.call.opens_at, zone.value) : ''))
const originalDeadlineLabel = computed(() =>
  props.call && wasExtended(props.call) ? dateTime(props.call.closes_at, zone.value) : '',
)
/**
 * `results_expected_at` est une DATE : on n'en affiche que le jour. Passer par
 * `dateTime()` produirait « 15 novembre 2026 à 09:00 » — une heure que personne
 * n'a saisie, née de la conversion vers le fuseau de l'édition. Midi UTC sert
 * seulement à ce que la conversion ne fasse pas changer de jour.
 */
const resultsLabel = computed(() =>
  props.call?.results_expected_at
    ? date(`${props.call.results_expected_at}T12:00:00Z`, zone.value)
    : '',
)
</script>

<template>
  <section
    v-if="props.call && phase"
    id="appel-a-propositions"
    class="scroll-mt-24 rounded-xl border border-border bg-surface-raised p-5 sm:p-6"
    aria-labelledby="appel-titre"
  >
    <h2 id="appel-titre" class="font-display text-xl leading-snug">
      {{ tr(props.call.title) }}
    </h2>

    <p v-if="props.call.description" class="mt-3 text-sm text-text-secondary">
      {{ tr(props.call.description) }}
    </p>

    <dl class="mt-5 space-y-4">
      <div>
        <dt
          class="text-xs uppercase text-text-subtle"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{
            t(
              phase === 'upcoming'
                ? 'event.public.call.opensAt'
                : phase === 'open'
                  ? 'event.public.call.deadline'
                  : 'event.public.call.closedAt',
            )
          }}
        </dt>
        <dd class="mt-1 text-sm text-text">
          {{ phase === 'upcoming' ? opensLabel : deadlineLabel }}
          <span class="block text-xs text-text-muted">
            {{ t('common.datetime.zoneOf', { zone: props.edition.city ?? props.edition.timezone }) }}
          </span>
          <!-- La prolongation ne remplace pas l'échéance annoncée : elle s'ajoute. -->
          <span v-if="originalDeadlineLabel" class="mt-1 block text-xs text-text-muted">
            {{ t('event.public.call.extendedFrom', { date: originalDeadlineLabel }) }}
          </span>
        </dd>
      </div>

      <div v-if="resultsLabel">
        <dt
          class="text-xs uppercase text-text-subtle"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ t('event.public.call.results') }}
        </dt>
        <dd class="mt-1 text-sm text-text">{{ resultsLabel }}</dd>
      </div>

      <div>
        <dt
          class="text-xs uppercase text-text-subtle"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          {{ t('event.public.call.rules') }}
        </dt>
        <dd class="mt-1 text-sm text-text-secondary">
          {{
            t('event.public.call.rulesValue', {
              max: props.call.max_proposals_per_organization ?? t('common.labels.none'),
              speakers: `${props.call.min_speakers}–${props.call.max_speakers}`,
              duration: props.call.default_duration_minutes,
            })
          }}
        </dd>
      </div>
    </dl>

    <a
      v-if="props.call.guidelines_url"
      :href="props.call.guidelines_url"
      target="_blank"
      rel="noopener noreferrer"
      class="mt-5 inline-flex items-center gap-1.5 text-sm text-accent"
    >
      {{ t('event.public.call.guidelines') }}
      <UiIcon name="external-link" size="0.9rem" />
      <span class="sr-only">{{ t('common.a11y.externalLink') }}</span>
    </a>

    <p v-if="phase === 'closed'" class="mt-5 text-sm text-text-muted">
      {{ t('event.public.call.closedNote') }}
    </p>
  </section>
</template>
