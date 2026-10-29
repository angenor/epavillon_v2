<script setup lang="ts">
import type { Proposal } from '~/types/programme/proposal'

/**
 * LE DOSSIER TEL QUE L'ORGANISATION L'A ÉCRIT : objectifs, présentation,
 * résultats attendus, publics visés. Les faits (créneau, langues, pays,
 * contraintes) sont à côté, dans `DossierFacts`.
 *
 * ON LE LIT POUR LE JUGER, d'où la densité assumée et les paragraphes bornés à
 * `--measure` : rien n'est tronqué, un dossier qu'on abrège est un dossier qu'on
 * juge à moitié.
 *
 * LA PRÉSENTATION DÉTAILLÉE EST DU HTML RESTREINT, assaini par l'API à
 * l'écriture. `UiRichContent` le rend comme la page publique, pour que le comité
 * voie ce que le public verra.
 *
 * LES PUBLICS VISÉS SONT UNE LISTE, pas une phrase (`target_audiences`, tableau
 * de `i18n_text`) : la v1 en faisait une chaîne découpée à la virgule.
 */

interface Props {
  proposal: Proposal
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const HEADING = 'text-sm font-semibold tracking-caps text-text-subtle uppercase'
</script>

<template>
  <div class="flex flex-col gap-8">
    <section aria-labelledby="dossier-objectives">
      <h3 id="dossier-objectives" :class="HEADING">
        {{ t('admin.proposal.review.dossier.objectives') }}
      </h3>
      <p class="mt-2 max-w-(--measure) text-lg leading-relaxed">
        {{ tr(props.proposal.objectives) }}
      </p>
    </section>

    <section aria-labelledby="dossier-presentation">
      <h3 id="dossier-presentation" :class="HEADING">
        {{ t('admin.proposal.review.dossier.presentation') }}
      </h3>
      <UiRichContent class="mt-2 max-w-(--measure)" :html="tr(props.proposal.detailed_presentation)" />
    </section>

    <section v-if="props.proposal.expected_outcomes" aria-labelledby="dossier-outcomes">
      <h3 id="dossier-outcomes" :class="HEADING">
        {{ t('admin.proposal.review.dossier.outcomes') }}
      </h3>
      <p class="mt-2 max-w-(--measure) leading-relaxed text-text-secondary">
        {{ tr(props.proposal.expected_outcomes) }}
      </p>
    </section>

    <section v-if="props.proposal.target_audiences.length > 0" aria-labelledby="dossier-audiences">
      <h3 id="dossier-audiences" :class="HEADING">
        {{ t('admin.proposal.review.dossier.audiences') }}
      </h3>
      <ul class="mt-2 flex flex-wrap gap-2">
        <li v-for="(audience, index) in props.proposal.target_audiences" :key="index">
          <UiBadge :label="tr(audience)" icon="users" />
        </li>
      </ul>
    </section>
  </div>
</template>
