<script setup lang="ts">
/**
 * UNE RUBRIQUE DU DOSSIER, modifiable sur place (29/10).
 *
 * Lecture d'abord, « Modifier » ouvre la rubrique dans la page, « Enregistrer »
 * ou « Annuler » la referme : pas d'aller-retour vers le formulaire de dépôt,
 * qui perdait des personnes peu à l'aise avec les écrans. Une seule rubrique
 * ouverte à la fois : c'est la page qui en décide (`editing`).
 */

interface Props {
  id: string
  title: string
  description?: string
  /** La rubrique se modifie-t-elle maintenant ? Sinon, aucun bouton. */
  canEdit: boolean
  editing: boolean
  saving?: boolean
  /** Une autre rubrique est ouverte : on ne superpose pas deux modifications. */
  locked?: boolean
  /** Refus de l'API, affiché tel quel. */
  error?: string | null
}

const props = withDefaults(defineProps<Props>(), { description: undefined, saving: false, locked: false, error: null })
const emit = defineEmits<{ edit: []; save: []; cancel: [] }>()

const { t } = useI18n()
</script>

<template>
  <section
    :id="props.id"
    :aria-labelledby="`${props.id}-title`"
    class="scroll-mt-24 rounded-lg border bg-surface-raised p-5 sm:p-6"
    :class="props.editing ? 'border-accent ring-1 ring-accent' : 'border-border'"
  >
    <div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
      <div class="min-w-0">
        <h2 :id="`${props.id}-title`" class="text-xl font-semibold text-heading">{{ props.title }}</h2>
        <p v-if="props.description" class="mt-1 max-w-(--measure) text-sm text-text-muted">{{ props.description }}</p>
      </div>
      <UiButton
        v-if="props.canEdit && !props.editing"
        variant="secondary"
        icon="edit"
        :disabled="props.locked"
        @click="emit('edit')"
      >
        {{ t('common.actions.edit') }}
      </UiButton>
    </div>

    <div class="mt-4">
      <slot v-if="props.editing" name="edit" />
      <slot v-else />
    </div>

    <template v-if="props.editing">
      <UiAlert v-if="props.error" class="mt-4" intent="danger" :message="props.error" live />
      <div class="mt-6 flex flex-wrap justify-end gap-3 border-t border-border-subtle pt-4">
        <UiButton variant="secondary" :disabled="props.saving" @click="emit('cancel')">
          {{ t('common.actions.cancel') }}
        </UiButton>
        <UiButton variant="primary" :loading="props.saving" @click="emit('save')">
          {{ t('common.actions.save') }}
        </UiButton>
      </div>
    </template>
  </section>
</template>
