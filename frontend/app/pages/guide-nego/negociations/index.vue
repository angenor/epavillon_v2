<script setup lang="ts">
import { dayKeyInZone } from '~/utils/datetime'
import {
  estDeMonGroupe,
  etatAffiche,
  etatVide,
  filtrer,
  jourAOuvrir,
  joursDeLaBande,
  sessionsDuJour,
  type Suivis,
} from '~/utils/guide-nego/sessions'

/**
 * Écran 07 — les sessions de négociation officielles, un jour à la fois. Rien d'autre :
 * ni les Réunions de la Francophonie, ni le Pavillon (ADR-008).
 *
 * **Coupée, la source se montre coupée** (FR-039, principe XII) : « Lecture impossible »
 * ou « Affichage suspendu », et aucune session à côté — jamais « aucune session ».
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

type Filtre = 'miennes' | 'toutes'

const { t } = useI18n()
const { dayLong, zoneOf, timeWithZone } = useDateTime()
const { momentLisible } = useGnMomentLecture()
const connexion = useGnConnexion()
const session = useGnSession()
const thematiques = useGnThematiques()
const groupes = useGnGroupes()
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

const sousTitre = computed(() => {
  const e = edition.value
  if (!e) return undefined
  return e.city ? t('guide-nego.negociations.sous-titre', { edition: e.libelle, ville: e.city }) : e.libelle
})

const attente = computed(() => !etat.value.pret && !tentee.value)

// Le jour et le filtre survivent à l'aller-retour vers une fiche.
const jourChoisi = useState<string | null>('gn-sessions-jour', () => null)
const filtreChoisi = useState<Filtre | null>('gn-sessions-filtre', () => null)

const aujourdhui = computed(() => dayKeyInZone(maintenant.value, fuseau.value))
const jours = computed(() => (servie.value ? joursDeLaBande(servie.value.sessions, fuseau.value) : []))
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
const onglets = computed(() => [
  { valeur: 'miennes', libelle: t('guide-nego.negociations.filtre.miennes') },
  { valeur: 'toutes', libelle: t('guide-nego.negociations.filtre.toutes') },
])
const filtreModele = computed<string>({
  get: () => filtre.value,
  set: (valeur) => (filtre.value = valeur === 'toutes' ? 'toutes' : 'miennes'),
})

const duJour = computed(() => (servie.value ? sessionsDuJour(servie.value.sessions, jour.value, fuseau.value) : []))
const affichees = computed(() => filtrer(duJour.value, filtre.value, suivis.value))

const compteur = computed(() => {
  const n = affichees.value.length
  if (n === 0 && duJour.value.length > 0) return t('guide-nego.negociations.compteur-sur', { n, total: duJour.value.length })
  return t('guide-nego.negociations.compteur', { count: n }, n)
})

const lignes = computed(() =>
  affichees.value.map((s) => ({
    session: s,
    etat: etatAffiche(s, maintenant.value, fuseau.value),
    thematique: s.theme ? (thematiques.nomDe(s.theme) ?? null) : null,
    monGroupe: estDeMonGroupe(s, suivis.value.groupes),
  })),
)

const legendeDuJour = computed(() => {
  if (!jour.value) return ''
  const zone = ville.value ?? fuseau.value.split('/').pop()?.replace(/_/g, ' ') ?? ''
  return t('guide-nego.negociations.jour', { jour: dayLong(`${jour.value}T12:00:00Z`, 'UTC'), zone: zoneOf(zone) })
})

const lueA = computed(() => {
  const moment = momentLisible(servie.value?.luA ?? null)
  return moment ? t('guide-nego.negociations.source-lue', { moment }) : t('guide-nego.negociations.source')
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
  <GnEcran
    :titre="t('guide-nego.negociations.titre')"
    :sous-titre="sousTitre"
    :ce-qui-se-lit="t('guide-nego.negociations.hors-connexion')"
  >
    <template #connexion>
      <span v-if="connexionCoupee" class="gn-sessions__injoignable">
        <GnPicto nom="warn" :taille="16" />
        {{ t('guide-nego.negociations.injoignable') }}
      </span>
      <GnLigneConnexion v-else :en-ligne="enLigne" :lu-a="etat.luA" />
    </template>
    <template #action>
      <NuxtLink to="/guide-nego/negociations/agenda" class="gn-sessions__agenda">
        <GnPicto nom="calendar" :taille="20" />
        {{ t('guide-nego.negociations.mon-agenda') }}
      </NuxtLink>
    </template>

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

    <GnLectureImpossible
      v-else-if="coupee"
      :raison="coupee.raison"
      :depuis="coupee.depuis"
      :programme="coupee.programme"
      :en-cours="relecture"
      @reessayer="relire()"
    />

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

    <template v-else>
      <GnBandeJours v-model="jour" :jours="jours" :aujourdhui="aujourdhui" class="gn-sessions__bande" />

      <div class="gn-sessions__filtre">
        <GnOngletsFiltre
          v-if="!sansThematique"
          v-model="filtreModele"
          :onglets="onglets"
          :libelle="t('guide-nego.negociations.filtre.libelle')"
        />
        <span class="gn-sessions__compteur" role="status">{{ compteur }}</span>
      </div>

      <div v-if="sansThematique" class="gn-sessions__invitation">
        <GnPicto nom="info" :taille="20" />
        <p>
          {{ t('guide-nego.negociations.sans-thematique') }}
          <NuxtLink to="/guide-nego/thematiques?depuis=negociations" class="gn-sessions__invitation-lien">
            {{ t('guide-nego.negociations.choisir-thematiques') }}
          </NuxtLink>
        </p>
      </div>

      <p class="gn-sessions__source">
        <GnPicto nom="check-circle" :taille="18" />
        {{ lueA }}
      </p>

      <p class="gn-sessions__jour">{{ legendeDuJour }}</p>

      <div v-if="lignes.length" class="gn-sessions__liste">
        <GnLigneSession
          v-for="l in lignes"
          :key="l.session.id"
          :session="l.session"
          :etat="l.etat"
          :fuseau="fuseau"
          :ville="ville"
          :thematique="l.thematique"
          :mon-groupe="l.monGroupe"
          :vers="`/guide-nego/negociations/${l.session.id}`"
        />
      </div>

      <div v-else-if="vide" class="gn-sessions__vide">
        <GnEtatVide picto="calendar" :titre="vide.titre" :texte="vide.texte" />
        <GnBouton variante="discret" @clic="filtre = 'toutes'">
          {{ t('guide-nego.negociations.vide-miennes.voir-toutes') }}
        </GnBouton>
        <GnBouton variante="discret" vers="/guide-nego/thematiques?depuis=negociations">
          {{ t('guide-nego.negociations.vide-miennes.modifier') }}
        </GnBouton>
      </div>
    </template>

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
  </GnEcran>
</template>

<style>
[data-app="guide-nego"] .gn-sessions__injoignable {
  display: inline-flex;
  align-items: flex-start;
  gap: 6px;
  color: var(--gn-danger);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-sessions__injoignable .gn-picto {
  margin-block-start: 3px;
}

[data-app="guide-nego"] .gn-sessions__agenda {
  flex: none;
  min-height: var(--gn-cible);
  padding-inline: var(--gn-espace-8);
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-sessions__agenda:active {
  background: var(--gn-presse);
}

[data-app="guide-nego"] .gn-sessions__attente {
  padding-top: var(--gn-espace-16);
}

/* La bande sort des marges : elle défile jusqu'au bord de l'écran. */
[data-app="guide-nego"] .gn-sessions__bande {
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding-inline: var(--gn-espace-8);
}

[data-app="guide-nego"] .gn-sessions__filtre {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  min-height: var(--gn-onglet-filtre-hauteur);
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
}

[data-app="guide-nego"] .gn-sessions__compteur {
  flex: none;
  margin-inline-start: auto;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-13);
  line-height: var(--gn-interligne-13);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-sessions__invitation {
  margin-top: var(--gn-espace-12);
  display: flex;
  align-items: flex-start;
  gap: var(--gn-espace-8);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  color: var(--gn-texte);
}

[data-app="guide-nego"] .gn-sessions__invitation .gn-picto {
  color: var(--gn-information);
}

[data-app="guide-nego"] .gn-sessions__invitation-lien {
  width: fit-content;
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  color: var(--gn-accent);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-sessions__source {
  margin-top: var(--gn-espace-12);
  padding: var(--gn-espace-8) var(--gn-espace-12);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-8);
  border: var(--gn-filet-1) solid var(--gn-filet-fort);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-sessions__source .gn-picto {
  color: var(--gn-etat-source-officielle);
}

[data-app="guide-nego"] .gn-sessions__jour {
  padding-top: var(--gn-espace-12);
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-demi-gras);
}

[data-app="guide-nego"] .gn-sessions__jour::first-letter {
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-sessions__vide {
  display: flex;
  flex-direction: column;
  align-items: center;
  text-align: center;
}

[data-app="guide-nego"] .gn-sessions__vide .gn-vide {
  align-items: center;
}

[data-app="guide-nego"] .gn-sessions__programme {
  min-height: var(--gn-cible);
  margin-block: var(--gn-espace-16);
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  text-decoration: underline;
  text-underline-offset: 4px;
}
</style>
