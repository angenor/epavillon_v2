<script setup lang="ts">
import type { GlossaryEntry } from '~/types/negotiation-savoir'
import type { SaisieDeTerme } from '~/utils/guide-nego/proposition'

/**
 * « Proposer un terme » — maquette 06, écran 06b. Ouverte à tout compte ; sans compte,
 * la feuille propose de se connecter et garde la saisie, qu'elle rouvre au retour.
 * Posée sur chaque écran qui l'ouvre : c'est elle qui se rouvre après la connexion.
 */
const props = defineProps<{ terme: string }>()
const ouverte = defineModel<boolean>({ required: true })

const { t } = useI18n()
const route = useRoute()
const savoir = useGnSavoir()
const session = useGnSession()
const connexion = useGnConnexion()
const propositions = useGnTermesProposes()

const MAXIMUM = 200

const vue = ref<'saisie' | 'compte' | 'envoye' | 'garde'>('saisie')
const terme = ref('')
const contexte = ref('')
const erreur = ref<string | undefined>(undefined)

let reprise: SaisieDeTerme | null = null

watch(ouverte, (oui) => {
  if (!oui) return
  void session.assurer()
  terme.value = reprise?.terme ?? props.terme
  contexte.value = reprise?.contexte ?? ''
  reprise = null
  vue.value = 'saisie'
  erreur.value = undefined
})

onMounted(async () => {
  reprise = await propositions.reprendre(route.fullPath)
  if (reprise) ouverte.value = true
})

const deja = computed<GlossaryEntry | null>(() => (terme.value.trim() ? savoir.resoudre(terme.value) : null))

async function envoyer(): Promise<void> {
  const saisie = { terme: terme.value.trim(), contexte: contexte.value }
  if (!saisie.terme) {
    erreur.value = t('gn-feuille-proposer-terme.terme.requis')
    return
  }
  if ([...saisie.terme].length > MAXIMUM) {
    erreur.value = t('gn-feuille-proposer-terme.terme.long', { maximum: MAXIMUM })
    return
  }
  if (!propositions.connectee.value) {
    propositions.garder(route.fullPath, saisie)
    vue.value = 'compte'
    return
  }
  const enLigne = connexion.etat.value.enLigne
  await propositions.proposer(saisie)
  vue.value = enLigne ? 'envoye' : 'garde'
}

const fini = computed(() => vue.value === 'envoye' || vue.value === 'garde')
</script>

<template>
  <GnFeuilleBasse
    v-model="ouverte"
    :titre="t(fini ? 'gn-feuille-proposer-terme.envoye.titre' : 'gn-feuille-proposer-terme.titre')"
    :sous-titre="vue === 'saisie' ? t('gn-feuille-proposer-terme.texte') : undefined"
    :fermeture="fini ? t('gn-feuille-proposer-terme.fermer') : undefined"
  >
    <form v-if="vue === 'saisie'" class="gn-proposer" novalidate @submit.prevent="envoyer">
      <GnChamp
        v-model="terme"
        :libelle="t('gn-feuille-proposer-terme.terme.libelle')"
        :aide="t('gn-feuille-proposer-terme.terme.aide')"
        :erreur="erreur"
        lang="en"
        autocapitalize="off"
        autocomplete="off"
        spellcheck="false"
        @input="erreur = undefined"
      />
      <p v-if="deja" class="gn-proposer__deja" role="status">
        {{ t('gn-feuille-proposer-terme.deja', { terme: deja.term }) }}
        <NuxtLink :to="`/guide-nego/lexique/${deja.slug}`" class="gn-proposer__lien">
          {{ t('gn-feuille-proposer-terme.ouvrir') }}
        </NuxtLink>
      </p>
      <GnZoneTexte
        v-model="contexte"
        :libelle="t('gn-feuille-proposer-terme.contexte.libelle')"
        :indication="t('gn-feuille-proposer-terme.contexte.indication')"
        :maximum="600"
      />
      <GnBouton type="submit" picto="send" :desactive="Boolean(deja)">
        {{ t('gn-feuille-proposer-terme.envoyer') }}
      </GnBouton>
    </form>

    <div v-else-if="vue === 'compte'" class="gn-proposer">
      <p class="gn-proposer__texte">{{ t('gn-feuille-proposer-terme.compte.texte') }}</p>
      <GnBouton vers="/guide-nego/compte" picto="user">{{ t('gn-feuille-proposer-terme.compte.creer') }}</GnBouton>
      <GnBouton variante="secondaire" vers="/guide-nego/connexion">{{ t('gn-feuille-proposer-terme.compte.connexion') }}</GnBouton>
    </div>

    <p v-else class="gn-proposer__texte" role="status">
      {{ t(`gn-feuille-proposer-terme.envoye.${vue}`, { terme: terme.trim() }) }}
    </p>
  </GnFeuilleBasse>
</template>

<style>
[data-app="guide-nego"] .gn-proposer {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-proposer__texte,
[data-app="guide-nego"] .gn-proposer__deja {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-proposer__texte {
  padding-bottom: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-proposer__lien {
  display: inline-flex;
  align-items: center;
  min-height: var(--gn-cible);
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>
