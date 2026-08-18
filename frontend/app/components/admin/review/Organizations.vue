<script setup lang="ts">
import type { ProposalOrganizationEntry } from '~/types/admin-review'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES ORGANISATIONS DU DOSSIER, AVEC LEUR HISTORIQUE DE PARTICIPATION.
 *
 * POURQUOI L'HISTORIQUE FIGURE ICI, et pas dans une fiche qu'il faudrait ouvrir
 * dans un autre onglet : la question « cette organisation a-t-elle déjà tenu ce
 * qu'elle annonce ? » se pose EN NOTANT, pas après. Trois dossiers déposés dont
 * trois écartés, ou huit activités tenues sur quatre éditions, ne se lisent pas
 * de la même façon — et c'est précisément ce que
 * `analytics.mv_organization_scorecard` compte.
 *
 * LE CHIFFRE EST DONNÉ, PAS INTERPRÉTÉ. L'écran n'écrit ni « organisation
 * fiable » ni « à surveiller » : il montre les dépôts, les acceptations, les
 * éditions couvertes et la moyenne obtenue. Un jugement automatique à partir
 * d'un ratio ferait exactement ce que le comité est là pour faire.
 *
 * UNE CO-ORGANISATION NON CONFIRMÉE EST SIGNALÉE. `confirmed_at` nul veut dire
 * qu'un tiers a été annoncé sans avoir répondu : le comité doit le savoir avant
 * de compter cette organisation parmi les soutiens du dossier.
 */

interface Props {
  entries: ProposalOrganizationEntry[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

/** Le taux de sélection s'affiche en pour cent, jamais en fraction décimale. */
function ratioPercent(ratio: number): number {
  return Math.round(ratio * 100)
}
</script>

<template>
  <section aria-labelledby="review-organizations-title">
    <h3 id="review-organizations-title" class="text-sm font-semibold tracking-wide uppercase">
      {{ t('admin.proposal.review.organizations.title') }}
    </h3>

    <ul class="mt-3 flex flex-col gap-3">
      <li
        v-for="entry in props.entries"
        :key="entry.link.organization_id"
        class="rounded-md border border-border bg-surface px-4 py-3"
      >
        <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
          <span class="font-semibold text-text">
            {{ entry.organization?.legal_name ?? '' }}
          </span>
          <span v-if="entry.organization?.acronym" class="text-sm text-text-subtle">
            {{ entry.organization.acronym }}
          </span>
          <UiBadge
            size="sm"
            :intent="entry.link.role === 'lead' ? 'info' : 'neutral'"
            :label="t(`admin.proposal.review.organizations.role.${entry.link.role}`)"
          />
          <!-- ENGAGEMENT D'UN TIERS NON CONFIRMÉ : à dire, pas à taire. -->
          <UiBadge
            v-if="entry.link.role !== 'lead' && entry.link.confirmed_at === null"
            size="sm"
            intent="warning"
            icon="warning"
            :label="t('admin.proposal.review.organizations.pending')"
          />
        </div>

        <!-- L'HISTORIQUE, EN UNE LIGNE DENSE. Une organisation nouvelle le dit
             en toutes lettres : « aucun dépôt » et « première proposition » ne
             se lisent pas pareil, et le second est une information utile. -->
        <dl v-if="entry.track_record" class="mt-2 flex flex-wrap gap-x-5 gap-y-1 text-sm">
          <div class="flex gap-1.5">
            <dt class="text-text-subtle">{{ t('admin.proposal.review.organizations.record.title') }}</dt>
            <dd class="text-text-secondary">
              {{ t('admin.proposal.review.organizations.record.submitted', entry.track_record.propositions_deposees) }}
              <template v-if="entry.track_record.propositions_deposees > 0">
                ·
                {{ t('admin.proposal.review.organizations.record.editions', entry.track_record.evenements_couverts) }}
              </template>
            </dd>
          </div>

          <div v-if="entry.track_record.propositions_deposees > 0" class="flex gap-1.5">
            <dd class="text-success">
              {{ t('admin.proposal.review.organizations.record.accepted', { count: entry.track_record.propositions_acceptees }) }}
            </dd>
            <dd v-if="entry.track_record.propositions_rejetees > 0" class="text-text-muted">
              {{ t('admin.proposal.review.organizations.record.rejected', { count: entry.track_record.propositions_rejetees }) }}
            </dd>
          </div>

          <div v-if="entry.track_record.sessions_realisees > 0">
            <dd class="text-text-secondary">
              {{ t('admin.proposal.review.organizations.record.sessions', entry.track_record.sessions_realisees) }}
            </dd>
          </div>

          <div v-if="entry.track_record.ratio_acceptation !== null">
            <dd class="text-text-secondary">
              {{
                t('admin.proposal.review.organizations.record.ratio', {
                  ratio: ratioPercent(entry.track_record.ratio_acceptation),
                })
              }}
            </dd>
          </div>

          <div v-if="entry.track_record.note_moyenne_obtenue !== null">
            <dd class="text-text-secondary">
              {{
                t('admin.proposal.review.organizations.record.averageScore', {
                  score: entry.track_record.note_moyenne_obtenue,
                })
              }}
            </dd>
          </div>
        </dl>

        <p v-else class="mt-2 text-sm text-text-muted">
          {{ t('admin.proposal.review.organizations.record.firstTime') }}
        </p>

        <p
          v-if="entry.link.role !== 'lead' && entry.link.confirmed_at"
          class="mt-1 text-xs text-text-subtle"
        >
          {{
            t('admin.proposal.review.organizations.confirmedOn', {
              date: date(entry.link.confirmed_at, props.timezone),
            })
          }}
        </p>
      </li>
    </ul>
  </section>
</template>
