<script setup lang="ts">
import type { I18nText } from '~/types/shared'

/**
 * UN CHAMP MULTILINGUE DE LA BASE — `platform.i18n_text`.
 *
 * POURQUOI CE COMPOSANT EXISTE. Presque tout ce que la gestion des événements
 * écrit est un `platform.i18n_text` : titre d'édition, nom de salle, libellé de
 * critère, titre de journée spéciale. Ce sont des DONNÉES, pas des traductions
 * d'interface — un administrateur les modifie depuis le back-office, elles ne
 * vivent pas dans un fichier i18n. Les saisir demande donc deux champs, un par
 * langue, dans chacun des sept formulaires de cet écran. Sans ce composant, la
 * paire serait recopiée une trentaine de fois, et le premier oubli de l'anglais
 * passerait inaperçu.
 *
 * LE FRANÇAIS EST LA LANGUE PIVOT, ET C'EST UNE RÈGLE DU MODÈLE, pas une
 * préférence : `platform.t()` se replie sur le français quand la locale demandée
 * manque. Le français est donc obligatoire dès que le champ l'est ; l'anglais ne
 * l'est jamais, et son absence se signale plutôt qu'elle ne bloque.
 *
 * Le contrôle est un `input` ou un `textarea` selon `multiline`, ce qui évite un
 * second composant pour la seule différence de hauteur.
 */

interface Props {
  modelValue: I18nText | null
  label: string
  hint?: string
  /** Message d'erreur porté par le français — le seul qui puisse manquer. */
  error?: string
  required?: boolean
  disabled?: boolean
  multiline?: boolean
  /**
   * TEXTE ENRICHI plutôt qu'une zone de texte nue.
   *
   * Réservé aux textes qui sont LUS PAR LE PUBLIC et qui gagnent à être
   * structurés — la description d'une édition, comme la présentation détaillée
   * d'une proposition (A4). Structure seulement : ni police, ni couleur, ni
   * taille. C'est ce qui permet de rendre le même texte dans les deux thèmes,
   * dans un courriel et dans un programme imprimé sans le réécrire.
   */
  rich?: boolean
  rows?: number
  maxlength?: number
}

const props = withDefaults(defineProps<Props>(), { rows: 4 })
const emit = defineEmits<{ 'update:modelValue': [value: I18nText | null] }>()

const { t } = useI18n()

const active = ref<'fr' | 'en'>('fr')

const tabs = computed(() => [
  { value: 'fr', label: t('admin.event.form.localeTab.fr') },
  { value: 'en', label: t('admin.event.form.localeTab.en') },
])

/**
 * `I18nText` exige `fr`. Une saisie EN COURS ne l'a pas encore : on travaille donc
 * sur un enregistrement libre, et l'on ne remonte un `I18nText` que lorsque le
 * français est là. Tant qu'il manque, la valeur est `null` — ce que la validation
 * du formulaire signale comme un champ obligatoire vide, et non comme un titre
 * dont la langue pivot serait perdue.
 */
type PartialI18n = Record<string, string | undefined>

const value = computed<PartialI18n>(() => props.modelValue ?? {})

function setLocale(locale: 'fr' | 'en', next: string): void {
  const merged: PartialI18n = { ...value.value }
  // Une chaîne vide n'est pas une traduction : on retire la clé plutôt que de
  // laisser `{"en": ""}` en base, que `platform.t()` servirait comme une
  // traduction anglaise valide — un titre vide affiché au public anglophone.
  if (next.trim()) merged[locale] = next
  else delete merged[locale]

  const fr = merged.fr
  emit('update:modelValue', fr === undefined ? null : ({ ...merged, fr } as I18nText))
}

/** L'anglais manque-t-il alors que le français est renseigné ? */
const missingEnglish = computed(() => Boolean(value.value.fr?.trim()) && !value.value.en?.trim())
</script>

<template>
  <div>
    <div class="mb-2 flex items-center justify-between gap-3">
      <div
        class="inline-flex rounded-md border border-border bg-surface-sunken p-0.5"
        role="tablist"
        :aria-label="props.label"
      >
        <button
          v-for="tab in tabs"
          :key="tab.value"
          type="button"
          role="tab"
          :aria-selected="active === tab.value"
          class="min-h-(--target-compact) cursor-pointer rounded px-3 text-sm transition-colors"
          :class="
            active === tab.value
              ? 'bg-surface-raised font-semibold text-text shadow-xs'
              : 'text-text-muted hover:text-text'
          "
          @click="active = tab.value as 'fr' | 'en'"
        >
          {{ tab.label }}
        </button>
      </div>

      <!-- L'anglais absent n'est pas une erreur : la base se replie sur le
           français. On le dit, on ne le refuse pas. -->
      <p v-if="missingEnglish && active === 'en'" class="text-xs text-text-subtle">
        {{ t('admin.event.form.localeTab.missing') }}
      </p>
    </div>

    <!-- TEXTE ENRICHI. `ClientOnly` : l'éditeur manipule le DOM et n'existe que
         dans le navigateur ; au rendu serveur, un cadre de la bonne hauteur. -->
    <ClientOnly v-if="props.rich">
      <UiRichText
        :model-value="value[active] ?? ''"
        :label="props.label"
        :hint="props.hint"
        :error="active === 'fr' ? props.error : undefined"
        :required="props.required && active === 'fr'"
        :disabled="props.disabled"
        :maxlength="props.maxlength"
        :rows="props.rows"
        @update:model-value="(next: string) => setLocale(active, next)"
      />
      <template #fallback>
        <UiTextarea
          :label="props.label"
          :hint="props.hint"
          :rows="props.rows"
          readonly
        />
      </template>
    </ClientOnly>

    <UiTextarea
      v-else-if="props.multiline"
      :model-value="value[active] ?? ''"
      :label="props.label"
      :hint="props.hint"
      :error="active === 'fr' ? props.error : undefined"
      :required="props.required && active === 'fr'"
      :disabled="props.disabled"
      :rows="props.rows"
      :maxlength="props.maxlength"
      auto-grow
      @update:model-value="(next: string) => setLocale(active, next)"
    />
    <UiInput
      v-else
      :model-value="value[active] ?? ''"
      :label="props.label"
      :hint="props.hint"
      :error="active === 'fr' ? props.error : undefined"
      :required="props.required && active === 'fr'"
      :disabled="props.disabled"
      :maxlength="props.maxlength"
      @update:model-value="(next: string) => setLocale(active, next)"
    />
  </div>
</template>
