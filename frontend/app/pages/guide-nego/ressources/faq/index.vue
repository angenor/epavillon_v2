<script setup lang="ts">
import type { FaqEntry } from '~/types/negotiation-savoir'
import { NOMS_DE_PICTO, type NomDePicto } from '~/utils/guide-nego/pictogrammes'

/**
 * La FAQ — maquette 05, écran 01. Tout se lit dans le savoir gardé : aucune frappe ne
 * questionne le réseau. Public : l'état « accès refusé » est sans objet.
 *
 * La ligne du parcours clôt les rubriques, avec son avancée. « Poser une question à
 * un expert » mène à son écran, qui pose lui-même le verrou sans l'accès.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const router = useRouter()
const savoir = useGnSavoir()
const connexion = useGnConnexion()
const session = useGnSession()

const lirePremier = (valeur: unknown): string => {
  const premier = Array.isArray(valeur) ? valeur[0] : valeur
  return typeof premier === 'string' ? premier : ''
}

const saisie = ref(lirePremier(route.query.q))
watch(saisie, (q) => {
  if (q !== lirePremier(route.query.q)) void router.replace({ query: q.trim() ? { q } : {} })
})

const parcours = useGnParcours()
onMounted(() => {
  void savoir.assurer()
  void parcours.assurer()
})
const avance = computed(() => parcours.etat.value)

const etat = computed(() => savoir.etat.value)
const entrees = computed(() => savoir.faq.value)
const chargement = computed(() => !etat.value.pret)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)
const vide = computed(() => !chargement.value && !jamaisLu.value && entrees.value.length === 0)

const cherche = computed(() => saisie.value.trim())
const resultats = computed(() => savoir.chercherDansLaFaq(cherche.value))

const rubriques = computed(() =>
  savoir.rubriques.value
    .map((r) => ({ ...r, compte: entrees.value.filter((e) => e.section_code === r.code).length }))
    .filter((r) => r.compte > 0),
)
const pictoDe = (icon: string | null): NomDePicto =>
  NOMS_DE_PICTO.find((n) => n === icon) ?? 'toc'

const vers = (e: FaqEntry) => `/guide-nego/ressources/faq/${e.id}`

function ouvrirLaPremiere(): void {
  const premiere = resultats.value.trouves[0]?.valeur
  if (premiere) void navigateTo(vers(premiere))
}

const sousTitre = computed(() =>
  etat.value.valeur ? t('guide-nego.faq.compte', { count: entrees.value.length }, entrees.value.length) : undefined,
)

useHead({ title: t('guide-nego.faq.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.faq.titre')"
    :sous-titre="sousTitre"
    retour="/guide-nego/ressources"
  >
    <template v-if="!connexion.etat.value.enLigne" #connexion>
      <GnLigneConnexion :en-ligne="false" :lu-a="etat.luA" />
    </template>

    <div class="gn-faq">
      <GnChampRecherche
        v-model="saisie"
        :libelle="t('guide-nego.faq.champ')"
        :indication="t('guide-nego.faq.champ')"
        :disabled="jamaisLu || vide || undefined"
        @keydown.enter="ouvrirLaPremiere"
      />

      <GnChargement v-if="chargement" forme="squelette" :lignes="5" :libelle="t('guide-nego.faq.chargement')" />

      <GnEtatErreur
        v-else-if="jamaisLu"
        :titre="t('guide-nego.faq.erreur.titre')"
        :texte="t('guide-nego.faq.erreur.texte')"
      />

      <GnEtatVide
        v-else-if="vide"
        picto="quiz"
        :titre="t('guide-nego.faq.vide.titre')"
        :texte="t('guide-nego.faq.vide.texte')"
      />

      <template v-else-if="cherche">
        <ul v-if="resultats.trouves.length" role="list" aria-live="polite">
          <li v-for="r in resultats.trouves" :key="r.valeur.id">
            <GnLigneQuestion :entree="r.valeur" :vers="vers(r.valeur)" :surligne="cherche" />
          </li>
        </ul>
        <p class="gn-faq__compte" role="status">
          {{ t('guide-nego.faq.trouvees', { count: resultats.trouves.length, total: entrees.length }, resultats.trouves.length) }}
        </p>
        <p v-if="!resultats.trouves.length" class="gn-faq__aucune">{{ t('guide-nego.faq.aucune', { saisie: cherche }) }}</p>
      </template>

      <template v-else>
        <section>
          <GnEnteteGroupe :titre="t('guide-nego.faq.rubriques')" />
          <GnLigneReglage
            v-for="r in rubriques"
            :key="r.code"
            :libelle="r.label"
            :picto="pictoDe(r.icon)"
            :vers="`/guide-nego/ressources/faq/rubrique/${r.code}`"
          >
            <span class="gn-faq__nombre">{{ r.compte }}</span>
          </GnLigneReglage>
          <GnLigneReglage
            v-if="avance.total"
            :libelle="t('guide-nego.parcours.ligne')"
            :valeur="t('guide-nego.parcours.faites', { faites: avance.faites, total: avance.total }, avance.faites)"
            picto="star"
            vers="/guide-nego/ressources/parcours"
            class="gn-faq__parcours"
            derniere
          >
            <template #sous><GnProgression :part="avance.faites / avance.total" decoratif /></template>
          </GnLigneReglage>
        </section>

        <section v-if="savoir.plusLues.value.length">
          <GnEnteteGroupe :titre="t('guide-nego.faq.plus-lues')" />
          <ul role="list">
            <li v-for="e in savoir.plusLues.value" :key="e.id">
              <GnLigneQuestion :entree="e" :vers="vers(e)" />
            </li>
          </ul>
        </section>
      </template>

      <div v-if="!chargement" class="gn-faq__expert">
        <GnBouton variante="secondaire" picto="send" vers="/guide-nego/ressources/faq/question">
          {{ t('guide-nego.faq.expert') }}
        </GnBouton>
        <GnBouton v-if="session.connectee.value" variante="discret" vers="/guide-nego/ressources/faq/mes-questions">
          {{ t('guide-nego.faq.mes-questions') }}
        </GnBouton>
      </div>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-faq {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-faq__nombre,
[data-app="guide-nego"] .gn-faq__compte {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-faq__compte {
  padding-block: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-faq__aucune {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  overflow-wrap: anywhere;
}

/* « Ma première COP » de la maquette 05 : la jauge fine de 4 sous le titre. */
[data-app="guide-nego"] .gn-faq__parcours .gn-reglage__texte {
  gap: 6px;
  padding-block: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-faq__parcours .gn-progression {
  block-size: 4px;
}

[data-app="guide-nego"] .gn-faq li:last-child > .gn-ligne-question {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-faq__expert {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-espace-20);
}
</style>
