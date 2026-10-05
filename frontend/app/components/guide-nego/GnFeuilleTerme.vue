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
        <p class="gn-feuille-terme__terme">
          <span class="gn-feuille-terme__langue" aria-hidden="true">{{ t('gn-feuille-terme.langue') }}</span>
          <i lang="en">{{ entree.term }}</i>
        </p>
        <p class="gn-feuille-terme__traduction">
          {{ entree.acronym ? t('gn-feuille-terme.avec-sigle', { traduction: entree.translation, sigle: entree.acronym }) : entree.translation }}
        </p>
        <p class="gn-feuille-terme__definition">{{ entree.definition }}</p>
      </template>
      <template v-else>
        <p class="gn-feuille-terme__terme">
          <span class="gn-feuille-terme__langue" aria-hidden="true">{{ t('gn-feuille-terme.langue') }}</span>
          <i lang="en">{{ terme }}</i>
        </p>
        <p class="gn-feuille-terme__definition">{{ t('gn-feuille-terme.absent') }}</p>
      </template>

      <p v-if="pages" class="gn-feuille-terme__pages">{{ t('gn-feuille-terme.pages', { pages }) }}</p>

      <GnBouton v-if="entree" picto="text-size" :vers="`/guide-nego/lexique/${entree.slug}`">
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
/* Le bloc « terme trouvé » de la maquette 03, dans la feuille. */
[data-app="guide-nego"] .gn-feuille-terme {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-16);
  padding-block: var(--gn-espace-4);
}

[data-app="guide-nego"] .gn-feuille-terme__terme {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-18);
  line-height: var(--gn-interligne-18);
}

/* La maquette écrit le terme anglais en romain : la marque « EN » dit déjà la langue. */
[data-app="guide-nego"] .gn-feuille-terme__terme i {
  font-style: normal;
}

[data-app="guide-nego"] .gn-feuille-terme__langue {
  flex: none;
  padding: 4px 8px;
  border-radius: var(--gn-rayon-6);
  background: var(--gn-bloc-releve);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-11);
  line-height: var(--gn-interligne-11);
  font-weight: var(--gn-graisse-extra-gras);
  letter-spacing: var(--gn-approche-11);
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-feuille-terme__terme i {
  font-style: normal;
}

[data-app="guide-nego"] .gn-feuille-terme__traduction {
  color: var(--gn-accent);
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-44);
  line-height: var(--gn-interligne-44);
  font-weight: var(--gn-graisse-extra-gras);
  letter-spacing: var(--gn-approche-44);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-feuille-terme__definition {
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-feuille-terme__pages {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}
</style>
