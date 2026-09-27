<script setup lang="ts">
import type {
  AdminPathway,
  AdminPathwayGroup,
  AdminPathwayOrderInput,
  AdminPathwayStep,
  AdminPathwayStepInput,
} from '~/types/admin-negotiation-savoir'
import type { I18nText, Uuid } from '~/types/shared'
import type { SelectOption } from '~/types/ui'
import type { DocumentFormFailure } from '~/components/admin/negotiation/DocumentForm.vue'

// Chaque écriture rend le parcours entier : l'état local est remplacé, jamais recalculé ici.
const props = defineProps<{ modelValue: AdminPathway }>()
const emit = defineEmits<{ 'update:modelValue': [value: AdminPathway] }>()

const { t, locale } = useI18n()
const api = useApi()

const peutEcrire = computed(() => props.modelValue.can_publish)
const libelle = (texte: I18nText | null | undefined): string => resolveI18nText(texte, locale.value)

const geste = ref<string | null>(null)
const echec = ref<string | null>(null)
const resultat = ref<string | null>(null)

const echecDe = (erreur: unknown): DocumentFormFailure => ({
  message: apiErrorMessage(erreur, t),
  field: erreur instanceof ApiRequestError ? erreur.field : null,
})

/** Rend `true` si l'écriture a réussi ; l'échec d'un formulaire se montre dans son dialogue. */
async function ecrire(
  nom: string,
  appel: () => Promise<AdminPathway | null>,
  cle: string,
  surEchec?: (e: DocumentFormFailure) => void,
): Promise<boolean> {
  if (geste.value) return false
  geste.value = nom
  echec.value = null
  resultat.value = null
  try {
    // Une suppression ne rend rien : le parcours se relit.
    emit('update:modelValue', (await appel()) ?? (await api.adminNegotiationSavoir.parcours()))
    resultat.value = t(cle)
    return true
  } catch (erreur) {
    if (surEchec) surEchec(echecDe(erreur))
    else echec.value = apiErrorMessage(erreur, t)
    return false
  } finally {
    geste.value = null
  }
}

// --- Ordre -----------------------------------------------------------------------

function ordreActuel(): AdminPathwayOrderInput {
  return { groups: props.modelValue.groups.map((g) => ({ id: g.id, step_ids: g.steps.map((s) => s.id) })) }
}

function permuter<T>(liste: T[], index: number, sens: -1 | 1): void {
  const [element] = liste.splice(index, 1)
  if (element !== undefined) liste.splice(index + sens, 0, element)
}

const envoyerOrdre = (ordre: AdminPathwayOrderInput, nom: string, cle = 'admin.negociations.parcours.result.ordered') =>
  ecrire(nom, () => api.adminNegotiationSavoir.ordonnerParcours(ordre), cle)

function deplacerGroupe(index: number, sens: -1 | 1): void {
  const ordre = ordreActuel()
  permuter(ordre.groups, index, sens)
  void envoyerOrdre(ordre, `order-${props.modelValue.groups[index]?.id}`)
}

function deplacerEtape(groupIndex: number, stepIndex: number, sens: -1 | 1): void {
  const ordre = ordreActuel()
  const groupe = ordre.groups[groupIndex]
  if (!groupe) return
  permuter(groupe.step_ids, stepIndex, sens)
  void envoyerOrdre(ordre, `order-${groupe.step_ids[stepIndex + sens]}`)
}

// --- Groupes -----------------------------------------------------------------------

const dialogueGroupe = ref<{ id: Uuid | null; fr: string; en: string; published: boolean; erreur: string | null } | null>(
  null,
)

function ouvrirGroupe(groupe: AdminPathwayGroup | null): void {
  dialogueGroupe.value = {
    id: groupe?.id ?? null,
    fr: groupe?.label.fr ?? '',
    en: groupe?.label.en ?? '',
    published: false,
    erreur: null,
  }
}

async function enregistrerGroupe(): Promise<void> {
  const d = dialogueGroupe.value
  if (!d) return
  if (!d.fr.trim()) {
    d.erreur = t('admin.negociations.parcours.group.error.label')
    return
  }
  const label: I18nText = { fr: d.fr.trim(), ...(d.en.trim() ? { en: d.en.trim() } : {}) }
  const ok = await ecrire(
    'group-save',
    () =>
      d.id
        ? api.adminNegotiationSavoir.modifierGroupe(d.id, { label })
        : api.adminNegotiationSavoir.creerGroupe({ label, is_published: d.published }),
    d.id ? 'admin.negociations.parcours.result.groupSaved' : 'admin.negociations.parcours.result.groupCreated',
    (e) => (d.erreur = e.message),
  )
  if (ok) dialogueGroupe.value = null
}

const publierGroupe = (groupe: AdminPathwayGroup) =>
  ecrire(
    `publish-${groupe.id}`,
    () => api.adminNegotiationSavoir.modifierGroupe(groupe.id, { is_published: !groupe.is_published }),
    groupe.is_published ? 'admin.negociations.parcours.result.unpublished' : 'admin.negociations.parcours.result.published',
  )

// --- Étapes ------------------------------------------------------------------------

const dialogueEtape = ref<{ step: AdminPathwayStep | null; groupId: Uuid; failure: DocumentFormFailure | null } | null>(
  null,
)

async function enregistrerEtape(entree: AdminPathwayStepInput): Promise<void> {
  const d = dialogueEtape.value
  if (!d) return
  const id = d.step?.id
  const ok = await ecrire(
    'step-save',
    () =>
      id ? api.adminNegotiationSavoir.modifierEtape(id, entree) : api.adminNegotiationSavoir.creerEtape(entree),
    id ? 'admin.negociations.parcours.result.stepSaved' : 'admin.negociations.parcours.result.stepCreated',
    (e) => (d.failure = e),
  )
  if (ok) dialogueEtape.value = null
}

const publierEtape = (step: AdminPathwayStep) =>
  ecrire(
    `publish-${step.id}`,
    () => api.adminNegotiationSavoir.modifierEtape(step.id, { is_published: !step.is_published }),
    step.is_published ? 'admin.negociations.parcours.result.unpublished' : 'admin.negociations.parcours.result.published',
  )

const dialogueDeplacement = ref<{ step: AdminPathwayStep; target: string } | null>(null)

const optionsDeGroupe = computed<SelectOption[]>(() =>
  props.modelValue.groups
    .filter((g) => g.id !== dialogueDeplacement.value?.step.group_id)
    .map((g) => ({ value: g.id, label: libelle(g.label) })),
)

async function deplacerVersGroupe(): Promise<void> {
  const d = dialogueDeplacement.value
  if (!d?.target) return
  const ordre = ordreActuel()
  for (const g of ordre.groups) g.step_ids = g.step_ids.filter((id) => id !== d.step.id)
  ordre.groups.find((g) => g.id === d.target)?.step_ids.push(d.step.id)
  if (await envoyerOrdre(ordre, 'move', 'admin.negociations.parcours.result.moved')) dialogueDeplacement.value = null
}

// --- Suppressions ----------------------------------------------------------------------

const aSupprimer = ref<{ kind: 'group' | 'step'; id: Uuid; label: string } | null>(null)

async function supprimer(): Promise<void> {
  const cible = aSupprimer.value
  if (!cible) return
  await ecrire(
    'delete',
    async () => {
      if (cible.kind === 'group') await api.adminNegotiationSavoir.supprimerGroupe(cible.id)
      else await api.adminNegotiationSavoir.supprimerEtape(cible.id)
      return null
    },
    cible.kind === 'group' ? 'admin.negociations.parcours.result.groupDeleted' : 'admin.negociations.parcours.result.stepDeleted',
  )
  aSupprimer.value = null
}

function descriptionDuLien(step: AdminPathwayStep): string {
  const lien = step.link
  if (!lien) return ''
  const cible = lien.target_label ?? t('admin.negociations.parcours.step.link.unknownTarget')
  const precisions = [
    lien.section,
    lien.page ? t('admin.negociations.parcours.step.link.pageShort', { page: lien.page }) : null,
  ].filter(Boolean)
  return t('admin.negociations.parcours.step.link.summary', {
    kind: t(`admin.negociations.parcours.step.link.kind.${lien.kind}`),
    target: precisions.length ? `${cible} (${precisions.join(', ')})` : cible,
  })
}
</script>

<template>
  <div class="space-y-6">
    <div v-if="peutEcrire" class="flex flex-wrap gap-3">
      <UiButton icon="plus" @click="ouvrirGroupe(null)">{{ t('admin.negociations.parcours.group.new') }}</UiButton>
    </div>

    <UiAlert v-if="echec" intent="danger" live :message="echec" />
    <UiAlert v-else-if="resultat" intent="success" live compact dismissible :message="resultat" />

    <UiEmptyState
      v-if="props.modelValue.groups.length === 0"
      icon="list-ordered"
      :title="t('admin.negociations.parcours.empty.title')"
      :description="t('admin.negociations.parcours.empty.description')"
    />

    <section
      v-for="(groupe, gi) in props.modelValue.groups"
      :key="groupe.id"
      class="rounded-lg border border-border bg-surface"
      :aria-labelledby="`groupe-${groupe.id}`"
    >
      <header class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3">
        <div class="flex min-w-0 flex-wrap items-center gap-2">
          <h2 :id="`groupe-${groupe.id}`" class="text-lg font-semibold break-words">{{ libelle(groupe.label) }}</h2>
          <UiBadge
            :intent="groupe.is_published ? 'success' : 'neutral'"
            size="sm"
            :label="t(groupe.is_published ? 'admin.negociations.parcours.published' : 'admin.negociations.parcours.unpublished')"
          />
          <span class="text-sm text-text-muted">
            {{ t('admin.negociations.parcours.group.steps', { count: groupe.steps.length }, groupe.steps.length) }}
          </span>
        </div>
        <div v-if="peutEcrire" class="flex flex-wrap gap-1">
          <UiButton
            variant="ghost"
            size="sm"
            icon="chevron-up"
            icon-only
            :label="t('admin.negociations.parcours.up')"
            :disabled="gi === 0 || geste !== null"
            @click="deplacerGroupe(gi, -1)"
          />
          <UiButton
            variant="ghost"
            size="sm"
            icon="chevron-down"
            icon-only
            :label="t('admin.negociations.parcours.down')"
            :disabled="gi === props.modelValue.groups.length - 1 || geste !== null"
            @click="deplacerGroupe(gi, 1)"
          />
          <UiButton variant="ghost" size="sm" icon="edit" @click="ouvrirGroupe(groupe)">
            {{ t('admin.negociations.parcours.group.rename') }}
          </UiButton>
          <UiButton
            variant="ghost"
            size="sm"
            :icon="groupe.is_published ? 'eye-off' : 'eye'"
            :loading="geste === `publish-${groupe.id}`"
            @click="publierGroupe(groupe)"
          >
            {{ t(groupe.is_published ? 'admin.negociations.parcours.unpublish' : 'admin.negociations.parcours.publish') }}
          </UiButton>
          <UiButton
            v-if="groupe.steps.length === 0"
            variant="ghost"
            size="sm"
            icon="trash"
            @click="aSupprimer = { kind: 'group', id: groupe.id, label: libelle(groupe.label) }"
          >
            {{ t('admin.negociations.parcours.delete') }}
          </UiButton>
        </div>
      </header>

      <p v-if="groupe.steps.length === 0" class="px-4 py-3 text-sm text-text-muted">
        {{ t('admin.negociations.parcours.group.empty') }}
      </p>
      <ol v-else class="divide-y divide-border">
        <li
          v-for="(step, si) in groupe.steps"
          :key="step.id"
          class="flex flex-col gap-3 px-4 py-3 lg:flex-row lg:items-start lg:justify-between"
        >
          <div class="min-w-0">
            <p class="font-medium break-words">{{ libelle(step.label) }}</p>
            <p v-if="step.detail" class="mt-0.5 text-sm break-words text-text-secondary">{{ libelle(step.detail) }}</p>
            <p v-if="step.origin_label" class="mt-0.5 text-sm text-text-muted">
              {{ t('admin.negociations.parcours.step.originShort', { origin: libelle(step.origin_label) }) }}
            </p>
            <p v-if="step.link" class="mt-0.5 text-sm break-words text-text-muted">{{ descriptionDuLien(step) }}</p>
            <div class="mt-1.5 flex flex-wrap items-center gap-2">
              <UiBadge
                v-if="!step.is_published"
                size="sm"
                :label="t('admin.negociations.parcours.unpublished')"
              />
              <span class="text-xs text-text-muted">
                {{ t('admin.negociations.parcours.step.checks', { count: step.checks }, step.checks) }}
              </span>
            </div>
          </div>

          <div v-if="peutEcrire" class="flex shrink-0 flex-wrap gap-1">
            <UiButton
              variant="ghost"
              size="sm"
              icon="chevron-up"
              icon-only
              :label="t('admin.negociations.parcours.up')"
              :disabled="si === 0 || geste !== null"
              @click="deplacerEtape(gi, si, -1)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="chevron-down"
              icon-only
              :label="t('admin.negociations.parcours.down')"
              :disabled="si === groupe.steps.length - 1 || geste !== null"
              @click="deplacerEtape(gi, si, 1)"
            />
            <UiButton
              variant="ghost"
              size="sm"
              icon="edit"
              @click="dialogueEtape = { step, groupId: groupe.id, failure: null }"
            >
              {{ t('admin.negociations.parcours.step.edit') }}
            </UiButton>
            <UiButton
              v-if="props.modelValue.groups.length > 1"
              variant="ghost"
              size="sm"
              icon="arrow-right"
              @click="dialogueDeplacement = { step, target: '' }"
            >
              {{ t('admin.negociations.parcours.step.move') }}
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              :icon="step.is_published ? 'eye-off' : 'eye'"
              :loading="geste === `publish-${step.id}`"
              @click="publierEtape(step)"
            >
              {{ t(step.is_published ? 'admin.negociations.parcours.unpublish' : 'admin.negociations.parcours.publish') }}
            </UiButton>
            <UiButton
              v-if="step.checks === 0"
              variant="ghost"
              size="sm"
              icon="trash"
              @click="aSupprimer = { kind: 'step', id: step.id, label: libelle(step.label) }"
            >
              {{ t('admin.negociations.parcours.delete') }}
            </UiButton>
          </div>
        </li>
      </ol>

      <div v-if="peutEcrire" class="border-t border-border px-4 py-3">
        <UiButton
          variant="secondary"
          size="sm"
          icon="plus"
          @click="dialogueEtape = { step: null, groupId: groupe.id, failure: null }"
        >
          {{ t('admin.negociations.parcours.step.new') }}
        </UiButton>
      </div>
    </section>

    <UiModal
      :open="dialogueGroupe !== null"
      :title="t(dialogueGroupe?.id ? 'admin.negociations.parcours.group.renameTitle' : 'admin.negociations.parcours.group.newTitle')"
      @update:open="(ouvert: boolean) => !ouvert && (dialogueGroupe = null)"
    >
      <form v-if="dialogueGroupe" class="space-y-4" novalidate @submit.prevent="enregistrerGroupe">
        <UiInput
          v-model="dialogueGroupe.fr"
          :label="t('admin.negociations.parcours.group.label.fr')"
          :maxlength="200"
          required
        />
        <UiInput v-model="dialogueGroupe.en" :label="t('admin.negociations.parcours.group.label.en')" :maxlength="200" />
        <UiCheckbox
          v-if="!dialogueGroupe.id"
          v-model="dialogueGroupe.published"
          :label="t('admin.negociations.parcours.group.published')"
        />
        <UiAlert v-if="dialogueGroupe.erreur" intent="danger" live :message="dialogueGroupe.erreur" />
        <div class="flex flex-wrap justify-end gap-3">
          <UiButton variant="ghost" @click="dialogueGroupe = null">{{ t('admin.negociations.parcours.cancel') }}</UiButton>
          <UiButton type="submit" :loading="geste === 'group-save'">{{ t('admin.negociations.parcours.save') }}</UiButton>
        </div>
      </form>
    </UiModal>

    <UiModal
      :open="dialogueEtape !== null"
      size="lg"
      :title="t(dialogueEtape?.step ? 'admin.negociations.parcours.step.editTitle' : 'admin.negociations.parcours.step.newTitle')"
      @update:open="(ouvert: boolean) => !ouvert && (dialogueEtape = null)"
    >
      <AdminNegotiationPathwayStepForm
        v-if="dialogueEtape"
        :key="dialogueEtape.step?.id ?? `nouvelle-${dialogueEtape.groupId}`"
        :step="dialogueEtape.step"
        :group-id="dialogueEtape.groupId"
        :submitting="geste === 'step-save'"
        :failure="dialogueEtape.failure"
        @submit="enregistrerEtape"
        @cancel="dialogueEtape = null"
      />
    </UiModal>

    <UiModal
      :open="dialogueDeplacement !== null"
      :title="t('admin.negociations.parcours.step.moveTitle')"
      :description="dialogueDeplacement ? libelle(dialogueDeplacement.step.label) : undefined"
      @update:open="(ouvert: boolean) => !ouvert && (dialogueDeplacement = null)"
    >
      <form v-if="dialogueDeplacement" class="space-y-4" novalidate @submit.prevent="deplacerVersGroupe">
        <UiSelect
          v-model="dialogueDeplacement.target"
          :label="t('admin.negociations.parcours.step.moveTarget')"
          :placeholder="t('admin.negociations.parcours.step.moveTargetPlaceholder')"
          :options="optionsDeGroupe"
          required
        />
        <p class="text-sm text-text-muted">{{ t('admin.negociations.parcours.step.moveHint') }}</p>
        <div class="flex flex-wrap justify-end gap-3">
          <UiButton variant="ghost" @click="dialogueDeplacement = null">{{ t('admin.negociations.parcours.cancel') }}</UiButton>
          <UiButton type="submit" :disabled="!dialogueDeplacement.target" :loading="geste === 'move'">
            {{ t('admin.negociations.parcours.step.moveConfirm') }}
          </UiButton>
        </div>
      </form>
    </UiModal>

    <UiModal
      :open="aSupprimer !== null"
      :title="t(aSupprimer?.kind === 'group' ? 'admin.negociations.parcours.confirm.groupTitle' : 'admin.negociations.parcours.confirm.stepTitle')"
      :description="aSupprimer ? t('admin.negociations.parcours.confirm.question', { label: aSupprimer.label }) : undefined"
      @update:open="(ouvert: boolean) => !ouvert && (aSupprimer = null)"
    >
      <div class="flex flex-wrap justify-end gap-3">
        <UiButton variant="ghost" @click="aSupprimer = null">{{ t('admin.negociations.parcours.cancel') }}</UiButton>
        <UiButton variant="danger" :loading="geste === 'delete'" @click="supprimer">
          {{ t('admin.negociations.parcours.delete') }}
        </UiButton>
      </div>
    </UiModal>
  </div>
</template>
