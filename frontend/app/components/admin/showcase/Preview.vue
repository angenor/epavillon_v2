<script setup lang="ts">
import type { ShowcaseRow } from '~/types/views'

/**
 * L'APERÇU DE LA DIAPOSITIVE — RENDU PAR LES COMPOSANTS PUBLICS EUX-MÊMES.
 *
 * ── LA SEULE DÉCISION DE CE FICHIER ─────────────────────────────────────────
 *
 * Ce n'est PAS une mise en page : c'est un aiguillage. Une diapositive du
 * bandeau se rend par `HomeShowcaseSlide`, une épingle du panneau latéral par
 * `HomeAsidePin` — les composants MÊMES que sert l'accueil public. Dessiner ici
 * une seconde version du bandeau divergerait au premier ajustement de charte, et
 * l'éditeur cesserait de croire ce qu'il voit : il composerait à l'aveugle,
 * exactement comme dans la v1.
 *
 * Le contrat s'y prêtait déjà : `api.adminShowcase.form()` rend
 * `preview: ShowcaseRow`, c'est-à-dire une ligne de `content.v_showcase`, et
 * c'est la seule chose que ces deux composants acceptent. Rien du contexte
 * n'entre — ni route, ni store, ni instant.
 *
 * ── CE QUE L'APERÇU AJOUTE, ET POURQUOI CE N'EST PAS DE L'ORNEMENT ──────────
 *
 * Deux lignes de lecture que le rendu ne peut pas donner :
 *   · LE FOND RETENU. L'éditeur a joint jusqu'à trois fichiers ; il voit le
 *     résultat, pas la règle qui a tranché. « Vidéo » ou « Photographie » lui
 *     dit lequel sort, et donc lequel remplacer.
 *   · L'ADRESSE DU LIEN. Le bandeau n'affiche que son libellé ; une URL fautive
 *     ne se voit qu'au clic, c'est-à-dire trop tard.
 *
 * ── LA VIDÉO NE TOURNE PAS DANS L'APERÇU ────────────────────────────────────
 *
 * `skip-video` : une vidéo en boucle à côté d'un formulaire qu'on remplit
 * pendant dix minutes est une gêne, pas une information. L'affiche — la vignette
 * de rôle `cover` — dit déjà ce que le visiteur verra, et la ligne « Fond
 * retenu » annonce qu'il s'agit bien d'une vidéo.
 */

interface Props {
  row: ShowcaseRow
}

const props = defineProps<Props>()

const { t } = useI18n()
const { tr } = useI18nText()

/** Le fond effectivement retenu — l'ordre de repli du bandeau, pas un autre. */
const backgroundKind = computed<'video' | 'image' | 'color' | 'none'>(() => {
  if (props.row.background_video) return 'video'
  if (props.row.background_image) return 'image'
  if (props.row.background_color_hex) return 'color'
  return 'none'
})

const linkLabel = computed(() => tr(props.row.link_label).trim())
</script>

<template>
  <section
    class="rounded-lg border border-border bg-surface-raised"
    :aria-label="t('admin.showcase.form.preview.title')"
  >
    <header
      class="flex flex-wrap items-center justify-between gap-3 border-b border-border-subtle px-4 py-3"
    >
      <h2 class="font-display text-base leading-snug">
        {{ t('admin.showcase.form.preview.title') }}
      </h2>
      <UiBadge size="sm" icon="grid">
        {{ t(`admin.showcase.form.placement.${props.row.placement}`) }}
      </UiBadge>
    </header>

    <div class="px-4 py-4">
      <!-- LE RENDU PUBLIC, TEL QUEL. Le composant remplit la boîte qu'on lui
           donne et n'impose aucune hauteur : c'est ici qu'on décide du cadre. -->
      <!-- PAS DE 16/9 IMPOSÉ ICI, ET C'EST UNE CORRECTION. Dans cette colonne de
           350 px, un 16/9 fait 197 px de haut pour un contenu qui en demande
           450 : l'aperçu montrait le quart d'une diapositive, coupé haut et bas
           par `overflow-hidden`, ce qui est pire que pas d'aperçu du tout — on
           croit voir ce que le public verra.

           La hauteur naturelle est aussi la plus HONNÊTE : le bandeau public ne
           tient un ratio qu'à partir de `lg` ; en dessous, il grandit avec son
           contenu, exactement comme ici. Le plancher garde l'allure d'un
           bandeau quand la diapositive est encore vide. -->
      <div v-if="props.row.placement === 'home_hero'" class="min-h-56 overflow-hidden rounded-md">
        <HomeShowcaseSlide :slide="props.row" compact skip-video />
      </div>

      <!-- Une épingle du panneau « À venir » n'est pas un bandeau réduit :
           c'est un autre composant, et l'aperçu doit montrer celui-là. -->
      <HomeAsidePin v-else :pin="props.row" />

      <dl class="mt-4 space-y-3 border-t border-border-subtle pt-4 text-sm">
        <div class="flex flex-wrap items-baseline gap-x-2">
          <dt class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
            {{ t('admin.showcase.form.preview.background.label') }}
          </dt>
          <dd class="flex items-center gap-1.5 text-text-secondary">
            <UiIcon
              :name="
                backgroundKind === 'video' ? 'video' : backgroundKind === 'image' ? 'monitor' : 'grid'
              "
              size="0.95rem"
              aria-hidden="true"
            />
            {{ t(`admin.showcase.form.preview.background.${backgroundKind}`) }}
          </dd>
        </div>

        <div v-if="props.row.link_url">
          <dt class="text-xs font-semibold tracking-wide text-text-subtle uppercase">
            {{ t('admin.showcase.form.preview.link') }}
          </dt>
          <dd class="mt-0.5">
            <span class="font-semibold text-text">
              {{ linkLabel || t('admin.showcase.form.preview.linkNoLabel') }}
            </span>
            <span class="mt-0.5 block truncate font-mono text-xs text-text-subtle">
              {{ props.row.link_url }}
            </span>
          </dd>
        </div>
      </dl>
    </div>
  </section>
</template>
