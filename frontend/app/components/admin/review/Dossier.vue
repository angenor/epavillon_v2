<script setup lang="ts">
import type { Proposal } from '~/types/programme/proposal'
import type { ScheduleThemeBadge } from '~/types/views'
import type { TimeZoneName } from '~/types/shared'

/**
 * LE DOSSIER TEL QUE L'ORGANISATION L'A ÉCRIT — présentation, objectifs,
 * résultats attendus, publics visés, créneau souhaité.
 *
 * ON LE LIT POUR LE JUGER, d'où la densité assumée et les paragraphes en pleine
 * mesure : ces gens lisent des documents de négociation, un texte de six lignes
 * étalé sur toute la largeur d'un écran de 27 pouces ne leur rend pas service.
 * `max-w-(--measure)` borne la ligne à une longueur lisible, et rien d'autre
 * n'est tronqué : un dossier qu'on abrège est un dossier qu'on juge à moitié.
 *
 * LA PRÉSENTATION DÉTAILLÉE EST DU HTML RESTREINT, assaini par l'API à
 * l'écriture (`proposals.detailed_presentation`) : gras, italique, listes,
 * sous-titres, citations, liens — ni police, ni couleur. `UiRichContent` le rend
 * avec la typographie de la charte ; c'est le même composant que la page
 * publique, pour que le comité voie ce que le public verra.
 *
 * LE CRÉNEAU SOUHAITÉ PORTE SON FUSEAU, celui de l'ÉDITION. Une session
 * demandée « à 14 h » l'est à Belém, pas à Québec : c'est la règle de toute la
 * plateforme, et c'est ici qu'elle décide d'un conflit de programmation.
 *
 * LES PUBLICS VISÉS SONT UNE LISTE, pas une phrase. Le modèle les stocke un par
 * entrée (`target_audiences`, tableau de `i18n_text`) précisément pour qu'ils se
 * comptent, se filtrent et se réaffichent — la v1 en faisait une chaîne unique
 * que ses gabarits découpaient à la virgule.
 */

interface Props {
  proposal: Proposal
  themes: ScheduleThemeBadge[]
  timezone: TimeZoneName
  /** Nom de ville de l'édition, qui NOMME le fuseau (« heure de Belém »). */
  zoneLabel: string
  /** Nom du pays concerné, résolu depuis la base ; nul quand il n'y en a pas. */
  countryName: string | null
  submitterName: string | null
  contactName: string | null
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

const hasSlot = computed(
  () => props.proposal.preferred_start_at !== null && props.proposal.preferred_end_at !== null,
)
</script>

<template>
  <div class="flex flex-col gap-8">
    <!-- IDENTITÉ DU DOSSIER : ce qui se lit d'un coup d'œil avant le texte. -->
    <section aria-labelledby="dossier-identity">
      <h2 id="dossier-identity" class="sr-only">{{ t('admin.proposal.review.tabs.dossier') }}</h2>

      <UiThemeTagList v-if="props.themes.length > 0" :themes="props.themes" :max="6" />

      <dl class="mt-4 grid gap-x-8 gap-y-4 sm:grid-cols-2">
        <div>
          <dt class="text-xs tracking-wide text-text-subtle uppercase">
            {{ t('admin.proposal.review.dossier.slot') }}
          </dt>
          <dd class="mt-1">
            <UiZonedTime
              v-if="hasSlot"
              :start="props.proposal.preferred_start_at!"
              :end="props.proposal.preferred_end_at"
              :timezone="props.timezone"
              :zone-label="props.zoneLabel"
              format="withDate"
            />
            <span v-else class="text-text-muted">{{ t('admin.proposal.review.dossier.noSlot') }}</span>
            <p v-if="props.proposal.duration_minutes" class="text-sm text-text-subtle">
              {{ t('admin.proposal.review.dossier.duration', { count: props.proposal.duration_minutes }) }}
              <template v-if="props.proposal.requested_sessions > 1">
                ·
                {{
                  t('admin.proposal.review.dossier.sessionsRequested', props.proposal.requested_sessions)
                }}
              </template>
            </p>
          </dd>
        </div>

        <div>
          <dt class="text-xs tracking-wide text-text-subtle uppercase">
            {{ t('admin.proposal.review.dossier.languages') }}
          </dt>
          <dd class="mt-1 text-text-secondary">
            {{ props.proposal.language_codes.join(' · ').toUpperCase() }}
          </dd>
        </div>

        <div v-if="props.countryName">
          <dt class="text-xs tracking-wide text-text-subtle uppercase">
            {{ t('admin.proposal.review.dossier.country') }}
          </dt>
          <dd class="mt-1 text-text-secondary">{{ props.countryName }}</dd>
        </div>

        <div v-if="props.submitterName">
          <dt class="text-xs tracking-wide text-text-subtle uppercase">
            {{ t('admin.proposal.review.dossier.submitter') }}
          </dt>
          <dd class="mt-1 text-text-secondary">
            {{ props.submitterName }}
            <span v-if="props.contactName && props.contactName !== props.submitterName" class="block text-sm text-text-subtle">
              {{ t('admin.proposal.review.dossier.contact', { name: props.contactName }) }}
            </span>
          </dd>
        </div>
      </dl>

      <!-- LES CONTRAINTES DE PROGRAMMATION SONT UN AVERTISSEMENT, pas un
           paragraphe de plus : elles décideront du créneau retenu, et le
           planificateur (A9) les relira. -->
      <UiAlert
        v-if="props.proposal.scheduling_constraints"
        class="mt-4"
        intent="warning"
        icon="calendar"
        :title="t('admin.proposal.review.dossier.constraints')"
        :message="props.proposal.scheduling_constraints"
      />
    </section>

    <section v-if="props.proposal.summary" aria-labelledby="dossier-summary">
      <h3 id="dossier-summary" class="text-sm font-semibold tracking-wide uppercase">
        {{ t('admin.proposal.review.dossier.summary') }}
      </h3>
      <p class="mt-2 max-w-(--measure) text-lg leading-relaxed text-text-secondary">
        {{ tr(props.proposal.summary) }}
      </p>
    </section>

    <section aria-labelledby="dossier-objectives">
      <h3 id="dossier-objectives" class="text-sm font-semibold tracking-wide uppercase">
        {{ t('admin.proposal.review.dossier.objectives') }}
      </h3>
      <p class="mt-2 max-w-(--measure) leading-relaxed text-text-secondary">
        {{ tr(props.proposal.objectives) }}
      </p>
    </section>

    <section aria-labelledby="dossier-presentation">
      <h3 id="dossier-presentation" class="text-sm font-semibold tracking-wide uppercase">
        {{ t('admin.proposal.review.dossier.presentation') }}
      </h3>
      <UiRichContent class="mt-2 max-w-(--measure)" :html="tr(props.proposal.detailed_presentation)" />
    </section>

    <section v-if="props.proposal.expected_outcomes" aria-labelledby="dossier-outcomes">
      <h3 id="dossier-outcomes" class="text-sm font-semibold tracking-wide uppercase">
        {{ t('admin.proposal.review.dossier.outcomes') }}
      </h3>
      <p class="mt-2 max-w-(--measure) leading-relaxed text-text-secondary">
        {{ tr(props.proposal.expected_outcomes) }}
      </p>
    </section>

    <section v-if="props.proposal.target_audiences.length > 0" aria-labelledby="dossier-audiences">
      <h3 id="dossier-audiences" class="text-sm font-semibold tracking-wide uppercase">
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
