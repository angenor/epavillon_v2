<script setup lang="ts">
import { CLE_FILE_THEMATIQUES, choixValidable } from '~/utils/guide-nego/thematiques'
import { CLE_FILE_GROUPES } from '~/composables/guide-nego/useGnGroupes'

/**
 * Écran 06 — « Mes thématiques ». **Un seul écran pour deux usages** : le
 * premier choix, dernière marche du parcours d'entrée, et la modification
 * depuis le profil. Ce qui les distingue tient en trois choses — l'indicateur
 * d'étapes, le libellé du bouton, et où l'on revient.
 *
 * **Les libellés viennent de la base** (FR-001) : le vocabulaire des filières de
 * négociation, résolu dans la langue de la personne avec repli sur le français.
 * Aucun nom de thématique n'est écrit ici ni dans un fichier de traduction.
 *
 * **L'enregistrement passe par la file**, avec du réseau comme sans (FR-009) :
 * un seul chemin, donc un seul comportement. Un choix qui arrive après un autre,
 * fait ailleurs entre-temps, reçoit `412` : il est abandonné, l'état vrai est
 * relu, et l'écran le dit — jamais de fusion, que personne n'aurait choisie.
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t } = useI18n()
const route = useRoute()
const session = useGnSession()
const connexion = useGnConnexion()
const thematiques = useGnThematiques()
const groupes = useGnGroupes()
const { avis, effacerLAvis } = useGnFile()

const choix = ref<Set<string>>(new Set())
const choixGroupes = ref<Set<string>>(new Set())
const envoi = ref(false)

onMounted(async () => {
  await session.assurer()
  if (session.connectee.value) await Promise.all([thematiques.assurer(), groupes.assurer()])
  else await thematiques.assurerLeVocabulaire()
})

// Ce que le compte suit commande les cases : à l'arrivée de la lecture, et à
// chaque relecture — c'est ainsi qu'un choix fait sur un autre appareil
// s'impose ici, sans message d'échec (FR-006).
watch(thematiques.mesCodes, (codes) => (choix.value = new Set(codes)), { immediate: true })
watch(groupes.mesCodes, (codes) => (choixGroupes.value = new Set(codes)), { immediate: true })

/** Sans compte, rien ne se choisit : les thématiques suivent le compte (FR-010). */
const compteManquant = computed(() => session.pret.value && !session.connectee.value)

/** Premier passage tant que rien n'est suivi : l'écran porte alors ses étapes. */
const premiereEntree = computed(() => thematiques.mesCodes.value.length === 0)
const depuisLesSessions = computed(() => route.query.depuis === 'negociations')
const retour = computed(() => {
  if (depuisLesSessions.value) return '/guide-nego/negociations'
  return premiereEntree.value ? '/guide-nego' : '/guide-nego/ressources/reglages'
})

const chargement = computed(() => !thematiques.vocabulairePret.value)
const aucuneProposee = computed(
  () => thematiques.vocabulairePret.value && thematiques.thematiques.value.length === 0,
)
const enEchec = computed(() => aucuneProposee.value && !thematiques.vocabulaireLu.value)

const codesChoisis = computed(() => [...choix.value])
const validable = computed(() => choixValidable(codesChoisis.value))
const resume = computed(() => thematiques.resumeDe(codesChoisis.value))

/** L'avis de la file ne concerne cet écran que s'il porte sa clé. */
const CLES_DE_L_ECRAN = [CLE_FILE_THEMATIQUES, CLE_FILE_GROUPES]
const perimee = computed(() => !!avis.value && CLES_DE_L_ECRAN.includes(avis.value.cle) && avis.value.sort === 'perimee')
const refusee = computed(() => !!avis.value && CLES_DE_L_ECRAN.includes(avis.value.cle) && avis.value.sort === 'refusee')

/** Aucun groupe coché est un choix : toutes les coordinations passent alors le filtre. */
const groupesProposes = computed(() => groupes.groupes.value)
const groupesChanges = computed(
  () => [...choixGroupes.value].sort().join(',') !== [...groupes.mesCodes.value].sort().join(','),
)

function basculerGroupe(code: string, coche: boolean): void {
  const prochain = new Set(choixGroupes.value)
  if (coche) prochain.add(code)
  else prochain.delete(code)
  choixGroupes.value = prochain
  effacerLAvis()
}

function basculer(code: string, coche: boolean): void {
  const prochain = new Set(choix.value)
  if (coche) prochain.add(code)
  else prochain.delete(code)
  choix.value = prochain
  effacerLAvis()
}

async function valider(): Promise<void> {
  if (!validable.value || envoi.value) return
  const premier = premiereEntree.value
  envoi.value = true
  try {
    await thematiques.enregistrer(codesChoisis.value)
    if (groupesChanges.value && !perimee.value && !refusee.value) await groupes.enregistrer(choixGroupes.value)
  } finally {
    envoi.value = false
  }
  // Un choix périmé laisse la personne sur l'écran : elle vient de recevoir
  // l'état vrai et doit pouvoir le regarder avant de décider.
  if (perimee.value || refusee.value) return
  await navigateTo(depuisLesSessions.value ? '/guide-nego/negociations' : premier ? '/guide-nego' : '/guide-nego/ressources/reglages')
}

useHead({ title: t('guide-nego.thematiques.titre') })
</script>

<template>
  <GnEcran
    :titre="t('guide-nego.thematiques.titre')"
    :sous-titre="t('guide-nego.thematiques.sous-titre')"
    :retour="retour"
    :onglets="false"
  >
    <div v-if="premiereEntree && !compteManquant" class="gn-themes__etapes">
      <GnEtapes :courante="3" :libelle="t('guide-nego.thematiques.etape')" />
    </div>

    <!-- Accès refusé : les thématiques suivent le compte, pas l'appareil. -->
    <template v-if="compteManquant">
      <GnEtatVide
        picto="user"
        :titre="t('guide-nego.thematiques.sans-compte.titre')"
        :texte="t('guide-nego.thematiques.sans-compte.texte')"
      />
      <div class="gn-themes__sorties">
        <GnBouton variante="principal" vers="/guide-nego/compte">
          {{ t('guide-nego.thematiques.sans-compte.creer-mon-compte') }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/connexion">
          {{ t('guide-nego.thematiques.sans-compte.me-connecter') }}
        </GnBouton>
      </div>
    </template>

    <template v-else>
      <p class="gn-themes__aide">{{ t('guide-nego.thematiques.aide') }}</p>

      <GnChargement
        v-if="chargement"
        forme="squelette"
        :lignes="6"
        :libelle="t('guide-nego.thematiques.chargement')"
      />

      <GnEtatErreur
        v-else-if="enEchec"
        :titre="t('guide-nego.thematiques.erreur.titre')"
        :texte="t('guide-nego.thematiques.erreur.texte')"
        :sortie="t('guide-nego.thematiques.erreur.reessayer')"
        @sortie="thematiques.rafraichirLeVocabulaire()"
      />

      <!-- Aucune thématique publiée : on le dit, et l'entrée n'est pas bloquée. -->
      <template v-else-if="aucuneProposee">
        <GnEtatVide
          picto="filter"
          :titre="t('guide-nego.thematiques.vide.titre')"
          :texte="t('guide-nego.thematiques.vide.texte')"
        />
        <div class="gn-themes__sorties">
          <GnBouton variante="principal" vers="/guide-nego">
            {{ t('guide-nego.thematiques.plus-tard') }}
          </GnBouton>
        </div>
      </template>

      <template v-else>
        <div class="gn-themes__liste">
          <GnCase
            v-for="(thematique, index) in thematiques.thematiques.value"
            :key="thematique.code"
            :libelle="thematiques.nomDe(thematique.code) ?? thematique.code"
            :model-value="choix.has(thematique.code)"
            :derniere="index === thematiques.thematiques.value.length - 1"
            @update:model-value="basculer(thematique.code, $event)"
          />
        </div>

        <section v-if="groupesProposes.length" class="gn-themes__groupes" aria-labelledby="gn-themes-groupes">
          <h2 id="gn-themes-groupes" class="gn-themes__groupes-titre">{{ t('guide-nego.thematiques.groupes.titre') }}</h2>
          <p class="gn-themes__aide">{{ t('guide-nego.thematiques.groupes.aide') }}</p>
          <div class="gn-themes__liste">
            <GnCase
              v-for="(groupe, index) in groupesProposes"
              :key="groupe.code"
              :libelle="groupes.nomDe(groupe.code) ?? groupe.code"
              :model-value="choixGroupes.has(groupe.code)"
              :derniere="index === groupesProposes.length - 1"
              @update:model-value="basculerGroupe(groupe.code, $event)"
            />
          </div>
        </section>

        <!-- Le message du 412 vient de l'API : elle seule sait ce qui a changé. -->
        <p v-if="perimee || refusee" class="gn-themes__avis" role="status">
          <GnPicto nom="refresh" :taille="20" />
          <span>{{ avis?.message ?? t('guide-nego.thematiques.perimee') }}</span>
        </p>

        <GnLigneInformation
          v-if="!connexion.etat.value.enLigne"
          :texte="t('guide-nego.thematiques.hors-connexion')"
        />

        <div class="gn-themes__pied">
          <p class="gn-themes__recapitulatif" role="status">
            <template v-if="resume.nombre > 0">
              {{ t('guide-nego.thematiques.recapitulatif', { count: resume.nombre, noms: resume.noms.join(', ') }, resume.nombre) }}
            </template>
            <template v-else>{{ t('guide-nego.thematiques.aucune') }}</template>
          </p>

          <!-- À zéro, la validation se refuse ET DIT POURQUOI (FR-005) : un
               bouton gris sans phrase laisse chercher ce qui manque. -->
          <p v-if="!validable" class="gn-themes__exigence">
            {{ t('guide-nego.thematiques.au-moins-une') }}
          </p>

          <GnBouton
            variante="principal"
            :desactive="!validable"
            :chargement="envoi"
            @clic="valider"
          >
            {{ premiereEntree ? t('guide-nego.thematiques.valider') : t('guide-nego.thematiques.enregistrer') }}
          </GnBouton>

          <GnBouton v-if="premiereEntree" variante="discret" vers="/guide-nego">
            {{ t('guide-nego.thematiques.plus-tard') }}
          </GnBouton>
          <GnBouton v-else variante="discret" vers="/guide-nego/ressources/reglages">
            {{ t('guide-nego.thematiques.retour-au-profil') }}
          </GnBouton>
        </div>
      </template>
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-themes__etapes {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-themes__aide {
  padding-top: var(--gn-espace-12);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte-2);
  max-width: var(--gn-mesure-lecture);
}

[data-app="guide-nego"] .gn-themes__liste {
  display: flex;
  flex-direction: column;
  border-top: var(--gn-filet-1) solid var(--gn-filet);
  margin-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-themes__groupes {
  padding-top: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-themes__groupes-titre {
  font-size: var(--gn-taille-20);
  line-height: var(--gn-interligne-20);
  font-weight: var(--gn-graisse-gras);
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-themes__avis {
  display: flex;
  gap: var(--gn-espace-8);
  align-items: flex-start;
  padding-top: var(--gn-espace-12);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-information);
}

[data-app="guide-nego"] .gn-themes__pied {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}

[data-app="guide-nego"] .gn-themes__recapitulatif {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
  color: var(--gn-texte-2);
  text-align: center;
}

[data-app="guide-nego"] .gn-themes__exigence {
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-attention-aplat-texte);
  text-align: center;
}

[data-app="guide-nego"] .gn-themes__sorties {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-top: var(--gn-entre-blocs);
}
</style>
