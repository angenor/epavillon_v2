<script setup lang="ts">
/**
 * UNE RUBRIQUE DU DOSSIER, modifiable sur place (29/10).
 *
 * Lecture d'abord, « Modifier » ouvre la rubrique dans la page, « Enregistrer »
 * ou « Annuler » la referme : pas d'aller-retour vers le formulaire de dépôt.
 * Une seule rubrique ouverte à la fois : c'est la page qui en décide (`editing`).
 *
 * Direction condensée (04/11) : plus de carte, une section séparée de ses
 * voisines par un filet ; en modification, elle s'encadre.
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
    class="scroll-mt-24"
    :class="
      props.editing
        ? 'my-2 rounded-lg border-2 border-accent-solid bg-surface-raised px-3 py-5 sm:-mx-5 sm:px-5 sm:py-6'
        : 'border-t border-border-subtle py-7 first-of-type:border-t-0'
    "
  >
    <div class="flex flex-wrap items-start justify-between gap-x-4 gap-y-2">
      <div class="min-w-0">
        <h3 :id="`${props.id}-title`" class="font-condensed text-[1.45rem] leading-tight font-semibold text-heading">{{ props.title }}</h3>
        <p v-if="props.description" class="mt-1 max-w-[62ch] text-sm text-text-muted">{{ props.description }}</p>
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
