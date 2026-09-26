<script setup lang="ts">
import { dayKeyInZone } from '~/utils/datetime'
import { chevauchements, sessionsDeLAgenda } from '~/utils/guide-nego/agenda'
import { estDeMonGroupe, etatAffiche, jourAOuvrir, joursDeLaBande } from '~/utils/guide-nego/sessions'
import {
  joursAvecReunions,
  lignesDuJour,
  repereSignale,
  reunionsDeLAgenda,
  reunionsParJour,
} from '~/utils/guide-nego/signalements'

/**
 * Écran 09 · 3a — « Mon agenda » : les sessions de négociation suivies, un jour à la fois.
 * Un chevauchement se signale sur les deux lignes et ne s'empêche jamais (FR-033) ;
 * une annulée reste, barrée, sans rappel. Les réunions non annoncées gardées s'y mêlent,
 * et restent quand la source est coupée (FR-022).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const { dayLong, zoneOf } = useDateTime()
const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.negociations-agenda.${cle}`, params)

const connexion = useGnConnexion()
const compte = useGnSession()
const thematiques = useGnThematiques()
const groupes = useGnGroupes()
const agenda = useGnAgenda()
const lecture = useGnSessions()
const { affichage, etat, edition, sansEdition } = lecture

const maintenant = shallowRef(new Date())
let horloge: ReturnType<typeof setInterval> | undefined
const tentee = ref(false)
const relecture = ref(false)

async function relire(): Promise<void> {
  relecture.value = true
  try {
    agenda.assurer()
    await lecture.rafraichir()
  } finally {
    relecture.value = false
    tentee.value = true
  }
}

onMounted(async () => {
  horloge = setInterval(() => (maintenant.value = new Date()), 60_000)
  await compte.assurer()
  void relire()
  await Promise.all([thematiques.assurerLeVocabulaire(), groupes.assurer()])
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
const sansCompte = computed(() => compte.pret.value && !compte.connectee.value)
const servie = computed(() => (affichage.value.etat === 'sert' ? affichage.value : null))
const coupee = computed(() => (affichage.value.etat === 'coupe' ? affichage.value : null))
const fuseau = computed(() => servie.value?.fuseau ?? edition.value?.timezone ?? 'UTC')
const ville = computed(() => servie.value?.ville ?? edition.value?.city ?? null)
const attente = computed(() => !compte.pret.value || ((!etat.value.pret || !agenda.pret.value) && !tentee.value))

const suivies = computed(() => sessionsDeLAgenda(agenda.agenda.value, servie.value?.sessions ?? []))
const croisees = computed(() => chevauchements(suivies.value))
const reunionsSuivies = computed(() => reunionsDeLAgenda(agenda.agenda.value, lecture.reunionsDuReseau.value))
const nomDeThematique = (code: string | null) => (code ? (thematiques.nomDe(code) ?? null) : null)

const aujourdhui = computed(() => dayKeyInZone(maintenant.value, fuseau.value))
const jours = computed(() =>
  joursAvecReunions(joursDeLaBande(suivies.value, fuseau.value), reunionsSuivies.value, maintenant.value, fuseau.value),
)
const jourChoisi = useState<string | null>('gn-agenda-jour', () => null)
const jour = computed<string>({
  get: () => {
    const choisi = jourChoisi.value
    if (choisi && jours.value.includes(choisi)) return choisi
    return jourAOuvrir(jours.value, maintenant.value, fuseau.value) ?? ''
  },
  set: (valeur) => (jourChoisi.value = valeur),
})

const lignes = computed(() =>
  lignesDuJour(suivies.value, reunionsSuivies.value, jour.value, maintenant.value, fuseau.value).map((l) => {
    if (l.genre === 'reseau') {
      return {
        cle: l.reunion.id,
        session: null,
        reunion: l.reunion,
        etat: undefined,
        thematique: nomDeThematique(l.reunion.theme),
        monGroupe: false,
        chevauche: [],
        rappel: false,
        signale: null,
        vers: `/guide-nego/negociations/reseau/${l.reunion.id}?depuis=agenda`,
      }
    }
    const s = l.session
    const etatDeLaSession = etatAffiche(s, maintenant.value, fuseau.value)
    return {
      cle: s.id,
      session: s,
      reunion: null,
      etat: etatDeLaSession,
      thematique: nomDeThematique(s.theme),
      monGroupe: estDeMonGroupe(s, groupes.mesCodes.value),
      chevauche: croisees.value.get(s.id) ?? [],
      rappel: etatDeLaSession !== 'annulee' && (agenda.entree(s.id)?.remind ?? false),
      signale: repereSignale(s, maintenant.value, fuseau.value, locale.value)?.heure ?? null,
      vers: `/guide-nego/negociations/${s.id}?depuis=agenda`,
    }
  }),
)

const reunionsDeLaCoupure = computed(() => reunionsParJour(reunionsSuivies.value, maintenant.value, fuseau.value))
function legende(unJour: string): string {
  const zone = ville.value ?? fuseau.value.split('/').pop()?.replace(/_/g, ' ') ?? ''
  const texte = k('jour', { jour: dayLong(`${unJour}T12:00:00Z`, 'UTC'), zone: zoneOf(zone) })
  return texte.charAt(0).toUpperCase() + texte.slice(1)
}

const sousTitre = computed(() => (jour.value && !coupee.value ? legende(jour.value) : undefined))

useHead({ title: k('titre') })
</script>

<template>
  <GnEcran :titre="k('titre')" :sous-titre="sousTitre" :ce-qui-se-lit="k('hors-connexion')">
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="etat.luA" />
    </template>

    <GnChargement v-if="attente" forme="squelette" :lignes="6" :libelle="k('chargement')" class="gn-agenda__attente" />

    <div v-else-if="sansCompte" class="gn-agenda__sans-compte">
      <GnEtatVide picto="calendar" :titre="k('sans-compte.titre')" :texte="k('sans-compte.texte')" />
      <GnBouton vers="/guide-nego/connexion" picto="user">{{ k('sans-compte.connexion') }}</GnBouton>
      <GnBouton variante="secondaire" vers="/guide-nego/compte">{{ k('sans-compte.creer') }}</GnBouton>
    </div>

    <GnEtatVide
      v-else-if="sansEdition"
      picto="nego"
      :titre="k('sans-edition.titre')"
      :texte="k('sans-edition.texte')"
    />

    <template v-else-if="coupee">
      <GnLectureImpossible
        :raison="coupee.raison"
        :depuis="coupee.depuis"
        :programme="coupee.programme"
        :en-cours="relecture"
        @reessayer="relire()"
      >
        <template v-if="reunionsDeLaCoupure.length" #default>
          <GnPicto nom="info" :taille="18" />
          {{ t('guide-nego.negociations.reseau-garde') }}
        </template>
      </GnLectureImpossible>
      <template v-for="j in reunionsDeLaCoupure" :key="j.jour">
        <GnEnteteGroupe :titre="legende(j.jour)" :compteur="j.reunions.length" />
        <GnLigneSession
          v-for="r in j.reunions"
          :key="r.id"
          forme="agenda"
          :reunion="r"
          :fuseau="fuseau"
          :ville="ville"
          :thematique="nomDeThematique(r.theme)"
          :vers="`/guide-nego/negociations/reseau/${r.id}?depuis=agenda`"
        />
      </template>
    </template>

    <GnEtatVide
      v-else-if="(!servie || !agenda.connu.value) && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lu.titre')"
      :texte="k('jamais-lu.texte')"
    />

    <GnEtatErreur
      v-else-if="!servie || !agenda.connu.value"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="k('erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide
      v-else-if="!suivies.length && !reunionsSuivies.length"
      picto="calendar"
      :titre="k('vide.titre')"
      :texte="k('vide.texte')"
      :sortie="k('vide.sortie')"
      sortie-vers="/guide-nego/negociations"
    />

    <template v-else>
      <GnBandeJours v-model="jour" :jours="jours" :aujourdhui="aujourdhui" class="gn-agenda__bande" />
      <GnEnteteGroupe :titre="k('section')" :compteur="lignes.length" />
      <GnLigneSession
        v-for="l in lignes"
        :key="l.cle"
        forme="agenda"
        :session="l.session"
        :reunion="l.reunion"
        :etat="l.etat"
        :fuseau="fuseau"
        :ville="ville"
        :thematique="l.thematique"
        :mon-groupe="l.monGroupe"
        :chevauche="l.chevauche"
        :rappel="l.rappel"
        :signale="l.signale"
        :vers="l.vers"
      />
    </template>
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-agenda__attente {
  padding-top: var(--gn-espace-16);
}

/* La bande sort des marges : elle défile jusqu'au bord de l'écran. */
[data-app="guide-nego"] .gn-agenda__bande {
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-agenda__sans-compte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  padding-bottom: var(--gn-espace-16);
}
</style>
