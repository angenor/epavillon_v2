<script setup lang="ts">
import type { ProposalHistoryEntry } from '~/types/programme/proposal'
import type { I18nText, TimeZoneName } from '~/types/shared'

/**
 * L'HISTORIQUE DU DOSSIER, CHAMP PAR CHAMP — « l'ancienne plateforme ne le
 * permettait pas ».
 *
 * IL N'EST PAS TENU À LA MAIN. C'est un sous-produit du journal d'audit :
 * `platform.tg_audit()` écrit chaque modification dans `platform.audit_log`, et
 * `platform.entity_history()` la dépile en une ligne par champ. La v1 avait une
 * table `activity_modifications` alimentée par le code applicatif — elle ne
 * couvrait que ce qui passait par le bon chemin, et rien de ce qui se corrigeait
 * en console.
 *
 * LES VALEURS SONT DES DOCUMENTS JSON, pas des chaînes. Un titre est un
 * `i18n_text`, une durée un nombre, un statut une chaîne, un public visé un
 * tableau. Ce composant les rend SELON LEUR FORME plutôt que de les
 * concaténer — un `[object Object]` dans un historique est une information
 * perdue, et personne ne va la rechercher en base.
 *
 * LE NOM DU CHAMP EST TRADUIT, JAMAIS AFFICHÉ BRUT. `detailed_presentation` est
 * un nom de colonne ; « Présentation détaillée » est ce que lit une
 * organisation. Un champ sans traduction retombe sur son nom technique plutôt
 * que de disparaître : une modification cachée serait pire qu'un mot barbare.
 */

interface Props {
  entries: ProposalHistoryEntry[]
  /** Fuseau d'affichage — celui de l'édition, comme partout ailleurs. */
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t, te } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

/** Un champ traduit, ou son nom technique à défaut. */
function fieldLabel(field: string): string {
  const key = `organization.workspace.proposal.history.field.${field}`
  return te(key) ? t(key) : field
}

/**
 * VALEURS QUI NE SE LISENT PAS TELLES QUELLES, et qui sont justement les plus
 * fréquentes du journal.
 *
 * `status` porte des valeurs d'ENUM — `under_review`, `changes_requested` — que
 * la base écrit et qu'aucune organisation n'a à déchiffrer ; les colonnes de
 * date portent des instants ISO, que personne ne lit. Les afficher brutes,
 * c'était livrer le journal d'audit à la place de l'historique.
 */
const STATUS_LABEL_PREFIX = 'organization.workspace.proposals.timeline.'
const FORMAT_LABEL_PREFIX = 'organization.workspace.proposal.tracking.formatValue.'

/** Champs dont la valeur est un instant, à formater dans le fuseau de l'édition. */
const DATE_FIELDS = new Set([
  'submitted_at',
  'decided_at',
  'preferred_start_at',
  'preferred_end_at',
])

/**
 * Rend la valeur d'un champ donné, en tenant compte de ce que ce champ EST.
 * Sans cette étape, l'écran affiche « accepted » et « 2027-11-09T14:00:00-03:00 ».
 */
function renderField(field: string | null, value: unknown): string {
  if (value === null || value === undefined || value === '') {
    return t('organization.workspace.proposal.history.emptyValue')
  }
  if (field === 'status' && typeof value === 'string') {
    const key = `${STATUS_LABEL_PREFIX}${value}`
    return te(key) ? t(key) : value
  }
  if (field === 'format' && typeof value === 'string') {
    const key = `${FORMAT_LABEL_PREFIX}${value}`
    return te(key) ? t(key) : value
  }
  if (field && DATE_FIELDS.has(field) && typeof value === 'string') {
    return dateTime(value, props.timezone) || value
  }
  return plainText(value)
}

/**
 * Rend une valeur d'audit lisible.
 *
 * L'ordre des cas suit leur fréquence dans le journal : textes multilingues
 * d'abord, chaînes ensuite, tableaux et nombres enfin.
 */
function renderValue(value: unknown): string {
  if (value === null || value === undefined || value === '') {
    return t('organization.workspace.proposal.history.emptyValue')
  }
  if (typeof value === 'string') return value
  if (typeof value === 'number') return String(value)
  if (typeof value === 'boolean') return String(value)
  if (Array.isArray(value)) {
    return value.map((item) => renderValue(item)).join(' · ')
  }
  if (typeof value === 'object') {
    // Un `i18n_text` se reconnaît à sa clé française, que la base exige.
    const candidate = value as Record<string, unknown>
    if (typeof candidate.fr === 'string') return tr(value as I18nText)
    return JSON.stringify(value)
  }
  return String(value)
}

/**
 * Le HTML de la présentation détaillée n'est PAS rendu ici, et c'est délibéré :
 * dans un historique, ce qui compte est le texte qui a changé, pas sa mise en
 * forme. Les balises sont donc retirées à l'affichage — sans quoi la comparaison
 * « avant / après » opposerait deux blocs de balisage.
 */
function plainText(value: unknown): string {
  return renderValue(value).replace(/<[^>]*>/g, ' ').replace(/\s+/g, ' ').trim()
}
</script>

<template>
  <section aria-labelledby="workspace-history-title">
    <h2 id="workspace-history-title" class="text-xl font-semibold">
      {{ t('organization.workspace.proposal.history.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('organization.workspace.proposal.history.description') }}
    </p>

    <UiEmptyState
      v-if="props.entries.length === 0"
      class="mt-6"
      icon="clock"
      :title="t('organization.workspace.proposal.history.empty.title')"
      :description="t('organization.workspace.proposal.history.empty.description')"
    />

    <ol v-else class="mt-6 flex flex-col divide-y divide-border-subtle">
      <li v-for="(entry, index) in props.entries" :key="`${entry.occurred_at}-${entry.field ?? 'insert'}-${index}`" class="py-4">
        <p class="flex flex-wrap items-baseline gap-x-2 gap-y-0.5">
          <span class="font-semibold text-text">
            {{
              entry.field === null
                ? t('organization.workspace.proposal.history.created')
                : t('organization.workspace.proposal.history.changed', { field: fieldLabel(entry.field) })
            }}
          </span>
          <time :datetime="entry.occurred_at" class="text-sm tabular-nums text-text-subtle">
            {{ dateTime(entry.occurred_at, props.timezone) }}
          </time>
          <span class="text-sm text-text-subtle">
            {{
              t('organization.workspace.proposal.history.by', {
                actor: entry.actor_label ?? t('organization.workspace.proposal.history.unknownActor'),
              })
            }}
          </span>
        </p>

        <!-- Avant / après, l'un sous l'autre plutôt que côte à côte : sur un
             téléphone, deux colonnes de texte long deviennent deux colonnes de
             deux mots. L'ancienne valeur est barrée — la comparaison se fait à
             l'œil, sans lire les étiquettes. -->
        <dl v-if="entry.field !== null" class="mt-2 flex flex-col gap-1.5 text-sm">
          <div class="flex flex-wrap gap-x-2">
            <dt class="shrink-0 text-text-subtle">{{ t('organization.workspace.proposal.history.before') }}</dt>
            <dd class="min-w-0 flex-1 text-text-muted line-through">
              {{ renderField(entry.field, entry.old_value) }}
            </dd>
          </div>
          <div class="flex flex-wrap gap-x-2">
            <dt class="shrink-0 text-text-subtle">{{ t('organization.workspace.proposal.history.after') }}</dt>
            <dd class="min-w-0 flex-1 text-text-secondary">
              {{ renderField(entry.field, entry.new_value) }}
            </dd>
          </div>
        </dl>
      </li>
    </ol>
  </section>
</template>
