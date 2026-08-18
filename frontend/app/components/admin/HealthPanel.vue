<script setup lang="ts">
import type { HealthSeverity, OperationalHealthRow } from '~/types/analytics'

/**
 * SANTÉ OPÉRATIONNELLE — `analytics.v_operational_health`.
 *
 * CE QUI CASSE EN SILENCE. Si l'outbox n'est plus relayé, les confirmations
 * d'inscription ne partent plus — et personne ne s'en aperçoit avant les
 * réclamations des participants. C'est la zone qu'on regarde le matin, pas celle
 * qu'on consulte pour décider.
 *
 * LES SEUILS VIENNENT DE LA BASE, ET LA GRAVITÉ AUSSI. Chaque ligne porte sa
 * valeur ET ses deux seuils : la règle d'alerte vit à côté de la mesure. Ce
 * composant ne recalcule RIEN — il affiche `gravite` telle qu'elle est rendue.
 * Un seuil dupliqué dans un composant dérive au premier réglage.
 *
 * TROIS NIVEAUX, TROIS RENDUS, ET LE PLUS GRAVE EN TÊTE. `critique` en rouge —
 * c'est un échec, et le rouge est fait pour ça. `attention` en jaune — quelque
 * chose demande un regard. `ok` en gris : ce qui est clos, rangé, replié. Les
 * lignes au vert sont NOMBREUSES et ne doivent pas occuper l'écran : elles se
 * comptent en une phrase et se déplient à la demande.
 *
 * UN INDICATEUR N'EST PAS UN COMPTE. `analytique_perimee` porte un ÂGE EN
 * MINUTES, pas un nombre d'objets — la vue le documente. C'est pourquoi la
 * valeur est affichée nue, sans unité inventée par l'écran : le libellé de
 * l'indicateur, qui vient de la base, dit déjà ce qu'elle mesure.
 */

interface Props {
  rows: OperationalHealthRow[]
}

const props = defineProps<Props>()

const { t, te } = useI18n()

/**
 * LE LIBELLÉ D'UN INDICATEUR EST UN TEXTE D'INTERFACE, PAS UNE DONNÉE.
 *
 * La vue le rend en français (`v_operational_health.libelle`) — commode en
 * console, mais aucun administrateur ne peut le modifier depuis le back-office :
 * il est écrit dans le SQL. C'est donc bien une traduction, et la règle du projet
 * tranche dans ce sens. On traduit par le CODE, qui est stable, et on retombe sur
 * le libellé de la vue pour un indicateur ajouté en base avant de l'être ici —
 * un texte français vaut mieux qu'une clé nue.
 */
function labelOf(row: OperationalHealthRow): string {
  const key = `admin.dashboard.health.indicator.${row.code}`
  return te(key) ? t(key) : row.libelle
}

const TONES: Record<HealthSeverity, { badge: string; border: string; icon: string }> = {
  critique: { badge: 'bg-danger-surface text-danger', border: 'border-l-danger', icon: 'error' },
  attention: { badge: 'bg-warning-surface text-warning', border: 'border-l-warning', icon: 'warning' },
  ok: { badge: 'bg-neutral-surface text-neutral', border: 'border-l-border', icon: 'check' },
}

const alerting = computed(() => props.rows.filter((row) => row.gravite !== 'ok'))
const healthy = computed(() => props.rows.filter((row) => row.gravite === 'ok'))

const showHealthy = ref(false)

/**
 * Le détail utile, réduit à une phrase. La vue rend un document JSON libre —
 * plus ancien élément, dernière erreur, ventilation par tâche — dont l'écran ne
 * garde que ce qui aide à décider s'il faut ouvrir un terminal.
 */
function detailOf(row: OperationalHealthRow): string | null {
  const detail = row.detail as Record<string, unknown>
  const erreur = detail.derniere_erreur
  if (typeof erreur === 'string' && erreur.length > 0) return erreur
  const taches = detail.taches
  if (taches && typeof taches === 'object' && Object.keys(taches).length > 0) {
    return Object.entries(taches as Record<string, number>)
      .map(([tache, n]) => `${tache} (${n})`)
      .join(', ')
  }
  return null
}
</script>

<template>
  <section aria-labelledby="admin-health-title">
    <div class="mb-4 flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
      <h2 id="admin-health-title" class="text-xl font-semibold">
        {{ t('admin.dashboard.health.title') }}
      </h2>
      <p class="text-sm text-text-subtle">{{ t('admin.dashboard.health.subtitle') }}</p>
    </div>

    <p
      v-if="alerting.length === 0"
      class="rounded-md border border-border bg-surface-raised px-4 py-3 text-sm text-text-secondary"
    >
      <UiIcon name="check-circle" class="mr-2 inline text-success" size="1.125rem" />
      {{ t('admin.dashboard.health.allClear', props.rows.length) }}
    </p>

    <ul v-else class="flex flex-col gap-2">
      <li
        v-for="row in alerting"
        :key="row.code"
        class="flex flex-wrap items-baseline gap-x-3 gap-y-1 rounded-md border border-border border-l-(length:--border-thick) bg-surface-raised px-4 py-3"
        :class="TONES[row.gravite].border"
      >
        <span
          class="inline-flex shrink-0 items-center gap-1.5 rounded-sm px-2 py-0.5 text-xs font-bold uppercase"
          :class="TONES[row.gravite].badge"
          :style="{ letterSpacing: 'var(--tracking-caps)' }"
        >
          <UiIcon :name="TONES[row.gravite].icon" size="0.875rem" :stroke-width="2" />
          {{ t(`admin.dashboard.health.severity.${row.gravite}`) }}
        </span>

        <span class="min-w-0 flex-1">
          <span class="block font-semibold text-text">{{ labelOf(row) }}</span>
          <span v-if="detailOf(row)" class="mt-0.5 block truncate text-sm text-text-muted">
            {{ detailOf(row) }}
          </span>
        </span>

        <span class="shrink-0 text-right">
          <span class="block font-mono text-lg font-bold tabular-nums text-text">{{ row.valeur }}</span>
          <!-- LES SEUILS SONT AFFICHÉS, et c'est ce qui rend la gravité
               vérifiable : « 34, seuil 20 » se discute ; « rouge » ne se discute
               pas. -->
          <span class="block text-xs text-text-subtle">
            {{ t('admin.dashboard.health.threshold', { value: row.seuil_attention }) }}
          </span>
        </span>
      </li>
    </ul>

    <!-- Ce qui va bien se compte, et se déplie à la demande. -->
    <div v-if="healthy.length > 0" class="mt-3">
      <UiButton variant="ghost" size="sm" @click="showHealthy = !showHealthy">
        {{
          showHealthy
            ? t('admin.dashboard.health.hideHealthy')
            : t('admin.dashboard.health.showHealthy', healthy.length)
        }}
      </UiButton>

      <ul v-if="showHealthy" class="mt-2 flex flex-col divide-y divide-border-subtle rounded-md border border-border-subtle bg-surface-raised">
        <li
          v-for="row in healthy"
          :key="row.code"
          class="flex items-baseline justify-between gap-3 px-4 py-2"
        >
          <span class="min-w-0 truncate text-sm text-text-secondary">{{ labelOf(row) }}</span>
          <span class="shrink-0 font-mono text-sm tabular-nums text-text-muted">
            {{ row.valeur }}
            <span class="text-text-subtle">
              / {{ row.seuil_attention }}
            </span>
          </span>
        </li>
      </ul>
    </div>
  </section>
</template>
