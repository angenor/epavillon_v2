<script setup lang="ts">
import type { MergePreview, MergeTransferLine } from '~/types/admin-organizations'

/**
 * LE DÉCOMPTE DE CE QUI SERA TRANSFÉRÉ.
 *
 * « Décompte de ce qui sera transféré : membres, propositions, co-organisations,
 * sessions, dénominations » — la demande du prompt, tenue par le REGISTRE plutôt
 * que par une liste écrite ici. Chaque ligne est une entrée de
 * `org.organization_references`, et le jour où un module de plus s'y déclare,
 * elle apparaît sans qu'on touche à ce composant. Seul son LIBELLÉ est à
 * traduire : « programme.proposal_organizations » ne se montre pas à un chargé
 * de programme.
 *
 * TROIS CHIFFRES, PAS UN. Transférées, supprimées-car-déjà-présentes,
 * supprimées. La colonne du milieu est celle qui surprend, et c'est justement
 * pour cela qu'elle est nommée : une personne adhérente des DEUX fiches ne compte
 * qu'une fois après la fusion — sa seconde adhésion disparaît, et rien n'est
 * perdu puisqu'elle est déjà membre de l'autre côté.
 *
 * LES LIGNES À ZÉRO NE S'AFFICHENT PAS. Dix-huit entrées de registre dont quatre
 * portent quelque chose : montrer les quatorze autres à zéro noierait le décompte
 * réel. La ligne de total, elle, porte toujours sur l'ensemble.
 */

interface Props {
  preview: MergePreview
}

const props = defineProps<Props>()

const { t } = useI18n()

const lines = computed(() => significantTransfers(props.preview.transfers))
const totals = computed(() => transferTotals(props.preview.transfers))

/**
 * Libellé traduit d'une table du registre ; à défaut, sa clé technique.
 *
 * La clé i18n est `schéma.table` et NON la clé complète du registre, qui porte
 * aussi la colonne : les points d'une clé de traduction sont des séparateurs de
 * niveau pour vue-i18n, et `org.organization_names.organization_id` y devient un
 * chemin à trois étages. Aucune table du registre ne déclare deux colonnes, la
 * paire suffit donc à désigner l'entrée.
 */
function tableLabel(line: MergeTransferLine): string {
  const key = `admin.organization.merge.transfer.tables.${line.ref_schema}.${line.ref_table}`
  const label = t(key)
  return label === key ? `${line.ref_schema}.${line.ref_table}` : label
}
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised">
    <header class="border-b border-border px-4 py-3">
      <h2 class="text-lg font-semibold text-text">
        {{ t('admin.organization.merge.transfer.title') }}
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.organization.merge.transfer.description') }}
      </p>
    </header>

    <p v-if="lines.length === 0" class="px-4 py-6 text-sm text-text-muted">
      {{ t('admin.organization.merge.transfer.none') }}
    </p>

    <ul v-else class="divide-y divide-border">
      <li
        v-for="line in lines"
        :key="`${line.ref_schema}.${line.ref_table}.${line.ref_column}`"
        class="flex flex-wrap items-baseline justify-between gap-x-6 gap-y-1 px-4 py-2.5"
      >
        <span class="text-sm text-text">{{ tableLabel(line) }}</span>

        <span class="flex flex-wrap items-baseline gap-x-4 text-sm">
          <span v-if="line.reassigned > 0" class="text-text">
            <span class="font-mono tabular-nums">{{ line.reassigned }}</span>
            <span class="ml-1 text-xs text-text-muted">
              {{ t('admin.organization.merge.transfer.reassigned') }}
            </span>
          </span>

          <!-- Le chiffre qui surprend, et qu'il faut donc expliquer. -->
          <span
            v-if="line.deduped > 0"
            class="text-warning"
            :title="t('admin.organization.merge.transfer.dedupedHint')"
          >
            <span class="font-mono tabular-nums">{{ line.deduped }}</span>
            <span class="ml-1 text-xs">
              {{ t('admin.organization.merge.transfer.deduped') }}
            </span>
          </span>

          <span
            v-if="line.deleted > 0"
            class="text-danger"
            :title="t('admin.organization.merge.transfer.deletedHint')"
          >
            <span class="font-mono tabular-nums">{{ line.deleted }}</span>
            <span class="ml-1 text-xs">
              {{ t('admin.organization.merge.transfer.deleted') }}
            </span>
          </span>
        </span>
      </li>
    </ul>

    <p class="border-t border-border px-4 py-2.5 text-sm font-medium text-text">
      {{ t('admin.organization.merge.transfer.total', totals.touched) }}
    </p>

    <!-- LES DÉNOMINATIONS SONT TRAITÉES HORS REGISTRE par la fonction de fusion
         (§ 6, étape 1) : elles deviennent des variantes de la fiche conservée,
         et c'est ce qui fait qu'une recherche sur l'ancien nom trouve encore la
         bonne fiche. Elles méritent donc d'être NOMMÉES, pas comptées. -->
    <section
      v-if="props.preview.transferred_names.length > 0"
      class="border-t border-border px-4 py-3"
    >
      <h3 class="text-sm font-semibold text-text">
        {{ t('admin.organization.merge.transfer.names.title') }}
      </h3>
      <p class="mt-1 max-w-(--measure) text-xs text-text-muted">
        {{ t('admin.organization.merge.transfer.names.description') }}
      </p>
      <ul class="mt-2 flex flex-wrap gap-1.5">
        <li v-for="entry in props.preview.transferred_names" :key="`${entry.kind}-${entry.name}`">
          <UiBadge
            size="sm"
            :intent="entry.already_present ? 'neutral' : 'info'"
            :label="entry.name"
            :title="
              entry.already_present
                ? t('admin.organization.merge.transfer.names.alreadyPresent')
                : t('admin.organization.detail.names.kind.' + entry.kind)
            "
          />
        </li>
      </ul>
    </section>

    <section
      v-if="props.preview.transferred_domains.length > 0"
      class="border-t border-border px-4 py-3"
    >
      <h3 class="text-sm font-semibold text-text">
        {{ t('admin.organization.merge.transfer.domains.title') }}
      </h3>
      <ul class="mt-2 flex flex-col gap-1">
        <li
          v-for="entry in props.preview.transferred_domains"
          :key="entry.domain"
          class="flex flex-wrap items-baseline gap-x-2 text-sm"
        >
          <span class="font-mono text-text">{{ entry.domain }}</span>
          <span :class="entry.verified_at ? 'text-xs text-success' : 'text-xs text-text-subtle'">
            {{
              entry.verified_at
                ? t('admin.organization.merge.transfer.domains.verified')
                : t('admin.organization.merge.transfer.domains.unverified')
            }}
          </span>
          <span v-if="entry.already_present" class="text-xs text-text-subtle">
            {{ t('admin.organization.merge.transfer.domains.alreadyPresent') }}
          </span>
        </li>
      </ul>
    </section>

    <!-- CE QUE LA FUSION PRÉSERVE — le rappel demandé par le prompt. Il n'est
         pas décoratif : sans lui, « fusionner » se lit comme « supprimer », et
         personne n'ose. -->
    <section class="border-t border-border bg-info-surface px-4 py-3">
      <h3 class="text-sm font-semibold text-info">
        {{ t('admin.organization.merge.reversible.title') }}
      </h3>
      <ul class="mt-1.5 flex flex-col gap-1 text-sm text-text">
        <li class="flex items-start gap-2">
          <UiIcon name="check" size="1rem" class="mt-0.5 shrink-0 text-info" />
          <span>{{ t('admin.organization.merge.reversible.record') }}</span>
        </li>
        <li class="flex items-start gap-2">
          <UiIcon name="check" size="1rem" class="mt-0.5 shrink-0 text-info" />
          <span>{{ t('admin.organization.merge.reversible.urls') }}</span>
        </li>
        <li class="flex items-start gap-2">
          <UiIcon name="check" size="1rem" class="mt-0.5 shrink-0 text-info" />
          <span>{{ t('admin.organization.merge.reversible.trace') }}</span>
        </li>
      </ul>
    </section>
  </section>
</template>
