<script setup lang="ts">
import type { TaxonomyTerm } from '~/types/reference'
import type { DraftDocument, DraftIssue, ProposalDraft } from '~/types/proposal-form'
import { DOCUMENT_MAX_BYTES } from '~/types/proposal-form'
import type { SelectOption } from '~/types/ui'

/**
 * ÉTAPE 6 — LES PIÈCES DU DOSSIER : présentation, note de cadrage, annexes.
 *
 * LES TYPES DE DOCUMENT VIENNENT DE LA BASE (taxonomie `document_type`), pas
 * d'une liste écrite ici : un administrateur peut en ajouter un depuis le
 * back-office, et le formulaire doit le proposer sans déploiement.
 *
 * LE REFUS EST IMMÉDIAT, ET IL REPREND CELUI DE LA BASE. `media.attachable_roles`
 * déclare pour le rôle `('programme','proposals','document')` les formats
 * acceptés et vingt-cinq mégaoctets au plus ; `media.tg_validate_attachment()`
 * refuse le reste. Laisser partir un fichier trop lourd pour le rejeter à
 * l'arrivée, c'est une minute perdue sur une connexion lente — et une raison
 * d'abandonner le dossier.
 *
 * LE TÉLÉVERSEMENT RÉEL N'EXISTE PAS ENCORE, et l'écran ne fait pas semblant : le
 * fichier est RETENU, son nom, son format et sa taille sont conservés, et le
 * dépôt sur le stockage (URL signée, `media.assets`, déduplication par
 * empreinte) viendra au prompt B6. Ce qui est écrit ici — contrôle de format, de
 * taille, titre obligatoire, visibilité — ne changera pas à ce moment-là.
 *
 * LA VISIBILITÉ EST UN CHOIX EXPLICITE (`proposal_documents.is_public`) : une
 * note de cadrage interne au comité n'a pas à se retrouver sur la page publique
 * de l'activité une fois celle-ci retenue. Le défaut est PRIVÉ, comme la colonne.
 */

const draft = defineModel<ProposalDraft>({ required: true })

interface Props {
  documentTypes: TaxonomyTerm[]
  issues: DraftIssue[]
}

const props = defineProps<Props>()

const { t, locale } = useI18n()
const { tr } = useI18nText()

const typeOptions = computed<SelectOption[]>(() =>
  props.documentTypes.map((term) => ({ value: term.code, label: tr(term.label) })),
)

const maxSizeLabel = computed(() => formatByteSize(DOCUMENT_MAX_BYTES, locale.value))

const fileError = ref<string | null>(null)

function issuesOf(document: DraftDocument): DraftIssue[] {
  return props.issues.filter((entry) => entry.field.startsWith(`documents.${document.key}.`))
}

function errorOfDocument(document: DraftDocument, suffix: string): string | undefined {
  const issue = issuesOf(document).find((entry) => entry.field.endsWith(`.${suffix}`))
  return issue ? t(issue.messageKey, issue.params ?? {}) : undefined
}

function onFilesChosen(event: Event): void {
  const input = event.target as HTMLInputElement
  const files = Array.from(input.files ?? [])
  fileError.value = null

  for (const file of files) {
    const upload = {
      file_name: file.name,
      mime_type: file.type,
      byte_size: file.size,
      asset_id: null,
    }

    // Même contrôle que `media.attachable_roles`, appliqué avant tout envoi.
    const rejection = rejectDocument(upload)
    if (rejection) {
      fileError.value = t(rejection, { file: file.name, size: maxSizeLabel.value })
      continue
    }

    const document: DraftDocument = {
      key: draftKey('document', draft.value.documents.length),
      upload,
      // Le nom du fichier amorce le titre : neuf fois sur dix il convient, et
      // laisser le champ vide, c'est produire des pièces sans intitulé.
      title: file.name.replace(/\.[^.]+$/, ''),
      document_type_code: null,
      is_public: false,
    }
    draft.value.documents = [...draft.value.documents, document]
  }

  input.value = ''
}

function remove(key: string): void {
  draft.value.documents = draft.value.documents.filter((entry) => entry.key !== key)
}

function update(key: string, patch: Partial<DraftDocument>): void {
  draft.value.documents = draft.value.documents.map((entry) =>
    entry.key === key ? { ...entry, ...patch } : entry,
  )
}
</script>

<template>
  <div class="grid gap-5">
    <header>
      <h2 class="font-display text-xl text-text">
        {{ t('proposal.form.step-documents.title') }}
      </h2>
      <p class="mt-1 max-w-(--measure) text-sm text-text-muted">
        {{ t('proposal.form.step-documents.description') }}
      </p>
    </header>

    <ul v-if="draft.documents.length > 0" class="grid gap-3">
      <li
        v-for="document in draft.documents"
        :key="document.key"
        class="rounded-md border bg-surface-raised px-4 py-3"
        :class="issuesOf(document).length > 0 ? 'border-danger-border' : 'border-border'"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <p class="flex min-w-0 items-start gap-2">
            <UiIcon name="document" size="1.2rem" class="mt-0.5 shrink-0 text-text-muted" />
            <span class="min-w-0">
              <span class="block font-mono text-sm break-all text-text-secondary">
                {{ document.upload.file_name }}
              </span>
              <span class="block text-sm text-text-subtle">
                {{ formatByteSize(document.upload.byte_size, locale) }}
              </span>
            </span>
          </p>

          <UiButton
            variant="ghost"
            size="sm"
            icon="trash"
            icon-only
            :label="t('proposal.form.step-documents.remove', { file: document.upload.file_name })"
            @click="remove(document.key)"
          />
        </div>

        <div class="mt-3 grid gap-4 sm:grid-cols-2">
          <UiInput
            :model-value="document.title"
            :label="t('proposal.form.step-documents.fields.title.label')"
            :error="errorOfDocument(document, 'title')"
            required
            @update:model-value="update(document.key, { title: $event })"
          />
          <UiSelect
            :model-value="document.document_type_code"
            :options="typeOptions"
            :label="t('proposal.form.step-documents.fields.type.label')"
            :placeholder="t('proposal.form.step-documents.fields.type.placeholder')"
            @update:model-value="update(document.key, { document_type_code: $event })"
          />
        </div>

        <!-- L'interrupteur mesure 80 × 40 : il lui faut sa propre rangée et un
             écart plus large que celui des champs, sinon il vient toucher la
             ligne de saisie qui le précède. -->
        <div class="mt-4">
          <UiSwitch
            :model-value="document.is_public"
            :label="t('proposal.form.step-documents.fields.public.label')"
            :hint="t('proposal.form.step-documents.fields.public.hint')"
            @update:model-value="update(document.key, { is_public: $event })"
          />
        </div>

        <p
          v-if="errorOfDocument(document, 'file')"
          role="alert"
          class="mt-2 text-sm font-bold text-danger"
        >
          {{ errorOfDocument(document, 'file') }}
        </p>
      </li>
    </ul>

    <UiEmptyState
      v-else
      compact
      icon="document"
      :title="t('proposal.form.step-documents.empty.title')"
      :description="t('proposal.form.step-documents.empty.description')"
    />

    <div>
      <label class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-2 rounded-md border border-border px-4 py-2 text-sm font-bold text-text-secondary hover:bg-surface-hover">
        <UiIcon name="upload" size="1rem" />
        {{ t('proposal.form.step-documents.choose') }}
        <input
          type="file"
          multiple
          accept="application/pdf,application/vnd.openxmlformats-officedocument.presentationml.presentation,application/vnd.openxmlformats-officedocument.wordprocessingml.document"
          class="sr-only"
          @change="onFilesChosen"
        >
      </label>

      <p class="mt-2 text-sm text-text-muted">
        {{ t('common.file.types', { types: t('proposal.form.step-documents.acceptedTypes') }) }}
        · {{ t('common.file.maxSize', { size: maxSizeLabel }) }}
      </p>

      <p v-if="fileError" role="alert" class="mt-2 text-sm font-bold text-danger">
        {{ fileError }}
      </p>
    </div>

    <p class="flex items-start gap-2 text-sm text-text-muted">
      <UiIcon name="info" size="1.05rem" class="mt-0.5 shrink-0" />
      {{ t('proposal.form.step-documents.pendingUpload') }}
    </p>
  </div>
</template>
