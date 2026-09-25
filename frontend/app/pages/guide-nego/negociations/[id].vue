<script setup lang="ts">
import type { OfficialSession } from '~/types/negotiation-sessions'
import { dayKeyInZone } from '~/utils/datetime'
import { estDeMonGroupe, etatAffiche } from '~/utils/guide-nego/sessions'

/**
 * Écran 08 — la fiche d'une session officielle, lue dans la liste gardée : elle s'ouvre
 * sans réseau avec ce qui a été lu. La source coupée ne laisse voir aucune fiche
 * (FR-039) : l'heure et la salle de ce matin ont pu changer.
 *
 * Aucune section de documents : la source officielle n'en lie aucun (FR-029).
 */
definePageMeta({ layout: 'guide-nego' })
defineI18nRoute(false)

const { t, locale } = useI18n()
const { dayLong, time, zoneLabel, zoneOffsetShort } = useDateTime()
const { tr } = useI18nText()
const { momentLisible } = useGnMomentLecture()
const route = useRoute()
const connexion = useGnConnexion()
const compte = useGnSession()
const thematiques = useGnThematiques()
const groupes = useGnGroupes()
const agenda = useGnAgenda()
const lecture = useGnSessions()
const { affichage, etat, sansEdition } = lecture

const k = (cle: string, params: Record<string, unknown> = {}) => t(`guide-nego.negociations-fiche.${cle}`, params)

const id = computed(() => String(route.params.id ?? ''))
const retour = computed(() =>
  route.query.depuis === 'agenda' ? '/guide-nego/negociations/agenda' : '/guide-nego/negociations',
)

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
  await compte.assurer()
  agenda.assurer()
  await Promise.all([thematiques.assurerLeVocabulaire(), groupes.assurer()])
})
onBeforeUnmount(() => clearInterval(horloge))

const enLigne = computed(() => connexion.etat.value.enLigne)
const servie = computed(() => (affichage.value.etat === 'sert' ? affichage.value : null))
const coupee = computed(() => (affichage.value.etat === 'coupe' ? affichage.value : null))
const attente = computed(() => !etat.value.pret && !tentee.value)
const session = computed<OfficialSession | null>(() => lecture.session(id.value))
const fuseau = computed(() => servie.value?.fuseau ?? 'UTC')
const ville = computed(() => servie.value?.ville ?? undefined)

const etatSession = computed(() => (session.value ? etatAffiche(session.value, maintenant.value, fuseau.value) : null))
const annulee = computed(() => etatSession.value === 'annulee')

/** En anglais, la traduction française n'a rien à faire à l'écran. */
const traduit = computed(() => (locale.value === 'fr' ? (session.value?.title_fr ?? null) : null))
const titre = computed(() => traduit.value ?? session.value?.title_en ?? k('titre'))
const type = computed(() => (session.value?.type ? tr(session.value.type.label) : null))
const fil = computed(() => (type.value ? k('fil', { type: type.value }) : k('fil-sans-type')))

function plage(debut: string, fin: string | null): string {
  const d = time(debut, fuseau.value)
  return fin ? k('plage', { debut: d, fin: time(fin, fuseau.value) }) : d
}

const heure = computed(() => {
  const s = session.value
  if (!s) return null
  const valeur = plage(s.start_at, s.end_at)
  const p = s.previous
  let avant: string | null = null
  if (p && (p.start_at !== s.start_at || p.end_at !== s.end_at)) {
    avant = plage(p.start_at, p.end_at)
    // Changée de jour, l'heure seule tromperait.
    if (dayKeyInZone(p.start_at, fuseau.value) !== dayKeyInZone(s.start_at, fuseau.value)) {
      avant = k('plage-jour', { jour: dayLong(p.start_at, fuseau.value), plage: avant })
    }
  }
  const precision = k('heure-precision', {
    jour: dayLong(s.start_at, fuseau.value),
    zone: zoneLabel(fuseau.value, ville.value),
    decalage: zoneOffsetShort(fuseau.value, s.start_at),
  })
  return { valeur, avant, precision }
})

const salle = computed(() => {
  const s = session.value
  if (!s) return null
  const avant = s.previous?.venue && s.previous.venue !== s.venue ? s.previous.venue : null
  return { valeur: s.venue ?? k('salle-inconnue'), avant }
})

const ordreDuJour = computed(() => {
  const point = session.value?.agenda_item
  return point ? k('ordre-du-jour', { code: point.code, intitule: point.title }) : k('hors-ordre-du-jour')
})

const rattachement = computed(() => {
  const s = session.value
  if (!s) return ''
  if (s.group) {
    const groupe = tr(s.group.label)
    return estDeMonGroupe(s, groupes.mesCodes.value) ? k('mon-groupe', { groupe }) : groupe
  }
  return (s.theme ? thematiques.nomDe(s.theme) : null) ?? k('sans-thematique')
})

const luA = computed(() => momentLisible(servie.value?.luA ?? null))

// --- Agenda et rappel -------------------------------------------------------

const dansLAgenda = computed(() => agenda.dansLAgenda(id.value))
const rappel = computed<boolean>({
  get: () => agenda.entree(id.value)?.remind ?? false,
  set: (voulu) => void agenda.rappeler(id.value, voulu),
})
const detailDuRappel = computed(() => {
  if (annulee.value) return k('rappel.annulee')
  return dansLAgenda.value ? undefined : k('rappel.hors-agenda')
})

const feuilleCompte = ref(false)
const message = ref<{ texte: string; action?: string; rang: number } | null>(null)
const annoncer = (texte: string, action?: string) =>
  (message.value = { texte, action, rang: (message.value?.rang ?? 0) + 1 })
let rappelAvantRetrait = false

async function basculerAgenda(): Promise<void> {
  if (!compte.connectee.value) {
    feuilleCompte.value = true
    return
  }
  if (dansLAgenda.value) {
    rappelAvantRetrait = rappel.value
    await agenda.retirer(id.value)
    annoncer(k('agenda.retiree'), k('agenda.annuler'))
  } else {
    await agenda.ajouter(id.value)
    annoncer(k('agenda.ajoutee'))
  }
}

function annulerLeRetrait(): void {
  message.value = null
  void agenda.ajouter(id.value, rappelAvantRetrait)
}

useHead({ title: titre })
</script>

<template>
  <GnEcran
    :titre="titre"
    :sous-titre="session ? fil : undefined"
    :retour="retour"
    :onglets="false"
    :ce-qui-se-lit="k('hors-connexion')"
  >
    <template #connexion>
      <GnLigneConnexion :en-ligne="enLigne" :lu-a="etat.luA" />
    </template>

    <GnChargement v-if="attente" forme="squelette" :lignes="6" :libelle="k('chargement')" class="gn-session__attente" />

    <GnEtatVide
      v-else-if="sansEdition"
      picto="nego"
      :titre="k('sans-edition.titre')"
      :texte="k('sans-edition.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnLectureImpossible
      v-else-if="coupee"
      :raison="coupee.raison"
      :depuis="coupee.depuis"
      :programme="coupee.programme"
      :en-cours="relecture"
      @reessayer="relire()"
    />

    <GnEtatVide
      v-else-if="!servie && !enLigne"
      picto="wifi-off"
      :titre="k('jamais-lue.titre')"
      :texte="k('jamais-lue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <GnEtatErreur
      v-else-if="!servie"
      :titre="k('erreur.titre')"
      :texte="k('erreur.texte')"
      :sortie="k('erreur.reessayer')"
      @sortie="relire()"
    />

    <GnEtatVide
      v-else-if="!session"
      picto="nego"
      :titre="k('inconnue.titre')"
      :texte="k('inconnue.texte')"
      :sortie="k('sortie')"
      :sortie-vers="retour"
    />

    <div v-else class="gn-session" :class="{ 'gn-session--annulee': annulee }">
      <div class="gn-session__tete">
        <template v-if="traduit">
          <p class="gn-session__anglais" lang="en">
            <abbr class="gn-session__en" :title="k('anglais')">EN</abbr>
            {{ session.title_en }}
          </p>
          <p class="gn-session__traduction">
            <GnPicto nom="translate" :taille="16" />
            {{ k('traduction') }}
          </p>
        </template>
        <div class="gn-session__etat">
          <GnEtatSession v-if="etatSession" :session="session" :etat="etatSession" :fuseau="fuseau" />
        </div>
      </div>

      <div class="gn-session__corps">
        <h2 class="gn-session__intertitre">
          <span>{{ k('source.titre') }}</span>
          <span v-if="luA" class="gn-session__lu">{{ k('source.lu', { moment: luA }) }}</span>
        </h2>

        <dl class="gn-session__lignes">
          <div v-if="heure" class="gn-session__ligne">
            <dt>{{ k('cle.heure') }}</dt>
            <dd>
              <GnValeurChangee
                :valeur="heure.valeur"
                :avant="heure.avant"
                :precision="heure.precision"
                :eteinte="annulee"
                :maintenant="etatSession === 'en-cours'"
              />
            </dd>
          </div>
          <div v-if="salle" class="gn-session__ligne">
            <dt>{{ k('cle.salle') }}</dt>
            <dd><GnValeurChangee :valeur="salle.valeur" :avant="salle.avant" forte :eteinte="annulee" /></dd>
          </div>
          <div v-if="session.type" class="gn-session__ligne">
            <dt>{{ k('cle.type') }}</dt>
            <dd class="gn-session__valeur">
              {{ type }}<template v-if="session.type.term_en">
                —
                <NuxtLink
                  :to="{ path: '/guide-nego/lexique', query: { terme: session.type.term_en } }"
                  class="gn-session__terme"
                  lang="en"
                  :aria-label="k('terme', { terme: session.type.term_en })"
                >{{ session.type.term_en }}</NuxtLink>
              </template>
            </dd>
          </div>
          <div class="gn-session__ligne">
            <dt>{{ k('cle.ordre-du-jour') }}</dt>
            <dd class="gn-session__valeur">{{ ordreDuJour }}</dd>
          </div>
          <div v-if="session.open_access !== null" class="gn-session__ligne">
            <dt>{{ k('cle.acces') }}</dt>
            <dd><GnMarqueEtat :etat="session.open_access ? 'ouverte' : 'acces-limite'" /></dd>
          </div>
          <div class="gn-session__ligne">
            <dt>{{ k('cle.thematique') }}</dt>
            <dd class="gn-session__valeur">{{ rattachement }}</dd>
          </div>
        </dl>

        <p class="gn-session__origine">
          <GnPicto nom="check-circle" :taille="18" />
          {{ luA ? k('origine-lue', { moment: luA }) : k('origine') }}
        </p>
        <a v-if="session.source_url" :href="session.source_url" class="gn-session__original" target="_blank" rel="noopener">
          <GnPicto nom="external" :taille="20" />
          {{ k('original') }}
        </a>

        <div class="gn-session__rappel">
          <GnPicto nom="bell" class="gn-session__cloche" />
          <GnInterrupteur
            v-model="rappel"
            :libelle="k('rappel.libelle')"
            :detail="detailDuRappel"
            :desactive="!dansLAgenda || annulee"
            derniere
          />
        </div>
        <p class="gn-session__phrase">{{ k('rappel.phrase') }}</p>
      </div>

      <div class="gn-session__actions">
        <GnBouton
          v-if="dansLAgenda"
          variante="secondaire"
          picto="check"
          actif
          @clic="basculerAgenda"
        >
          {{ k('agenda.dedans') }}
        </GnBouton>
        <GnBouton
          v-else
          picto="calendar"
          :desactive="annulee"
          :chargement="compte.connectee.value && !agenda.pret.value"
          @clic="basculerAgenda"
        >
          {{ k('agenda.ajouter') }}
        </GnBouton>
      </div>
    </div>

    <GnFeuilleBasse v-model="feuilleCompte" :titre="k('compte.titre')" :sous-titre="k('compte.texte')">
      <div class="gn-session__sorties-compte">
        <GnBouton vers="/guide-nego/compte" picto="user">{{ k('compte.creer') }}</GnBouton>
        <GnBouton variante="secondaire" vers="/guide-nego/connexion">{{ k('compte.connexion') }}</GnBouton>
      </div>
    </GnFeuilleBasse>

    <GnMessageEphemere
      v-if="message"
      :key="message.rang"
      :texte="message.texte"
      :action="message.action"
      @agir="annulerLeRetrait"
      @fini="message = null"
    />
  </GnEcran>
</template>

<style>
/* Titre barré : l'annulée reste lisible, mais ne se confond pas avec une session qui tient. */
[data-app="guide-nego"] .gn-ecran:has(.gn-session--annulee) .gn-entete__titre {
  text-decoration: line-through;
  text-decoration-thickness: 2px;
}

[data-app="guide-nego"] .gn-ecran__contenu:has(> .gn-session) {
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-session {
  flex: 1;
  display: flex;
  flex-direction: column;
}

[data-app="guide-nego"] .gn-session__attente {
  padding-top: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-session__tete {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding-top: var(--gn-espace-12);
}

[data-app="guide-nego"] .gn-session__anglais,
[data-app="guide-nego"] .gn-session__traduction {
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  overflow-wrap: anywhere;
}

[data-app="guide-nego"] .gn-session__en {
  font-weight: var(--gn-graisse-demi-gras);
  text-decoration: none;
}

[data-app="guide-nego"] .gn-session__traduction {
  display: flex;
  align-items: center;
  gap: 6px;
}

[data-app="guide-nego"] .gn-session__etat .gn-marque-etat {
  font-size: var(--gn-taille-17);
  line-height: var(--gn-interligne-17);
}

[data-app="guide-nego"] .gn-session__corps {
  flex: 1;
  display: flex;
  flex-direction: column;
  padding-bottom: var(--gn-espace-16);
}

[data-app="guide-nego"] .gn-session__intertitre {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: var(--gn-espace-12);
  padding: var(--gn-espace-16) 0 6px;
  border-bottom: var(--gn-filet-3) solid var(--gn-filet-fort);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
}

[data-app="guide-nego"] .gn-session__intertitre > span:first-child {
  letter-spacing: 0.02em;
  text-transform: uppercase;
}

[data-app="guide-nego"] .gn-session__lu {
  font-weight: var(--gn-graisse-regulier);
}

[data-app="guide-nego"] .gn-session__ligne {
  display: flex;
  justify-content: space-between;
  gap: var(--gn-espace-12);
  padding-block: 10px;
  border-bottom: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-session__ligne:last-child {
  border-bottom: none;
}

[data-app="guide-nego"] .gn-session__ligne dt {
  flex: none;
  padding-top: 2px;
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-session__ligne dd {
  min-width: 0;
  display: flex;
  justify-content: flex-end;
}

[data-app="guide-nego"] .gn-session__ligne dd.gn-session__valeur {
  display: block;
  font-weight: var(--gn-graisse-demi-gras);
  text-align: end;
  overflow-wrap: anywhere;
}

/* Le terme anglais ouvre le lexique : la ligne reste basse, la cible déborde en hauteur. */
[data-app="guide-nego"] .gn-session__terme {
  padding-block: 12px;
  color: inherit;
  font-style: italic;
  text-decoration: underline;
  text-decoration-thickness: 2px;
  text-decoration-color: var(--gn-accent);
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-session__origine {
  align-self: flex-start;
  margin-top: var(--gn-espace-16);
  padding: 6px 10px;
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: var(--gn-filet-1) solid var(--gn-filet-fort);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-session__origine .gn-picto {
  color: var(--gn-etat-source-officielle);
}

[data-app="guide-nego"] .gn-session__original {
  width: fit-content;
  min-height: var(--gn-cible);
  display: flex;
  align-items: center;
  gap: 6px;
  color: var(--gn-accent);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
  font-weight: var(--gn-graisse-gras);
  text-decoration: underline;
  text-underline-offset: 4px;
}

[data-app="guide-nego"] .gn-session__rappel {
  margin-top: var(--gn-espace-8);
  display: flex;
  align-items: center;
  gap: var(--gn-espace-12);
  border-block: var(--gn-filet-1) solid var(--gn-filet);
}

[data-app="guide-nego"] .gn-session__rappel .gn-interrupteur {
  flex: 1;
  min-width: 0;
}

[data-app="guide-nego"] .gn-session__cloche {
  flex: none;
  color: var(--gn-titre);
}

[data-app="guide-nego"] .gn-session__phrase {
  padding-top: var(--gn-espace-8);
  color: var(--gn-texte-2);
  font-size: var(--gn-taille-15);
  line-height: var(--gn-interligne-15);
}

[data-app="guide-nego"] .gn-session__actions {
  position: sticky;
  bottom: 0;
  margin-inline: calc(-1 * var(--gn-marge-ecran));
  padding: var(--gn-espace-16) var(--gn-marge-ecran) calc(var(--gn-espace-16) + env(safe-area-inset-bottom));
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
  border-top: var(--gn-filet-1) solid var(--gn-filet);
  background: var(--gn-fond);
}

[data-app="guide-nego"] .gn-session__sorties-compte {
  display: flex;
  flex-direction: column;
  gap: var(--gn-espace-8);
}
</style>
