<script setup lang="ts">
import type { ComposerOptionsValue, MailingTemplate, MailTemplate } from '~/types/admin-mailings'
import { MAILING_TEMPLATES } from '~/types/admin-mailings'

/**
 * LES GABARITS EN TÊTE DE LA FENÊTRE DE RÉDACTION (29/10) : une bande qui
 * défile de côté, nom et silhouette seuls ; « Étendre » ajoute les
 * descriptions. Les modèles par défaut, puis les gabarits de l'édition de
 * l'envoi et les communs.
 */

interface Props {
  modelValue: ComposerOptionsValue
  /** Le gabarit a été présélectionné pour les déposants, pas choisi. */
  preselected?: boolean
}

const props = defineProps<Props>()
const emit = defineEmits<{
  'update:modelValue': [value: ComposerOptionsValue]
  /** Un gabarit choisi à la main : la présélection ne le remplace plus. */
  'template-chosen': []
}>()

const { t } = useI18n()
const api = useApi()
const adminScope = useAdminScopeStore()
const localePath = useLocalePath()
const titleId = useId()
const defaultsId = useId()
const customsId = useId()

const expanded = ref(false)
const customTemplates = ref<MailTemplate[]>([])
const customError = ref<string | null>(null)

watch(
  () => props.modelValue.eventId,
  async (eventId) => {
    customError.value = null
    if (!eventId) return
    try {
      customTemplates.value = await api.adminMailings.templates.list(eventId, adminScope.scope)
    } catch {
      customTemplates.value = []
      customError.value = t('admin.mailing.composer.options.customError')
    }
  },
  { immediate: true },
)

function chooseDefault(template: MailingTemplate): void {
  emit('template-chosen')
  emit('update:modelValue', { ...props.modelValue, template, customTemplateId: '' })
}

function chooseCustom(custom: MailTemplate): void {
  emit('template-chosen')
  emit('update:modelValue', { ...props.modelValue, template: custom.layout, customTemplateId: custom.id })
}

const isDefaultChosen = (template: MailingTemplate) =>
  !props.modelValue.customTemplateId && props.modelValue.template === template
const isCustomChosen = (custom: MailTemplate) => props.modelValue.customTemplateId === custom.id

// Le gabarit retenu reste en vue : une présélection peut tomber au bout de la bande.
const scroller = ref<HTMLElement | null>(null)

function reveal(): void {
  const box = scroller.value
  const card = box?.querySelector<HTMLElement>('[data-chosen="true"]')
  if (!box || !card) return
  const start = card.offsetLeft
  const end = start + card.offsetWidth
  if (start >= box.scrollLeft && end <= box.scrollLeft + box.clientWidth) return
  box.scrollTo({ left: Math.max(0, start - 8), behavior: 'smooth' })
}

watch(
  () => [props.modelValue.template, props.modelValue.customTemplateId, customTemplates.value.length, expanded.value],
  () => nextTick(reveal),
)
onMounted(reveal)

const cardClass = (chosen: boolean) => [
  expanded.value ? 'w-60 items-start' : 'items-center',
  chosen ? 'border-accent bg-accent-surface' : 'border-border bg-surface-raised hover:border-border-strong',
]
</script>

<template>
  <section :aria-labelledby="titleId">
    <div class="flex flex-wrap items-center justify-between gap-x-4">
      <div class="flex min-w-0 flex-wrap items-baseline gap-x-3">
        <h3 :id="titleId" class="text-sm font-semibold text-text">{{ t('admin.mailing.composer.options.template') }}</h3>
        <p v-if="props.preselected" class="text-xs text-text-subtle" role="status">
          {{ t('admin.mailing.composer.options.preselected') }}
        </p>
      </div>
      <div class="flex items-center">
        <button
          type="button"
          class="inline-flex min-h-(--target-min) cursor-pointer items-center gap-1 px-2 text-sm text-text-link hover:text-text-link-hover"
          :aria-expanded="expanded"
          @click="expanded = !expanded"
        >
          <UiIcon :name="expanded ? 'chevron-up' : 'chevron-down'" size="1rem" />
          {{ expanded ? t('admin.mailing.composer.options.reduce') : t('admin.mailing.composer.options.expand') }}
        </button>
        <NuxtLink
          :to="localePath('/admin/courriels/gabarits')"
          class="inline-flex min-h-(--target-min) items-center px-2 text-sm text-text-link underline-offset-2 hover:underline"
        >
          {{ t('admin.mailing.composer.options.manage') }}
        </NuxtLink>
      </div>
    </div>

    <div
      ref="scroller"
      role="radiogroup"
      :aria-labelledby="titleId"
      class="relative flex gap-4 overflow-x-auto overscroll-x-contain px-0.5 pt-0.5 pb-2"
    >
      <div role="group" :aria-labelledby="defaultsId" class="flex shrink-0 flex-col gap-1">
        <p :id="defaultsId" class="text-xs font-semibold text-text-muted">{{ t('admin.mailing.composer.options.defaults') }}</p>
        <div class="flex gap-2">
          <label
            v-for="tpl in MAILING_TEMPLATES"
            :key="tpl"
            :data-chosen="isDefaultChosen(tpl)"
            class="flex min-h-(--target-min) shrink-0 cursor-pointer gap-2.5 rounded-lg border px-2.5 py-2 transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-focus"
            :class="cardClass(isDefaultChosen(tpl))"
          >
            <input
              type="radio"
              name="mailing-template"
              class="sr-only"
              :value="tpl"
              :checked="isDefaultChosen(tpl)"
              @change="chooseDefault(tpl)"
            >
            <AdminMailingsTemplateThumb :layout="tpl" />
            <span class="min-w-0">
              <span class="block text-sm font-semibold" :class="expanded ? 'text-balance' : 'whitespace-nowrap'">
                {{ t(`admin.mailing.composer.templates.${tpl}.name`) }}
              </span>
              <span v-if="expanded" class="mt-0.5 block text-xs text-text-subtle">
                {{ t(`admin.mailing.composer.templates.${tpl}.hint`) }}
              </span>
            </span>
          </label>
        </div>
      </div>

      <div
        v-if="customTemplates.length"
        role="group"
        :aria-labelledby="customsId"
        class="flex shrink-0 flex-col gap-1 border-l border-border pl-4"
      >
        <p :id="customsId" class="text-xs font-semibold text-text-muted">{{ t('admin.mailing.composer.options.custom') }}</p>
        <div class="flex gap-2">
          <label
            v-for="custom in customTemplates"
            :key="custom.id"
            :data-chosen="isCustomChosen(custom)"
            class="flex min-h-(--target-min) shrink-0 cursor-pointer gap-2.5 rounded-lg border px-2.5 py-2 transition-colors has-[:focus-visible]:outline-2 has-[:focus-visible]:outline-offset-2 has-[:focus-visible]:outline-focus"
            :class="cardClass(isCustomChosen(custom))"
          >
            <input
              type="radio"
              name="mailing-template"
              class="sr-only"
              :value="custom.id"
              :checked="isCustomChosen(custom)"
              @change="chooseCustom(custom)"
            >
            <AdminMailingsTemplateThumb :layout="custom.layout" />
            <span class="min-w-0">
              <span class="block text-sm font-semibold" :class="expanded ? 'text-balance' : 'whitespace-nowrap'">
                {{ custom.name }}
              </span>
              <span v-if="expanded" class="mt-0.5 block text-xs text-text-subtle">
                {{ t('admin.mailing.composer.options.customHint', { layout: t(`admin.mailing.composer.templates.${custom.layout}.name`) }) }}
              </span>
            </span>
          </label>
        </div>
      </div>
    </div>
    <UiAlert v-if="customError" class="mt-1" intent="warning" compact :message="customError" />
  </section>
</template>
