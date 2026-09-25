<script setup lang="ts">
/**
 * Section 5, huitième lot : la FAQ (étape 2) — la ligne de question, vérifiée ou à
 * revoir, la source sous ses deux formes, et le retour sur une réponse. Spécimens de la
 * maquette 05.
 */
const { t } = useI18n()
const k = (cle: string) => t(`gn-planche-composants-faq.${cle}`)

const VERS = '/guide-nego/ressources/faq'

const verifiee = computed(() => ({ question: k('question'), verified_on: '2026-11-12', status: 'published' as const }))
const aRevoir = computed(() => ({ question: k('question-badge'), verified_on: '2026-11-12', status: 'to_review' as const }))
const citation = computed(() => ({
  document_id: '00000000-0000-0000-0000-000000000000',
  document_title: k('guide'),
  section_label: k('annexe'),
  page_from: 74,
  quote: k('citation'),
}))
const merci = ref(false)
const ligne = computed(() => ({ external_title: k('iisd'), section_label: k('contexte'), page_from: 38, page_to: 39 }))
</script>

<template>
  <div class="gn-planche-composants__lot">
    <GnPlancheSection :titre="k('ligne')" :propos="k('ligne-propos')">
      <div class="gn-planche-composants__vitrine">
        <GnLigneQuestion :entree="verifiee" :vers="VERS" />
        <GnLigneQuestion :entree="aRevoir" :vers="VERS" />
      </div>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('source')" :propos="k('source-propos')">
      <div class="gn-planche-composants__vitrine">
        <GnSource :source="citation" />
        <GnSource :source="ligne" />
      </div>
    </GnPlancheSection>

    <GnPlancheSection :titre="k('retour')" :propos="k('retour-propos')">
      <div class="gn-planche-composants__vitrine">
        <GnRetourUtile :merci="merci" @oui="merci = true" @non="merci = true" />
        <GnRetourUtile merci />
      </div>
    </GnPlancheSection>
  </div>
</template>
