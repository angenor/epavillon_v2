<script setup lang="ts">
import type {
  DuplicateSide,
  MergeField,
  MergeFieldComparison,
  MergeSideKey,
} from '~/types/admin-organizations'
import type { I18nText } from '~/types/shared'

/**
 * LA VUE COMPARÉE, CHAMP PAR CHAMP.
 *
 * QUATRE ÉTATS PAR LIGNE, ET ILS NE DEMANDENT PAS LE MÊME GESTE :
 *   · identiques — rien à décider. La ligne reste, discrète : voir ce qui
 *     concorde rassure autant que voir ce qui diverge ;
 *   · renseigné d'un seul côté — la valeur existante est retenue d'office, et
 *     l'écran le dit. Compléter la fiche conservée avec ce que l'autre apporte
 *     est tout l'intérêt de la manœuvre ;
 *   · les deux renseignés et différents — ARBITRAGE. Deux boutons radio, aucun
 *     présélectionné, et le décompte des champs restants empêche de valider à
 *     l'aveugle ;
 *   · NON ARBITRABLE — l'adresse d'URL. Elle se compare, elle ne se déplace pas :
 *     la fiche absorbée garde la sienne pour toujours, et c'est ce qui fait que
 *     ses anciens liens continuent de fonctionner. Aucun bouton, une mention.
 *
 * LA COLONNE DE GAUCHE EST TOUJOURS LA FICHE CONSERVÉE. Elle ne change pas de
 * place quand on inverse le sens de la fusion : c'est le CONTENU des colonnes
 * qui bascule. Une colonne qui se déplacerait sous les yeux de l'opérateur au
 * moment où il inverse est le meilleur moyen de lui faire cocher la mauvaise
 * valeur.
 *
 * LES VALEURS SONT RENDUES SELON LEUR FORME, jamais concaténées : un pays et un
 * type d'organisation s'affichent par leur libellé résolu, une description est un
 * `i18n_text` dont on montre le français, une adresse reste une adresse.
 */

interface Props {
  comparisons: MergeFieldComparison[]
  /** Champs que l'API compare mais refuse de déplacer — `ORG_MERGE_FIELD_NOT_ARBITRABLE`. */
  nonArbitrable: MergeField[]
  source: DuplicateSide
  target: DuplicateSide
  choices: Partial<Record<MergeField, MergeSideKey>>
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ choose: [field: MergeField, side: MergeSideKey] }>()

const { t } = useI18n()
const { tr } = useI18nText()

/** Valeur affichable d'un champ : son libellé résolu quand il en a un. */
function display(value: unknown, label: I18nText | string | null): string | null {
  if (label !== null) return typeof label === 'string' ? label : tr(label)
  if (value === null || value === undefined) return null
  if (typeof value === 'string') return value.trim().length > 0 ? value : null
  if (typeof value === 'object') {
    const text = tr(value as I18nText)
    return text.trim().length > 0 ? text : null
  }
  return String(value)
}

function sideValue(comparison: MergeFieldComparison, side: MergeSideKey): string | null {
  return side === 'source'
    ? display(comparison.source_value, comparison.source_label)
    : display(comparison.target_value, comparison.target_label)
}

/** Le champ se compare-t-il sans pouvoir se déplacer ? */
function isFixed(comparison: MergeFieldComparison): boolean {
  return props.nonArbitrable.includes(comparison.field)
}

const unresolved = computed(() =>
  unresolvedFields(
    props.comparisons.filter((comparison) => !isFixed(comparison)),
    props.choices,
  ),
)

/** Le champ est-il à trancher par un humain ? */
function isContested(comparison: MergeFieldComparison): boolean {
  return !isFixed(comparison) && comparison.differs && comparison.filled === 'both'
}

/** Côté retenu pour ce champ — la cible tant que rien n'est décidé. */
function chosen(comparison: MergeFieldComparison): MergeSideKey | null {
  if (!comparison.differs) return null
  // Un champ non arbitrable reste celui de la fiche conservée, sans arbitrage.
  if (isFixed(comparison)) return 'target'
  if (comparison.filled === 'source') return 'source'
  if (comparison.filled === 'target') return 'target'
  return props.choices[comparison.field] ?? null
}

function rowName(field: MergeField): string {
  return `merge-choice-${field}`
}
</script>

<template>
  <section class="rounded-lg border border-border bg-surface-raised">
    <header class="border-b border-border px-4 py-3">
      <h2 class="text-lg font-semibold text-text">
        {{ t('admin.organization.merge.compare.title') }}
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('admin.organization.merge.compare.description') }}
      </p>

      <p
        class="mt-2 text-sm font-medium"
        :class="unresolved.length > 0 ? 'text-warning' : 'text-success'"
        aria-live="polite"
      >
        {{
          unresolved.length > 0
            ? t('admin.organization.merge.compare.unresolved', unresolved.length)
            : t('admin.organization.merge.compare.resolved')
        }}
      </p>
    </header>

    <!-- En-tête des deux colonnes. La fiche CONSERVÉE est toujours à gauche ;
         inverser le sens échange les contenus, pas les colonnes. -->
    <div
      class="sticky top-0 z-10 hidden grid-cols-[10rem_1fr_1fr] gap-4 border-b border-border bg-surface-sunken px-4 py-2 sm:grid"
    >
      <span class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
        {{ t('admin.organization.merge.compare.fields.legal_name') }}
      </span>
      <span class="min-w-0 truncate text-sm font-semibold text-success">
        {{ props.target.legal_name }}
        <span class="block text-xs font-normal text-text-subtle">
          {{ t('admin.organization.merge.direction.absorbing') }}
        </span>
      </span>
      <span class="min-w-0 truncate text-sm font-semibold text-text-muted">
        {{ props.source.legal_name }}
        <span class="block text-xs font-normal text-text-subtle">
          {{ t('admin.organization.merge.direction.absorbed') }}
        </span>
      </span>
    </div>

    <ul class="divide-y divide-border">
      <li
        v-for="comparison in props.comparisons"
        :key="comparison.field"
        class="px-4 py-3"
        :class="isContested(comparison) ? 'bg-warning-surface/40' : ''"
      >
        <div class="grid gap-2 sm:grid-cols-[10rem_1fr_1fr] sm:gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium text-text">
              {{ t('admin.organization.merge.compare.fields.' + comparison.field) }}
            </p>
            <p v-if="isFixed(comparison)" class="text-xs text-text-subtle">
              {{ t('admin.organization.merge.reversible.urls') }}
            </p>
            <p v-else-if="isContested(comparison)" class="text-xs font-semibold text-warning">
              {{ t('admin.organization.merge.compare.contested') }}
            </p>
            <p v-else-if="!comparison.differs" class="text-xs text-text-subtle">
              {{ t('admin.organization.merge.compare.identical') }}
            </p>
            <p v-else class="text-xs text-text-subtle">
              {{ t('admin.organization.merge.compare.onlyOn') }}
            </p>
          </div>

          <!-- Les deux valeurs, cible puis source. Un champ à trancher porte un
               bouton radio ; un champ décidé porte la mention « valeur retenue ». -->
          <div
            v-for="side in (['target', 'source'] as MergeSideKey[])"
            :key="side"
            class="min-w-0"
          >
            <label
              v-if="isContested(comparison)"
              class="flex min-h-(--target-min) cursor-pointer items-start gap-2 rounded-md border p-2 transition-colors"
              :class="
                props.choices[comparison.field] === side
                  ? 'border-accent bg-accent-surface'
                  : 'border-border hover:bg-surface-hover'
              "
            >
              <input
                type="radio"
                class="mt-1 size-4 accent-[var(--color-accent)]"
                :name="rowName(comparison.field)"
                :value="side"
                :checked="props.choices[comparison.field] === side"
                :disabled="props.disabled"
                @change="emit('choose', comparison.field, side)"
              >
              <span class="min-w-0 text-sm break-words text-text">
                {{ sideValue(comparison, side) ?? t('admin.organization.merge.compare.empty') }}
              </span>
            </label>

            <div v-else class="p-2">
              <p
                class="text-sm break-words"
                :class="sideValue(comparison, side) ? 'text-text' : 'text-text-subtle'"
              >
                {{ sideValue(comparison, side) ?? t('admin.organization.merge.compare.empty') }}
              </p>
              <p v-if="comparison.differs && chosen(comparison) === side" class="mt-1 text-xs text-success">
                {{ t('admin.organization.merge.compare.kept') }}
              </p>
            </div>
          </div>
        </div>
      </li>
    </ul>
  </section>
</template>
