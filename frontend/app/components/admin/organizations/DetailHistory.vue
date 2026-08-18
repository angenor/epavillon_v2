<script setup lang="ts">
import type { OrganizationHistoryEntry, OrganizationMergeEntry } from '~/types/admin-organizations'
import type { I18nText, TimeZoneName } from '~/types/shared'

/**
 * L'HISTORIQUE D'UNE FICHE, CHAMP PAR CHAMP.
 *
 * CE N'EST PAS UNE TABLE, et c'est ce qui le rend fiable :
 * `platform.entity_history('org', 'organizations', id)` dépile le journal
 * d'audit, alimenté par un trigger. Toute écriture y figure — y compris les
 * corrections faites en console, que la table `activity_modifications` de la v1,
 * alimentée à la main par le code applicatif, ne voyait jamais.
 *
 * LES VALEURS SONT DES DOCUMENTS JSON, PAS DES CHAÎNES : `old_data -> field`. Un
 * nom est une chaîne, une description un `i18n_text`, un statut un code. On les
 * rend selon leur forme plutôt que de les concaténer — une description
 * multilingue affichée brute donnerait `{"fr":"…","en":"…"}` sous les yeux d'un
 * chargé de programme.
 *
 * LES FUSIONS SONT MONTRÉES À PART, en tête. Ce ne sont pas des modifications de
 * champ mais des événements d'une autre nature : elles portent un motif, un
 * décompte de lignes déplacées, et elles expliquent à elles seules pourquoi une
 * fiche a doublé de membres un mardi après-midi.
 */

interface Props {
  history: OrganizationHistoryEntry[]
  merges: OrganizationMergeEntry[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t, te } = useI18n()
const { tr } = useI18nText()
const { dateTime } = useDateTime()

/** Libellé d'un champ modifié ; à défaut, son nom technique. */
function fieldLabel(field: string): string {
  const normalized = field.replace('organization_domains.verified_at', 'domainVerified').replace(
    'organization_names.is_confirmed',
    'nameConfirmed',
  )
  const key = `admin.organization.detail.history.fields.${normalized}`
  return te(key) ? t(key) : field
}

/** Un instant ISO, tel que l'audit le sérialise dans son document JSON. */
const ISO_INSTANT = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/

/**
 * Valeur rendue selon sa FORME — et selon son CHAMP quand la forme ne suffit pas.
 *
 * Trois cas qu'un `String(value)` rendrait illisibles, et qui sont précisément
 * ceux que l'audit produit le plus souvent :
 *   · un instant — `verified_at` vaut `2026-03-11T09:15:00Z` dans le document
 *     JSON, ce qui n'est une date pour personne ;
 *   · un STATUT — `candidate`, `merged` : des codes du modèle, dont la liste est
 *     déjà traduite pour la liste des organisations. On la réutilise plutôt que
 *     d'en écrire une seconde ;
 *   · un document multilingue, qu'on résout au lieu d'imprimer son JSON.
 */
function valueLabel(field: string | null, value: unknown): string {
  if (value === null || value === undefined || value === '') {
    return t('admin.organization.detail.history.emptyValue')
  }
  if (typeof value === 'boolean') {
    return t(value ? 'common.labels.yes' : 'common.labels.no')
  }
  if (typeof value === 'object') {
    const text = tr(value as I18nText)
    return text.trim().length > 0 ? text : t('admin.organization.detail.history.emptyValue')
  }

  const raw = String(value)
  if (ISO_INSTANT.test(raw)) return dateTime(raw, props.timezone)

  if (field === 'status') {
    const key = `admin.organization.list.status.${raw}`
    if (te(key)) return t(key)
  }
  return raw
}

function actorLabel(entry: OrganizationHistoryEntry): string {
  return entry.actor_label
    ? t('admin.organization.detail.history.by', { name: entry.actor_label })
    : t('admin.organization.detail.history.system')
}
</script>

<template>
  <section>
    <h2 class="text-lg font-semibold text-text">
      {{ t('admin.organization.detail.history.title') }}
    </h2>
    <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
      {{ t('admin.organization.detail.history.description') }}
    </p>

    <!-- LES FUSIONS D'ABORD : un événement d'une autre nature qu'une modification
         de champ, et le seul qui explique un saut de tous les compteurs. -->
    <section v-if="props.merges.length > 0" class="mt-4">
      <h3 class="text-sm font-semibold tracking-wide text-text-subtle uppercase">
        {{ t('admin.organization.detail.merges.title') }}
      </h3>
      <ul class="mt-2 flex flex-col gap-2">
        <li
          v-for="entry in props.merges"
          :key="entry.id"
          class="rounded-md border border-border bg-surface-sunken px-3 py-2"
        >
          <p class="text-sm font-medium text-text">
            {{
              t('admin.organization.detail.merges.entry', {
                source: entry.source_name,
                target: entry.target_name,
              })
            }}
          </p>
          <p class="mt-0.5 text-xs text-text-muted">
            {{
              t('admin.organization.detail.merges.by', {
                name: entry.performed_by_name ?? '—',
                date: dateTime(entry.performed_at, props.timezone),
              })
            }}
            —
            {{
              t(
                'admin.organization.detail.merges.rows',
                Object.values(entry.rows_reassigned).reduce((sum, n) => sum + n, 0),
              )
            }}
          </p>
          <p v-if="entry.reason" class="mt-1 text-xs text-text">
            {{ t('admin.organization.detail.merges.reason', { reason: entry.reason }) }}
          </p>
        </li>
      </ul>
    </section>

    <UiEmptyState
      v-if="props.history.length === 0"
      class="mt-4"
      compact
      :title="t('admin.organization.detail.history.empty')"
    />

    <ol v-else class="mt-4 flex flex-col gap-3">
      <li
        v-for="(entry, index) in props.history"
        :key="`${entry.occurred_at}-${entry.field ?? 'insert'}-${index}`"
        class="border-l-2 border-border pl-4"
      >
        <p class="text-sm font-medium text-text">
          {{
            entry.action === 'insert' || entry.field === null
              ? t('admin.organization.detail.history.created')
              : t('admin.organization.detail.history.changed', { field: fieldLabel(entry.field) })
          }}
        </p>
        <p class="text-xs text-text-muted">
          {{ dateTime(entry.occurred_at, props.timezone) }} — {{ actorLabel(entry) }}
        </p>

        <dl
          v-if="entry.field !== null"
          class="mt-1.5 grid gap-x-4 gap-y-0.5 text-xs sm:grid-cols-[auto_1fr]"
        >
          <dt class="text-text-subtle">{{ t('admin.organization.detail.history.from') }}</dt>
          <dd class="break-words text-text-muted line-through decoration-border">
            {{ valueLabel(entry.field, entry.old_value) }}
          </dd>
          <dt class="text-text-subtle">{{ t('admin.organization.detail.history.to') }}</dt>
          <dd class="break-words text-text">{{ valueLabel(entry.field, entry.new_value) }}</dd>
        </dl>
      </li>
    </ol>
  </section>
</template>
