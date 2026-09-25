<script setup lang="ts">
import type { CorrectionNote } from '~/types/negotiation-documents'
import type { ModeDeLecture } from '~/utils/guide-nego/appareil-lecture'
import { assetUrl } from '~/utils/asset'

/**
 * Section 5, sixième lot : le lecteur du PDF d'origine (étape 1b) — le choix du mode, et
 * `GnLecteurPages` sur le petit PDF d'exemple, avec ses notes en marge. Les notes sont de
 * vrais `GnMargeNote`, placés par le lecteur sur la page dessinée, pas une imitation.
 */
const { t } = useI18n()
const k = (cle: string) => t(`gn-planche-composants-lecteur-pdf.${cle}`)

const modeBarre = ref<ModeDeLecture>('pages')
const modeFeuille = ref<ModeDeLecture>('texte')

// Lu d'un bloc, comme une copie gardée : le serveur de fichiers du site ignore les plages.
const octets = ref<Uint8Array | null>(null)
onMounted(async () => {
  const reponse = await fetch(assetUrl('/gn-exemples/documents/petit.pdf')).catch(() => null)
  if (reponse?.ok) octets.value = new Uint8Array(await reponse.arrayBuffer())
})

// Le passage est le texte du PDF d'exemple, pas une traduction : la page ne change pas de langue.
const PASSAGE_PAGE_1 = 'Le bilan mondial (en anglais Global Stocktake) est adopté à Dubaï.'
const notes = computed(() => {
  const note = (id: string, page: number, passage: string | null): CorrectionNote => ({
    id,
    document_id: 'planche',
    page_index: page,
    passage,
    body: k('note-texte'),
    author_name: k('note-auteur'),
    posted_at: '2026-11-10T09:00:00Z',
  })
  return new Map([
    [1, [note('planche-passage', 1, PASSAGE_PAGE_1)]],
    [2, [note('planche-tete', 2, null)]],
  ])
})
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection :titre="k('choix')" :propos="k('choix-propos')">
      <span class="gn-planche-composants__legende">{{ k('choix-barre') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnChoixMode v-model="modeBarre" variante="barre" />
      </div>
      <span class="gn-planche-composants__legende">{{ k('choix-feuille') }}</span>
      <div class="gn-planche-composants__vitrine">
        <GnChoixMode v-model="modeFeuille" />
      </div>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('pages')" :propos="k('pages-propos')">
      <div class="gn-planche-cadre-pdf">
        <GnLecteurPages v-if="octets" :source="{ octets }" :page-initiale="1" :notes-par-page="notes" vignette />
      </div>
      <p class="gn-planche-note">{{ k('pages-note') }}</p>
    </GnPlancheSection>
  </div>
</template>

<style>
/* Le `transform` enferme le visionneur, fixe dans le lecteur, dans le cadre de la planche. */
[data-app="guide-nego"] .gn-planche-cadre-pdf {
  position: relative;
  transform: translateZ(0);
  block-size: 520px;
  overflow: hidden;
  border: var(--gn-filet-1) solid var(--gn-filet);
}
</style>
