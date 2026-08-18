<script setup lang="ts">
import type { ProposalDocumentEntry } from '~/types/admin-review'
import type { TimeZoneName } from '~/types/shared'

/**
 * LES PIÈCES DU DOSSIER, CONSULTABLES.
 *
 * UN FICHIER EN QUARANTAINE EST ANNONCÉ, PAS PROPOSÉ. `media.assets.status` ne
 * vaut `ready` que si l'analyse antivirus est passée ; une pièce refusée
 * (`scan_verdict: 'infected'`) doit se lire comme MANQUANTE au dossier — c'est
 * une information pour le comité, qui juge alors sur pièces incomplètes, et un
 * lien mort à sa place lui ferait croire à une panne d'affichage.
 *
 * INTERNE OU PUBLIABLE : `proposal_documents.is_public` dit si la pièce paraîtra
 * sur la page publique de l'activité une fois celle-ci programmée. Le comité
 * voit les deux — c'est son dossier —, mais la distinction se lit, parce qu'un
 * budget prévisionnel joint par erreur en publiable se corrige avant la
 * publication, pas après.
 *
 * LA BASE NE STOCKE AUCUNE URL : l'adresse est composée à la lecture par l'API
 * (`media.object_url()`). Le composant reçoit donc un objet, jamais un chemin à
 * fabriquer.
 */

interface Props {
  entries: ProposalDocumentEntry[]
  timezone: TimeZoneName
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()
const { date } = useDateTime()

/** Le poids en mégaoctets, à une décimale : un octet près n'aide personne. */
function megabytes(bytes: number): string {
  return (bytes / 1_048_576).toFixed(1)
}
</script>

<template>
  <section aria-labelledby="review-documents-title">
    <h3 id="review-documents-title" class="text-sm font-semibold tracking-wide uppercase">
      {{ t('admin.proposal.review.documents.title') }}
    </h3>

    <p v-if="props.entries.length === 0" class="mt-3 text-sm text-text-muted">
      {{ t('admin.proposal.review.documents.empty') }}
    </p>

    <ul v-else class="mt-3 flex flex-col gap-2">
      <li
        v-for="entry in props.entries"
        :key="entry.document.id"
        class="flex flex-wrap items-center gap-x-3 gap-y-1 rounded-md border border-border bg-surface px-3 py-2"
      >
        <UiIcon name="document" size="1.15rem" :stroke-width="1.7" class="shrink-0 text-text-subtle" />

        <span class="min-w-0 flex-1">
          <span class="block font-medium text-text">{{ tr(entry.document.title) }}</span>
          <span class="block text-xs text-text-subtle">
            <template v-if="entry.asset">
              {{ entry.asset.original_filename }} ·
              {{ t('admin.proposal.review.documents.size', { size: megabytes(entry.asset.byte_size) }) }} ·
            </template>
            {{
              t('admin.proposal.review.documents.uploadedAt', {
                date: date(entry.document.uploaded_at, props.timezone),
              })
            }}
          </span>
        </span>

        <UiBadge
          size="sm"
          :intent="entry.document.is_public ? 'info' : 'neutral'"
          :label="
            entry.document.is_public
              ? t('admin.proposal.review.documents.public')
              : t('admin.proposal.review.documents.internal')
          "
        />

        <!-- QUARANTAINE : un avertissement à la place du bouton, jamais les deux.
             Un lien désactivé sans explication se clique trois fois. -->
        <span
          v-if="entry.url === null"
          class="flex items-center gap-1.5 text-sm text-danger"
        >
          <UiIcon name="ban" size="1rem" :stroke-width="1.8" />
          {{
            entry.asset?.scan_verdict === 'infected'
              ? t('admin.proposal.review.documents.quarantined')
              : t('admin.proposal.review.documents.unavailable')
          }}
        </span>

        <UiButton
          v-else
          variant="secondary"
          size="sm"
          icon="download"
          :href="entry.url"
          :label="t('admin.proposal.review.documents.open', { title: tr(entry.document.title) })"
          icon-only
        />
      </li>
    </ul>
  </section>
</template>
