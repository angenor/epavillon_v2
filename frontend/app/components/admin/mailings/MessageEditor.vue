<script setup lang="ts">
/**
 * ÉTAPE 2, LE MESSAGE : objet, corps (le même éditeur que les autres textes
 * riches du back-office), variables, et le bloc « Réponses ».
 *
 * LE BLOC « RÉPONSES » DIT À QUOI SERT LA COPIE. La boîte d'envoi ne se
 * consulte pas : l'équipe en copie voit les réponses de ceux qui « répondent à
 * tous ». Chaque destinataire reçoit son propre message ; une adresse en copie
 * en reçoit donc un par destinataire, et l'écran le chiffre à l'étape suivante.
 */
interface Fields {
  subject: string
  body: string
  ccText: string
  bccText: string
  replyTo: string
}

interface Props {
  modelValue: Fields
  variables: string[]
  errors: Partial<Record<keyof Fields, string>>
  disabled?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{ 'update:modelValue': [value: Fields] }>()

const { t } = useI18n()

function set<K extends keyof Fields>(key: K, value: Fields[K]): void {
  emit('update:modelValue', { ...props.modelValue, [key]: value })
}

const copied = ref<string | null>(null)
const tokenOf = variableToken

/** L'éditeur riche ne sait pas insérer à la position du curseur : on copie, on colle. */
async function copyVariable(name: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(variableToken(name))
    copied.value = name
    setTimeout(() => {
      if (copied.value === name) copied.value = null
    }, 2000)
  } catch {
    copied.value = null
  }
}
</script>

<template>
  <section class="space-y-6" :aria-label="t('admin.mailing.compose.steps.message')">
    <UiInput
      :model-value="props.modelValue.subject"
      :label="t('admin.mailing.compose.message.subject')"
      :error="props.errors.subject"
      :maxlength="250"
      :disabled="props.disabled"
      required
      block
      @update:model-value="set('subject', $event)"
    />

    <div>
      <p class="text-sm font-semibold text-text">{{ t('admin.mailing.compose.message.variables') }}</p>
      <p class="mt-0.5 text-xs text-text-subtle">{{ t('admin.mailing.compose.message.variablesHint') }}</p>
      <ul class="mt-2 flex flex-wrap gap-2">
        <li v-for="name in props.variables" :key="name">
          <button
            type="button"
            class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-2 rounded-md border border-border bg-surface-raised px-3 font-mono text-sm text-text transition-colors hover:bg-surface-hover"
            :aria-label="t('admin.mailing.compose.message.copyVariable', { name: t(`admin.mailing.common.variable.${name}`) })"
            @click="copyVariable(name)"
          >
            <span v-text="tokenOf(name)" />
            <span class="font-sans text-xs text-text-subtle">
              {{ copied === name ? t('admin.mailing.compose.message.copied') : t(`admin.mailing.common.variable.${name}`) }}
            </span>
          </button>
        </li>
      </ul>
    </div>

    <UiRichText
      :model-value="props.modelValue.body"
      :label="t('admin.mailing.compose.message.body')"
      :hint="t('admin.mailing.compose.message.bodyHint')"
      :error="props.errors.body"
      :disabled="props.disabled"
      :rows="12"
      required
      @update:model-value="set('body', $event)"
    />

    <fieldset class="rounded-lg border border-border bg-surface-raised p-4 sm:p-5">
      <legend class="px-1 font-display text-lg font-semibold">
        {{ t('admin.mailing.compose.replies.title') }}
      </legend>
      <p class="max-w-(--measure) text-sm text-text-secondary">
        {{ t('admin.mailing.compose.replies.intro') }}
      </p>

      <div class="mt-4 grid gap-4 md:grid-cols-2">
        <UiInput
          :model-value="props.modelValue.ccText"
          :label="t('admin.mailing.compose.replies.cc')"
          :hint="t('admin.mailing.compose.replies.ccHint')"
          :error="props.errors.ccText"
          :disabled="props.disabled"
          inputmode="email"
          block
          @update:model-value="set('ccText', $event)"
        />
        <UiInput
          :model-value="props.modelValue.replyTo"
          type="email"
          :label="t('admin.mailing.compose.replies.replyTo')"
          :hint="t('admin.mailing.compose.replies.replyToHint')"
          :error="props.errors.replyTo"
          :disabled="props.disabled"
          block
          @update:model-value="set('replyTo', $event)"
        />
        <UiInput
          :model-value="props.modelValue.bccText"
          :label="t('admin.mailing.compose.replies.bcc')"
          :hint="t('admin.mailing.compose.replies.bccHint')"
          :error="props.errors.bccText"
          :disabled="props.disabled"
          inputmode="email"
          block
          @update:model-value="set('bccText', $event)"
        />
      </div>
    </fieldset>
  </section>
</template>
