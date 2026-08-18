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

/**
 * L'IDENTIFIANT EST TENU ICI, ET NON LAISSÉ À `UiFormField`.
 *
 * Il doit être posé sur l'élément `contenteditable`, que ProseMirror crée au
 * montage : il faut donc le connaître AVANT, ce que la portée du créneau de
 * `UiFormField` ne permet pas. Le poser sur l'enveloppe — ce que faisait la
 * version précédente — le mettait sur un `div` qui n'est pas la zone de saisie.
 */
const generatedId = useId()
const fieldId = computed(() => props.id ?? `rich-text-${generatedId}`)

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
        /**
         * LA CLASSE ET LES ESPACEMENTS VONT SUR L'ÉLÉMENT ÉDITABLE, PAS SUR SON
         * ENVELOPPE — et cela répare deux défauts d'un coup.
         *
         *  1. LA ZONE CLIQUABLE. `EditorContent` rend une enveloppe qui contient
         *     l'élément `contenteditable`. Porter la hauteur minimale sur
         *     l'enveloppe laissait l'élément éditable à la hauteur de son
         *     contenu : sur un champ de huit lignes vide, seule la PREMIÈRE
         *     était cliquable, et cliquer dans le vide en dessous ne donnait pas
         *     le focus. La hauteur et le rembourrage sont donc ici, sur ce que
         *     l'on clique.
         *
         *  2. L'ESPACEMENT DES PARAGRAPHES. La règle `.rich-text > * + *` de
         *     `main.css` vise les ENFANTS DIRECTS. Posée sur l'enveloppe, elle ne
         *     rencontrait qu'un seul enfant — l'élément éditable — et ne
         *     s'appliquait donc à aucun paragraphe : le texte s'écrivait serré
         *     puis s'affichait aéré une fois publié. C'est exactement ce que
         *     l'en-tête de `main.css` interdit — « un texte qui change d'allure
         *     entre le moment où on l'écrit et celui où il est publié ».
         */
        class: 'rich-text px-3 py-2.5 outline-none',
        id: fieldId.value,
        // Un `contenteditable` n'est pas un contrôle de formulaire : `role` et
        // `aria-multiline` sont ce qui le fait annoncer comme une zone de texte.
        role: 'textbox',
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

/**
 * L'AIDE ET L'ERREUR, ANNONCÉES SUR LA ZONE DE SAISIE.
 *
 * `UiFormField` compose ses identifiants à partir de celui du champ ; on les
 * recompose donc à l'identique et on les pose sur l'élément `contenteditable`.
 * Ils ne peuvent pas passer par `editorProps`, figé au montage : une erreur
 * apparaît APRÈS, à la validation. On écrit donc l'attribut sur le nœud, ce que
 * ProseMirror laisse faire.
 *
 * Sans cela, le message « ce champ est obligatoire » s'affiche à l'écran et n'est
 * annoncé à personne — c'est le seul champ du projet dans ce cas, parce que c'est
 * le seul qui ne soit pas un contrôle de formulaire natif.
 */
watchEffect(() => {
  const dom = editor.value?.view.dom
  if (!dom) return

  const described = [
    props.hint ? `${fieldId.value}-hint` : null,
    props.error ? `${fieldId.value}-error` : null,
  ].filter(Boolean)

  if (described.length > 0) dom.setAttribute('aria-describedby', described.join(' '))
  else dom.removeAttribute('aria-describedby')

  if (props.error) dom.setAttribute('aria-invalid', 'true')
  else dom.removeAttribute('aria-invalid')

  if (props.required) dom.setAttribute('aria-required', 'true')
  else dom.removeAttribute('aria-required')
})

onBeforeUnmount(() => {
  editor.value?.destroy()
  editor.value = null
})
</script>

<template>
  <UiFormField
    :id="fieldId"
    :label="props.label"
    :hint="props.hint"
    :error="props.error"
    :required="props.required"
    :disabled="props.disabled"
    :readonly="props.readonly"
    :counter="counter"
    :counter-exceeded="exceeded"
  >
    <template #default>
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
             d'allure entre la saisie et la publication.
             Elle est posée sur l'élément ÉDITABLE (voir `editorProps` plus haut),
             et non sur cette enveloppe : c'est ce qui rend toute la hauteur
             cliquable et ce qui fait porter l'espacement aux paragraphes. -->
        <EditorContent
          :editor="editor ?? undefined"
          class="ui-rich-text-host"
          :style="{ '--rich-text-min-height': `${props.rows * 1.65}rem` }"
        />
      </div>
    </template>
  </UiFormField>
</template>

<style scoped>
/**
 * L'enveloppe ne fait que transmettre : c'est l'élément éditable qui porte la
 * hauteur, et donc la surface que l'on clique.
 *
 * `:deep()` est nécessaire — le `contenteditable` est créé par ProseMirror et ne
 * traverse pas le compilateur de ce composant, il ne porte donc pas l'attribut de
 * portée.
 */
.ui-rich-text-host :deep(.ProseMirror) {
  min-height: var(--rich-text-min-height);
}

/**
 * `max-width: var(--measure)` vient de `.rich-text`, et il a sa raison d'être en
 * LECTURE : une ligne de texte trop longue se lit mal. En SAISIE, il laisserait
 * une bande morte à droite du champ — visiblement dans le cadre, sans effet au
 * clic. On rend donc la zone éditable pleine largeur et on garde la mesure sur
 * les paragraphes eux-mêmes.
 */
.ui-rich-text-host :deep(.ProseMirror) {
  max-width: none;
}

.ui-rich-text-host :deep(.ProseMirror) > * {
  max-width: var(--measure);
}
</style>
