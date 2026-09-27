<script setup lang="ts">
import type { DocumentReading } from '~/types/negotiation-documents'
import type { GlossaryEntry } from '~/types/negotiation-savoir'
import { chercherDansLeDocument } from '~/utils/guide-nego/lecteur'

/**
 * Le terme anglais touché dans le lecteur — maquette 04, écran 10. Il se résout sur le
 * lexique gardé, sans réseau, par son terme, son sigle ou une variante (R6) ; absent,
 * la feuille le dit et offre de le proposer. Les pages sont un fait du document.
 */
const props = defineProps<{ lecture: DocumentReading | null }>()
const terme = defineModel<string | null>('terme', { required: true })

const { t, locale } = useI18n()
const savoir = useGnSavoir()

const ouverte = computed({
  get: () => terme.value !== null,
  set: (oui: boolean) => {
    if (!oui) terme.value = null
  },
})

watch(ouverte, (oui) => {
  if (oui) void savoir.assurer()
})

const entree = computed<GlossaryEntry | null>(() => (terme.value ? savoir.resoudre(terme.value) : null))
const famille = computed(() => savoir.familles.value.find((f) => f.code === entree.value?.family_code)?.label)

const pages = computed(() => {
  if (!terme.value || !props.lecture) return ''
  const etiquettes = [...new Set(chercherDansLeDocument(props.lecture, terme.value).map((p) => p.etiquette))]
  return new Intl.ListFormat(locale.value, { type: 'conjunction' }).format(etiquettes)
})

const aProposer = ref('')
const proposition = ref(false)

async function proposer(): Promise<void> {
  aProposer.value = terme.value ?? ''
  terme.value = null
  await nextTick()
  proposition.value = true
}
</script>

<template>
  <GnFeuilleBasse
    v-model="ouverte"
    :titre="t('gn-feuille-terme.titre')"
    :sous-titre="famille"
    :fermeture="t('gn-feuille-terme.revenir')"
  >
    <div class="gn-feuille-terme">
      <template v-if="entree">
        <p class="gn-feuille-terme__terme" lang="en">
          <i>{{ entree.term }}</i>
        </p>
        <p class="gn-feuille-terme__traduction">
          {{ entree.acronym ? t('gn-feuille-terme.avec-sigle', { traduction: entree.translation, sigle: entree.acronym }) : entree.translation }}
        </p>
        <p class="gn-feuille-terme__definition">{{ entree.definition }}</p>
      </template>
      <template v-else>
        <p class="gn-feuille-terme__terme" lang="en">
          <i>{{ terme }}</i>
        </p>
        <p class="gn-feuille-terme__definition">{{ t('gn-feuille-terme.absent') }}</p>
      </template>

      <p v-if="pages" class="gn-feuille-terme__pages">{{ t('gn-feuille-terme.pages', { pages }) }}</p>

      <GnBouton v-if="entree" variante="secondaire" picto="text-size" :vers="`/guide-nego/lexique/${entree.slug}`">
        {{ t('gn-feuille-terme.ouvrir') }}
      </GnBouton>
      <GnBouton v-else picto="plus" @clic="proposer">
        {{ t('gn-feuille-terme.proposer', { terme }) }}
      </GnBouton>
    </div>
  </GnFeuilleBasse>

  <GnFeuilleProposerTerme v-model="proposition" :terme="aProposer" />
</template>

<style>
[data-app="guide-nego"] .gn-feuille-terme {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-bottom: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-feuille-terme__terme {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-feuille-terme__traduction {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-feuille-terme__definition,
[data-app="guide-nego"] .gn-feuille-terme__pages {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
}

[data-app="guide-nego"] .gn-feuille-terme__pages {
  margin-bottom: var(--gn-espace-4);
}
</style>
