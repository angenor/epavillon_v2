<script setup lang="ts">
/**
 * Section 5, huitième lot : la FAQ (étape 2) — la ligne de question, vérifiée ou à
 * revoir, la source sous ses deux formes, le retour sur une réponse, et l'étape du
 * parcours. Spécimens de la maquette 05.
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
const etape = (label: string, detail: string | null = null, origine: string | null = null) => ({
  id: label, label, detail, origin_label: origine, link: null, sort_order: 0,
})
const cochees = ref(new Set([k('etape-guide')]))
const basculer = (id: string) => {
  const suite = new Set(cochees.value)
  if (!suite.delete(id)) suite.add(id)
  cochees.value = suite
}
const etapes = computed(() => [
  { etape: etape(k('etape-guide')), vers: VERS, cible: k('etape-lire') },
  { etape: etape(k('etape-atelier'), null, k('etape-origine')), vers: null, cible: null },
  { etape: etape(k('etape-coordination'), k('etape-detail')), vers: null, cible: null },
])
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

    <GnPlancheSection :titre="k('etape')" :propos="k('etape-propos')">
      <div class="gn-planche-composants__vitrine">
        <div>
          <GnEtapeParcours
            v-for="(e, i) in etapes"
            :key="e.etape.id"
            :etape="e.etape"
            :cochee="cochees.has(e.etape.id)"
            :vers="e.vers"
            :cible="e.cible"
            :derniere="i === etapes.length - 1"
            @basculer="basculer(e.etape.id)"
          />
        </div>
      </div>
    </GnPlancheSection>
  </div>
</template>
