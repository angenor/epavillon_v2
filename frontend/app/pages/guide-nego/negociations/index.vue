<script setup lang="ts">
import { dayKeyInZone } from '~/utils/datetime'
import type { OptionDeFeuille } from '~/components/guide-nego/GnFeuilleBasse.vue'
import { sessionsDeLAgenda } from '~/utils/guide-nego/agenda'
import {
  estDeMonGroupe,
  etatAffiche,
  etatVide,
  filtrer,
  jourAOuvrir,
  joursDeLaBande,
  placeDuMaintenant,
  sessionsDuJour,
  type Suivis,
} from '~/utils/guide-nego/sessions'
import {
  joursAvecReunions,
  lignesDuJour,
  repereSignale,
  reunionPasseLeFiltre,
  reunionsParJour,
} from '~/utils/guide-nego/signalements'

/**
 * Les sessions de négociation officielles (Nuit 02), un jour à la fois, en frise. Rien d'autre :
 * ni les Réunions de la Francophonie, ni le Pavillon (ADR-008).
 *
 * **Coupée, la source se montre coupée** (FR-039, principe XII) : « Lecture impossible »
 * ou « Affichage suspendu », et aucune session à côté — jamais « aucune session ».
 * Les réunions non annoncées, elles, restent : elles ne viennent pas de la source (FR-022).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

type Filtre = 'miennes' | 'toutes'

const { t, locale } = useI18n()
const { dayLong, zoneOf, timeWithZone, time } = useDateTime()
const { momentLisible } = useGnMomentLecture()
const connexion = useGnConnexion()
const session = useGnSession()
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
  if (session.connectee.value) agenda.assurer()
  await Promise.all([
    session.connectee.value ? thematiques.assurer() : thematiques.assurerLeVocabulaire(),
    groupes.assurer(),
  ])
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
const servie = computed(() => (affichage.value.etat === 'sert' ? affichage.value : null))
const coupee = computed(() => (affichage.value.etat === 'coupe' ? affichage.value : null))
const programme = computed(() => servie.value?.programme ?? coupee.value?.programme ?? null)
const fuseau = computed(() => servie.value?.fuseau ?? edition.value?.timezone ?? 'UTC')
const ville = computed(() => servie.value?.ville ?? edition.value?.city ?? null)

const attente = computed(() => !etat.value.pret && !tentee.value)

// Le jour et le filtre survivent à l'aller-retour vers une fiche.
const jourChoisi = useState<string | null>('gn-sessions-jour', () => null)
const filtreChoisi = useState<Filtre | null>('gn-sessions-filtre', () => null)

const aujourdhui = computed(() => dayKeyInZone(maintenant.value, fuseau.value))
const reunions = lecture.reunionsDuReseau
const jours = computed(() =>
  servie.value
    ? joursAvecReunions(joursDeLaBande(servie.value.sessions, fuseau.value), reunions.value, maintenant.value, fuseau.value)
    : [],
)
const jour = computed<string>({
  get: () => {
    const choisi = jourChoisi.value
    if (choisi && jours.value.includes(choisi)) return choisi
    return jourAOuvrir(jours.value, maintenant.value, fuseau.value) ?? ''
  },
  set: (valeur) => (jourChoisi.value = valeur),
})

const suivis = computed<Suivis>(() => ({ thematiques: thematiques.mesCodes.value, groupes: groupes.mesCodes.value }))
/** Sans thématique suivie, « Mes thématiques » ne filtrerait rien de sensé : « Toutes », et l'invitation à choisir. */
const sansThematique = computed(() => suivis.value.thematiques.length === 0)
const filtre = computed<Filtre>({
  get: () => (sansThematique.value ? 'toutes' : (filtreChoisi.value ?? 'miennes')),
  set: (valeur) => (filtreChoisi.value = valeur),
})
const feuille = ref(false)
const options = computed<OptionDeFeuille[]>(() => [
  ...(sansThematique.value
    ? []
    : [
        { valeur: 'miennes', libelle: t('guide-nego.negociations.filtre.miennes'), picto: filtre.value === 'miennes' ? 'check-circle' : 'circle' } as const,
        { valeur: 'toutes', libelle: t('guide-nego.negociations.filtre.toutes'), picto: filtre.value === 'toutes' ? 'check-circle' : 'circle' } as const,
      ]),
  { valeur: 'agenda', libelle: t('guide-nego.negociations.mon-agenda'), picto: 'calendar' },
  { valeur: 'thematiques', libelle: t('guide-nego.negociations.choisir-thematiques'), picto: 'filter' },
])

function choisir(option: OptionDeFeuille): void {
  feuille.value = false
  if (option.valeur === 'miennes' || option.valeur === 'toutes') filtre.value = option.valeur
  else if (option.valeur === 'agenda') void navigateTo('/guide-nego/negociations/agenda')
  else void navigateTo('/guide-nego/thematiques?depuis=negociations')
}

const duJour = computed(() => (servie.value ? sessionsDuJour(servie.value.sessions, jour.value, fuseau.value) : []))
const affichees = computed(() => filtrer(duJour.value, filtre.value, suivis.value))

const reunionsFiltrees = computed(() =>
  filtre.value === 'toutes' ? reunions.value : reunions.value.filter((r) => reunionPasseLeFiltre(r, suivis.value.thematiques)),
)
const dansLAgenda = computed(
  () => new Set(sessionsDeLAgenda(agenda.agenda.value, servie.value?.sessions ?? []).map((s) => s.id)),
)

const lignes = computed(() =>
  lignesDuJour(affichees.value, reunionsFiltrees.value, jour.value, maintenant.value, fuseau.value).map((l) =>
    l.genre === 'session'
      ? {
          cle: l.session.id,
          session: l.session,
          reunion: null,
          etat: etatAffiche(l.session, maintenant.value, fuseau.value),
          debut: l.session.start_at,
          mienne: dansLAgenda.value.has(l.session.id),
          monGroupe: estDeMonGroupe(l.session, suivis.value.groupes),
          signale: repereSignale(l.session, maintenant.value, fuseau.value, locale.value)?.heure ?? null,
          vers: `/guide-nego/negociations/${l.session.id}`,
        }
      : {
          cle: l.reunion.id,
          session: null,
          reunion: l.reunion,
          etat: undefined,
          debut: l.reunion.start_at,
          mienne: false,
          monGroupe: false,
          signale: null,
          vers: `/guide-nego/negociations/reseau/${l.reunion.id}`,
        },
  ),
)

/** Source coupée : les réunions du réseau à venir, par jour, sous la phrase de la maquette (07 · 1c). */
const reunionsDeLaCoupure = computed(() =>
  reunionsParJour(reunions.value, maintenant.value, fuseau.value).map((j) => ({ ...j, legende: legende(j.jour) })),
)

function legende(unJour: string): string {
  const zone = ville.value ?? fuseau.value.split('/').pop()?.replace(/_/g, ' ') ?? ''
  return t('guide-nego.negociations.jour', { jour: dayLong(`${unJour}T12:00:00Z`, 'UTC'), zone: zoneOf(zone) })
}

/** La barre « MAINTENANT », aujourd'hui seulement : nulle un autre jour. */
const maintenantA = computed(() =>
  jour.value === aujourdhui.value ? placeDuMaintenant(lignes.value.map((l) => l.debut), maintenant.value) : null,
)
const heureDeMaintenant = computed(() => time(maintenant.value, fuseau.value))

/** « Mes thématiques · heure d'Antalya · source officielle à 10:12 » */
const contexte = computed(() => {
  const zone = zoneOf(ville.value ?? fuseau.value.split('/').pop()?.replace(/_/g, ' ') ?? '')
  const moment = momentLisible(servie.value?.luA ?? null)
  const source = moment ? t('guide-nego.negociations.source-lue', { moment }) : t('guide-nego.negociations.source')
  return [t(`guide-nego.negociations.filtre.${filtre.value}`), zone, source].join(' · ')
})

const vide = computed(() => {
  if (!servie.value) return null
  const e = etatVide(servie.value.sessions, jour.value, suivis.value, maintenant.value, fuseau.value)
  const prochain = e.prochain
    ? t('guide-nego.negociations.vide-miennes.prochain', {
        quand: dayLong(e.prochain.start_at, fuseau.value),
        heure: timeWithZone(e.prochain.start_at, fuseau.value, ville.value ?? undefined),
      })
    : t('guide-nego.negociations.vide-miennes.aucun-prochain')
  const autres = t('guide-nego.negociations.vide-miennes.autres', { count: e.autresDuJour }, e.autresDuJour)
  return {
    titre: jour.value === aujourdhui.value
      ? t('guide-nego.negociations.vide-miennes.titre-aujourdhui')
      : t('guide-nego.negociations.vide-miennes.titre-ce-jour'),
    texte: `${prochain} ${autres}`,
  }
})

const connexionCoupee = computed(() => enLigne.value && coupee.value?.raison === 'unreachable')

useHead({ title: t('guide-nego.negociations.titre') })
</script>

<template>
  <GnEcran :titre="t('guide-nego.negociations.titre')" :ce-qui-se-lit="t('guide-nego.negociations.hors-connexion')">
    <template #entete>
      <header class="gn-sessions__entete">
        <h1 class="gn-sessions__titre">{{ t('guide-nego.negociations.titre') }}</h1>
        <GnBoutonRond picto="filter" :libelle="t('guide-nego.negociations.filtre.libelle')" @clic="feuille = true" />
      </header>
    </template>

    <p v-if="connexionCoupee" class="gn-sessions__injoignable">
      <GnPicto nom="warn" :taille="16" />
      {{ t('guide-nego.negociations.injoignable') }}
    </p>

    <GnChargement
      v-if="attente"
      forme="squelette"
      :lignes="6"
      :libelle="t('guide-nego.negociations.chargement')"
      class="gn-sessions__attente"
    />

    <GnEtatVide
      v-else-if="sansEdition"
      picto="nego"
      :titre="t('guide-nego.negociations.sans-edition.titre')"
      :texte="t('guide-nego.negociations.sans-edition.texte')"
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
        <p class="gn-sessions__jour">{{ j.legende }}</p>
        <GnFriseSession
          v-for="r in j.reunions"
          :key="r.id"
          :reunion="r"
          :fuseau="fuseau"
          :vers="`/guide-nego/negociations/reseau/${r.id}`"
        />
      </template>
    </template>

    <GnEtatErreur
      v-else-if="!servie"
      :titre="t('guide-nego.negociations.erreur.titre')"
      :texte="t(enLigne ? 'guide-nego.negociations.erreur.texte' : 'guide-nego.negociations.erreur.texte-hors-connexion')"
      :sortie="t('guide-nego.negociations.erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide
      v-else-if="!jours.length"
      picto="calendar"
      :titre="t('guide-nego.negociations.vide.titre')"
      :texte="t('guide-nego.negociations.vide.texte')"
    />

    <div v-else class="gn-sessions">
      <GnBandeJours v-model="jour" :jours="jours" :aujourdhui="aujourdhui" class="gn-sessions__bande" />

      <p class="gn-sessions__contexte">{{ contexte }}</p>

      <div v-if="sansThematique" class="gn-sessions__invitation">
        <p>{{ t('guide-nego.negociations.sans-thematique') }}</p>
        <NuxtLink to="/guide-nego/thematiques?depuis=negociations" class="gn-sessions__lien">
          {{ t('guide-nego.negociations.choisir-thematiques') }}
        </NuxtLink>
      </div>

      <div v-if="lignes.length" class="gn-sessions__frise">
        <template v-for="(l, i) in lignes" :key="l.cle">
          <GnFriseMaintenant v-if="maintenantA === i" :heure="heureDeMaintenant" />
          <GnFriseSession
            :session="l.session"
            :reunion="l.reunion"
            :etat="l.etat"
            :fuseau="fuseau"
            :mienne="l.mienne"
            :mon-groupe="l.monGroupe"
            :signale="l.signale"
            :vers="l.vers"
          />
        </template>
        <GnFriseMaintenant v-if="maintenantA === lignes.length" :heure="heureDeMaintenant" />
      </div>

      <div v-else-if="vide" class="gn-sessions__vide">
        <h2 class="gn-sessions__vide-titre">{{ vide.titre }}</h2>
        <p class="gn-sessions__vide-texte">{{ vide.texte }}</p>
        <button type="button" class="gn-sessions__lien" @click="filtre = 'toutes'">
          {{ t('guide-nego.negociations.vide-miennes.voir-toutes') }}
        </button>
        <NuxtLink to="/guide-nego/thematiques?depuis=negociations" class="gn-sessions__lien">
          {{ t('guide-nego.negociations.vide-miennes.modifier') }}
        </NuxtLink>
      </div>

      <NuxtLink :to="{ path: '/guide-nego/negociations/non-annoncee', query: { jour } }" class="gn-sessions__lien">
        <GnPicto nom="flag" :taille="20" />
        {{ t('guide-nego.negociations.non-annoncee') }}
      </NuxtLink>
    </div>

    <a
      v-if="programme && !attente && !coupee"
      :href="programme"
      class="gn-sessions__programme"
      target="_blank"
      rel="noopener"
    >
      <GnPicto nom="external" :taille="18" />
      {{ t('guide-nego.negociations.lien-ccnucc') }}
    </a>

    <GnFeuilleBasse
      v-model="feuille"
      :titre="t('guide-nego.negociations.filtre.libelle')"
      :options="options"
      :fermeture="t('guide-nego.negociations.filtre.fermer')"
      @choisir="choisir"
    />
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-sessions__entete {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-sessions__titre {
  max-width: 11em;
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-28);
  line-height: var(--gn-interligne-28);
  font-weight: var(--gn-graisse-gras);
  letter-spacing: var(--gn-approche-28);
}

[data-app="guide-nego"] .gn-sessions__entete .gn-bouton-rond {
  flex: none;
}

[data-app="guide-nego"] .gn-sessions {
  padding-top: 18px;
  display: flex;
  flex-direction: column;
  gap: 18px;
}

[data-app="guide-nego"] .gn-sessions__injoignable {
  padding-top: var(--gn-espace-12);
  display: flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--gn-danger);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-sessions__attente {
  padding-top: 18px;
}

/* La bande sort de la marge de droite : elle défile jusqu'au bord de l'écran. */
[data-app="guide-nego"] .gn-sessions__bande {
  margin-right: calc(-1 * var(--gn-marge-ecran));
}

[data-app="guide-nego"] .gn-sessions__contexte {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
}

[data-app="guide-nego"] .gn-sessions__invitation {
  padding: 14px var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-4);
  background: var(--gn-fond-2);
  border-radius: var(--gn-rayon-20);
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-sessions__frise {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-sessions__jour {
  padding-top: var(--gn-espace-12);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-sessions__jour::first-letter {
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-sessions__vide {
  padding: 18px var(--gn-espace-16);
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  background: var(--gn-fond-2);
  border-radius: var(--gn-rayon-24);
}

[data-app="guide-nego"] .gn-sessions__vide-titre {
  font-family: var(--gn-police-titre);
  font-size: var(--gn-taille-16);
  line-height: var(--gn-interligne-16);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-sessions__vide-texte {
  color: var(--gn-texte-lecture);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-sessions__lien {
  width: fit-content;
  min-height: var(--gn-cible);
  padding: 0;
  display: flex;
  align-items: center;
  gap: 6px;
  border: none;
  background: none;
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: none;
  cursor: pointer;
}

[data-app="guide-nego"] .gn-sessions__programme {
  min-height: var(--gn-cible);
  margin-block: var(--gn-espace-16);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-14);
  line-height: var(--gn-interligne-14);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>
