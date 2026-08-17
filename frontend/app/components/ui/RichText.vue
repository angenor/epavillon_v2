<script setup lang="ts">
import { Editor, EditorContent } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import type { FieldProps } from '~/types/ui'

/**
 * ÉDITEUR DE TEXTE RICHE — la présentation détaillée d'une proposition, et
 * demain le corps d'un article ou le compte rendu d'une activité.
 *
 * CE QU'IL PERMET, ET RIEN DE PLUS : gras, italique, deux niveaux de
 * sous-titres, listes à puces et numérotées, citation, trait de séparation.
 * **Ni police, ni taille, ni couleur, ni alignement, ni surlignage.** Ce n'est
 * pas une restriction technique mais la règle du projet : la mise en forme
 * appartient à la charte, jamais au déposant. Un éditeur qui offre une palette
 * de couleurs produit, en deux campagnes, un programme où chaque activité a sa
 * typographie — et le premier thème sombre le rend illisible, un texte peint en
 * noir restant noir sur fond noir.
 *
 * LA CONSÉQUENCE EST STRUCTURELLE, pas cosmétique : le contenu ne porte QUE de
 * la structure (« ceci est un titre », « ceci est une liste »), et l'apparence
 * est décidée à l'affichage, par les jetons. C'est ce qui permet de le rendre
 * dans les deux thèmes, dans un courriel, dans un PDF de programme, sans
 * réécrire une ligne de contenu.
 *
 * LE HTML PRODUIT N'EST PAS DE CONFIANCE. Il est rédigé par un tiers : l'API
 * l'assainira à l'écriture, avec la même liste blanche que cette barre d'outils.
 * L'éditeur limite ce qu'on peut TAPER, il ne protège pas de ce qu'on peut
 * COLLER — c'est une commodité de saisie, pas un filtre de sécurité.
 *
 * COMPTEUR SUR LE TEXTE, PAS SUR LE BALISAGE. Compter les caractères du HTML
 * ferait grossir le décompte à chaque mise en gras, et un texte de 900 signes
 * pourrait afficher 1 400 sans qu'une ligne ait été ajoutée. On compte donc ce
 * que la personne a écrit, et rien d'autre.
 *
 * RENDU CLIENT UNIQUEMENT : ProseMirror manipule le DOM et n'a rien à faire au
 * rendu serveur. L'instance est créée dans `onMounted`, et l'appelant enveloppe
 * ce composant dans un `<ClientOnly>` dont le repli montre un cadre de la bonne
 * hauteur — sans quoi la page saute à l'hydratation.
 */

interface Props extends FieldProps {
  /** Fragment HTML restreint. Chaîne vide pour un contenu vierge. */
  modelValue?: string
  placeholder?: string
  /** Limite sur le TEXTE, balisage exclu. Le compteur suit cette valeur. */
  maxlength?: number
  /** Hauteur minimale, en lignes. */
  rows?: number
}

const props = withDefaults(defineProps<Props>(), { rows: 8 })
const emit = defineEmits<{ 'update:modelValue': [value: string] }>()

const { t } = useI18n()

const editor = shallowRef<Editor | null>(null)
/** Longueur du TEXTE brut, tenue à jour par l'éditeur. */
const textLength = ref(0)

const exceeded = computed(() => Boolean(props.maxlength && textLength.value > props.maxlength))

const counter = computed(() => {
  if (!props.maxlength) return undefined
  return exceeded.value
    ? t('form.counterExceeded', {
        count: textLength.value,
        max: props.maxlength,
        over: textLength.value - props.maxlength,
      })
    : t('form.counter', { count: textLength.value, max: props.maxlength })
})

/**
 * Les seules commandes offertes. L'ordre est celui de la fréquence d'usage :
 * on met en gras cent fois pour une citation.
 */
const TOOLS = [
  { name: 'bold', icon: 'bold', action: 'toggleBold' },
  { name: 'italic', icon: 'italic', action: 'toggleItalic' },
  { name: 'bulletList', icon: 'list', action: 'toggleBulletList' },
  { name: 'orderedList', icon: 'list-ordered', action: 'toggleOrderedList' },
  { name: 'blockquote', icon: 'quote', action: 'toggleBlockquote' },
] as const

function isActive(name: string, attrs?: Record<string, unknown>): boolean {
  return editor.value?.isActive(name, attrs) ?? false
}

function run(action: string): void {
  const chain = editor.value?.chain().focus()
  if (!chain) return
  // Les commandes de StarterKit sont typées une à une ; l'appel dynamique passe
  // par un index, ce qui reste sûr : la liste ci-dessus est fermée.
  const command = (chain as unknown as Record<string, () => { run: () => void }>)[action]
  if (typeof command === 'function') command.call(chain).run()
}

function toggleHeading(level: 3 | 4): void {
  editor.value?.chain().focus().toggleHeading({ level }).run()
}

onMounted(() => {
  editor.value = new Editor({
    content: props.modelValue ?? '',
    editable: !props.disabled && !props.readonly,
    extensions: [
      StarterKit.configure({
        // NIVEAUX 3 ET 4 SEULEMENT : le titre de l'activité est un h1, le titre
        // de section un h2. Laisser choisir h1 dans un corps de texte casse la
        // hiérarchie du document et, avec elle, la navigation au lecteur d'écran.
        heading: { levels: [3, 4] },
        // Ni image, ni bloc de code : une présentation d'activité n'en contient
        // pas, et chacun ouvrirait une surface d'attaque et un cas d'affichage.
        codeBlock: false,
      }),
    ],
    editorProps: {
      attributes: {
        class: 'outline-none',
        'aria-multiline': 'true',
      },
    },
    onUpdate: ({ editor: instance }) => {
      textLength.value = instance.getText().length
      // Un document vierge rend `<p></p>` : on renvoie une chaîne vide, sans
      // quoi tout champ « vide » compterait comme rempli à la validation.
      const html = instance.getHTML()
      emit('update:modelValue', instance.getText().trim().length === 0 ? '' : html)
    },
  })
  textLength.value = editor.value.getText().length
})

/** Contenu remplacé par l'extérieur — reprise d'un brouillon, remise à zéro. */
watch(
  () => props.modelValue,
  (value) => {
    const instance = editor.value
    if (!instance) return
    if ((value ?? '') === instance.getHTML()) return
    instance.commands.setContent(value ?? '', { emitUpdate: false })
    textLength.value = instance.getText().length
  },
)

watch(
  () => [props.disabled, props.readonly],
  () => editor.value?.setEditable(!props.disabled && !props.readonly),
)

onBeforeUnmount(() => {
  editor.value?.destroy()
  editor.value = null
})
</script>

<template>
  <UiFormField
    :id="props.id"
    :label="props.label"
    :hint="props.hint"
    :error="props.error"
    :required="props.required"
    :disabled="props.disabled"
    :readonly="props.readonly"
    :counter="counter"
    :counter-exceeded="exceeded"
  >
    <template #default="{ control }">
      <div
        class="rounded-md border bg-surface-raised"
        :class="[
          props.error || exceeded
            ? 'border-(length:--border-medium) border-danger'
            : 'border-(length:--border-thin) border-border-strong focus-within:border-focus',
          props.disabled ? 'opacity-60' : '',
        ]"
      >
        <!-- BARRE D'OUTILS. Aucune commande de couleur ni de police : ce qui
             n'est pas offert ne se corrige pas ensuite. -->
        <div
          v-if="!props.readonly && !props.disabled"
          class="flex flex-wrap items-center gap-1 border-b border-border px-2 py-1.5"
          role="toolbar"
          :aria-label="t('form.richText.toolbar')"
        >
          <button
            v-for="tool in TOOLS"
            :key="tool.name"
            type="button"
            class="flex size-9 cursor-pointer items-center justify-center rounded transition-colors duration-(--duration-fast)"
            :class="
              isActive(tool.name)
                ? 'bg-accent-surface text-accent'
                : 'text-text-secondary hover:bg-surface-hover'
            "
            :aria-pressed="isActive(tool.name)"
            :title="t(`form.richText.${tool.name}`)"
            @click="run(tool.action)"
          >
            <UiIcon :name="tool.icon" size="1.05rem" />
            <span class="sr-only">{{ t(`form.richText.${tool.name}`) }}</span>
          </button>

          <span class="mx-1 h-5 w-px bg-separator" aria-hidden="true" />

          <button
            v-for="level in ([3, 4] as const)"
            :key="level"
            type="button"
            class="flex h-9 min-w-9 cursor-pointer items-center justify-center rounded px-2 font-display text-sm transition-colors duration-(--duration-fast)"
            :class="
              isActive('heading', { level })
                ? 'bg-accent-surface text-accent'
                : 'text-text-secondary hover:bg-surface-hover'
            "
            :aria-pressed="isActive('heading', { level })"
            :title="t('form.richText.heading', { level: level - 2 })"
            @click="toggleHeading(level)"
          >
            {{ t('form.richText.headingShort', { level: level - 2 }) }}
          </button>
        </div>

        <!-- ZONE DE SAISIE. La classe `rich-text` porte la charte : c'est la
             MÊME que celle du rendu en lecture, définie une fois dans main.css.
             Deux jeux de styles pour un même contenu, c'est un texte qui change
             d'allure entre la saisie et la publication. -->
        <EditorContent
          :id="control.id"
          :editor="editor ?? undefined"
          class="rich-text px-3 py-2.5"
          :style="{ minHeight: `${props.rows * 1.6}rem` }"
        />
      </div>
    </template>
  </UiFormField>
</template>
