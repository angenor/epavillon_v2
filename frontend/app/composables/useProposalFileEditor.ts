import type { EditableProposal, ProposalDraft, ProposalFormStep } from '~/types/proposal-form'
import type { ProposalDocumentEntry } from '~/types/admin-review'
import type { ProposalReferences } from './useProposalReferences'

/** Les rubriques de la fiche qui se modifient par une étape du formulaire de dépôt. */
export type ProposalFileSection = Extract<
  ProposalFormStep,
  'organizations' | 'presentation' | 'classification' | 'speakers' | 'schedule'
>

interface ProposalFileEditorData {
  editable: EditableProposal
  references: ProposalReferences
  documents: ProposalDocumentEntry[]
}

/**
 * LA FICHE DU DOSSIER EN MODIFICATION, rubrique par rubrique (29/10).
 *
 * L'API n'a qu'une écriture, le dossier ENTIER (`PUT /proposals/{id}`) : chaque
 * rubrique part d'une copie du dossier tel qu'enregistré, ne change que sa
 * partie, et renvoie le tout. Les étapes du formulaire de dépôt servent telles
 * quelles : mêmes champs, mêmes contrôles, une seule implémentation.
 */
export function useProposalFileEditor(proposalId: Ref<string>, onSaved: () => Promise<void>) {
  const api = useApi()
  const auth = useAuthStore()
  const { t } = useI18n()

  const { data, refresh } = useAsyncData<ProposalFileEditorData | null>(
    () => `proposal-file-editor-${proposalId.value}`,
    async () => {
      const editable = await api.proposals.forEdit(proposalId.value)
      if (!editable) return null
      const [references, documents] = await Promise.all([
        loadProposalReferences(api, editable.event_id),
        api.proposals.documents(editable.proposal_id).catch(() => []),
      ])
      return { editable, references, documents }
    },
    { lazy: true, watch: [proposalId] },
  )

  const saved = computed<ProposalDraft | null>(() =>
    data.value ? draftFromReopened(data.value.editable.draft, data.value.documents.map(draftDocumentOf)) : null,
  )

  const editing = ref<ProposalFileSection | null>(null)
  const working = ref<ProposalDraft | null>(null)
  const saving = ref(false)
  const error = ref<string | null>(null)
  const tried = ref(false)

  const issues = computed(() => {
    const references = data.value?.references
    if (!working.value || !editing.value || !references?.call || !references.edition) return []
    return validateProposalDraft(working.value, references.call, references.edition).filter(
      (issue) => issue.step === editing.value,
    )
  })

  /** Les défauts ne s'affichent qu'après une tentative : une rubrique qui s'ouvre en rouge réprimande avant la faute. */
  const shownIssues = computed(() => (tried.value ? issues.value : []))

  function edit(section: ProposalFileSection): void {
    if (!saved.value) return
    // Copie détachée : annuler ne doit rien laisser dans le dossier affiché.
    working.value = JSON.parse(JSON.stringify(saved.value)) as ProposalDraft
    editing.value = section
    error.value = null
    tried.value = false
  }

  function cancel(): void {
    editing.value = null
    working.value = null
    error.value = null
  }

  async function save(): Promise<void> {
    const current = working.value
    const context = data.value
    const person = auth.person
    if (!current || !context || !person || !context.editable.call_id) return
    tried.value = true
    if (issues.value.some((issue) => issue.severity === 'error')) return

    saving.value = true
    error.value = null
    try {
      await api.proposals.saveDraft(person.id, {
        proposal_id: context.editable.proposal_id,
        call_id: context.editable.call_id,
        event_id: context.editable.event_id,
        draft: current,
      })
      await Promise.all([refresh(), onSaved()])
      cancel()
    } catch (thrown) {
      error.value = apiErrorMessage(thrown, (key) => t(key))
    } finally {
      saving.value = false
    }
  }

  return { data, saved, editing, working, saving, error, shownIssues, edit, cancel, save, refresh }
}
