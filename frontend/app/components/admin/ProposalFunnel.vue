<script setup lang="ts">
import type { ProposalFunnelRow } from '~/types/analytics'

/**
 * L'ENTONNOIR DES PROPOSITIONS — `analytics.mv_proposal_funnel`.
 *
 * CINQ ÉTAGES, ET ILS SE DÉDUISENT LES UNS DES AUTRES : ouverts → déposés → en
 * évaluation → décidés → retenus. La largeur d'une barre est la part du TOTAL,
 * jamais de l'étage précédent : rapportée au précédent, chaque étage remplirait
 * la largeur et l'entonnoir ne s'amincirait jamais — ce qui est précisément ce
 * qu'un entonnoir doit montrer.
 *
 * UNE COULEUR POUR LA SÉRIE, DEUX POUR LES ISSUES. Les quatre premiers étages
 * partagent l'accent : c'est un même flux qui se rétrécit, pas quatre choses
 * différentes. Seules les issues prennent leur couleur d'état — vert pour ce qui
 * est retenu, rouge pour ce qui est écarté —, parce que ce sont des états, et
 * que la couleur distingue des états ; elle ne décore pas.
 *
 * LES SORTIES SONT DITES, PAS DEVINÉES. Brouillons jamais déposés, dossiers
 * retirés, dossiers écartés : ce sont eux qui expliquent l'écart entre deux
 * étages. Un entonnoir qui ne montre que ce qui avance laisse croire que le
 * reste s'est évaporé.
 *
 * DEUX TAUX, ET ILS NE DISENT PAS LA MÊME CHOSE — le modèle les sépare
 * délibérément : la sélectivité du comité se calcule sur les dossiers tranchés,
 * le rendement de l'appel sur tout ce qui a été déposé, retraits compris.
 */

interface Props {
  funnel: ProposalFunnelRow
}

const props = defineProps<Props>()

const { t, locale } = useI18n()

interface Stage {
  key: string
  count: number
  tone: 'accent' | 'success' | 'danger'
}

const stages = computed<Stage[]>(() => {
  const f = props.funnel
  return [
    { key: 'opened', count: f.total, tone: 'accent' },
    { key: 'submitted', count: f.deposees, tone: 'accent' },
    {
      key: 'inReview',
      count: f.en_attente_affectation + f.en_revue + f.modifications_demandees,
      tone: 'accent',
    },
    { key: 'decided', count: f.decidees, tone: 'accent' },
    { key: 'accepted', count: f.acceptees, tone: 'success' },
  ]
})

const BAR_TONES: Record<Stage['tone'], string> = {
  accent: 'bg-accent-solid',
  success: 'bg-success-solid',
  danger: 'bg-danger-solid',
}

/** Part du total, plancher visuel à 1 % pour qu'un étage non vide reste visible. */
function width(count: number): string {
  const total = Math.max(1, props.funnel.total)
  return `${Math.max(count === 0 ? 0 : 1, (count / total) * 100)}%`
}

/**
 * « 62 % » — les taux du modèle sont des RATIOS entre 0 et 1.
 *
 * Un taux nul n'est pas un zéro : `taux_acceptation` vaut `null` quand aucun
 * dossier n'a été tranché, et afficher « 0 % » ferait passer un comité qui n'a
 * pas commencé pour un comité qui a tout refusé.
 */
function percent(ratio: number | null): string {
  if (ratio === null) return t('common.labels.none')
  return t('common.formats.percent', {
    value: new Intl.NumberFormat(locale.value).format(Math.round(ratio * 100)),
  })
}

const exits = computed(() => [
  { key: 'drafts', count: props.funnel.brouillons },
  { key: 'withdrawn', count: props.funnel.retirees },
  { key: 'rejected', count: props.funnel.rejetees },
])
</script>

<template>
  <section aria-labelledby="admin-funnel-title">
    <h3 id="admin-funnel-title" class="mb-4 text-base font-semibold text-text">
      {{ t('admin.dashboard.funnel.title') }}
    </h3>

    <ul class="flex flex-col gap-2">
      <li v-for="stage in stages" :key="stage.key" class="grid grid-cols-[1fr_auto] items-center gap-x-3">
        <span class="col-span-2 flex items-baseline justify-between gap-2">
          <span class="text-sm text-text-secondary">
            {{ t(`admin.dashboard.funnel.stage.${stage.key}`) }}
          </span>
          <span class="font-mono text-sm font-bold tabular-nums text-text">{{ stage.count }}</span>
        </span>
        <span class="col-span-2 h-2.5 rounded-sm bg-surface-sunken">
          <span
            class="block h-full rounded-sm"
            :class="BAR_TONES[stage.tone]"
            :style="{ width: width(stage.count) }"
          />
        </span>
      </li>
    </ul>

    <!-- CE QUI SORT DE L'ENTONNOIR, à part : c'est l'explication des écarts. -->
    <dl class="mt-5 grid grid-cols-3 gap-3 border-t border-border-subtle pt-4">
      <div v-for="exit in exits" :key="exit.key">
        <dt class="text-xs text-text-subtle">{{ t(`admin.dashboard.funnel.exit.${exit.key}`) }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">{{ exit.count }}</dd>
      </div>
    </dl>

    <dl class="mt-4 grid grid-cols-2 gap-x-4 gap-y-3 border-t border-border-subtle pt-4 sm:grid-cols-3">
      <div>
        <dt class="text-xs text-text-subtle">{{ t('admin.dashboard.funnel.rates.selectivity') }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">
          {{ percent(props.funnel.taux_acceptation) }}
        </dd>
      </div>
      <div>
        <dt class="text-xs text-text-subtle">{{ t('admin.dashboard.funnel.rates.yield') }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">
          {{ percent(props.funnel.taux_acceptation_sur_depots) }}
        </dd>
      </div>
      <div>
        <dt class="text-xs text-text-subtle">{{ t('admin.dashboard.funnel.rates.organizations') }}</dt>
        <dd class="font-mono text-lg font-bold tabular-nums text-text">
          {{ props.funnel.organisations_distinctes }}
        </dd>
      </div>
    </dl>
  </section>
</template>
