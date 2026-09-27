<script setup lang="ts">
import type { PublicScheduleRow } from '~/types/views'
import { dayKeyInZone } from '~/utils/datetime'
import {
  activitesDuJour,
  cheminDeLActivite,
  etatDeLActivite,
  jourDuPavillonAOuvrir,
  joursDeLaBande,
  joursSuivants,
  rediffusionsDeLaVeille,
} from '~/utils/guide-nego/pavillon'

/**
 * La section Pavillon de l'onglet Francophonie (10 · 1c) : le bloc de lieu, la bande des
 * jours de toute l'édition, le jour choisi, les rediffusions de la veille, puis le jour
 * suivant. **Jamais filtrée par les thématiques suivies** (FR-003). Le jour et le fuseau
 * se disent une fois, dans le sous-titre de la page, qui lit `gn-pavillon-jour-affiche`.
 */
const { t } = useI18n()
const { dayLong } = useDateTime()
const { tr } = useI18nText()
const connexion = useGnConnexion()
const session = useGnSession()
const edition = useGnEdition()
const lecture = useGnPavillon()
const inscriptions = useGnInscriptionsPavillon()

const k = (cle: string, params: Record<string, unknown> = {}, n?: number) =>
  n === undefined ? t(`gn-section-pavillon.${cle}`, params) : t(`gn-section-pavillon.${cle}`, params, n)

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
const tentee = ref(false)
const relecture = ref(false)

async function relire(): Promise<void> {
  relecture.value = true
  try {
    await lecture.rafraichir()
  } finally {
    relecture.value = false
    tentee.value = true
  }
}

onMounted(async () => {
  horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
  void relire()
  await session.assurer()
  inscriptions.assurer()
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
const attente = computed(() => !lecture.pret.value && !tentee.value)
const lue = computed(() => lecture.etat.value.valeur !== null)
const fuseau = computed(() => lecture.fuseau.value ?? 'UTC')
const ville = computed(() => edition.edition.value?.city ?? null)
const activites = computed(() => lecture.activites.value)

const lieu = computed(() => {
  const l = lecture.lieu.value
  return l ? { nom: tr(l.name), adresse: l.address, plan: l.map_url } : null
})

const aujourdhui = computed(() => dayKeyInZone(maintenant.value, fuseau.value))
const jours = computed(() => joursDeLaBande(activites.value, fuseau.value))

// Le jour survit à l'aller-retour vers une fiche.
const jourChoisi = useState<string | null>('gn-pavillon-jour', () => null)
const jourAffiche = useState<string | null>('gn-pavillon-jour-affiche', () => null)
const jour = computed<string>({
  get: () => {
    const choisi = jourChoisi.value
    if (choisi && jours.value.includes(choisi)) return choisi
    return jourDuPavillonAOuvrir(jours.value, maintenant.value, fuseau.value) ?? ''
  },
  set: (valeur) => (jourChoisi.value = valeur),
})
watchEffect(() => (jourAffiche.value = jour.value || null))
onBeforeUnmount(() => (jourAffiche.value = null))

const ligne = (a: PublicScheduleRow) => ({
  activite: a,
  etat: etatDeLActivite(a, maintenant.value),
  marque: inscriptions.marque(a.id, maintenant.value),
})

const estAujourdhui = computed(() => jour.value === aujourdhui.value)
const majuscule = (texte: string) => texte.charAt(0).toLocaleUpperCase() + texte.slice(1)
const nomDuJour = (j: string) => majuscule(dayLong(`${j}T12:00:00Z`, 'UTC'))

const duJour = computed(() => activitesDuJour(activites.value, jour.value, fuseau.value).map(ligne))
const veille = computed(() =>
  estAujourdhui.value ? rediffusionsDeLaVeille(activites.value, jour.value, fuseau.value).map(ligne) : [],
)
const suivant = computed(() => joursSuivants(activites.value, jour.value, fuseau.value)[0] ?? null)

const vers = (a: PublicScheduleRow) => cheminDeLActivite(a.slug)

function allerAuJourSuivant(): void {
  if (!suivant.value) return
  jour.value = suivant.value.jour
  window.scrollTo({ top: 0 })
}
</script>

<template>
  <div class="gn-pavillon">
    <GnChargement v-if="attente" forme="squelette" :lignes="4" :libelle="k('chargement')" class="gn-pavillon__attente" />

    <GnEtatVide
      v-else-if="lecture.sansEdition.value"
      picto="franco"
      :titre="k('sans-edition.titre')"
      :texte="k('sans-edition.texte')"
    />

    <GnEtatVide
      v-else-if="!lue && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lue.titre')"
      :texte="k('jamais-lue.texte')"
    />

    <GnEtatErreur
      v-else-if="!lue"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="relecture ? undefined : k('erreur.reessayer')"
      @sortie="relire()"
    />

    <template v-else>
      <GnBlocLieu v-if="lieu" :nom="lieu.nom" :adresse="lieu.adresse" :plan="lieu.plan" />

      <GnEtatVide v-if="!jours.length" picto="franco" :titre="k('non-publie.titre')" :texte="k('non-publie.texte')" />

      <template v-else>
        <GnBandeJours v-model="jour" :jours="jours" :aujourdhui="aujourdhui" class="gn-pavillon__bande" />

        <GnEnteteGroupe :titre="estAujourdhui ? k('aujourdhui') : nomDuJour(jour)" :compteur="duJour.length" />
        <div v-if="duJour.length" class="gn-pavillon__liste">
          <GnLigneActivite
            v-for="l in duJour"
            :key="l.activite.id"
            v-bind="l"
            :fuseau="fuseau"
            :ville="ville"
            :vers="vers(l.activite)"
          />
        </div>
        <p v-else class="gn-pavillon__rien">{{ k('jour-vide') }}</p>

        <template v-if="veille.length">
          <GnEnteteGroupe :titre="k('hier')" :compteur="veille.length" />
          <div class="gn-pavillon__liste">
            <GnLigneActivite
              v-for="l in veille"
              :key="l.activite.id"
              v-bind="l"
              :fuseau="fuseau"
              :ville="ville"
              :vers="vers(l.activite)"
            />
          </div>
        </template>

        <button v-if="suivant" type="button" class="gn-pavillon__suivants" @click="allerAuJourSuivant">
          <span class="gn-pavillon__suivants-corps">
            <span class="gn-pavillon__suivants-titre">{{ k('suivants') }}</span>
            <span class="gn-pavillon__suivants-note">
              {{ k('suivant', { jour: nomDuJour(suivant.jour), count: suivant.activites.length }, suivant.activites.length) }}
            </span>
          </span>
          <GnPicto nom="chevron" :taille="24" class="gn-pavillon__suivants-chevron" />
        </button>
      </template>
    </template>
  </div>
</template>

<style>
[data-app="guide-nego"] .gn-pavillon {
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-pavillon__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-pavillon__bande {
  margin-top: var(--gn-espace-8);
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-pavillon__rien {
  padding-block: var(--gn-espace-16);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-pavillon__suivants {
  width: 100%;
  min-height: var(--gn-cible);
  margin-top: var(--gn-espace-16);
  padding-block: var(--gn-ligne-air);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-top: var(--gn-filet-3) solid var(--gn-filet-fort);
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
  background: none;
  color: var(--gn-texte);
  text-align: start;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-pavillon__suivants:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-pavillon__suivants-corps {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

[data-app="guide-nego"] .gn-pavillon__suivants-titre {
  color: var(--gn-accent);
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-pavillon__suivants-note {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-pavillon__suivants-chevron {
  flex: none;
  color: var(--gn-picto-secondaire);
}
</style>
