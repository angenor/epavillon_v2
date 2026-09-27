<script setup lang="ts">
import type { PathwayLink } from '~/types/negotiation-savoir'
import { destinationDeLEtape } from '~/utils/guide-nego/ma-premiere-cop'

/**
 * Le parcours « Ma première COP » — maquette 05, écran 04. Il se lit dans le savoir
 * gardé ; les coches suivent `useGnParcours`. Public : l'état « accès refusé » est
 * sans objet.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const savoir = useGnSavoir()
const parcours = useGnParcours()
const connexion = useGnConnexion()

let arreter: (() => void) | null = null
onMounted(() => {
  void parcours.assurer()
  arreter = parcours.suivre()
})
onBeforeUnmount(() => arreter?.())

const etat = computed(() => savoir.etat.value)
const groupes = computed(() => savoir.parcours.value)
const avance = computed(() => parcours.etat.value)
const chargement = computed(() => !etat.value.pret)
const jamaisLu = computed(() => !chargement.value && !etat.value.valeur)
const vide = computed(() => !chargement.value && !jamaisLu.value && avance.value.total === 0)

const slugDe = (id: string) => savoir.lexique.value.find((e) => e.id === id)?.slug ?? null
const vers = (lien: PathwayLink | null) => (lien ? destinationDeLEtape(lien, slugDe) : null)
function cible(lien: PathwayLink | null): string | null {
  if (!lien) return null
  if (lien.kind === 'faq') return savoir.entreeDeFaq(lien.target_id)?.question ?? null
  if (lien.kind === 'glossary') return savoir.lexique.value.find((e) => e.id === lien.target_id)?.term ?? null
  return t('guide-nego.parcours.document')
}

const sousTitre = computed(() =>
  avance.value.total ? t('guide-nego.parcours.en-etapes', { count: avance.value.total }, avance.value.total) : undefined,
)
const prochaine = computed(() => {
  const suite = avance.value.prochaine
  return suite ? t('guide-nego.parcours.prochaine', { groupe: suite.groupe.label }) : t('guide-nego.parcours.tout-coche')
})

useHead({ title: t('guide-nego.parcours.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.parcours.titre')"
    :sous-titre="sousTitre"
    retour="/guide-nego/ressources/faq"
    :ce-qui-se-lit="t('guide-nego.parcours.sur-le-telephone')"
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="connexion.etat.value.enLigne" :lu-a="etat.luA" />
    </template>

    <div class="gn-parcours">
      <GnChargement v-if="chargement" forme="squelette" :lignes="6" :libelle="t('guide-nego.parcours.chargement')" />

      <GnEtatErreur
        v-else-if="jamaisLu"
        :titre="t('guide-nego.parcours.erreur.titre')"
        :texte="t('guide-nego.parcours.erreur.texte')"
      />

      <GnEtatVide
        v-else-if="vide"
        picto="star"
        :titre="t('guide-nego.parcours.vide.titre')"
        :texte="t('guide-nego.parcours.vide.texte')"
      />

      <template v-else>
        <GnAnnonce
          v-if="parcours.misesAJourAilleurs.value"
          :texte="t('guide-nego.parcours.ailleurs')"
          @fermer="parcours.oublierLaMiseAJour()"
        />

        <div class="gn-parcours__avance">
          <p class="gn-parcours__compte">
            <span>{{ t('guide-nego.parcours.faites', { faites: avance.faites, total: avance.total }, avance.faites) }}</span>
            <span class="gn-parcours__prochaine">{{ prochaine }}</span>
          </p>
          <GnProgression :part="avance.faites / avance.total" decoratif />
        </div>

        <section v-for="g in groupes" :key="g.id">
          <GnEnteteGroupe
            :titre="g.label"
            :compteur="t('guide-nego.parcours.sur', { faites: avance.parGroupe[g.id]?.faites ?? 0, total: g.steps.length })"
          />
          <GnEtapeParcours
            v-for="(e, i) in g.steps"
            :key="e.id"
            :etape="e"
            :cochee="parcours.coches.value.has(e.id)"
            :vers="vers(e.link)"
            :cible="cible(e.link)"
            :derniere="i === g.steps.length - 1"
            @basculer="parcours.basculer(e.id)"
          />
        </section>

        <p class="gn-parcours__pied">{{ t('guide-nego.parcours.pied') }}</p>
      </template>
    </div>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-parcours {
  display: flex;
  flex-direction: column;
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-parcours__avance {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-block: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-parcours__compte {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: var(--gn-espace-4) var(--gn-espace-8);
  color: var(--gn-titre);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-parcours__prochaine {
  color: var(--gn-texte-2);
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-parcours__pied {
  padding-block: var(--gn-espace-16);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}
</style>
