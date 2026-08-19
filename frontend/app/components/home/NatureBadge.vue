<script setup lang="ts">
import type { ColorHex, I18nText } from '~/types/shared'

/**
 * L'ÉYCLETTE D'UN CONTENU DE VITRINE — sa NATURE : témoignage, innovation,
 * bonne pratique, annonce, chiffre clé, parole de négociateur.
 *
 * LE LIBELLÉ ET LA COULEUR VIENNENT DE LA BASE, pas d'ici. Ce sont les colonnes
 * `nature_label` et `nature_color` de `content.v_showcase`, elles-mêmes tirées
 * de `reference.taxonomy_terms` : un administrateur ajoute une nature depuis le
 * back-office, et elle s'affiche sans qu'une ligne change dans le front. Figer
 * ces six termes dans un fichier i18n ou dans la feuille de style serait le
 * défaut n° 1 de la v1, mot pour mot.
 *
 * DEUX FONDS, DEUX RÈGLES DE LISIBILITÉ. Sur un aplat clair, la pastille suit le
 * dessin de `UiThemeTag` : teinte à 12 %, bordure à 40 %, texte en
 * `--color-text`. Sur le bandeau — une photographie voilée — le texte passe en
 * `--color-text-on-inverse` et la teinte monte, sans quoi une couleur sombre
 * (`#1d1a5b`, le chiffre clé) disparaîtrait dans le voile. Dans les deux cas la
 * couleur de la base ne porte QUE le point et la teinte : jamais le texte, dont
 * rien ne garantirait alors le contraste.
 *
 * Ce n'est pas `UiThemeTag` réutilisé : celui-ci ne connaît que la surface
 * claire, et lui ajouter un ton reviendrait à faire porter à un composant
 * d'interface partagé une contrainte qui n'appartient qu'à ce bandeau.
 */

interface Props {
  /** `v_showcase.nature_label` — nul si le terme a été désactivé. */
  label: I18nText | null
  /** `v_showcase.nature_color` — `null` est un cas normal du modèle. */
  color: ColorHex | null
  /** `inverse` : posée sur le bandeau voilé. `surface` : sur un fond de page. */
  tone?: 'inverse' | 'surface'
  size?: 'sm' | 'md'
}

const props = withDefaults(defineProps<Props>(), { tone: 'surface', size: 'md' })

const { tr } = useI18nText()

const text = computed(() => tr(props.label))

/**
 * Sans couleur en base, la pastille retombe sur les jetons neutres. Un terme
 * sans `color_hex` est parfaitement valide — ce n'est pas une anomalie à
 * signaler par un gris d'erreur.
 */
const style = computed(() => {
  const color = props.color
  if (!color) return undefined
  const tint = props.tone === 'inverse' ? '30%' : '12%'
  const edge = props.tone === 'inverse' ? '65%' : '40%'
  return {
    backgroundColor: `color-mix(in srgb, ${color} ${tint}, transparent)`,
    borderColor: `color-mix(in srgb, ${color} ${edge}, transparent)`,
  }
})
</script>

<template>
  <span
    v-if="text"
    class="inline-flex items-center gap-2 rounded-full border font-bold uppercase"
    :class="[
      props.tone === 'inverse'
        ? 'border-border-on-inverse bg-surface-inverse-raised text-text-on-inverse'
        : 'border-border bg-surface-sunken text-text',
      props.size === 'sm' ? 'px-2.5 py-0.5 text-[0.6875rem]' : 'px-3 py-1 text-xs',
    ]"
    :style="[style, { letterSpacing: 'var(--tracking-caps)' }]"
  >
    <span
      v-if="props.color"
      class="size-2 shrink-0 rounded-full"
      :style="{ backgroundColor: props.color }"
      aria-hidden="true"
    />
    {{ text }}
  </span>
</template>
